//! ↔ squirrel/sqstate.h + sqstate.cpp
//!
//! SQSharedState：字符串表、metamethod 名表、registry/consts、11 张默认委托表、
//! RefTable、GC。C++ 的 GC 用侵入式双链 + MARK_FLAG 复用引用计数；
//! Rust 改为「Weak 注册表 + 从根 BFS 标记」，语义等价（详见 collect_garbage）。

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use crate::array::SQArray;
use crate::class::{SQClass, SQInstance};
use crate::closure::{SQClosure, SQGenerator, SQNativeClosure};
use crate::config::*;
use crate::object::Value;
use crate::table::SQTable;
use crate::userdata::SQUserData;
use crate::funcproto::SQFunctionProto;
use crate::vm::VmData;

/// ↔ SQMetaMethod
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum MetaMethod {
    Add = 0,
    Sub = 1,
    Mul = 2,
    Div = 3,
    Unm = 4,
    Modulo = 5,
    Set = 6,
    Get = 7,
    Typeof = 8,
    Nexti = 9,
    Cmp = 10,
    Call = 11,
    Cloned = 12,
    NewSlot = 13,
    DelSlot = 14,
    ToString = 15,
    NewMember = 16,
    Inherited = 17,
}

pub const MM_NAMES: [&str; 18] = [
    "_add", "_sub", "_mul", "_div", "_unm", "_modulo", "_set", "_get", "_typeof", "_nexti", "_cmp",
    "_call", "_cloned", "_newslot", "_delslot", "_tostring", "_newmember", "_inherited",
];

/// ↔ SQCOMPILERERROR
pub type CompilerErrorHandler = Rc<dyn Fn(&VmData, &str, &str, SQInteger, SQInteger)>;
/// ↔ SQPRINTFUNCTION（宿主已格式化好的文本）
///
/// 参数是**字节切片**而不是 `&str`：参考实现是非 UNICODE 的 `char` 构建，
/// `print((255).tochar())` 写出的是裸字节 0xFF；如果这里用 `&str`，
/// 所有宿主都被迫走 `from_utf8_lossy`，非法 UTF-8 会被换成 U+FFFD（实测差异）。
pub type PrintFn = Rc<dyn Fn(&VmData, &[u8])>;
/// ↔ SQDEBUGHOOK
pub type DebugHookFn = Rc<dyn Fn(&VmData, SQInteger, &str, SQInteger, &str)>;

/// ↔ SQStringTable：字符串实习表（C++ 的桶数组不可观测，用 HashMap + Weak）
///
/// **已死条目的键必须清掉**：C++ 侧 `SQString` 引用计数归零时会被
/// `SQStringTable::Remove` 摘除；本实现的值是 `Weak`（字符串对象本身由 `Rc`
/// 即时释放 ✓），但 HashMap 的**键**（字符串字节）会一直留着——实测
/// `local s = ""; for (i<20000) s += "x";` 峰值 232MB（正好是所有中间串长度之和），
/// 而参考实现只要 3MB。所以按"插入字节数"计数，超过预算就清一遍死条目。
pub struct StringTable {
    map: RefCell<HashMap<Box<[u8]>, Weak<crate::object::SQString>>>,
    /// 上次清扫以来插入的键字节数
    dirty: Cell<usize>,
    /// 上次清扫以来插入的条目数（短串多的时候按字节算永远到不了预算）
    dirty_count: Cell<usize>,
}

/// 字符串表清扫预算（插入这么多字节 **或** 这么多条目就清一次死条目）
const STRINGTABLE_SWEEP_BYTES: usize = 4 << 20;
const STRINGTABLE_SWEEP_ENTRIES: usize = 4096;

impl StringTable {
    pub fn new() -> Self {
        StringTable {
            map: RefCell::new(HashMap::new()),
            dirty: Cell::new(0),
            dirty_count: Cell::new(0),
        }
    }

    /// 实习表当前条目数（含弱引用已死的条目，供测试断言内存有界）
    pub fn len(&self) -> usize {
        self.map.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// ↔ SQStringTable::Add
    pub fn add(&self, s: &[u8]) -> Rc<crate::object::SQString> {
        let mut map = self.map.borrow_mut();
        if let Some(w) = map.get(s) {
            if let Some(rc) = w.upgrade() {
                return rc;
            }
        }
        let rc = Rc::new(crate::object::SQString {
            hash: crate::object::hashstr(s),
            val: s.into(),
            weakref_slot: std::cell::RefCell::new(None),
        });
        map.insert(s.into(), Rc::downgrade(&rc));
        let d = self.dirty.get() + s.len() + 32;
        let c = self.dirty_count.get() + 1;
        if d >= STRINGTABLE_SWEEP_BYTES || c >= STRINGTABLE_SWEEP_ENTRIES {
            map.retain(|_, w| w.strong_count() > 0);
            self.dirty.set(0);
            self.dirty_count.set(0);
        } else {
            self.dirty.set(d);
            self.dirty_count.set(c);
        }
        rc
    }
}

/// ↔ RefTable：sq_addref/sq_release 的对象注册表（GC 根之一）
#[derive(Default)]
pub struct RefTable {
    inner: RefCell<HashMap<usize, (Value, SQUnsignedInteger)>>,
}

impl RefTable {
    pub fn add_ref(&self, obj: &Value) {
        let mut inner = self.inner.borrow_mut();
        let e = inner.entry(obj.identity()).or_insert_with(|| (obj.clone(), 0));
        e.1 += 1;
    }

    /// ↔ RefTable::Release：返回 true 表示引用已彻底移除
    pub fn release(&self, obj: &Value) -> bool {
        let mut inner = self.inner.borrow_mut();
        if let Some((_, cnt)) = inner.get_mut(&obj.identity()) {
            *cnt -= 1;
            if *cnt == 0 {
                inner.remove(&obj.identity());
                return true;
            }
        }
        false
    }

    pub fn get_ref_count(&self, obj: &Value) -> SQUnsignedInteger {
        self.inner
            .borrow()
            .get(&obj.identity())
            .map(|(_, c)| *c)
            .unwrap_or(0)
    }

    fn roots(&self) -> Vec<Value> {
        self.inner.borrow().values().map(|(v, _)| v.clone()).collect()
    }

    /// ↔ RefTable::Finalize
    pub fn finalize(&self) {
        self.inner.borrow_mut().clear();
    }
}

/// GC 注册表项（↔ SQCollectable 挂入 _gc_chain）
pub enum GcEntry {
    Table(Weak<RefCell<SQTable>>),
    Array(Weak<RefCell<SQArray>>),
    Userdata(Weak<RefCell<SQUserData>>),
    Closure(Weak<RefCell<SQClosure>>),
    NativeClosure(Weak<RefCell<SQNativeClosure>>),
    Generator(Weak<RefCell<SQGenerator>>),
    Class(Weak<RefCell<SQClass>>),
    Instance(Weak<RefCell<SQInstance>>),
    Thread(Weak<RefCell<VmData>>),
    Outer(Weak<RefCell<crate::closure::SQOuter>>),
    FuncProto(Weak<SQFunctionProto>),
}

impl GcEntry {

    fn value(&self) -> Option<Value> {
        match self {
            GcEntry::Table(w) => w.upgrade().map(Value::Table),
            GcEntry::Array(w) => w.upgrade().map(Value::Array),
            GcEntry::Userdata(w) => w.upgrade().map(Value::Userdata),
            GcEntry::Closure(w) => w.upgrade().map(Value::Closure),
            GcEntry::NativeClosure(w) => w.upgrade().map(Value::NativeClosure),
            GcEntry::Generator(w) => w.upgrade().map(Value::Generator),
            GcEntry::Class(w) => w.upgrade().map(Value::Class),
            GcEntry::Instance(w) => w.upgrade().map(Value::Instance),
            GcEntry::Thread(w) => w.upgrade().map(Value::Thread),
            GcEntry::Outer(w) => w.upgrade().map(Value::Outer),
            GcEntry::FuncProto(w) => w.upgrade().map(Value::FuncProto),
        }
    }
}

/// ↔ SQSharedState
pub struct SharedState {
    pub metamethods: RefCell<Vec<Value>>,
    pub metamethodsmap: RefCell<Value>, // Table: name -> idx
    pub systemstrings: RefCell<Vec<Value>>,
    pub types: RefCell<Vec<Value>>,
    pub stringtable: StringTable,
    pub refs_table: RefTable,
    pub registry: RefCell<Value>,   // Table
    pub consts: RefCell<Value>,     // Table
    pub constructoridx: RefCell<Value>, // String "constructor"
    pub gc_chain: RefCell<Vec<GcEntry>>,
    /// 注册表长度到达该值就清一遍"弱引用已死"的条目（见 `gc_add`）
    pub gc_prune_at: std::cell::Cell<usize>,
    /// ↔ _root_vm：根 VM 的 weakref（sq_open 时设置）
    pub root_vm: RefCell<Value>,
    pub table_default_delegate: RefCell<Value>,
    pub array_default_delegate: RefCell<Value>,
    pub string_default_delegate: RefCell<Value>,
    pub number_default_delegate: RefCell<Value>,
    pub generator_default_delegate: RefCell<Value>,
    pub closure_default_delegate: RefCell<Value>,
    pub thread_default_delegate: RefCell<Value>,
    pub class_default_delegate: RefCell<Value>,
    pub instance_default_delegate: RefCell<Value>,
    pub weakref_default_delegate: RefCell<Value>,

    pub compilererrorhandler: RefCell<Option<CompilerErrorHandler>>,
    pub printfunc: RefCell<Option<PrintFn>>,
    pub errorfunc: RefCell<Option<PrintFn>>,
    pub debuginfo: std::cell::Cell<bool>,
    pub notifyallexceptions: std::cell::Cell<bool>,
    pub foreignptr: std::cell::Cell<SQUserPointer>,
    pub releasehook: RefCell<Option<crate::userdata::ReleaseHook>>,
}

impl SharedState {
    pub fn new() -> Rc<SharedState> {
        let ss = Rc::new(SharedState {
            metamethods: RefCell::new(Vec::new()),
            metamethodsmap: RefCell::new(Value::Null),
            systemstrings: RefCell::new(Vec::new()),
            types: RefCell::new(Vec::new()),
            stringtable: StringTable::new(),
            refs_table: RefTable::default(),
            registry: RefCell::new(Value::Null),
            consts: RefCell::new(Value::Null),
            constructoridx: RefCell::new(Value::Null),
            gc_chain: RefCell::new(Vec::new()),
        gc_prune_at: std::cell::Cell::new(65536),
            root_vm: RefCell::new(Value::Null),
            table_default_delegate: RefCell::new(Value::Null),
            array_default_delegate: RefCell::new(Value::Null),
            string_default_delegate: RefCell::new(Value::Null),
            number_default_delegate: RefCell::new(Value::Null),
            generator_default_delegate: RefCell::new(Value::Null),
            closure_default_delegate: RefCell::new(Value::Null),
            thread_default_delegate: RefCell::new(Value::Null),
            class_default_delegate: RefCell::new(Value::Null),
            instance_default_delegate: RefCell::new(Value::Null),
            weakref_default_delegate: RefCell::new(Value::Null),
            compilererrorhandler: RefCell::new(None),
            printfunc: RefCell::new(None),
            errorfunc: RefCell::new(None),
            debuginfo: std::cell::Cell::new(false),
            notifyallexceptions: std::cell::Cell::new(false),
            foreignptr: std::cell::Cell::new(0),
            releasehook: RefCell::new(None),
        });
        // ↔ SQSharedState::Init()（sqstate.cpp:92-155）
        ss.init();
        ss
    }

    fn init(self: &Rc<SharedState>) {
        *self.metamethodsmap.borrow_mut() = Value::Table(SQTable::create(self, crate::class::MT_LAST as SQInteger - 1));
        for name in MM_NAMES.iter() {
            let s = Value::String(self.stringtable.add(name.as_bytes()));
            self.metamethods.borrow_mut().push(s.clone());
            if let Value::Table(t) = &*self.metamethodsmap.borrow() {
                t.borrow_mut().new_slot(
                    &s,
                    &Value::Integer(self.metamethods.borrow().len() as SQInteger - 1),
                );
            }
        }
        // ↔ newsysstring（仅保持与 C++ 相同的常驻字符串集合）
        for name in [
            "null", "table", "array", "closure", "string", "userdata", "integer", "float",
            "userpointer", "function", "generator", "thread", "class", "instance", "bool",
        ] {
            self.systemstrings
                .borrow_mut()
                .push(Value::String(self.stringtable.add(name.as_bytes())));
        }
        *self.constructoridx.borrow_mut() = Value::String(self.stringtable.add(b"constructor"));
        *self.registry.borrow_mut() = Value::Table(SQTable::create(self, 0));
        *self.consts.borrow_mut() = Value::Table(SQTable::create(self, 0));
        // 11 张默认委托表：M5 由 baselib 填充内容，此处先建空表保证结构存在
        *self.table_default_delegate.borrow_mut() = Value::Table(SQTable::create(self, 0));
        *self.array_default_delegate.borrow_mut() = Value::Table(SQTable::create(self, 0));
        *self.string_default_delegate.borrow_mut() = Value::Table(SQTable::create(self, 0));
        *self.number_default_delegate.borrow_mut() = Value::Table(SQTable::create(self, 0));
        *self.generator_default_delegate.borrow_mut() = Value::Table(SQTable::create(self, 0));
        *self.closure_default_delegate.borrow_mut() = Value::Table(SQTable::create(self, 0));
        *self.thread_default_delegate.borrow_mut() = Value::Table(SQTable::create(self, 0));
        *self.class_default_delegate.borrow_mut() = Value::Table(SQTable::create(self, 0));
        *self.instance_default_delegate.borrow_mut() = Value::Table(SQTable::create(self, 0));
        *self.weakref_default_delegate.borrow_mut() = Value::Table(SQTable::create(self, 0));
    }

    pub(crate) fn gc_add(&self, e: GcEntry) {
        let mut chain = self.gc_chain.borrow_mut();
        chain.push(e);
        // 对象被 `Rc` 即时释放后，注册表里会留一个 `Weak` 已死的条目。
        // 参考实现不需要这种表（它用引用计数 + 周期回收），本实现的表若不清，
        // 纯 churn 脚本（`for(i<2e5) local t = {}`）会攒下几十 MB 条目。
        // 清理只丢"弱引用已失效"的条目——那正是对象已经析构的情况，
        // 与 GC 的 `None => 已析构` 分支同义，不改变任何可观察语义。
        if chain.len() >= self.gc_prune_at.get() {
            chain.retain(|x| x.value().is_some());
            let live = chain.len();
            self.gc_prune_at.set(std::cmp::max(65536, live.saturating_mul(2)));
        }
    }

    /// ↔ SQSharedState::GetMetaMethodIdxByName
    pub fn get_metamethod_idx_by_name(&self, name: &Value) -> SQInteger {
        if !matches!(name, Value::String(_)) {
            return -1;
        }
        if let Value::Table(t) = &*self.metamethodsmap.borrow() {
            if let Some(v) = t.borrow().get(name) {
                return v.as_integer();
            }
        }
        -1
    }

    pub fn intern(&self, s: &[u8]) -> Value {
        Value::String(self.stringtable.add(s))
    }

    // ---------------- GC ----------------

    /// ↔ RunMark + 遍历：从根出发收集可达对象地址集合。
    /// `current` 是发起 GC 的 VM（其自身已先快照入工作表，遍历中跳过以免重入借用）。
    fn run_mark(&self, roots: Vec<Value>) -> HashSet<usize> {
        let mut marked: HashSet<usize> = HashSet::new();
        let mut work: Vec<Value> = roots;
        while let Some(v) = work.pop() {
            let id = v.identity();
            if id == 0 || !marked.insert(id) {
                continue;
            }
            match &v {
                Value::Table(rc) => {
                    let t = rc.borrow();
                    if let Some(d) = &t.delegate {
                        work.push(Value::Table(d.clone()));
                    }
                    for n in t.nodes.iter() {
                        if !n.key.is_null() {
                            work.push(n.key.clone());
                            work.push(n.val.clone());
                        }
                    }
                }
                Value::Array(rc) => {
                    for x in rc.borrow().values.iter() {
                        work.push(x.clone());
                    }
                }
                Value::Userdata(rc) => {
                    if let Some(d) = &rc.borrow().delegate {
                        work.push(Value::Table(d.clone()));
                    }
                }
                Value::Closure(rc) => {
                    let c = rc.borrow();
                    if let Some(b) = &c.base {
                        work.push(Value::Class(b.clone()));
                    }
                    // proto 本体 + 子 proto
                    work.push(Value::FuncProto(c.function.clone()));
                    for o in c.outervalues.iter() {
                        work.push(o.clone());
                    }
                    for d in c.defaultparams.iter() {
                        work.push(d.clone());
                    }
                }
                Value::NativeClosure(rc) => {
                    for o in rc.borrow().outervalues.iter() {
                        work.push(o.clone());
                    }
                }
                Value::Generator(rc) => {
                    let g = rc.borrow();
                    for x in g.stack.iter() {
                        work.push(x.clone());
                    }
                    work.push(g.closure.clone());
                }
                Value::Class(rc) => {
                    let c = rc.borrow();
                    work.push(Value::Table(c.members.clone()));
                    if let Some(b) = &c.base {
                        work.push(Value::Class(b.clone()));
                    }
                    if !c.attributes.is_null() {
                        work.push(c.attributes.clone());
                    }
                    for dv in c.defaultvalues.iter() {
                        work.push(dv.val.clone());
                        work.push(dv.attrs.clone());
                    }
                    for m in c.methods.iter() {
                        work.push(m.val.clone());
                        work.push(m.attrs.clone());
                    }
                    for mm in c.metamethods.iter() {
                        work.push(mm.clone());
                    }
                }
                Value::Instance(rc) => {
                    let i = rc.borrow();
                    if let Some(cl) = &i.class {
                        work.push(Value::Class(cl.clone()));
                        for x in i.values.iter() {
                            work.push(x.clone());
                        }
                    }
                }
                Value::Outer(rc) => {
                    let o = rc.borrow();
                    if o.is_closed() {
                        work.push(o.value.clone());
                    }
                }
                Value::FuncProto(rc) => {
                    for l in rc.literals.iter() {
                        work.push(l.clone());
                    }
                    for f in rc.functions.iter() {
                        work.push(f.clone());
                    }
                }
                Value::Thread(_) => {
                    // 其他 VM。**正在执行中的 VM borrow 不到**：它的 RefCell 被外层
                    // 执行路径（`thread_call` 里的 `&mut VmData`）持有着，直接 `borrow()`
                    // 会 panic（`RefCell already mutably borrowed`，线程里
                    // `collectgarbage()` 就踩过）。这时它在根集合里由调用方快照
                    // （`VmData::outer_roots`）代表，这里跳过即可。
                    if let Value::Thread(rc) = &v {
                        if let Ok(vm) = rc.try_borrow() {
                            if !vm.lasterror.is_null() {
                                work.push(vm.lasterror.clone());
                            }
                            if !vm.errorhandler.is_null() {
                                work.push(vm.errorhandler.clone());
                            }
                            if !vm.debughook_closure.is_null() {
                                work.push(vm.debughook_closure.clone());
                            }
                            if !vm.roottable.is_null() {
                                work.push(vm.roottable.clone());
                            }
                            if !vm.temp_reg.is_null() {
                                work.push(vm.temp_reg.clone());
                            }
                            for x in vm.stack.iter() {
                                work.push(x.clone());
                            }
                            for ci in vm.callsstack.iter() {
                                work.push(ci.closure.clone());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        marked
    }

    /// 根集合（不含当前 VM 自身——其内容在调用点快照）
    fn base_roots(&self) -> Vec<Value> {
        let mut roots = Vec::new();
        roots.extend(self.refs_table.roots());
        roots.push(self.registry.borrow().clone());
        roots.push(self.consts.borrow().clone());
        roots.push(self.metamethodsmap.borrow().clone());
        roots.push(self.table_default_delegate.borrow().clone());
        roots.push(self.array_default_delegate.borrow().clone());
        roots.push(self.string_default_delegate.borrow().clone());
        roots.push(self.number_default_delegate.borrow().clone());
        roots.push(self.generator_default_delegate.borrow().clone());
        roots.push(self.thread_default_delegate.borrow().clone());
        roots.push(self.closure_default_delegate.borrow().clone());
        roots.push(self.class_default_delegate.borrow().clone());
        roots.push(self.instance_default_delegate.borrow().clone());
        roots.push(self.weakref_default_delegate.borrow().clone());
        roots
    }

    /// ↔ SQSharedState::CollectGarbage（sqstate.cpp:321-351）
    /// 返回回收的对象数。Rust 语义：不可达对象逐个 Finalize（清空内部引用），
    /// Rc 引用计数随之归零、析构（等价于 C++ 的 Finalize + 强制 Release）。
    pub fn collect_garbage(&self, current: &mut VmData) -> SQInteger {
        let mut roots = self.base_roots();
        // ↔ root_vm（weakref）解引用后标记：当前 VM 的内容在此快照
        let root_vm = self.root_vm.borrow().weakref_target();
        let is_current = match (&root_vm, current.self_ref()) {
            (Value::Thread(rc), Some(cur)) => Rc::ptr_eq(rc, &cur),
            _ => false,
        };
        // 当前 VM 自己的根**始终**要加：它是一种"被调起的 VM"（线程），
        // 其栈上的对象只能由它自己报出来（参考实现靠"线程对象递归标记自己的栈"，
        // 我们 borrow 不到正在执行的宿 VM，所以分两步：自己的根 + 宿 VM 的快照）。
        roots.extend(current.mark_roots());
        if !root_vm.is_null() && !is_current {
            // 宿 VM 正在执行（其 RefCell 被外层执行路径持有）→ 不能 borrow 它，
            // 用进入本 VM 时留下的根快照；快照是传递的（上一层快照也在里面）
            let snap = current.outer_roots.borrow();
            if snap.is_empty() {
                roots.push(root_vm);
            } else {
                roots.extend(snap.iter().cloned());
            }
        }
        let marked = self.run_mark(roots);

        let entries: Vec<GcEntry> = std::mem::take(&mut *self.gc_chain.borrow_mut());
        let mut n: SQInteger = 0;
        let mut kept = Vec::new();
        for e in entries {
            match e.value() {
                None => { /* 已析构，清理注册表 */ }
                Some(v) => {
                    if marked.contains(&v.identity()) {
                        kept.push(e);
                    } else {
                        finalize_value(&v);
                        n += 1;
                    }
                }
            }
        }
        *self.gc_chain.borrow_mut() = kept;
        n
    }

    /// ↔ SQSharedState::ResurrectUnreachable（sqstate.cpp:267-319）
    /// 返回 (数量, 不可达对象数组)。
    /// ↔ SQSharedState::ResurrectUnreachable
    ///
    /// 返回 (被复活对象数, Some(数组)) —— 没有任何不可达对象时返回 None（脚本看到 null），
    /// 与参考实现一致。**必须先把不可达对象快照下来再建返回数组**：返回数组自己也在
    /// GC 链里且此刻尚未被任何根引用，否则会把自己也算进结果。
    pub fn resurrect_unreachable(&self, current: &mut VmData) -> (SQInteger, Option<crate::object::ArrayRef>) {
        let mut roots = self.base_roots();
        let root_vm = self.root_vm.borrow().weakref_target();
        let is_current = match (&root_vm, current.self_ref()) {
            (Value::Thread(rc), Some(cur)) => Rc::ptr_eq(rc, &cur),
            _ => false,
        };
        // 当前 VM 自己的根**始终**要加：它是一种"被调起的 VM"（线程），
        // 其栈上的对象只能由它自己报出来（参考实现靠"线程对象递归标记自己的栈"，
        // 我们 borrow 不到正在执行的宿 VM，所以分两步：自己的根 + 宿 VM 的快照）。
        roots.extend(current.mark_roots());
        if !root_vm.is_null() && !is_current {
            // 宿 VM 正在执行（其 RefCell 被外层执行路径持有）→ 不能 borrow 它，
            // 用进入本 VM 时留下的根快照；快照是传递的（上一层快照也在里面）
            let snap = current.outer_roots.borrow();
            if snap.is_empty() {
                roots.push(root_vm);
            } else {
                roots.extend(snap.iter().cloned());
            }
        }
        let marked = self.run_mark(roots);

        let unreachable: Vec<Value> = {
            let entries = self.gc_chain.borrow();
            entries
                .iter()
                .filter_map(|e| e.value())
                .filter(|v| !marked.contains(&v.identity()))
                .collect()
        };
        if unreachable.is_empty() {
            return (0, None);
        }
        let ret = SQArray::create(self, 0);
        let mut n: SQInteger = 0;
        for v in unreachable {
            n += 1;
            // ↔ FUNCPROTO/OUTER 不进入结果数组，但仍计入 n
            if !matches!(v, Value::FuncProto(_) | Value::Outer(_)) {
                ret.borrow_mut().append(&v);
            }
        }
        (n, Some(ret))
    }
}

impl VmData {
    /// 当前 VM 作为 GC 根的内容快照（↔ SQVM::Mark）
    pub fn mark_roots(&mut self) -> Vec<Value> {
        let mut roots = Vec::new();
        if !self.lasterror.is_null() {
            roots.push(self.lasterror.clone());
        }
        if !self.errorhandler.is_null() {
            roots.push(self.errorhandler.clone());
        }
        if !self.debughook_closure.is_null() {
            roots.push(self.debughook_closure.clone());
        }
        if !self.roottable.is_null() {
            roots.push(self.roottable.clone());
        }
        if !self.temp_reg.is_null() {
            roots.push(self.temp_reg.clone());
        }
        roots.extend(self.stack.iter().cloned());
        roots.extend(self.callsstack.iter().map(|ci| ci.closure.clone()));
        // 进入本 VM 时留下的「调用方根快照」（见 `outer_roots`）：嵌套线程里
        // collectgarbage 时，宿 VM 正被外层执行路径可变借用、borrow 不到，
        // 只能靠这份快照把它的可达对象标上。
        roots.extend(self.outer_roots.borrow().iter().cloned());
        roots
    }
}

/// 不可达对象的 Finalize（↔ 各类型的 Finalize()；Rust 需清空全部持引用字段以打断环）。
/// FuncProto 的字面量只可能是标量/字符串（无环），无需 finalize。
fn finalize_value(v: &Value) {
    match v {
        Value::Table(rc) => rc.borrow_mut().finalize(),
        Value::Array(rc) => rc.borrow_mut().finalize(),
        Value::Userdata(rc) => rc.borrow_mut().finalize(),
        Value::Closure(rc) => rc.borrow_mut().finalize(),
        Value::NativeClosure(rc) => rc.borrow_mut().finalize(),
        Value::Generator(rc) => rc.borrow_mut().finalize(),
        Value::Class(rc) => rc.borrow_mut().finalize(),
        Value::Instance(rc) => rc.borrow_mut().finalize(),
        Value::Outer(rc) => rc.borrow_mut().finalize(),
        Value::Thread(rc) => rc.borrow_mut().finalize(),
        _ => {}
    }
}

// ↔ CompileTypemask（sqstate.cpp:35-74）
pub fn compile_typemask(res: &mut Vec<SQInt32>, typemask: &[u8]) -> bool {
    use crate::config::*;
    let mut i = 0;
    let mut mask: SQInteger = 0;
    while i < typemask.len() && typemask[i] != 0 {
        match typemask[i] {
            b'o' => mask |= _RT_NULL as SQInteger,
            b'i' => mask |= _RT_INTEGER as SQInteger,
            b'f' => mask |= _RT_FLOAT as SQInteger,
            b'n' => mask |= (_RT_FLOAT | _RT_INTEGER) as SQInteger,
            b's' => mask |= _RT_STRING as SQInteger,
            b't' => mask |= _RT_TABLE as SQInteger,
            b'a' => mask |= _RT_ARRAY as SQInteger,
            b'u' => mask |= _RT_USERDATA as SQInteger,
            b'c' => mask |= (_RT_CLOSURE | _RT_NATIVECLOSURE) as SQInteger,
            b'b' => mask |= _RT_BOOL as SQInteger,
            b'g' => mask |= _RT_GENERATOR as SQInteger,
            b'p' => mask |= _RT_USERPOINTER as SQInteger,
            b'v' => mask |= _RT_THREAD as SQInteger,
            b'x' => mask |= _RT_INSTANCE as SQInteger,
            b'y' => mask |= _RT_CLASS as SQInteger,
            b'r' => mask |= _RT_WEAKREF as SQInteger,
            b'.' => {
                res.push(-1);
                i += 1;
                continue;
            }
            b' ' => {
                i += 1;
                continue;
            }
            _ => return false,
        }
        i += 1;
        if i < typemask.len() && typemask[i] == b'|' {
            i += 1;
            if i >= typemask.len() || typemask[i] == 0 {
                return false;
            }
            continue;
        }
        res.push(mask as SQInt32);
        mask = 0;
    }
    true
}

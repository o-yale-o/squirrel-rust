//! ↔ squirrel/sqobject.h + sqobject.cpp（部分）
//!
//! SQObject 在 C++ 里是「类型标签 + union」；Rust 用 `Value` 枚举表达。
//! 引用语义用 `Rc` 复刻引用计数，weakref 用 `Weak` 复刻 `SQWeakRef`。

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::array::SQArray;
use crate::class::{SQClass, SQInstance};
use crate::closure::{SQClosure, SQGenerator, SQNativeClosure, SQOuter};
use crate::config::*;
use crate::funcproto::SQFunctionProto;
use crate::table::SQTable;
use crate::userdata::SQUserData;
use crate::vm::VmData;

pub type TableRef = Rc<RefCell<SQTable>>;
pub type ArrayRef = Rc<RefCell<SQArray>>;
pub type UserdataRef = Rc<RefCell<SQUserData>>;
pub type ClosureRef = Rc<RefCell<SQClosure>>;
pub type NativeClosureRef = Rc<RefCell<SQNativeClosure>>;
pub type GeneratorRef = Rc<RefCell<SQGenerator>>;
pub type OuterRef = Rc<RefCell<SQOuter>>;
pub type ClassRef = Rc<RefCell<SQClass>>;
pub type InstanceRef = Rc<RefCell<SQInstance>>;
pub type ThreadRef = Rc<RefCell<VmData>>;
pub type FuncProtoRef = Rc<SQFunctionProto>;
pub type WeakRefRef = Rc<RefCell<SQWeakRef>>;

/// ↔ SQObject（18 种类型）。Clone == C++ 拷贝构造（自动 AddRef）。
#[derive(Clone)]
pub enum Value {
    Null,
    Integer(SQInteger),
    Float(SQFloat),
    Bool(bool),
    String(Rc<SQString>),
    Table(TableRef),
    Array(ArrayRef),
    Userdata(UserdataRef),
    Closure(ClosureRef),
    NativeClosure(NativeClosureRef),
    Generator(GeneratorRef),
    UserPointer(SQUserPointer),
    Thread(ThreadRef),
    FuncProto(FuncProtoRef),
    Class(ClassRef),
    Instance(InstanceRef),
    WeakRef(WeakRefRef),
    Outer(OuterRef),
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Integer(i) => write!(f, "int({i})"),
            Value::Float(x) => write!(f, "float({x})"),
            Value::Bool(b) => write!(f, "bool({b})"),
            Value::String(s) => write!(f, "string({:?})", String::from_utf8_lossy(&s.val)),
            Value::UserPointer(p) => write!(f, "ptr({p:#x})"),
            Value::Table(_) => write!(f, "table"),
            Value::Array(_) => write!(f, "array"),
            Value::Userdata(_) => write!(f, "userdata"),
            Value::Closure(_) => write!(f, "closure"),
            Value::NativeClosure(_) => write!(f, "nativeclosure"),
            Value::Generator(_) => write!(f, "generator"),
            Value::Thread(_) => write!(f, "thread"),
            Value::FuncProto(_) => write!(f, "funcproto"),
            Value::Class(_) => write!(f, "class"),
            Value::Instance(_) => write!(f, "instance"),
            Value::WeakRef(_) => write!(f, "weakref"),
            Value::Outer(_) => write!(f, "outer"),
        }
    }
}

impl Value {
    /// ↔ sq_type(o)：带标志位的完整类型标签
    pub fn type_(&self) -> SQObjectType {
        match self {
            Value::Null => OT_NULL,
            Value::Integer(_) => OT_INTEGER,
            Value::Float(_) => OT_FLOAT,
            Value::Bool(_) => OT_BOOL,
            Value::String(_) => OT_STRING,
            Value::Table(_) => OT_TABLE,
            Value::Array(_) => OT_ARRAY,
            Value::Userdata(_) => OT_USERDATA,
            Value::Closure(_) => OT_CLOSURE,
            Value::NativeClosure(_) => OT_NATIVECLOSURE,
            Value::Generator(_) => OT_GENERATOR,
            Value::UserPointer(_) => OT_USERPOINTER,
            Value::Thread(_) => OT_THREAD,
            Value::FuncProto(_) => OT_FUNCPROTO,
            Value::Class(_) => OT_CLASS,
            Value::Instance(_) => OT_INSTANCE,
            Value::WeakRef(_) => OT_WEAKREF,
            Value::Outer(_) => OT_OUTER,
        }
    }

    pub fn raw_type(&self) -> SQObjectType {
        raw_type(self.type_())
    }

    pub fn is_ref_counted(&self) -> bool {
        self.type_() & SQOBJECT_REF_COUNTED != 0
    }

    pub fn is_delegable(&self) -> bool {
        self.type_() & SQOBJECT_DELEGABLE != 0
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_integer(&self) -> SQInteger {
        match self {
            Value::Integer(i) => *i,
            Value::Bool(b) => {
                if *b {
                    1
                } else {
                    0
                }
            }
            Value::Float(f) => c_f32_to_i64(*f),
            _ => 0,
        }
    }

    /// ↔ tofloat：int→float 走硬件舍入（round-to-nearest-even），与 MSVC 一致
    pub fn as_float(&self) -> SQFloat {
        match self {
            Value::Integer(i) => *i as SQFloat,
            Value::Float(f) => *f,
            Value::Bool(b) => (*b as SQInteger) as SQFloat,
            _ => 0.0,
        }
    }

    pub fn as_bool(&self) -> bool {
        !self.is_false()
    }

    /// ↔ SQVM::IsFalse（sqvm.cpp:680-693）
    pub fn is_false(&self) -> bool {
        match self {
            Value::Null => true,
            Value::Bool(b) => !*b,
            Value::Integer(i) => *i == 0,
            Value::Float(f) => *f == 0.0,
            _ => false,
        }
    }

    /// ↔ _stringval：字符串字节内容
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Value::String(s) => &s.val,
            _ => b"",
        }
    }

    /// 取出 weakref 指向的对象（死引用 → Null）。
    /// ↔ `_weakref(o)->_obj`
    pub fn weakref_target(&self) -> Value {
        match self {
            Value::WeakRef(w) => w.borrow().obj.get(),
            other => other.clone(),
        }
    }

    /// ↔ _realval：表/数组槽中存的 weakref 读取时自动解引用
    pub fn realval(&self) -> Value {
        match self {
            Value::WeakRef(w) => w.borrow().obj.get(),
            other => other.clone(),
        }
    }

    /// 对象身份地址（用于 GC 标记集合、RefTable 键）
    pub fn identity(&self) -> usize {
        match self {
            Value::String(r) => Rc::as_ptr(r) as usize,
            Value::Table(r) => Rc::as_ptr(r) as usize,
            Value::Array(r) => Rc::as_ptr(r) as usize,
            Value::Userdata(r) => Rc::as_ptr(r) as usize,
            Value::Closure(r) => Rc::as_ptr(r) as usize,
            Value::NativeClosure(r) => Rc::as_ptr(r) as usize,
            Value::Generator(r) => Rc::as_ptr(r) as usize,
            Value::Thread(r) => Rc::as_ptr(r) as usize,
            Value::FuncProto(r) => Rc::as_ptr(r) as usize,
            Value::Class(r) => Rc::as_ptr(r) as usize,
            Value::Instance(r) => Rc::as_ptr(r) as usize,
            Value::WeakRef(r) => Rc::as_ptr(r) as usize,
            Value::Outer(r) => Rc::as_ptr(r) as usize,
            _ => 0,
        }
    }

    /// ↔ GetWeakRef（惰性单例；同一对象多次调用返回同一个 SQWeakRef）
    pub fn get_weakref(&self) -> Value {
        // Rc<RefCell<T>> 类型：weakref_slot 在对象内部，经 RefCell 访问
        macro_rules! singleton_cell {
            ($rc:expr, $variant:ident) => {{
                let rc = $rc;
                if let Some(w) = rc.borrow().weakref_slot.borrow().clone() {
                    return w;
                }
                let w = Value::WeakRef(Rc::new(RefCell::new(SQWeakRef {
                    obj: WeakValue::$variant(Rc::downgrade(rc)),
                })));
                *rc.borrow_mut().weakref_slot.borrow_mut() = Some(w.clone());
                w
            }};
        }
        // 无 RefCell 的 Rc<T> 类型（String/FuncProto）：weakref_slot 自带 RefCell
        macro_rules! singleton_plain {
            ($rc:expr, $variant:ident) => {{
                let rc = $rc;
                if let Some(w) = rc.weakref_slot.borrow().clone() {
                    return w;
                }
                let w = Value::WeakRef(Rc::new(RefCell::new(SQWeakRef {
                    obj: WeakValue::$variant(Rc::downgrade(rc)),
                })));
                *rc.weakref_slot.borrow_mut() = Some(w.clone());
                w
            }};
        }
        match self {
            Value::String(r) => singleton_plain!(r, String),
            Value::Table(r) => singleton_cell!(r, Table),
            Value::Array(r) => singleton_cell!(r, Array),
            Value::Userdata(r) => singleton_cell!(r, Userdata),
            Value::Closure(r) => singleton_cell!(r, Closure),
            Value::NativeClosure(r) => singleton_cell!(r, NativeClosure),
            Value::Generator(r) => singleton_cell!(r, Generator),
            Value::Thread(r) => singleton_cell!(r, Thread),
            Value::FuncProto(r) => singleton_plain!(r, FuncProto),
            Value::Class(r) => singleton_cell!(r, Class),
            Value::Instance(r) => singleton_cell!(r, Instance),
            Value::Outer(r) => singleton_cell!(r, Outer),
            _ => self.clone(),
        }
    }
}

/// ↔ _rawval 比较 + 类型比较（表键匹配语义；浮点按位比较，0.0 与 -0.0 不同键）
pub fn raw_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Integer(x), Value::Integer(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => Rc::ptr_eq(x, y),
        (Value::Table(x), Value::Table(y)) => Rc::ptr_eq(x, y),
        (Value::Array(x), Value::Array(y)) => Rc::ptr_eq(x, y),
        (Value::Userdata(x), Value::Userdata(y)) => Rc::ptr_eq(x, y),
        (Value::Closure(x), Value::Closure(y)) => Rc::ptr_eq(x, y),
        (Value::NativeClosure(x), Value::NativeClosure(y)) => Rc::ptr_eq(x, y),
        (Value::Generator(x), Value::Generator(y)) => Rc::ptr_eq(x, y),
        (Value::UserPointer(x), Value::UserPointer(y)) => x == y,
        (Value::Thread(x), Value::Thread(y)) => Rc::ptr_eq(x, y),
        (Value::FuncProto(x), Value::FuncProto(y)) => Rc::ptr_eq(x, y),
        (Value::Class(x), Value::Class(y)) => Rc::ptr_eq(x, y),
        (Value::Instance(x), Value::Instance(y)) => Rc::ptr_eq(x, y),
        (Value::WeakRef(x), Value::WeakRef(y)) => Rc::ptr_eq(x, y),
        (Value::Outer(x), Value::Outer(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

/// ↔ HashObj（sqtable.h:15-23）
pub fn hash_obj(key: &Value) -> SQHash {
    match key {
        Value::String(s) => s.hash,
        Value::Float(f) => c_f32_to_i64(*f) as SQHash,
        Value::Bool(b) => (*b as SQInteger) as SQHash,
        Value::Integer(i) => *i as SQHash,
        other => hash_ptr(other.identity()),
    }
}

/// ↔ hashptr(p)：((SQInteger)p) >> 3（按有符号算术右移）
fn hash_ptr(p: usize) -> SQHash {
    ((p as SQInteger) >> 3) as SQHash
}

/// ↔ _hashstr（sqstring.h:5-13，Lua 4.0 风格采样哈希；逐位复刻）
pub fn hashstr(s: &[u8]) -> SQHash {
    let mut h: SQHash = s.len() as SQHash; /* seed */
    let step = (s.len() >> 5) + 1;
    let mut l1 = s.len();
    while l1 >= step {
        h ^= (h << 5)
            .wrapping_add(h >> 2)
            .wrapping_add(s[l1 - 1] as u16 as SQHash);
        l1 -= step;
    }
    h
}

/// ↔ SQString：不可变字节串，经 SharedState::stringtable 实习（interning）
pub struct SQString {
    pub hash: SQHash,
    pub val: Box<[u8]>,
    pub(crate) weakref_slot: RefCell<Option<Value>>,
}

impl SQString {
    pub fn len(&self) -> SQInteger {
        self.val.len() as SQInteger
    }

    /// ↔ SQString::Next（按索引迭代，值是字符码）。返回 (下一 refpos, key, val)。
    pub fn next(&self, refpos: &Value) -> Option<(SQInteger, Value, Value)> {
        let idx = translate_index(refpos) as SQInteger;
        if idx < self.len() {
            Some((
                idx + 1,
                Value::Integer(idx),
                Value::Integer(self.val[idx as usize] as SQUnsignedInteger as SQInteger),
            ))
        } else {
            None
        }
    }
}

/// ↔ SQWeakRef::_obj：指向某个引用计数对象的弱引用
pub enum WeakValue {
    String(Weak<SQString>),
    Table(Weak<RefCell<SQTable>>),
    Array(Weak<RefCell<SQArray>>),
    Userdata(Weak<RefCell<SQUserData>>),
    Closure(Weak<RefCell<SQClosure>>),
    NativeClosure(Weak<RefCell<SQNativeClosure>>),
    Generator(Weak<RefCell<SQGenerator>>),
    Thread(Weak<RefCell<VmData>>),
    FuncProto(Weak<SQFunctionProto>),
    Class(Weak<RefCell<SQClass>>),
    Instance(Weak<RefCell<SQInstance>>),
    Outer(Weak<RefCell<SQOuter>>),
}

impl WeakValue {
    /// ↔ 升级失败 == C++ 里对象析构时 `_obj` 被置 null
    pub fn get(&self) -> Value {
        match self {
            WeakValue::String(w) => w.upgrade().map(Value::String).unwrap_or(Value::Null),
            WeakValue::Table(w) => w.upgrade().map(Value::Table).unwrap_or(Value::Null),
            WeakValue::Array(w) => w.upgrade().map(Value::Array).unwrap_or(Value::Null),
            WeakValue::Userdata(w) => w.upgrade().map(Value::Userdata).unwrap_or(Value::Null),
            WeakValue::Closure(w) => w.upgrade().map(Value::Closure).unwrap_or(Value::Null),
            WeakValue::NativeClosure(w) => w
                .upgrade()
                .map(Value::NativeClosure)
                .unwrap_or(Value::Null),
            WeakValue::Generator(w) => w.upgrade().map(Value::Generator).unwrap_or(Value::Null),
            WeakValue::Thread(w) => w.upgrade().map(Value::Thread).unwrap_or(Value::Null),
            WeakValue::FuncProto(w) => w.upgrade().map(Value::FuncProto).unwrap_or(Value::Null),
            WeakValue::Class(w) => w.upgrade().map(Value::Class).unwrap_or(Value::Null),
            WeakValue::Instance(w) => w.upgrade().map(Value::Instance).unwrap_or(Value::Null),
            WeakValue::Outer(w) => w.upgrade().map(Value::Outer).unwrap_or(Value::Null),
        }
    }
}

/// ↔ SQWeakRef
pub struct SQWeakRef {
    pub obj: WeakValue,
}

/// ↔ TranslateIndex（sqobject.cpp:73-83）
pub fn translate_index(idx: &Value) -> SQUnsignedInteger {
    match idx {
        Value::Null => 0,
        Value::Integer(i) => *i as SQUnsignedInteger,
        _ => 0,
    }
}

/// ↔ IdType2Name（sqobject.cpp:15-42）
pub fn id_type2name(t: SQObjectType) -> Option<&'static str> {
    Some(match raw_type(t) {
        _RT_NULL => "null",
        _RT_INTEGER => "integer",
        _RT_FLOAT => "float",
        _RT_BOOL => "bool",
        _RT_STRING => "string",
        _RT_TABLE => "table",
        _RT_ARRAY => "array",
        _RT_GENERATOR => "generator",
        _RT_CLOSURE | _RT_NATIVECLOSURE => "function",
        _RT_USERDATA | _RT_USERPOINTER => "userdata",
        _RT_THREAD => "thread",
        _RT_FUNCPROTO => "function",
        _RT_CLASS => "class",
        _RT_INSTANCE => "instance",
        _RT_WEAKREF => "weakref",
        _RT_OUTER => "outer",
        _ => return None,
    })
}

pub fn get_type_name(obj: &Value) -> Option<&'static str> {
    id_type2name(obj.type_())
}

/// ↔ SQDelegable::SetDelegate 的环检测（sqobject.cpp:120-132）。
/// 委托是强引用。
pub fn set_delegate_checked(
    current: &mut Option<TableRef>,
    this_identity: usize,
    mt: Option<TableRef>,
) -> bool {
    let mut temp = mt.clone();
    if let Some(t) = &temp {
        if Rc::as_ptr(t) as usize == this_identity {
            return false;
        }
    }
    loop {
        let next = match &temp {
            Some(t) => t.borrow().delegate.clone(),
            None => break,
        };
        match next {
            Some(d) => {
                if Rc::as_ptr(&d) as usize == this_identity {
                    return false; // cycle detected
                }
                temp = Some(d);
            }
            None => break,
        }
    }
    *current = mt;
    true
}

/// 把值注册进 SharedState 的 GC 注册表（↔ ADD_TO_CHAIN）。
/// C++ 用侵入式双链；Rust 用 Weak 注册表 + GC 时清理失效项。
macro_rules! gc_register {
    ($ss:expr, $rc:expr, $variant:ident) => {{
        let entry = crate::state::GcEntry::$variant(std::rc::Rc::downgrade($rc));
        $ss.gc_add(entry);
    }};
}

pub(crate) use gc_register;

/// 为每个引用类型生成「weakref 惰性单例」字段与访问器。
/// 字段名固定为 `weakref_slot`，供 `Value::get_weakref` 使用。
#[macro_export]
macro_rules! weakref_slot {
    () => {
        pub(crate) weakref_slot: std::cell::RefCell<Option<$crate::object::Value>>,
    };
}

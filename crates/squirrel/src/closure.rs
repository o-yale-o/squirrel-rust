//! ↔ squirrel/sqclosure.h + Yield/Resume（sqobject.cpp:134-206）

use crate::config::*;
use crate::object::{ClosureRef, FuncProtoRef, GeneratorRef, Value, WeakRefRef};
use crate::vm::{CallInfo, VmData};

/// ↔ SQFUNCTION：原生函数。返回 SQ_OK/SQ_ERROR/SQ_SUSPEND_FLAG/SQ_TAILCALL_FLAG。
pub type NativeFn = std::rc::Rc<dyn Fn(&mut VmData) -> SQInteger>;

pub struct SQClosure {
    pub function: FuncProtoRef,
    /// ↔ _env：绑定环境（weakref，调用时替换栈槽 0 的 this）
    pub env: Option<WeakRefRef>,
    /// ↔ _root：根表（weakref，LOADROOT 用）
    pub root: Option<WeakRefRef>,
    /// ↔ _base：`base` 关键字指向的父类（强引用；类→方法闭包→父类成环，由 GC 收）
    pub base: Option<crate::object::ClassRef>,
    pub outervalues: Vec<Value>,
    pub defaultparams: Vec<Value>,
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

/// 迭代式释放：outer 值链可能很深（见 crate::release）
impl Drop for SQClosure {
    fn drop(&mut self) {
        let mut kids: Vec<Value> = std::mem::take(&mut self.outervalues);
        kids.append(&mut self.defaultparams);
        if let Some(b) = self.base.take() {
            kids.push(Value::Class(b));
        }
        crate::release::release_values(kids);
    }
}

impl SQClosure {
    /// ↔ SQClosure::Create
    pub fn create(
        ss: &crate::state::SharedState,
        function: FuncProtoRef,
        root: Option<WeakRefRef>,
    ) -> ClosureRef {
        let rc = ClosureRef::new(std::cell::RefCell::new(SQClosure {
            outervalues: vec![Value::Null; function.outervalues.len()],
            defaultparams: vec![Value::Null; function.defaultparams.len()],
            function,
            env: None,
            root,
            base: None,
            weakref_slot: std::cell::RefCell::new(None),
        }));
        crate::object::gc_register!(ss, &rc, Closure);
        rc
    }

    pub fn set_root(&mut self, r: Option<WeakRefRef>) {
        self.root = r;
    }

    /// ↔ SQClosure::Clone
    pub fn clone_closure(&self, ss: &crate::state::SharedState) -> ClosureRef {
        let ret = SQClosure::create(ss, self.function.clone(), self.root.clone());
        let mut c = ret.borrow_mut();
        c.env = self.env.clone();
        c.base = self.base.clone();
        c.outervalues = self.outervalues.clone();
        c.defaultparams = self.defaultparams.clone();
        drop(c);
        ret
    }

    /// ↔ SQClosure::Finalize（GC 用；Rust 版额外清空 base/function 以打断环）
    pub fn finalize(&mut self) {
        for o in self.outervalues.iter_mut() {
            *o = Value::Null;
        }
        for d in self.defaultparams.iter_mut() {
            *d = Value::Null;
        }
        self.base = None;
        self.env = None;
        self.root = None;
    }
}

/// ↔ SQOuter：闭包捕获的外部变量。
/// C++ 用 `_valptr`（指向栈槽或自身 `_value`）；Rust 用栈槽绝对索引，
/// 栈扩容不再需要 RelocateOuters。
pub struct SQOuter {
    /// ↔ _valptr 语义：Some(idx)=打开（指向栈槽），None=已关闭（值在 value 里）
    pub open_idx: Option<usize>,
    /// ↔ _idx 备用（Rust 下无需重定位，仅供调试）
    pub idx: SQInteger,
    /// ↔ _value：帧关闭后的值
    pub value: Value,
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

impl SQOuter {
    pub fn create(ss: &crate::state::SharedState, open_idx: usize) -> crate::object::OuterRef {
        let rc = crate::object::OuterRef::new(std::cell::RefCell::new(SQOuter {
            open_idx: Some(open_idx),
            idx: 0,
            value: Value::Null,
            weakref_slot: std::cell::RefCell::new(None),
        }));
        crate::object::gc_register!(ss, &rc, Outer);
        rc
    }

    pub fn is_closed(&self) -> bool {
        self.open_idx.is_none()
    }

    /// ↔ SQOuter::Finalize
    pub fn finalize(&mut self) {
        self.value = Value::Null;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GeneratorState {
    Running,
    Suspended,
    Dead,
}

/// ↔ SQGenerator
pub struct SQGenerator {
    pub closure: Value,
    pub stack: Vec<Value>,
    pub ci: CallInfo,
    pub etraps: Vec<crate::vm::SQExceptionTrap>,
    pub state: GeneratorState,
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

/// 迭代式释放：挂起栈里可能压着很深的值（见 crate::release）
impl Drop for SQGenerator {
    fn drop(&mut self) {
        let mut kids: Vec<Value> = std::mem::take(&mut self.stack);
        kids.push(std::mem::replace(&mut self.closure, Value::Null));
        crate::release::release_values(kids);
    }
}

impl SQGenerator {
    pub fn create(ss: &crate::state::SharedState, closure: &ClosureRef) -> GeneratorRef {
        let rc = GeneratorRef::new(std::cell::RefCell::new(SQGenerator {
            closure: Value::Closure(closure.clone()),
            stack: Vec::new(),
            ci: CallInfo::default(),
            etraps: Vec::new(),
            state: GeneratorState::Running,
            weakref_slot: std::cell::RefCell::new(None),
        }));
        crate::object::gc_register!(ss, &rc, Generator);
        rc
    }

    /// ↔ SQGenerator::Kill
    pub fn kill(&mut self) {
        self.state = GeneratorState::Dead;
        self.stack.clear();
        self.closure = Value::Null;
    }

    /// ↔ SQGenerator::Yield（sqobject.cpp:134-163）
    pub fn r#yield(&mut self, vm: &mut VmData, target: SQInteger) -> bool {
        if self.state == GeneratorState::Suspended {
            vm.raise_error_str("internal vm error, yielding dead generator");
            return false;
        }
        if self.state == GeneratorState::Dead {
            vm.raise_error_str("internal vm error, yielding a dead generator");
            return false;
        }
        let size = vm.top - vm.stackbase;
        self.stack.resize(size.max(0) as usize, Value::Null);

        let this_ = vm.stack[vm.stackbase as usize].clone();
        // ↔ this 槽转成 weakref，避免生成器与实例成环
        self.stack[0] = if this_.is_ref_counted() {
            this_.get_weakref()
        } else {
            this_
        };
        for n in 1..target {
            self.stack[n as usize] = vm.stack[(vm.stackbase + n) as usize].clone();
        }
        for j in 0..size {
            vm.stack[(vm.stackbase + j) as usize] = Value::Null;
        }

        self.ci = vm.last_callinfo().clone();
        self.ci.generator = None;
        for _ in 0..self.ci.etraps {
            // trap 栈为空（畸形字节码）时停止搬运，绝不 panic
            let mut et = match vm.etraps.pop() {
                Some(et) => et,
                None => break,
            };
            // 存相对栈基，resume 时可能恢复到不同的 _top
            et.stackbase -= vm.stackbase;
            et.stacksize -= vm.stackbase;
            self.etraps.push(et);
        }
        self.state = GeneratorState::Suspended;
        true
    }

    /// ↔ SQGenerator::Resume（sqobject.cpp:165-206）
    pub fn resume(&mut self, vm: &mut VmData, target: SQInteger, self_rc: &crate::object::GeneratorRef) -> bool {
        if self.state == GeneratorState::Dead {
            vm.raise_error_str("resuming dead generator");
            return false;
        }
        if self.state == GeneratorState::Running {
            vm.raise_error_str("resuming active generator");
            return false;
        }
        let size = self.stack.len() as SQInteger;
        let newbase = vm.top;
        if vm.enter_frame(newbase, newbase + size, false).is_err() {
            return false;
        }
        {
            let ci = vm.last_callinfo_mut();
            ci.generator = Some(self_rc.clone()); // ↔ v->ci->_generator = this
            ci.target = target as SQInt32;
            ci.closure = self.ci.closure.clone();
            ci.ip = self.ci.ip;
            ci.literals = self.ci.literals.clone();
            ci.ncalls = self.ci.ncalls;
            ci.etraps = self.ci.etraps;
            ci.root = self.ci.root;
        }

        for _ in 0..self.ci.etraps {
            // 恢复时若自身 trap 栈已空（畸形字节码），停止搬运，绝不 panic
            let mut et = match self.etraps.pop() {
                Some(et) => et,
                None => break,
            };
            // 恢复绝对栈基
            et.stackbase += newbase;
            et.stacksize += newbase;
            vm.etraps.push(et);
        }
        let this_ = self.stack[0].clone();
        vm.stack[vm.stackbase as usize] = this_.weakref_target();

        for n in 1..size {
            vm.stack[(vm.stackbase + n) as usize] = self.stack[n as usize].clone();
            self.stack[n as usize] = Value::Null;
        }

        self.state = GeneratorState::Running;
        if vm.debughook {
            vm.call_debug_hook('c' as SQInteger, 0);
        }
        true
    }

    /// ↔ SQGenerator::Finalize
    pub fn finalize(&mut self) {
        self.stack.clear();
        self.closure = Value::Null;
    }
}

/// ↔ SQNativeClosure
pub struct SQNativeClosure {
    pub nparamscheck: SQInteger,
    pub typecheck: Vec<SQInt32>,
    pub outervalues: Vec<Value>,
    pub env: Option<WeakRefRef>,
    pub function: NativeFn,
    pub name: Value,
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

impl SQNativeClosure {
    pub fn create(ss: &crate::state::SharedState, f: NativeFn, nouters: usize) -> crate::object::NativeClosureRef {
        let rc = crate::object::NativeClosureRef::new(std::cell::RefCell::new(SQNativeClosure {
            nparamscheck: 0,
            typecheck: Vec::new(),
            outervalues: vec![Value::Null; nouters],
            env: None,
            function: f,
            name: Value::Null,
            weakref_slot: std::cell::RefCell::new(None),
        }));
        crate::object::gc_register!(ss, &rc, NativeClosure);
        rc
    }

    /// ↔ SQNativeClosure::Clone
    pub fn clone_native(&self, ss: &crate::state::SharedState) -> crate::object::NativeClosureRef {
        let ret = SQNativeClosure::create(ss, self.function.clone(), self.outervalues.len());
        let mut c = ret.borrow_mut();
        c.env = self.env.clone();
        c.name = self.name.clone();
        c.outervalues = self.outervalues.clone();
        c.typecheck = self.typecheck.clone();
        c.nparamscheck = self.nparamscheck;
        drop(c);
        ret
    }

    /// ↔ SQNativeClosure::Finalize
    pub fn finalize(&mut self) {
        for o in self.outervalues.iter_mut() {
            *o = Value::Null;
        }
    }
}

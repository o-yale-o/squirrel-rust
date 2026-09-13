//! ↔ sqapi.cpp —— Rust 风格宿主 API。
//!
//! C++ 的 sq_open/sq_compile/sq_call 对应：
//! - `Vm::new(stacksize)`：创建 VM（含 SharedState、root table、base 库）
//! - `Vm::compile_buffer(src, name)`：编译为闭包
//! - `Vm::call(&closure, this, &args)`：带 this 调用（↔ sq_call）
//! - `Vm::get_global(name)` / `Vm::register_native(name, f, …)`：读写全局符号
//!
//! 典型嵌入流程见 `crates/squirrel-stdlib/examples/minimal.rs`（原版 `etc/minimal.c` 的移植）。

use std::cell::RefCell;
use std::rc::Rc;

use crate::config::*;
use crate::object::{ThreadRef, Value};
use crate::state::{PrintFn, SharedState};
use crate::vm::{ExecutionType, VmData};

/// 宿主持有的 VM 句柄（↔ HSQUIRRELVM）
pub struct Vm {
    pub ss: Rc<SharedState>,
    pub rc: ThreadRef,
}

/// ↔ sq_open
impl Vm {
    pub fn new(initialstacksize: SQInteger) -> Vm {
        let ss = SharedState::new();
        let rc: ThreadRef = Rc::new(RefCell::new(VmData::new(ss.clone(), initialstacksize)));
        rc.borrow().set_self_ref(&rc);
        {
            // ↔ _ss(v)->_root_vm = v->GetWeakRef(OT_THREAD)
            let w = Value::Thread(rc.clone()).get_weakref();
            *ss.root_vm.borrow_mut() = w;
            let mut v = rc.borrow_mut();
            v.init(None, initialstacksize);
        }
        Vm { ss, rc }
    }

    pub fn data(&self) -> std::cell::Ref<'_, VmData> {
        self.rc.borrow()
    }

    pub fn data_mut(&self) -> std::cell::RefMut<'_, VmData> {
        self.rc.borrow_mut()
    }

    /// ↔ sq_setprintfunc
    pub fn set_print_funcs(&self, print: Option<PrintFn>, error: Option<PrintFn>) {
        *self.ss.printfunc.borrow_mut() = print;
        *self.ss.errorfunc.borrow_mut() = error;
    }

    /// ↔ sq_compilebuffer：编译并包装为闭包（压入栈）
    pub fn compile_buffer(&self, src: &[u8], name: &[u8], raiseerror: bool) -> Result<Value, ()> {
        let mut v = self.rc.borrow_mut();
        let mut pos = 0usize;
        let src2 = src.to_vec();
        let readf = Box::new(move || {
            if pos < src2.len() {
                let c = src2[pos];
                pos += 1;
                c as SQInteger
            } else {
                0
            }
        });
        let lineinfo = self.ss.debuginfo.get();
        match crate::compiler::compile(&mut v, readf, name, raiseerror, lineinfo) {
            Ok(proto) => {
                let fp = match &proto {
                    Value::FuncProto(f) => f.clone(),
                    _ => unreachable!(),
                };
                let root_weak = match &v.roottable {
                    Value::Table(t) => Some(Value::Table(t.clone()).get_weakref_as_weakref()),
                    _ => None,
                };
                let clo = crate::closure::SQClosure::create(&self.ss, fp, root_weak);
                v.push(Value::Closure(clo.clone()));
                Ok(Value::Closure(clo))
            }
            Err(()) => Err(()),
        }
    }

    /// ↔ sq_pushroottable + sq_pushstring + sq_get：从 root table 取全局符号
    pub fn get_global(&self, name: &str) -> Option<Value> {
        let v = self.rc.borrow();
        let key = v.ss.intern(name.as_bytes());
        match &v.roottable {
            Value::Table(t) => t.borrow().get(&key),
            _ => None,
        }
    }

    /// ↔ sq_pushroottable + sq_newslot：注册全局符号（宿主注册原生函数用）
    pub fn set_global(&self, name: &str, value: Value) {
        let v = self.rc.borrow();
        let key = v.ss.intern(name.as_bytes());
        if let Value::Table(t) = &v.roottable {
            t.borrow_mut().new_slot(&key, &value);
        }
    }

    /// ↔ sq_newclosure + sq_setparamscheck + sq_setnativeclosurename：注册原生函数到全局
    ///
    /// `nparamscheck` 含 `this`（与 C++ 一致）：例如「1 个参数」的全局原生函数写 2；
    /// `typemask` 的首个 `.` 是 `this` 的占位，例如 `".n"`。
    pub fn register_native(
        &self,
        name: &str,
        f: crate::closure::NativeFn,
        nparamscheck: SQInteger,
        typemask: Option<&str>,
    ) {
        let v = self.rc.borrow();
        let nc = crate::closure::SQNativeClosure::create(&v.ss, f, 0);
        {
            let mut n = nc.borrow_mut();
            n.nparamscheck = nparamscheck;
            n.name = v.ss.intern(name.as_bytes());
            if let Some(tm) = typemask {
                crate::state::compile_typemask(&mut n.typecheck, tm.as_bytes());
            }
        }
        self.set_global(name, Value::NativeClosure(nc));
    }

    /// ↔ sq_call：以 `this` 调用一个可调用值（闭包 / 原生闭包 / 类构造）
    ///
    /// 与 `call_top` 的区别是这里自己压栈、自己收尾，宿主不必手工维护栈布局。
    pub fn call(
        &self,
        f: &Value,
        this: Value,
        args: &[Value],
        raiseerror: bool,
    ) -> Result<Value, Value> {
        let mut v = self.rc.borrow_mut();
        let oldtop = v.top;
        v.push(f.clone());
        v.push(this);
        for a in args {
            v.push(a.clone());
        }
        let nparams = args.len() as SQInteger + 1;
        let base = v.top - nparams;
        let mut out = Value::Null;
        let r = crate::vm::execute(
            &mut v,
            f,
            nparams,
            base,
            &mut out,
            raiseerror,
            ExecutionType::Call,
        );
        v.top = oldtop;
        match r {
            Ok(()) => Ok(out),
            Err(e) => Err(e),
        }
    }

    /// ↔ sq_call：调用栈顶闭包（参数已在其下）。返回值以 Result 交付。
    pub fn call_top(&self, nparams: SQInteger, raiseerror: bool) -> Result<Value, Value> {
        let mut v = self.rc.borrow_mut();
        let closure = v.get_up(-(nparams + 1));
        // ↔ C++ sq_call：stackbase = _top - params（指向第一个参数，闭包在其下）
        let base = v.top - nparams;
        let mut out = Value::Null;
        let r = crate::vm::execute(
            &mut v,
            &closure,
            nparams,
            base,
            &mut out,
            raiseerror,
            ExecutionType::Call,
        );
        match r {
            Ok(()) => {
                v.pop_n(nparams + 1);
                Ok(out)
            }
            Err(e) => Err(e),
        }
    }

    /// 高层便捷接口：编译 + 以 root table 为 this 调用
    pub fn run_buffer(&self, src: &[u8], name: &[u8]) -> Result<Value, String> {
        match self.compile_buffer(src, name, true) {
            Ok(_clo) => {
                // compile_buffer 已压入闭包；补上 this 参数（root table），nparams=1
                {
                    let mut v = self.rc.borrow_mut();
                    let rt = v.roottable.clone();
                    v.push(rt);
                }
                match self.call_top(1, true) {
                    Ok(r) => Ok(r),
                    Err(e) => Err(value_to_error_string(&e)),
                }
            }
            Err(()) => {
                let v = self.rc.borrow_mut();
                Err(value_to_error_string(&v.lasterror.clone()))
            }
        }
    }
}

pub fn value_to_error_string(e: &Value) -> String {
    match e {
        Value::String(s) => String::from_utf8_lossy(&s.val).into_owned(),
        _ => format!("{:?}", e),
    }
}

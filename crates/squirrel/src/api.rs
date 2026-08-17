//! ↔ sqapi.cpp —— Rust 风格宿主 API。
//!
//! C++ 的 sq_open/sq_compile/sq_call 对应：
//! - `Vm::new(stacksize)`：创建 VM（含 SharedState、root table、base 库）
//! - `Vm::compile_buffer(src, name)`：编译为闭包
//! - `Vm::call(&closure, args)`：以 root table 为 this 调用

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

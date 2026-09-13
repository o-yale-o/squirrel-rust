//! 宿主 API 补齐（P2）：member handle / `getfunctioninfo` / `setfreevariable`
//! 以及几条"内部已有、缺公开面"的能力（userdata+typetag、release hook、
//! foreignptr、引用计数、resurrect）。
//!
//! 语义全部按 `squirrel/sqapi.cpp` 对齐（本轮把参考实现的静态库编成 C 探针实测过
//! 关键几处，见 `tools/hostapi/ref_host.c` 与 `ref_host.txt`）：
//!
//! * `sq_getmemberhandle(v, idx, &h)`：**只接受类**（`_GETSAFE_OBJ(..., OT_CLASS)`），
//!   从类上取成员表项；**成功时会把键从栈上弹掉**；失败报 `wrong index`。
//!   `_static` = 该成员不是"字段"（即方法/静态字段），`_index` = 成员表项里的下标。
//! * `sq_getbyhandle(v, idx, h)`：`idx` 处的对象必须是**类或实例**；
//!   静态成员从类的 `_methods[i]` 取、实例字段从实例的 `_values[i]` 取
//!   （类上传入时取 `_defaultvalues[i]`）。
//! * `sq_setbyhandle(v, idx, h)`：把**栈顶**写进该成员并弹出（与 `sq_set` 不同，
//!   它不走 `_set` 元方法、也不做类型检查）。
//! * `sq_setfreevariable(v, idx, nval)`：把第 `nval` 个自由变量**替换**成栈顶的值
//!   （注意是替换槽位，不是写穿 open outer —— 参考实现就是 `_outervalues[nval] = val`），
//!   不弹栈；下标越界报 `index out of range`。
//! * `sq_getfunctioninfo(v, level, &fi)`：收的是**栈层级**（和 `sq_stackinfos` 一样），
//!   不是对象索引；`native` 标识是不是 native closure。

use crate::class::{is_field, member_idx};
use crate::config::SQInteger;
use crate::object::{ClassRef, InstanceRef, Value};
use crate::vm::VmData;

/// ↔ SQFunctionInfo（sqapi.h）
#[derive(Clone, Debug, Default)]
pub struct FunctionInfo {
    pub name: Option<Vec<u8>>,
    pub source: Option<Vec<u8>>,
    pub line: i64,
    pub native: bool,
}

/// ↔ HSQMEMBERHANDLE
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemberHandle {
    /// ↔ `_static`：非字段（方法/静态字段）为真
    pub is_static: bool,
    /// ↔ `_index`：成员表项下标
    pub index: usize,
}

/// ↔ `_getmemberbyhandle` 的读侧
fn member_value_of(target: &Value, handle: &MemberHandle) -> Option<Value> {
    match target {
        Value::Class(c) => {
            let cl = c.borrow();
            if handle.is_static {
                cl.methods.get(handle.index).map(|m| m.val.clone())
            } else {
                cl.defaultvalues.get(handle.index).map(|m| m.val.clone())
            }
        }
        Value::Instance(i) => {
            let inst = i.borrow();
            if handle.is_static {
                let c = inst.class.clone()?;
                drop(inst);
                let cl = c.borrow();
                cl.methods.get(handle.index).map(|m| m.val.clone())
            } else {
                inst.values.get(handle.index).cloned()
            }
        }
        _ => None,
    }
}

/// ↔ `_getmemberbyhandle` 的写侧
fn set_member_of(target: &Value, handle: &MemberHandle, val: Value) -> bool {
    match target {
        Value::Class(c) => {
            let mut cl = c.borrow_mut();
            let slot = if handle.is_static {
                cl.methods.get_mut(handle.index)
            } else {
                cl.defaultvalues.get_mut(handle.index)
            };
            match slot {
                Some(m) => {
                    m.val = val;
                    true
                }
                None => false,
            }
        }
        Value::Instance(i) => {
            if handle.is_static {
                let c = match i.borrow().class.clone() {
                    Some(c) => c,
                    None => return false,
                };
                let mut cl = c.borrow_mut();
                match cl.methods.get_mut(handle.index) {
                    Some(m) => {
                        m.val = val;
                        true
                    }
                    None => false,
                }
            } else {
                let mut inst = i.borrow_mut();
                if handle.index < inst.values.len() {
                    inst.values[handle.index] = val;
                    true
                } else {
                    false
                }
            }
        }
        _ => false,
    }
}

impl VmData {
    /// ↔ sq_getmemberhandle：从**类**上取成员 handle。
    /// `Err` 的两种情况与参考实现一致：非类 → `wrong type(expected class or instance)`；
    /// 类里没这个成员 → `wrong index`。
    pub fn get_member_handle(&self, cls: &Value, key: &Value) -> Result<MemberHandle, &'static str> {
        let c: ClassRef = match cls {
            Value::Class(c) => c.clone(),
            _ => return Err("wrong type(expected class or instance)"),
        };
        let entry = c.borrow().members.borrow().get(key);
        match entry {
            Some(v) => Ok(MemberHandle {
                is_static: !is_field(&v),
                index: member_idx(&v),
            }),
            None => Err("wrong index"),
        }
    }

    /// ↔ sq_getbyhandle
    pub fn get_by_handle(&self, target: &Value, handle: &MemberHandle) -> Option<Value> {
        member_value_of(target, handle)
    }

    /// ↔ sq_setbyhandle
    pub fn set_by_handle(&self, target: &Value, handle: &MemberHandle, val: Value) -> bool {
        set_member_of(target, handle, val)
    }

    /// ↔ sq_getfunctioninfo：按**栈层级**取函数信息（0 = 最内层帧）
    pub fn get_function_info(&self, level: usize) -> Option<FunctionInfo> {
        let idx = self.callsstack.len().checked_sub(level + 1)?;
        let ci = self.callsstack.get(idx)?;
        let ip = if ci.ip > 0 { ci.ip as SQInteger } else { 0 };
        match &ci.closure {
            Value::NativeClosure(n) => {
                let n = n.borrow();
                let name = match &n.name {
                    Value::String(s) => s.val.to_vec(),
                    _ => b"unknown".to_vec(),
                };
                Some(FunctionInfo {
                    name: Some(name),
                    source: None,
                    line: 0,
                    native: true,
                })
            }
            Value::Closure(c) => {
                let c = c.borrow();
                let proto = c.function.clone();
                let name = match &proto.name {
                    Value::String(s) => s.val.to_vec(),
                    _ => b"unknown".to_vec(),
                };
                let source = match &proto.sourcename {
                    Value::String(s) => s.val.to_vec(),
                    _ => b"unknown".to_vec(),
                };
                Some(FunctionInfo {
                    name: Some(name),
                    source: Some(source),
                    line: proto.get_line(ip) as i64,
                    native: false,
                })
            }
            _ => None,
        }
    }

    /// ↔ sq_setfreevariable：第 `nval` 个自由变量**替换**成 `val`（不弹栈）
    pub fn set_free_variable(&mut self, closure: &Value, nval: usize, val: Value) -> Result<(), &'static str> {
        let c = match closure {
            Value::Closure(c) => c.clone(),
            _ => return Err("wrong type(expected class or instance)"),
        };
        let n = {
            let cb = c.borrow();
            cb.function.outervalues.len()
        };
        if nval >= n {
            return Err("index out of range");
        }
        let mut cb = c.borrow_mut();
        if nval < cb.outervalues.len() {
            cb.outervalues[nval] = val;
            Ok(())
        } else {
            Err("index out of range")
        }
    }
}

/// 供宿主 API 用的实例字段快照（↔ `SQInstance::_values`）
pub fn instance_values(inst: &InstanceRef) -> Vec<Value> {
    inst.borrow().values.clone()
}

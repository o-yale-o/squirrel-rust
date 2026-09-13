//! ↔ squirrel/sqfuncproto.h + sqfuncproto.cpp（Save/Load 在 M8 序列化时补）
//!
//! C++ 用「单块内存 + 手工指针叠放」布局；Rust 用分列 Vec，逻辑等价。

use crate::config::*;
use crate::object::Value;
use crate::opcodes::SQInstruction;

/// ↔ SQOuterType
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum SQOuterType {
    OtLocal = 0,
    OtOuter = 1,
}

/// ↔ SQOuterVar（_src 是捕获源：局部槽位号或外层序号，整数）
#[derive(Clone)]
pub struct SQOuterVar {
    pub otype: SQOuterType,
    pub name: Value,
    pub src: Value,
}

/// ↔ SQLocalVarInfo
#[derive(Clone)]
pub struct SQLocalVarInfo {
    pub name: Value,
    pub start_op: SQUnsignedInteger,
    pub end_op: SQUnsignedInteger,
    pub pos: SQUnsignedInteger,
}

impl SQLocalVarInfo {
    pub fn new() -> Self {
        SQLocalVarInfo {
            name: Value::Null,
            start_op: 0,
            end_op: 0,
            pos: 0,
        }
    }
}

/// ↔ SQLineInfo
#[derive(Clone, Copy)]
pub struct SQLineInfo {
    pub line: SQInteger,
    pub op: SQInteger,
}

pub struct SQFunctionProto {
    pub sourcename: Value,
    pub name: Value,
    pub stacksize: SQInteger,
    pub bgenerator: bool,
    pub varparams: SQInteger,

    pub localvarinfos: Vec<SQLocalVarInfo>,
    pub lineinfos: Vec<SQLineInfo>,
    pub literals: Vec<Value>,
    pub parameters: Vec<Value>,
    pub functions: Vec<Value>, // 子函数原型（Value::FuncProto）
    pub outervalues: Vec<SQOuterVar>,
    pub defaultparams: Vec<SQInteger>,
    pub instructions: Vec<SQInstruction>,
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

/// 迭代式释放：字面量/子原型里可能嵌着容器（见 crate::release）
impl Drop for SQFunctionProto {
    fn drop(&mut self) {
        let mut kids: Vec<Value> = std::mem::take(&mut self.literals);
        kids.append(&mut self.functions);
        kids.append(&mut self.parameters);
        kids.push(std::mem::replace(&mut self.name, Value::Null));
        kids.push(std::mem::replace(&mut self.sourcename, Value::Null));
        crate::release::release_values(kids);
    }
}

impl SQFunctionProto {    /// 构造空结构（字段由编译器随后填充）
    pub fn empty(sourcename: Value, name: Value) -> SQFunctionProto {
        SQFunctionProto {
            sourcename,
            name,
            stacksize: 0,
            bgenerator: false,
            varparams: 0,
            localvarinfos: Vec::new(),
            lineinfos: Vec::new(),
            literals: Vec::new(),
            parameters: Vec::new(),
            functions: Vec::new(),
            outervalues: Vec::new(),
            defaultparams: Vec::new(),
            instructions: Vec::new(),
            weakref_slot: std::cell::RefCell::new(None),
        }
    }

    /// 包装为 Rc 并注册进 GC 表（↔ ADD_TO_CHAIN）。注意 Weak 会阻止 Rc::get_mut，
    /// 因此调用方必须先填好字段再 register。
    pub fn register(
        ss: &crate::state::SharedState,
        proto: SQFunctionProto,
    ) -> crate::object::FuncProtoRef {
        let rc = crate::object::FuncProtoRef::new(proto);
        let entry = crate::state::GcEntry::FuncProto(std::rc::Rc::downgrade(&rc));
        ss.gc_add(entry);
        rc
    }

    /// ↔ SQFunctionProto::Create（字段由编译器随后填充；M8 序列化 Load 用）
    pub fn create(
        ss: &crate::state::SharedState,
        sourcename: Value,
        name: Value,
    ) -> crate::object::FuncProtoRef {
        SQFunctionProto::register(ss, SQFunctionProto::empty(sourcename, name))
    }

    /// ↔ SQFunctionProto::GetLine（二分查找行号，含末尾回退扫描）
    pub fn get_line(&self, curr_op: SQInteger) -> SQInteger {
        if self.lineinfos.is_empty() {
            return 0;
        }
        let op = curr_op;
        #[allow(unused_assignments)]
        let mut line = self.lineinfos[0].line;
        let mut low: SQInteger = 0;
        let mut high: SQInteger = self.lineinfos.len() as SQInteger - 1;
        let mut mid: SQInteger = 0;
        while low <= high {
            mid = low + ((high - low) >> 1);
            let curop = self.lineinfos[mid as usize].op;
            if curop > op {
                high = mid - 1;
            } else if curop < op {
                if mid < self.lineinfos.len() as SQInteger - 1
                    && self.lineinfos[mid as usize + 1].op >= op
                {
                    break;
                }
                low = mid + 1;
            } else {
                break;
            }
        }
        while mid > 0 && self.lineinfos[mid as usize].op >= op {
            mid -= 1;
        }
        line = self.lineinfos[mid as usize].line;
        line
    }

    /// ↔ SQFunctionProto::GetLocal：调试信息取局部变量（压栈 + 返回变量名）
    pub fn get_local(
        &self,
        vm: &mut crate::vm::VmData,
        stackbase: SQUnsignedInteger,
        mut nseq: SQUnsignedInteger,
        nop: SQUnsignedInteger,
    ) -> Option<String> {
        let nvars = self.localvarinfos.len() as SQUnsignedInteger;
        let mut res = None;
        if nvars >= nseq {
            for i in 0..nvars as usize {
                let (start_op, end_op, pos, name) = {
                    let lvi = &self.localvarinfos[i];
                    (lvi.start_op, lvi.end_op, lvi.pos, lvi.name.clone())
                };
                if start_op <= nop && end_op >= nop {
                    if nseq == 0 {
                        let val = vm.stack[(stackbase + pos) as usize].clone();
                        vm.push(val);
                        if let Value::String(s) = &name {
                            res = Some(String::from_utf8_lossy(&s.val).into_owned());
                        }
                        break;
                    }
                    nseq -= 1;
                }
            }
        }
        res
    }

    /// ↔ SQFunctionProto::Finalize
    pub fn finalize(&mut self) {
        for l in self.literals.iter_mut() {
            *l = Value::Null;
        }
    }
}

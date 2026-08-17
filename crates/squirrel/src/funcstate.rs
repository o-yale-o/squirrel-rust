//! ↔ squirrel/sqfuncstate.h + sqfuncstate.cpp
//!
//! 寄存器分配（_vlocals/_targetstack）、跳转回填、窥孔优化（AddInstruction 的
//! 指令合并）、BuildProto。**窥孔优化的每个分支都逐条对应 C++**，因为指令
//! 序列必须与原版完全一致（.cnut 互通与差分测试的前提）。
//!
//! C++ 的 SQFuncState 父子链（PushChildState/PopChildState）在 Rust 里由编译器
//! 持有的 Vec<FuncState> 栈表达，parent 通过切片尾递归访问。

use std::rc::Rc;

use crate::config::*;
use crate::funcproto::{SQFunctionProto, SQLineInfo, SQLocalVarInfo, SQOuterVar, SQOuterType};
use crate::object::{TableRef, Value};
use crate::opcodes::*;
use crate::state::SharedState;
use crate::table::SQTable;

pub const UINT_MINUS_ONE: SQUnsignedInteger = u64::MAX;

/// 编译期错误（C++ 经 CompilerErrorFunc + longjmp 抛出；Rust 走 Result）
#[derive(Debug, Clone)]
pub struct CompileError {
    pub msg: String,
}

fn cerr<T>(msg: &str) -> Result<T, CompileError> {
    Err(CompileError {
        msg: msg.to_string(),
    })
}

pub struct FuncState {
    pub returnexp: SQInteger,
    pub vlocals: Vec<SQLocalVarInfo>,
    pub targetstack: Vec<SQInteger>,
    pub stacksize: SQInteger,
    pub varparams: bool,
    pub bgenerator: bool,
    pub unresolvedbreaks: Vec<SQInteger>,
    pub unresolvedcontinues: Vec<SQInteger>,
    pub functions: Vec<Value>, // FuncProto
    pub parameters: Vec<Value>,
    pub outervalues: Vec<SQOuterVar>,
    pub instructions: Vec<SQInstruction>,
    pub localvarinfos: Vec<SQLocalVarInfo>,
    /// 字面量池：常量值 -> 索引（用 SQTable 复刻键语义）
    pub literals: TableRef,
    pub nliterals: SQInteger,
    /// ↔ _strings：编译期间持有已创建的字符串/表引用（防 GC；Rust 里仅保活）
    pub strings_keepalive: Vec<Value>,
    pub name: Value,
    pub sourcename: Value,
    pub lineinfos: Vec<SQLineInfo>,
    pub scope_blocks: Vec<SQInteger>,
    pub breaktargets: Vec<SQInteger>,
    pub continuetargets: Vec<SQInteger>,
    pub defaultparams: Vec<SQInteger>,
    pub lastline: SQInteger,
    pub traps: SQInteger,
    pub outers: SQInteger,
    optimization: bool,
    pub ss: Rc<SharedState>,
    /// 是否为根 FuncState（↔ _parent == NULL）
    pub is_root: bool,
}

impl FuncState {
    /// ↔ SQFuncState::SQFuncState
    pub fn new(ss: &Rc<SharedState>, is_root: bool) -> FuncState {
        FuncState {
            returnexp: 0,
            vlocals: Vec::new(),
            targetstack: Vec::new(),
            stacksize: 0,
            varparams: false,
            bgenerator: false,
            unresolvedbreaks: Vec::new(),
            unresolvedcontinues: Vec::new(),
            functions: Vec::new(),
            parameters: Vec::new(),
            outervalues: Vec::new(),
            instructions: Vec::new(),
            localvarinfos: Vec::new(),
            literals: SQTable::create(ss, 0),
            nliterals: 0,
            strings_keepalive: Vec::new(),
            name: Value::Null,
            sourcename: Value::Null,
            lineinfos: Vec::new(),
            scope_blocks: Vec::new(),
            breaktargets: Vec::new(),
            continuetargets: Vec::new(),
            defaultparams: Vec::new(),
            lastline: 0,
            traps: 0,
            outers: 0,
            optimization: true,
            ss: ss.clone(),
            is_root,
        }
    }

    pub fn add_instruction_raw(
        &mut self,
        op: Op,
        arg0: SQInteger,
        arg1: SQInteger,
        arg2: SQInteger,
        arg3: SQInteger,
    ) -> Result<(), CompileError> {
        let i = SQInstruction::new(op, arg0, arg1, arg2, arg3);
        self.add_instruction(i)
    }

    /// ↔ SQFuncState::AddInstruction（含窥孔优化）
    pub fn add_instruction(&mut self, mut i: SQInstruction) -> Result<(), CompileError> {
        let size = self.instructions.len();
        if size > 0 && self.optimization {
            let prev = self.instructions[size - 1]; // Copy 出来做判断，避免借用冲突
            match i.opcode() {
                Op::JZ => {
                    if prev.opcode() == Op::Cmp && prev.arg1 < 0xFF {
                        // CMP + JZ → JCMP
                        let pi = &mut self.instructions[size - 1];
                        pi.op = Op::JCmp as u8;
                        pi.arg0 = prev.arg1 as u8;
                        pi.arg1 = i.arg1;
                        return Ok(());
                    }
                }
                Op::Set | Op::NewSlot => {
                    if i.arg0 == i.arg3 {
                        i.arg0 = 0xFF;
                    }
                }
                Op::SetOuter => {
                    if i.arg0 == i.arg2 {
                        i.arg0 = 0xFF;
                    }
                }
                Op::Return => {
                    if !self.is_root
                        && i.arg0 != MAX_FUNC_STACKSIZE as u8
                        && prev.opcode() == Op::Call
                        && self.returnexp < size as SQInteger - 1
                    {
                        self.instructions[size - 1].op = Op::TailCall as u8;
                    } else if prev.opcode() == Op::Close {
                        self.instructions[size - 1] = i;
                        return Ok(());
                    }
                }
                Op::Get => {
                    if prev.opcode() == Op::Load
                        && prev.arg0 as SQInteger == i.arg2 as SQInteger
                        && !self.is_local(prev.arg0 as SQUnsignedInteger)
                    {
                        // LOAD + GET → GETK
                        let pi = &mut self.instructions[size - 1];
                        pi.arg2 = i.arg1 as u8;
                        pi.op = Op::GetK as u8;
                        pi.arg0 = i.arg0;
                        return Ok(());
                    }
                }
                Op::PrepCall => {
                    if prev.opcode() == Op::Load
                        && prev.arg0 as SQInteger == i.arg1 as SQInteger
                        && !self.is_local(prev.arg0 as SQUnsignedInteger)
                    {
                        let pi = &mut self.instructions[size - 1];
                        pi.op = Op::PrepCallK as u8;
                        pi.arg0 = i.arg0;
                        pi.arg2 = i.arg2;
                        pi.arg3 = i.arg3;
                        return Ok(());
                    }
                }
                Op::AppendArray => {
                    let aat: SQInteger = match prev.opcode() {
                        Op::Load => AAT_LITERAL as SQInteger,
                        Op::LoadInt => AAT_INT as SQInteger,
                        Op::LoadBool => AAT_BOOL as SQInteger,
                        Op::LoadFloat => AAT_FLOAT as SQInteger,
                        _ => -1,
                    };
                    if aat != -1
                        && prev.arg0 as SQInteger == i.arg1 as SQInteger
                        && !self.is_local(prev.arg0 as SQUnsignedInteger)
                    {
                        let pi = &mut self.instructions[size - 1];
                        pi.op = Op::AppendArray as u8;
                        pi.arg0 = i.arg0;
                        pi.arg2 = aat as u8;
                        pi.arg3 = MAX_FUNC_STACKSIZE as u8;
                        return Ok(());
                    }
                }
                Op::Move => {
                    match prev.opcode() {
                        Op::Get | Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod | Op::BitW
                        | Op::LoadInt | Op::LoadFloat | Op::LoadBool | Op::Load => {
                            if prev.arg0 as SQInteger == i.arg1 as SQInteger {
                                self.instructions[size - 1].arg0 = i.arg0;
                                self.optimization = false;
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                    if prev.opcode() == Op::Move {
                        let pi = &mut self.instructions[size - 1];
                        pi.op = Op::DMove as u8;
                        pi.arg2 = i.arg0;
                        pi.arg3 = i.arg1 as u8;
                        return Ok(());
                    }
                }
                Op::Load => {
                    if prev.opcode() == Op::Load && i.arg1 < 256 {
                        let pi = &mut self.instructions[size - 1];
                        pi.op = Op::DLoad as u8;
                        pi.arg2 = i.arg0;
                        pi.arg3 = i.arg1 as u8;
                        return Ok(());
                    }
                }
                Op::Eq | Op::Ne => {
                    if prev.opcode() == Op::Load
                        && prev.arg0 as SQInteger == i.arg1 as SQInteger
                        && !self.is_local(prev.arg0 as SQUnsignedInteger)
                    {
                        let pi = &mut self.instructions[size - 1];
                        pi.op = i.op;
                        pi.arg0 = i.arg0;
                        pi.arg2 = i.arg2;
                        pi.arg3 = MAX_FUNC_STACKSIZE as u8;
                        return Ok(());
                    }
                }
                Op::LoadNulls => {
                    if prev.opcode() == Op::LoadNulls
                        && prev.arg0 as SQInteger + prev.arg1 as SQInteger == i.arg0 as SQInteger
                    {
                        let pi = &mut self.instructions[size - 1];
                        pi.arg1 += 1;
                        pi.op = Op::LoadNulls as u8;
                        return Ok(());
                    }
                }
                Op::Line => {
                    if prev.opcode() == Op::Line {
                        self.instructions.pop();
                        self.lineinfos.pop();
                    }
                }
                _ => {}
            }
        }
        self.optimization = true;
        self.instructions.push(i);
        Ok(())
    }

    /// ↔ SetInstructionParams
    pub fn set_instruction_params(
        &mut self,
        pos: SQInteger,
        arg0: SQInteger,
        arg1: SQInteger,
        arg2: SQInteger,
        arg3: SQInteger,
    ) {
        let pi = &mut self.instructions[pos as usize];
        pi.arg0 = arg0 as u8;
        pi.arg1 = arg1 as SQInt32;
        pi.arg2 = arg2 as u8;
        pi.arg3 = arg3 as u8;
    }

    /// ↔ SetInstructionParam
    pub fn set_instruction_param(&mut self, pos: SQInteger, arg: SQInteger, val: SQInteger) {
        let pi = &mut self.instructions[pos as usize];
        match arg {
            0 => pi.arg0 = val as u8,
            1 | 4 => pi.arg1 = val as SQInt32,
            2 => pi.arg2 = val as u8,
            3 => pi.arg3 = val as u8,
            _ => {}
        }
    }

    pub fn get_instruction(&self, pos: SQInteger) -> SQInstruction {
        self.instructions[pos as usize]
    }

    pub fn pop_instructions(&mut self, size: SQInteger) {
        for _ in 0..size {
            self.instructions.pop();
        }
    }

    /// ↔ AllocStackPos
    fn alloc_stack_pos(&mut self) -> Result<SQInteger, CompileError> {
        let npos = self.vlocals.len() as SQInteger;
        self.vlocals.push(SQLocalVarInfo::new());
        if self.vlocals.len() as SQInteger > self.stacksize {
            if self.stacksize > MAX_FUNC_STACKSIZE as SQInteger {
                return cerr("internal compiler error: too many locals");
            }
            self.stacksize = self.vlocals.len() as SQInteger;
        }
        Ok(npos)
    }

    /// ↔ PushTarget
    pub fn push_target(&mut self, n: SQInteger) -> Result<SQInteger, CompileError> {
        if n != -1 {
            self.targetstack.push(n);
            return Ok(n);
        }
        let n = self.alloc_stack_pos()?;
        self.targetstack.push(n);
        Ok(n)
    }

    /// ↔ GetUpTarget
    pub fn get_up_target(&self, n: SQInteger) -> SQInteger {
        self.targetstack[(self.targetstack.len() as SQInteger - 1 - n) as usize]
    }

    /// ↔ TopTarget
    pub fn top_target(&self) -> SQInteger {
        *self.targetstack.last().unwrap()
    }

    /// ↔ PopTarget
    pub fn pop_target(&mut self) -> SQInteger {
        let npos = *self.targetstack.last().unwrap();
        let t_name_is_null = self.vlocals[npos as usize].name.is_null();
        if t_name_is_null {
            self.vlocals.pop();
        }
        self.targetstack.pop();
        npos
    }

    pub fn get_stack_size(&self) -> SQInteger {
        self.vlocals.len() as SQInteger
    }

    /// ↔ CountOuters
    pub fn count_outers(&self, stacksize: SQInteger) -> SQInteger {
        let mut outers = 0;
        let mut k = self.vlocals.len() as SQInteger - 1;
        while k >= stacksize {
            let lvi = &self.vlocals[k as usize];
            k -= 1;
            if lvi.end_op == UINT_MINUS_ONE {
                outers += 1;
            }
        }
        outers
    }

    /// ↔ SetStackSize
    pub fn set_stack_size(&mut self, n: SQInteger) {
        let mut size = self.vlocals.len() as SQInteger;
        while size > n {
            size -= 1;
            let lvi = self.vlocals.last().unwrap().clone();
            if !lvi.name.is_null() {
                let mut lvi = lvi;
                if lvi.end_op == UINT_MINUS_ONE {
                    self.outers -= 1;
                }
                lvi.end_op = self.get_current_pos().max(0) as SQUnsignedInteger;
                self.localvarinfos.push(lvi);
            }
            self.vlocals.pop();
        }
    }

    /// ↔ IsConstant（查编译期常量表）
    pub fn is_constant(&self, name: &Value) -> Option<Value> {
        if let Value::Table(t) = &*self.ss.consts.borrow() {
            t.borrow().get(name)
        } else {
            None
        }
    }

    /// ↔ IsLocal
    pub fn is_local(&self, stkpos: SQUnsignedInteger) -> bool {
        if stkpos >= self.vlocals.len() as SQUnsignedInteger {
            return false;
        }
        !self.vlocals[stkpos as usize].name.is_null()
    }

    /// ↔ PushLocalVariable
    pub fn push_local_variable(&mut self, name: Value) -> SQInteger {
        let pos = self.vlocals.len() as SQInteger;
        let mut lvi = SQLocalVarInfo::new();
        lvi.name = name;
        lvi.start_op = (self.get_current_pos() + 1).max(0) as SQUnsignedInteger;
        lvi.pos = self.vlocals.len() as SQUnsignedInteger;
        self.vlocals.push(lvi);
        if self.vlocals.len() as SQInteger > self.stacksize {
            self.stacksize = self.vlocals.len() as SQInteger;
        }
        pos
    }

    /// ↔ GetLocalVariable（从尾向头找；字符串指针相等）
    pub fn get_local_variable(&self, name: &Value) -> SQInteger {
        let mut locals = self.vlocals.len() as SQInteger;
        while locals >= 1 {
            let lvi = &self.vlocals[(locals - 1) as usize];
            if let (Value::String(a), Value::String(b)) = (&lvi.name, name) {
                if std::rc::Rc::ptr_eq(a, b) {
                    return locals - 1;
                }
            }
            locals -= 1;
        }
        -1
    }

    /// ↔ MarkLocalAsOuter
    pub fn mark_local_as_outer(&mut self, pos: SQInteger) {
        let lvi = &mut self.vlocals[pos as usize];
        lvi.end_op = UINT_MINUS_ONE;
        self.outers += 1;
    }

    /// ↔ AddParameter
    pub fn add_parameter(&mut self, name: Value) {
        self.push_local_variable(name.clone());
        self.parameters.push(name);
    }

    /// ↔ AddLineInfos
    pub fn add_line_infos(&mut self, line: SQInteger, lineop: bool, force: bool) -> Result<(), CompileError> {
        if self.lastline != line || force {
            let li = SQLineInfo {
                line,
                op: self.get_current_pos() + 1,
            };
            if lineop {
                self.add_instruction_raw(Op::Line, 0, line, 0, 0)?;
            }
            if self.lastline != line {
                self.lineinfos.push(li);
            }
            self.lastline = line;
        }
        Ok(())
    }

    /// ↔ DiscardTarget
    pub fn discard_target(&mut self) -> Result<(), CompileError> {
        let discardedtarget = self.pop_target();
        let size = self.instructions.len();
        if size > 0 && self.optimization {
            let pi = self.instructions.last_mut().unwrap();
            match pi.opcode() {
                Op::Set | Op::NewSlot | Op::SetOuter | Op::Call => {
                    if pi.arg0 as SQInteger == discardedtarget {
                        pi.arg0 = 0xFF;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn get_current_pos(&self) -> SQInteger {
        self.instructions.len() as SQInteger - 1
    }

    pub fn snooze_opt(&mut self) {
        self.optimization = false;
    }

    pub fn add_default_param(&mut self, trg: SQInteger) {
        self.defaultparams.push(trg);
    }

    pub fn get_default_param_count(&self) -> SQInteger {
        self.defaultparams.len() as SQInteger
    }

    /// ↔ GetNumericConstant / GetConstant
    pub fn get_numeric_constant_int(&mut self, cons: SQInteger) -> Result<SQInteger, CompileError> {
        self.get_constant(&Value::Integer(cons))
    }

    pub fn get_numeric_constant_float(&mut self, cons: SQFloat) -> Result<SQInteger, CompileError> {
        self.get_constant(&Value::Float(cons))
    }

    pub fn get_constant(&mut self, cons: &Value) -> Result<SQInteger, CompileError> {
        let existing = self.literals.borrow().get(cons);
        let val = match existing {
            Some(v) => v.as_integer(),
            None => {
                let val = self.nliterals;
                self.literals.borrow_mut().new_slot(cons, &Value::Integer(val));
                self.nliterals += 1;
                if self.nliterals > MAX_LITERALS {
                    return cerr("internal compiler error: too many literals");
                }
                val
            }
        };
        Ok(val)
    }

    /// ↔ CreateString（经 SharedState 实习）
    pub fn create_string(&mut self, s: &[u8]) -> Value {
        let ns = self.ss.intern(s);
        self.strings_keepalive.push(ns.clone());
        ns
    }

    /// ↔ CreateTable
    pub fn create_table(&mut self) -> Value {
        let nt = Value::Table(SQTable::create(&self.ss, 0));
        self.strings_keepalive.push(nt.clone());
        nt
    }

    /// ↔ BuildProto
    pub fn build_proto(&mut self) -> Result<Rc<SQFunctionProto>, CompileError> {
        let mut f = SQFunctionProto::empty(self.sourcename.clone(), self.name.clone());
        f.stacksize = self.stacksize;
        f.bgenerator = self.bgenerator;
        f.varparams = self.varparams as SQInteger;

        // 字面量按索引放置（遍历顺序无关）
        f.literals = vec![Value::Null; self.nliterals as usize];
        {
            let lit = self.literals.borrow();
            let mut refpos = Value::Null;
            while let Some((np, key, val)) = lit.next(false, &refpos) {
                f.literals[val.as_integer() as usize] = key;
                refpos = Value::Integer(np);
            }
        }
        f.functions = self.functions.clone();
        f.parameters = self.parameters.clone();
        f.outervalues = self.outervalues.clone();
        f.localvarinfos = self.localvarinfos.clone();
        f.lineinfos = self.lineinfos.clone();
        f.defaultparams = self.defaultparams.clone();
        f.instructions = self.instructions.clone();
        Ok(SQFunctionProto::register(&self.ss, f))
    }

    /// ↔ SQFuncState::GenerateCode（在编译器里调用；此处保留占位语义）
    pub fn generate_code(&mut self) -> Result<(), CompileError> {
        Ok(())
    }
}

/// ↔ GetOuterVariable：沿 FuncState 栈向上查找名字。
/// `states` 为编译器当前的函数状态栈（最后一个元素是当前函数）。
pub fn get_outer_variable(states: &mut [FuncState], name: &Value) -> Result<SQInteger, CompileError> {
    let outers = states.last().unwrap().outervalues.len() as SQInteger;
    for i in 0..outers as usize {
        if let (Value::String(a), Value::String(b)) = (&states.last().unwrap().outervalues[i].name, name) {
            if std::rc::Rc::ptr_eq(a, b) {
                return Ok(i as SQInteger);
            }
        }
    }
    if states.len() > 1 {
        let (cur, parents) = states.split_last_mut().unwrap();
        let pos = parents.last().unwrap().get_local_variable(name);
        if pos == -1 {
            let pos = get_outer_variable(parents, name)?;
            if pos == -1 {
                return Ok(-1); // ↔ C++：找不到时不注册，交由上层处理
            }
            cur.outervalues.push(SQOuterVar {
                otype: SQOuterType::OtOuter,
                name: name.clone(),
                src: Value::Integer(pos),
            });
            return Ok(cur.outervalues.len() as SQInteger - 1);
        } else {
            parents.last_mut().unwrap().mark_local_as_outer(pos);
            cur.outervalues.push(SQOuterVar {
                otype: SQOuterType::OtLocal,
                name: name.clone(),
                src: Value::Integer(pos),
            });
            return Ok(cur.outervalues.len() as SQInteger - 1);
        }
    }
    Ok(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::raw_eq;

    fn fs() -> FuncState {
        let ss = SharedState::new();
        FuncState::new(&ss, true)
    }

    #[test]
    fn peephole_cmp_jz_to_jcmp() {
        let mut f = fs();
        f.add_instruction_raw(Op::Cmp, 1, 3, 2, 0).unwrap(); // arg1=3 < 0xFF
        f.add_instruction_raw(Op::JZ, 5, 10, 0, 0).unwrap();
        assert_eq!(f.instructions.len(), 1);
        let i = f.instructions[0];
        assert_eq!(i.opcode(), Op::JCmp);
        assert_eq!(i.arg0, 3); // 原 CMP 的 arg1
        assert_eq!(i.arg1, 10); // JZ 的跳转偏移
    }

    #[test]
    fn peephole_load_get_to_getk() {
        let mut f = fs();
        // LOAD 目标寄存器必须不是局部变量（vlocals 该位置无名 → 成立）
        f.add_instruction_raw(Op::Load, 2, 7, 0, 0).unwrap();
        f.add_instruction_raw(Op::Get, 1, 2, 2, 0).unwrap();
        assert_eq!(f.instructions.len(), 1);
        let i = f.instructions[0];
        assert_eq!(i.opcode(), Op::GetK);
        assert_eq!(i.arg0, 1);
        assert_eq!(i.arg1, 7); // 字面量索引来自 LOAD 的 arg1
        assert_eq!(i.arg2, 2); // GET 的 arg1
    }

    #[test]
    fn peephole_move_move_to_dmove() {
        let mut f = fs();
        f.add_instruction_raw(Op::Move, 1, 2, 0, 0).unwrap();
        f.add_instruction_raw(Op::Move, 3, 4, 0, 0).unwrap();
        assert_eq!(f.instructions.len(), 1);
        let i = f.instructions[0];
        assert_eq!(i.opcode(), Op::DMove);
        assert_eq!(i.arg0, 1);
        assert_eq!(i.arg1, 2);
        assert_eq!(i.arg2, 3);
        assert_eq!(i.arg3, 4);
    }

    #[test]
    fn peephole_load_load_to_dload() {
        let mut f = fs();
        f.add_instruction_raw(Op::Load, 1, 5, 0, 0).unwrap();
        f.add_instruction_raw(Op::Load, 2, 6, 0, 0).unwrap(); // arg1 < 256
        assert_eq!(f.instructions.len(), 1);
        let i = f.instructions[0];
        assert_eq!(i.opcode(), Op::DLoad);
        assert_eq!(i.arg0, 1);
        assert_eq!(i.arg1, 5);
        assert_eq!(i.arg2, 2);
        assert_eq!(i.arg3, 6);
    }

    #[test]
    fn peephole_loadnulls_merge() {
        let mut f = fs();
        f.add_instruction_raw(Op::LoadNulls, 2, 1, 0, 0).unwrap();
        f.add_instruction_raw(Op::LoadNulls, 3, 1, 0, 0).unwrap(); // arg0 = 2+1
        assert_eq!(f.instructions.len(), 1);
        let i = f.instructions[0];
        assert_eq!(i.opcode(), Op::LoadNulls);
        assert_eq!(i.arg0, 2);
        assert_eq!(i.arg1, 2); // 合并后两条
    }

    #[test]
    fn register_targets() {
        let mut f = fs();
        let t0 = f.push_target(-1).unwrap();
        let t1 = f.push_target(-1).unwrap();
        assert_eq!(t0, 0);
        assert_eq!(t1, 1);
        assert_eq!(f.top_target(), 1);
        assert_eq!(f.get_up_target(1), 0);
        assert_eq!(f.pop_target(), 1);
        assert_eq!(f.pop_target(), 0);
        assert_eq!(f.vlocals.len(), 0); // 无名目标随 pop 移除
    }

    #[test]
    fn outer_variable_capture() {
        let ss = SharedState::new();
        let name = ss.intern(b"xx");
        let mut states = vec![FuncState::new(&ss, true), FuncState::new(&ss, false)];
        states[0].push_local_variable(name.clone());
        let idx = get_outer_variable(&mut states, &name).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(states[1].outervalues.len(), 1);
        assert_eq!(states[1].outervalues[0].otype, SQOuterType::OtLocal);
        assert_eq!(states[0].outers, 1); // 父函数标记为 outer
        assert_eq!(states[0].vlocals[0].end_op, UINT_MINUS_ONE);
    }

    #[test]
    fn build_proto_literals_by_index() {
        let mut f = fs();
        let i1 = f.get_numeric_constant_int(42).unwrap();
        let i2 = f.get_numeric_constant_int(99).unwrap();
        let i3 = f.get_numeric_constant_float(0.5).unwrap();
        assert_eq!((i1, i2, i3), (0, 1, 2));
        f.add_instruction_raw(Op::Load, 0, i1, 0, 0).unwrap();
        f.add_instruction_raw(Op::Return, 1, 0, 0, 0).unwrap();
        let p = f.build_proto().unwrap();
        assert_eq!(p.literals.len(), 3);
        assert!(raw_eq(&p.literals[0], &Value::Integer(42)));
        assert!(raw_eq(&p.literals[2], &Value::Float(0.5)));
        assert_eq!(p.instructions.len(), 2);
    }
}


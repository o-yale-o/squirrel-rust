//! ↔ squirrel/sqcompiler.cpp —— 单遍递归下降编译器（1:1 移植）。
//!
//! C++ 里错误经 longjmp 跳出；Rust 全部走 `Result<_, CompileError>`。
//! `_fs`（当前函数状态）对应 `self.fstates` 栈的最后一个元素；
//! BEGIN/END_SCOPE、BEGIN/END_BREAKBLE_BLOCK 宏展开为下面的内联代码。

use std::rc::Rc;

use crate::config::*;
use crate::funcstate::{get_outer_variable, CompileError, FuncState};
use crate::lexer::*;
use crate::object::Value;
use crate::opcodes::*;
use crate::state::SharedState;
use crate::vm::VmData;

const EXPR: SQInteger = 1;
const OBJECT: SQInteger = 2;
const BASE: SQInteger = 3;
const LOCAL: SQInteger = 4;
const OUTER: SQInteger = 5;

/// ↔ SQExpState
#[derive(Clone, Copy)]
struct ExpState {
    etype: SQInteger,
    epos: SQInteger,
    donot_get: bool,
}

impl Default for ExpState {
    fn default() -> Self {
        ExpState {
            etype: EXPR,
            epos: -1,
            donot_get: false,
        }
    }
}

/// ↔ SQScope
#[derive(Clone, Copy)]
struct Scope {
    outers: SQInteger,
    stacksize: SQInteger,
}

fn cerr<T>(msg: impl Into<String>) -> Result<T, CompileError> {
    Err(CompileError { msg: msg.into() })
}

pub struct Compiler {
    token: Token,
    fstates: Vec<FuncState>,
    sourcename: Value,
    lex: SqlLexer,
    lineinfo: bool,
    raiseerror: bool,
    es: ExpState,
    scope: Scope,
    ss: Rc<SharedState>,
    /// 参数解析期间暂存默认参数槽位（create_function 用）
    pending_default_params: Vec<SQInteger>,
}

/// ↔ bool Compile(SQVM*, SQLEXREADFUNC, up, sourcename, out, raiseerror, lineinfo)
/// 返回 Err(()) 时 vm.lasterror 已设置、compilererrorhandler 已调用。
pub fn compile(
    vm: &mut VmData,
    readf: Box<dyn FnMut() -> SQInteger>,
    sourcename: &[u8],
    raiseerror: bool,
    lineinfo: bool,
) -> Result<Value, ()> {
    let ss = vm.ss.clone();
    let sourcename_val = ss.intern(sourcename);
    let mut c = Compiler {
        token: 0,
        fstates: Vec::new(),
        sourcename: sourcename_val.clone(),
        lex: SqlLexer::new(readf),
        lineinfo,
        raiseerror,
        es: ExpState::default(),
        scope: Scope {
            outers: 0,
            stacksize: 0,
        },
        ss,
        pending_default_params: Vec::new(),
    };
    match c.run() {
        Ok(proto) => Ok(proto),
        Err(e) => {
            if c.raiseerror {
                if let Some(handler) = &*c.ss.compilererrorhandler.borrow() {
                    let srcname = match &sourcename_val {
                        Value::String(s) => String::from_utf8_lossy(&s.val).into_owned(),
                        _ => "unknown".to_string(),
                    };
                    handler(vm, &e.msg, &srcname, c.lex.currentline, c.lex.currentcolumn);
                }
            }
            vm.lasterror = c.ss.intern(e.msg.as_bytes());
            Err(())
        }
    }
}

impl Compiler {
    fn fs(&mut self) -> &mut FuncState {
        self.fstates.last_mut().unwrap()
    }

    fn run(&mut self) -> Result<Value, CompileError> {
        // ↔ SQCompiler::Compile
        self.fstates.push(FuncState::new(&self.ss, true));
        let main_name = self.ss.intern(b"main");
        let srcname = self.sourcename.clone();
        {
            let fs = self.fs();
            fs.name = main_name;
            let this_name = fs.create_string(b"this");
            fs.add_parameter(this_name);
            let vargv = fs.create_string(b"vargv");
            fs.add_parameter(vargv);
            fs.varparams = true;
            fs.sourcename = srcname;
        }
        let stacksize = self.fs().get_stack_size();

        self.lex_token()?;
        while self.token > 0 {
            self.statement(true)?;
            if self.lex.prevtoken != b'}' as Token && self.lex.prevtoken != b';' as Token {
                self.optional_semicolon()?;
            }
        }
        {
            let fs = self.fs();
            fs.set_stack_size(stacksize);
        }
        let curline = self.lex.currentline;
        let li = self.lineinfo;
        self.fs().add_line_infos(curline, li, true)?;
        self.fs().add_instruction_raw(Op::Return, 0xFF, 0, 0, 0)?;
        self.fs().set_stack_size(0);
        let proto = self.fs().build_proto()?;
        Ok(Value::FuncProto(proto))
    }

    // ---------- 基础 ----------

    /// ↔ void Lex()
    fn lex_token(&mut self) -> Result<(), CompileError> {
        self.token = self
            .lex
            .lex()
            .map_err(|e| CompileError { msg: e.msg })?;
        Ok(())
    }

    /// ↔ SQObject Expect(SQInteger tok)
    fn expect(&mut self, tok: Token) -> Result<Value, CompileError> {
        if self.token != tok {
            if !(self.token == TK_CONSTRUCTOR && tok == TK_IDENTIFIER) {
                if tok > 255 {
                    let etypename = match tok {
                        TK_IDENTIFIER => "IDENTIFIER",
                        TK_STRING_LITERAL => "STRING_LITERAL",
                        TK_INTEGER => "INTEGER",
                        TK_FLOAT => "FLOAT",
                        _ => SqlLexer::tok2str(tok).unwrap_or("unknown token"),
                    };
                    return cerr(format!("expected '{}'", etypename));
                } else {
                    return cerr(format!("expected '{}'", tok as u8 as char));
                }
            }
        }
        let ret = match tok {
            TK_IDENTIFIER | TK_STRING_LITERAL => {
                let sv = self.lex.svalue().to_vec();
                self.fs().create_string(&sv)
            }
            TK_INTEGER => Value::Integer(self.lex.nvalue),
            TK_FLOAT => Value::Float(self.lex.fvalue),
            _ => Value::Null,
        };
        self.lex_token()?;
        Ok(ret)
    }

    /// ↔ IsEndOfStatement
    fn is_end_of_statement(&self) -> bool {
        self.lex.prevtoken == b'\n' as Token
            || self.token == SQUIRREL_EOB as Token
            || self.token == b'}' as Token
            || self.token == b';' as Token
    }

    fn optional_semicolon(&mut self) -> Result<(), CompileError> {
        if self.token == b';' as Token {
            self.lex_token()?;
            return Ok(());
        }
        if !self.is_end_of_statement() {
            return cerr("end of statement expected (; or lf)");
        }
        Ok(())
    }

    fn move_if_current_target_is_local(&mut self) -> Result<(), CompileError> {
        let trg = self.fs().top_target();
        if self.fs().is_local(trg as SQUnsignedInteger) {
            let trg = self.fs().pop_target();
            let dst = self.fs().push_target(-1)?;
            self.fs().add_instruction_raw(Op::Move, dst, trg, 0, 0)?;
        }
        Ok(())
    }

    // ---------- 语句 ----------

    /// ↔ Statements()
    fn statements(&mut self) -> Result<(), CompileError> {
        while self.token != b'}' as Token && self.token != TK_DEFAULT && self.token != TK_CASE {
            self.statement(true)?;
            if self.lex.prevtoken != b'}' as Token && self.lex.prevtoken != b';' as Token {
                self.optional_semicolon()?;
            }
        }
        Ok(())
    }

    /// ↔ Statement(bool closeframe)
    fn statement(&mut self, closeframe: bool) -> Result<(), CompileError> {
        let curline = self.lex.currentline;
        let li = self.lineinfo;
        self.fs().add_line_infos(curline, li, false)?;
        match self.token {
            t if t == b';' as Token => {
                self.lex_token()?;
            }
            TK_IF => self.if_statement()?,
            TK_WHILE => self.while_statement()?,
            TK_DO => self.do_while_statement()?,
            TK_FOR => self.for_statement()?,
            TK_FOREACH => self.for_each_statement()?,
            TK_SWITCH => self.switch_statement()?,
            TK_LOCAL => self.local_decl_statement()?,
            TK_RETURN | TK_YIELD => {
                let op = if self.token == TK_RETURN {
                    Op::Return
                } else {
                    self.fs().bgenerator = true;
                    Op::Yield
                };
                self.lex_token()?;
                if !self.is_end_of_statement() {
                    let retexp = self.fs().get_current_pos() + 1;
                    self.comma_expr()?;
                    let traps = self.fs().traps;
                    if op == Op::Return && traps > 0 {
                        self.fs().add_instruction_raw(Op::PopTrap, traps, 0, 0, 0)?;
                    }
                    // ↔ MSVC 参数从右到左求值：GetStackSize 先于 PopTarget
                    let ssz = self.fs().get_stack_size();
                    let popped = self.fs().pop_target();
                    self.fs().returnexp = retexp;
                    self.fs().add_instruction_raw(op, 1, popped, ssz, 0)?;
                } else {
                    let traps = self.fs().traps;
                    if op == Op::Return && traps > 0 {
                        self.fs().add_instruction_raw(Op::PopTrap, traps, 0, 0, 0)?;
                    }
                    self.fs().returnexp = -1;
                    let ssz = self.fs().get_stack_size();
                    self.fs().add_instruction_raw(op, 0xFF, 0, ssz, 0)?;
                }
            }
            TK_BREAK => {
                if self.fs().breaktargets.is_empty() {
                    return cerr("'break' has to be in a loop block");
                }
                let top = *self.fs().breaktargets.last().unwrap();
                if top > 0 {
                    self.fs().add_instruction_raw(Op::PopTrap, top, 0, 0, 0)?;
                }
                // ↔ RESOLVE_OUTERS()
                self.resolve_outers()?;
                self.fs().add_instruction_raw(Op::Jmp, 0, -1234, 0, 0)?;
                let pos = self.fs().get_current_pos();
                self.fs().unresolvedbreaks.push(pos);
                self.lex_token()?;
            }
            TK_CONTINUE => {
                if self.fs().continuetargets.is_empty() {
                    return cerr("'continue' has to be in a loop block");
                }
                let top = *self.fs().continuetargets.last().unwrap();
                if top > 0 {
                    self.fs().add_instruction_raw(Op::PopTrap, top, 0, 0, 0)?;
                }
                self.resolve_outers()?;
                self.fs().add_instruction_raw(Op::Jmp, 0, -1234, 0, 0)?;
                let pos = self.fs().get_current_pos();
                self.fs().unresolvedcontinues.push(pos);
                self.lex_token()?;
            }
            TK_FUNCTION => self.function_statement()?,
            TK_CLASS => self.class_statement()?,
            TK_ENUM => self.enum_statement()?,
            t if t == b'{' as Token => {
                // ↔ BEGIN_SCOPE()
                let oldscope = self.begin_scope();
                self.lex_token()?;
                self.statements()?;
                self.expect(b'}' as Token)?;
                if closeframe {
                    self.end_scope(oldscope)?;
                } else {
                    self.end_scope_no_close(oldscope);
                }
            }
            TK_TRY => self.try_catch_statement()?,
            TK_THROW => {
                self.lex_token()?;
                self.comma_expr()?;
                let arg = self.fs().pop_target();
                self.fs().add_instruction_raw(Op::Throw, arg, 0, 0, 0)?;
            }
            TK_CONST => {
                self.lex_token()?;
                let id = self.expect(TK_IDENTIFIER)?;
                self.expect(b'=' as Token)?;
                let val = self.expect_scalar()?;
                self.optional_semicolon()?;
                if let Value::Table(enums) = &*self.ss.consts.borrow() {
                    enums.borrow_mut().new_slot(&id, &val);
                }
            }
            _ => {
                self.comma_expr()?;
                self.fs().discard_target()?;
            }
        }
        self.fs().snooze_opt();
        Ok(())
    }

    // ↔ RESOLVE_OUTERS 宏
    fn resolve_outers(&mut self) -> Result<(), CompileError> {
        let scope = self.scope;
        if self.fs().get_stack_size() != scope.stacksize {
            let n = self.fs().count_outers(scope.stacksize);
            if n > 0 {
                self.fs().add_instruction_raw(Op::Close, 0, scope.stacksize, 0, 0)?;
            }
        }
        Ok(())
    }

    // ↔ BEGIN_SCOPE 宏（返回 __oldscope__）
    fn begin_scope(&mut self) -> Scope {
        let oldscope = self.scope;
        self.scope.outers = self.fs().outers;
        self.scope.stacksize = self.fs().get_stack_size();
        oldscope
    }

    // ↔ END_SCOPE_NO_CLOSE 宏
    fn end_scope_no_close(&mut self, oldscope: Scope) {
        if self.fs().get_stack_size() != self.scope.stacksize {
            let ssz = self.scope.stacksize;
            self.fs().set_stack_size(ssz);
        }
        self.scope = oldscope;
    }

    // ↔ END_SCOPE 宏
    fn end_scope(&mut self, oldscope: Scope) -> Result<(), CompileError> {
        let oldouters = self.fs().outers;
        if self.fs().get_stack_size() != self.scope.stacksize {
            let ssz = self.scope.stacksize;
            self.fs().set_stack_size(ssz);
            if oldouters != self.fs().outers {
                self.fs().add_instruction_raw(Op::Close, 0, ssz, 0, 0)?;
            }
        }
        self.scope = oldscope;
        Ok(())
    }

    // ---------- 表达式 ----------

    /// ↔ EmitDerefOp
    fn emit_deref_op(&mut self, op: Op) -> Result<(), CompileError> {
        let val = self.fs().pop_target();
        let key = self.fs().pop_target();
        let src = self.fs().pop_target();
        let trg = self.fs().push_target(-1)?;
        self.fs().add_instruction_raw(op, trg, src, key, val)?;
        Ok(())
    }

    /// ↔ Emit2ArgsOP
    fn emit_2args_op(&mut self, op: Op, p3: SQInteger) -> Result<(), CompileError> {
        let p2 = self.fs().pop_target();
        let p1 = self.fs().pop_target();
        let trg = self.fs().push_target(-1)?;
        self.fs().add_instruction_raw(op, trg, p1, p2, p3)?;
        Ok(())
    }

    /// ↔ EmitCompoundArith
    fn emit_compound_arith(&mut self, tok: Token, etype: SQInteger, pos: SQInteger) -> Result<(), CompileError> {
        match etype {
            LOCAL => {
                let p2 = self.fs().pop_target();
                let p1 = self.fs().pop_target();
                self.fs().push_target(p1)?;
                self.fs().add_instruction_raw(choose_arith_op_by_token(tok), p1, p2, p1, 0)?;
                self.fs().snooze_opt();
            }
            OBJECT | BASE => {
                let val = self.fs().pop_target();
                let key = self.fs().pop_target();
                let src = self.fs().pop_target();
                // ↔ _OP_COMPARITH mixes dest obj and source val in the arg1
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(
                    Op::Comparith,
                    trg,
                    (src << 16) | val,
                    key,
                    choose_comp_arith_char_by_token(tok),
                )?;
            }
            OUTER => {
                let val = self.fs().top_target();
                let tmp = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::GetOuter, tmp, pos, 0, 0)?;
                self.fs().add_instruction_raw(choose_arith_op_by_token(tok), tmp, val, tmp, 0)?;
                self.fs().pop_target();
                self.fs().pop_target();
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::SetOuter, trg, pos, tmp, 0)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// ↔ CommaExpr
    fn comma_expr(&mut self) -> Result<(), CompileError> {
        self.expression()?;
        while self.token == b',' as Token {
            self.fs().pop_target();
            self.lex_token()?;
            self.comma_expr()?;
        }
        Ok(())
    }

    /// ↔ Expression
    fn expression(&mut self) -> Result<(), CompileError> {
        let es = self.es;
        self.es.etype = EXPR;
        self.es.epos = -1;
        self.es.donot_get = false;
        self.logical_or_exp()?;
        match self.token {
            t if t == b'=' as Token
                || t == TK_NEWSLOT
                || t == TK_MINUSEQ
                || t == TK_PLUSEQ
                || t == TK_MULEQ
                || t == TK_DIVEQ
                || t == TK_MODEQ =>
            {
                let op = t;
                let ds = self.es.etype;
                let pos = self.es.epos;
                if ds == EXPR {
                    return cerr("can't assign expression");
                } else if ds == BASE {
                    return cerr("'base' cannot be modified");
                }
                self.lex_token()?;
                self.expression()?;

                match op {
                    TK_NEWSLOT => {
                        if ds == OBJECT || ds == BASE {
                            self.emit_deref_op(Op::NewSlot)?;
                        } else {
                            return cerr("can't 'create' a local slot");
                        }
                    }
                    t if t == b'=' as Token => match ds {
                        LOCAL => {
                            let src = self.fs().pop_target();
                            let dst = self.fs().top_target();
                            self.fs().add_instruction_raw(Op::Move, dst, src, 0, 0)?;
                        }
                        OBJECT | BASE => {
                            self.emit_deref_op(Op::Set)?;
                        }
                        OUTER => {
                            let src = self.fs().pop_target();
                            let dst = self.fs().push_target(-1)?;
                            self.fs().add_instruction_raw(Op::SetOuter, dst, pos, src, 0)?;
                        }
                        _ => {}
                    },
                    TK_MINUSEQ | TK_PLUSEQ | TK_MULEQ | TK_DIVEQ | TK_MODEQ => {
                        self.emit_compound_arith(op, ds, pos)?;
                    }
                    _ => {}
                }
            }
            t if t == b'?' as Token => {
                self.lex_token()?;
                let popped = self.fs().pop_target();
                self.fs().add_instruction_raw(Op::JZ, popped, 0, 0, 0)?;
                let jzpos = self.fs().get_current_pos();
                let trg = self.fs().push_target(-1)?;
                self.expression()?;
                let first_exp = self.fs().pop_target();
                if trg != first_exp {
                    self.fs().add_instruction_raw(Op::Move, trg, first_exp, 0, 0)?;
                }
                let endfirstexp = self.fs().get_current_pos();
                self.fs().add_instruction_raw(Op::Jmp, 0, 0, 0, 0)?;
                self.expect(b':' as Token)?;
                let jmppos = self.fs().get_current_pos();
                self.expression()?;
                let second_exp = self.fs().pop_target();
                if trg != second_exp {
                    self.fs().add_instruction_raw(Op::Move, trg, second_exp, 0, 0)?;
                }
                let cur = self.fs().get_current_pos();
                self.fs().set_instruction_param(jmppos, 1, cur - jmppos);
                self.fs().set_instruction_param(jzpos, 1, endfirstexp - jzpos + 1);
                self.fs().snooze_opt();
            }
            _ => {}
        }
        self.es = es;
        Ok(())
    }

    /// ↔ INVOKE_EXP
    fn invoke_exp(
        &mut self,
        f: fn(&mut Compiler) -> Result<(), CompileError>,
    ) -> Result<(), CompileError> {
        let es = self.es;
        self.es.etype = EXPR;
        self.es.epos = -1;
        self.es.donot_get = false;
        f(self)?;
        self.es = es;
        Ok(())
    }

    /// ↔ BIN_EXP
    fn bin_exp(
        &mut self,
        op: Op,
        f: fn(&mut Compiler) -> Result<(), CompileError>,
        op3: SQInteger,
    ) -> Result<(), CompileError> {
        self.lex_token()?;
        self.invoke_exp(f)?;
        let op1 = self.fs().pop_target();
        let op2 = self.fs().pop_target();
        let trg = self.fs().push_target(-1)?;
        self.fs().add_instruction_raw(op, trg, op1, op2, op3)?;
        self.es.etype = EXPR;
        Ok(())
    }

    fn logical_or_exp(&mut self) -> Result<(), CompileError> {
        self.logical_and_exp()?;
        loop {
            if self.token == TK_OR {
                let first_exp = self.fs().pop_target();
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::Or, trg, 0, first_exp, 0)?;
                let jpos = self.fs().get_current_pos();
                if trg != first_exp {
                    self.fs().add_instruction_raw(Op::Move, trg, first_exp, 0, 0)?;
                }
                self.lex_token()?;
                self.invoke_exp(Compiler::logical_or_exp)?;
                self.fs().snooze_opt();
                let second_exp = self.fs().pop_target();
                if trg != second_exp {
                    self.fs().add_instruction_raw(Op::Move, trg, second_exp, 0, 0)?;
                }
                self.fs().snooze_opt();
                let cur = self.fs().get_current_pos();
                self.fs().set_instruction_param(jpos, 1, cur - jpos);
                self.es.etype = EXPR;
                break;
            } else {
                return Ok(());
            }
        }
        Ok(())
    }

    fn logical_and_exp(&mut self) -> Result<(), CompileError> {
        self.bitwise_or_exp()?;
        loop {
            if self.token == TK_AND {
                let first_exp = self.fs().pop_target();
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::And, trg, 0, first_exp, 0)?;
                let jpos = self.fs().get_current_pos();
                if trg != first_exp {
                    self.fs().add_instruction_raw(Op::Move, trg, first_exp, 0, 0)?;
                }
                self.lex_token()?;
                self.invoke_exp(Compiler::logical_and_exp)?;
                self.fs().snooze_opt();
                let second_exp = self.fs().pop_target();
                if trg != second_exp {
                    self.fs().add_instruction_raw(Op::Move, trg, second_exp, 0, 0)?;
                }
                self.fs().snooze_opt();
                let cur = self.fs().get_current_pos();
                self.fs().set_instruction_param(jpos, 1, cur - jpos);
                self.es.etype = EXPR;
            } else {
                return Ok(());
            }
        }
    }

    fn bitwise_or_exp(&mut self) -> Result<(), CompileError> {
        self.bitwise_xor_exp()?;
        loop {
            if self.token == b'|' as Token {
                self.bin_exp(Op::BitW, Compiler::bitwise_xor_exp, BW_OR)?;
            } else {
                return Ok(());
            }
        }
    }

    fn bitwise_xor_exp(&mut self) -> Result<(), CompileError> {
        self.bitwise_and_exp()?;
        loop {
            if self.token == b'^' as Token {
                self.bin_exp(Op::BitW, Compiler::bitwise_and_exp, BW_XOR)?;
            } else {
                return Ok(());
            }
        }
    }

    fn bitwise_and_exp(&mut self) -> Result<(), CompileError> {
        self.eq_exp()?;
        loop {
            if self.token == b'&' as Token {
                self.bin_exp(Op::BitW, Compiler::eq_exp, BW_AND)?;
            } else {
                return Ok(());
            }
        }
    }

    fn eq_exp(&mut self) -> Result<(), CompileError> {
        self.comp_exp()?;
        loop {
            match self.token {
                TK_EQ => self.bin_exp(Op::Eq, Compiler::comp_exp, 0)?,
                TK_NE => self.bin_exp(Op::Ne, Compiler::comp_exp, 0)?,
                TK_3WAYSCMP => self.bin_exp(Op::Cmp, Compiler::comp_exp, CMP_3W)?,
                _ => return Ok(()),
            }
        }
    }

    fn comp_exp(&mut self) -> Result<(), CompileError> {
        self.shift_exp()?;
        loop {
            match self.token {
                t if t == b'>' as Token => self.bin_exp(Op::Cmp, Compiler::shift_exp, CMP_G)?,
                t if t == b'<' as Token => self.bin_exp(Op::Cmp, Compiler::shift_exp, CMP_L)?,
                TK_GE => self.bin_exp(Op::Cmp, Compiler::shift_exp, CMP_GE)?,
                TK_LE => self.bin_exp(Op::Cmp, Compiler::shift_exp, CMP_LE)?,
                TK_IN => self.bin_exp(Op::Exists, Compiler::shift_exp, 0)?,
                TK_INSTANCEOF => self.bin_exp(Op::InstanceOf, Compiler::shift_exp, 0)?,
                _ => return Ok(()),
            }
        }
    }

    fn shift_exp(&mut self) -> Result<(), CompileError> {
        self.plus_exp()?;
        loop {
            match self.token {
                TK_USHIFTR => self.bin_exp(Op::BitW, Compiler::plus_exp, BW_USHIFTR)?,
                TK_SHIFTL => self.bin_exp(Op::BitW, Compiler::plus_exp, BW_SHIFTL)?,
                TK_SHIFTR => self.bin_exp(Op::BitW, Compiler::plus_exp, BW_SHIFTR)?,
                _ => return Ok(()),
            }
        }
    }

    fn plus_exp(&mut self) -> Result<(), CompileError> {
        self.mult_exp()?;
        loop {
            match self.token {
                t if t == b'+' as Token || t == b'-' as Token => {
                    self.bin_exp(choose_arith_op_by_token(t), Compiler::mult_exp, 0)?
                }
                _ => return Ok(()),
            }
        }
    }

    fn mult_exp(&mut self) -> Result<(), CompileError> {
        self.prefixed_expr()?;
        loop {
            match self.token {
                t if t == b'*' as Token || t == b'/' as Token || t == b'%' as Token => {
                    self.bin_exp(choose_arith_op_by_token(t), Compiler::prefixed_expr_unit, 0)?
                }
                _ => return Ok(()),
            }
        }
    }

    fn prefixed_expr_unit(&mut self) -> Result<(), CompileError> {
        self.prefixed_expr()?;
        Ok(())
    }

    /// ↔ PrefixedExpr（返回 pos）
    fn prefixed_expr(&mut self) -> Result<SQInteger, CompileError> {
        let mut pos = self.factor()?;
        loop {
            match self.token {
                t if t == b'.' as Token => {
                    pos = -1;
                    self.lex_token()?;
                    let id = self.expect(TK_IDENTIFIER)?;
                    let lit = self.fs().get_constant(&id)?;
                    let trg = self.fs().push_target(-1)?;
                    self.fs().add_instruction_raw(Op::Load, trg, lit, 0, 0)?;
                    if self.es.etype == BASE {
                        self.emit_2args_op(Op::Get, 0)?;
                        pos = self.fs().top_target();
                        self.es.etype = EXPR;
                        self.es.epos = pos;
                    } else {
                        if self.need_get() {
                            self.emit_2args_op(Op::Get, 0)?;
                        }
                        self.es.etype = OBJECT;
                    }
                }
                t if t == b'[' as Token => {
                    if self.lex.prevtoken == b'\n' as Token {
                        return cerr(
                            "cannot break deref/or comma needed after [exp]=exp slot declaration",
                        );
                    }
                    self.lex_token()?;
                    self.expression()?;
                    self.expect(b']' as Token)?;
                    pos = -1;
                    if self.es.etype == BASE {
                        self.emit_2args_op(Op::Get, 0)?;
                        pos = self.fs().top_target();
                        self.es.etype = EXPR;
                        self.es.epos = pos;
                    } else {
                        if self.need_get() {
                            self.emit_2args_op(Op::Get, 0)?;
                        }
                        self.es.etype = OBJECT;
                    }
                }
                TK_MINUSMINUS | TK_PLUSPLUS => {
                    if self.is_end_of_statement() {
                        return Ok(pos);
                    }
                    let diff: SQInteger = if self.token == TK_MINUSMINUS { -1 } else { 1 };
                    self.lex_token()?;
                    match self.es.etype {
                        EXPR => return cerr("can't '++' or '--' an expression"),
                        BASE => return cerr("'base' cannot be modified"),
                        OBJECT => {
                            if self.es.donot_get {
                                return cerr("can't '++' or '--' an expression");
                            }
                            self.emit_2args_op(Op::PInc, diff)?;
                        }
                        LOCAL => {
                            let src = self.fs().pop_target();
                            let trg = self.fs().push_target(-1)?;
                            self.fs().add_instruction_raw(Op::PIncL, trg, src, 0, diff)?;
                        }
                        OUTER => {
                            let epos = self.es.epos;
                            let tmp1 = self.fs().push_target(-1)?;
                            let tmp2 = self.fs().push_target(-1)?;
                            self.fs().add_instruction_raw(Op::GetOuter, tmp2, epos, 0, 0)?;
                            self.fs().add_instruction_raw(Op::PIncL, tmp1, tmp2, 0, diff)?;
                            self.fs().add_instruction_raw(Op::SetOuter, tmp2, epos, tmp2, 0)?;
                            self.fs().pop_target();
                        }
                        _ => {}
                    }
                    self.es.etype = EXPR;
                    return Ok(pos);
                }
                t if t == b'(' as Token => {
                    match self.es.etype {
                        OBJECT => {
                            let key = self.fs().pop_target();
                            let table = self.fs().pop_target();
                            let closure = self.fs().push_target(-1)?;
                            let ttarget = self.fs().push_target(-1)?;
                            self.fs().add_instruction_raw(Op::PrepCall, closure, key, table, ttarget)?;
                        }
                        BASE => {
                            let trg = self.fs().push_target(-1)?;
                            self.fs().add_instruction_raw(Op::Move, trg, 0, 0, 0)?;
                        }
                        OUTER => {
                            let epos = self.es.epos;
                            let trg = self.fs().push_target(-1)?;
                            self.fs().add_instruction_raw(Op::GetOuter, trg, epos, 0, 0)?;
                            let trg2 = self.fs().push_target(-1)?;
                            self.fs().add_instruction_raw(Op::Move, trg2, 0, 0, 0)?;
                        }
                        _ => {
                            let trg = self.fs().push_target(-1)?;
                            self.fs().add_instruction_raw(Op::Move, trg, 0, 0, 0)?;
                        }
                    }
                    self.es.etype = EXPR;
                    self.lex_token()?;
                    self.function_call_args(false)?;
                }
                _ => return Ok(pos),
            }
        }
    }

    /// ↔ Factor（返回 pos；副作用：设置 _es）
    fn factor(&mut self) -> Result<SQInteger, CompileError> {
        match self.token {
            TK_STRING_LITERAL => {
                let sv = self.lex.svalue().to_vec();
                let s = self.fs().create_string(&sv);
                let lit = self.fs().get_constant(&s)?;
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::Load, trg, lit, 0, 0)?;
                self.lex_token()?;
            }
            TK_BASE => {
                self.lex_token()?;
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::GetBase, trg, 0, 0, 0)?;
                self.es.etype = BASE;
                self.es.epos = self.fs().top_target();
                return Ok(self.es.epos);
            }
            TK_IDENTIFIER | TK_CONSTRUCTOR | TK_THIS => {
                let id = match self.token {
                    TK_IDENTIFIER => {
                        let sv = self.lex.svalue().to_vec();
                        self.fs().create_string(&sv)
                    }
                    TK_THIS => self.fs().create_string(b"this"),
                    TK_CONSTRUCTOR => self.fs().create_string(b"constructor"),
                    _ => unreachable!(),
                };
                self.lex_token()?;
                let mut pos = self.fs().get_local_variable(&id);
                if pos != -1 {
                    // 局部变量（含 this）
                    self.fs().push_target(pos)?;
                    self.es.etype = LOCAL;
                    self.es.epos = pos;
                } else {
                    pos = get_outer_variable(&mut self.fstates, &id)?;
                    if pos != -1 {
                        // 自由变量
                        if self.need_get() {
                            self.es.epos = self.fs().push_target(-1)?;
                            let epos = self.es.epos;
                            self.fs().add_instruction_raw(Op::GetOuter, epos, pos, 0, 0)?;
                            // _es.etype 保持 EXPR（默认值）
                        } else {
                            self.es.etype = OUTER;
                            self.es.epos = pos;
                        }
                    } else if let Some(constant) = self.fs().is_constant(&id) {
                        // 命名常量
                        let constval;
                        if let Value::Table(ct) = &constant {
                            self.expect(b'.' as Token)?;
                            let constid = self.expect(TK_IDENTIFIER)?;
                            let v = ct.borrow().get(&constid);
                            match v {
                                Some(v) => constval = v,
                                None => {
                                    return cerr(format!(
                                        "invalid constant [{}.{}]",
                                        String::from_utf8_lossy(id.as_bytes()),
                                        String::from_utf8_lossy(constid.as_bytes())
                                    ))
                                }
                            }
                        } else {
                            constval = constant;
                        }
                        self.es.epos = self.fs().push_target(-1)?;
                        let epos = self.es.epos;
                        match &constval {
                            Value::Integer(i) => self.emit_load_const_int(*i, epos)?,
                            Value::Float(f) => self.emit_load_const_float(*f, epos)?,
                            Value::Bool(b) => {
                                let bval = if *b { 1 } else { 0 };
                                self.fs().add_instruction_raw(Op::LoadBool, epos, bval, 0, 0)?;
                            }
                            _ => {
                                let lit = self.fs().get_constant(&constval)?;
                                self.fs().add_instruction_raw(Op::Load, epos, lit, 0, 0)?;
                            }
                        }
                        self.es.etype = EXPR;
                    } else {
                        // 字段访问：压入 this 指针（槽 0）+ 键
                        self.fs().push_target(0)?;
                        let lit = self.fs().get_constant(&id)?;
                        let trg = self.fs().push_target(-1)?;
                        self.fs().add_instruction_raw(Op::Load, trg, lit, 0, 0)?;
                        if self.need_get() {
                            self.emit_2args_op(Op::Get, 0)?;
                        }
                        self.es.etype = OBJECT;
                    }
                }
                return Ok(self.es.epos);
            }
            TK_DOUBLE_COLON => {
                // "::"
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::LoadRoot, trg, 0, 0, 0)?;
                self.es.etype = OBJECT;
                self.token = b'.' as Token; // hack: drop into PrefixExpr, case '.'
                self.es.epos = -1;
                return Ok(self.es.epos);
            }
            TK_NULL => {
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::LoadNulls, trg, 1, 0, 0)?;
                self.lex_token()?;
            }
            TK_INTEGER => {
                let v = self.lex.nvalue;
                self.emit_load_const_int(v, -1)?;
                self.lex_token()?;
            }
            TK_FLOAT => {
                let v = self.lex.fvalue;
                self.emit_load_const_float(v, -1)?;
                self.lex_token()?;
            }
            TK_TRUE | TK_FALSE => {
                let b = if self.token == TK_TRUE { 1 } else { 0 };
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::LoadBool, trg, b, 0, 0)?;
                self.lex_token()?;
            }
            t if t == b'[' as Token => {
                // 数组
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::NewObj, trg, 0, 0, NOT_ARRAY)?;
                let apos = self.fs().get_current_pos();
                let mut key = 0;
                self.lex_token()?;
                while self.token != b']' as Token {
                    self.expression()?;
                    if self.token == b',' as Token {
                        self.lex_token()?;
                    }
                    let val = self.fs().pop_target();
                    let array = self.fs().top_target();
                    self.fs().add_instruction_raw(Op::AppendArray, array, val, AAT_STACK as SQInteger, 0)?;
                    key += 1;
                }
                self.fs().set_instruction_param(apos, 1, key);
                self.lex_token()?;
            }
            t if t == b'{' as Token => {
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::NewObj, trg, 0, 0, NOT_TABLE)?;
                self.lex_token()?;
                self.parse_table_or_class(b',' as Token, b'}' as Token)?;
            }
            TK_FUNCTION => self.function_exp(false)?,
            t if t == b'@' as Token => self.function_exp(true)?,
            TK_CLASS => {
                self.lex_token()?;
                self.class_exp()?;
            }
            t if t == b'-' as Token => {
                self.lex_token()?;
                match self.token {
                    TK_INTEGER => {
                        let v = -self.lex.nvalue;
                        self.emit_load_const_int(v, -1)?;
                        self.lex_token()?;
                    }
                    TK_FLOAT => {
                        let v = -self.lex.fvalue;
                        self.emit_load_const_float(v, -1)?;
                        self.lex_token()?;
                    }
                    _ => self.unary_op(Op::Neg)?,
                }
            }
            t if t == b'!' as Token => {
                self.lex_token()?;
                self.unary_op(Op::Not)?;
            }
            t if t == b'~' as Token => {
                self.lex_token()?;
                if self.token == TK_INTEGER {
                    let v = !self.lex.nvalue;
                    self.emit_load_const_int(v, -1)?;
                    self.lex_token()?;
                } else {
                    self.unary_op(Op::BwNot)?;
                }
            }
            TK_TYPEOF => {
                self.lex_token()?;
                self.unary_op(Op::TypeOf)?;
            }
            TK_RESUME => {
                self.lex_token()?;
                self.unary_op(Op::Resume)?;
            }
            TK_CLONE => {
                self.lex_token()?;
                self.unary_op(Op::Clone)?;
            }
            TK_RAWCALL => {
                self.lex_token()?;
                self.expect(b'(' as Token)?;
                self.function_call_args(true)?;
            }
            TK_MINUSMINUS | TK_PLUSPLUS => {
                let t = self.token;
                self.prefix_inc_dec(t)?;
            }
            TK_DELETE => {
                self.delete_expr()?;
            }
            t if t == b'(' as Token => {
                self.lex_token()?;
                self.comma_expr()?;
                self.expect(b')' as Token)?;
            }
            TK___LINE__ => {
                let line = self.lex.currentline;
                self.emit_load_const_int(line, -1)?;
                self.lex_token()?;
            }
            TK___FILE__ => {
                let s = self.sourcename.clone();
                let lit = self.fs().get_constant(&s)?;
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::Load, trg, lit, 0, 0)?;
                self.lex_token()?;
            }
            _ => return cerr("expression expected"),
        }
        self.es.etype = EXPR;
        Ok(-1)
    }

    /// ↔ EmitLoadConstInt
    fn emit_load_const_int(&mut self, value: SQInteger, target: SQInteger) -> Result<(), CompileError> {
        let target = if target < 0 {
            self.fs().push_target(-1)?
        } else {
            target
        };
        if value <= i32::MAX as SQInteger && value > i32::MIN as SQInteger {
            self.fs().add_instruction_raw(Op::LoadInt, target, value, 0, 0)?;
        } else {
            let lit = self.fs().get_numeric_constant_int(value)?;
            self.fs().add_instruction_raw(Op::Load, target, lit, 0, 0)?;
        }
        Ok(())
    }

    /// ↔ EmitLoadConstFloat
    fn emit_load_const_float(&mut self, value: SQFloat, target: SQInteger) -> Result<(), CompileError> {
        let target = if target < 0 {
            self.fs().push_target(-1)?
        } else {
            target
        };
        // sizeof(SQFloat)==sizeof(SQInt32)：直接编码位模式
        let bits = value.to_bits() as SQInteger;
        self.fs().add_instruction_raw(Op::LoadFloat, target, bits, 0, 0)?;
        Ok(())
    }

    /// ↔ UnaryOP
    fn unary_op(&mut self, op: Op) -> Result<(), CompileError> {
        self.prefixed_expr()?;
        if self.fs().targetstack.is_empty() {
            return cerr("cannot evaluate unary operator");
        }
        let src = self.fs().pop_target();
        let trg = self.fs().push_target(-1)?;
        self.fs().add_instruction_raw(op, trg, src, 0, 0)?;
        Ok(())
    }

    /// ↔ NeedGet
    fn need_get(&self) -> bool {
        match self.token {
            t if t == b'=' as Token
                || t == b'(' as Token
                || t == TK_NEWSLOT
                || t == TK_MODEQ
                || t == TK_MULEQ
                || t == TK_DIVEQ
                || t == TK_MINUSEQ
                || t == TK_PLUSEQ =>
            {
                return false;
            }
            TK_PLUSPLUS | TK_MINUSMINUS => {
                if !self.is_end_of_statement() {
                    return false;
                }
            }
            _ => {}
        }
        !self.es.donot_get
            || (self.es.donot_get && (self.token == b'.' as Token || self.token == b'[' as Token))
    }

    /// ↔ FunctionCallArgs
    fn function_call_args(&mut self, rawcall: bool) -> Result<(), CompileError> {
        let mut nargs: SQInteger = 1; // this
        while self.token != b')' as Token {
            self.expression()?;
            self.move_if_current_target_is_local()?;
            nargs += 1;
            if self.token == b',' as Token {
                self.lex_token()?;
                if self.token == b')' as Token {
                    return cerr("expression expected, found ')'");
                }
            }
        }
        self.lex_token()?;
        if rawcall {
            if nargs < 3 {
                return cerr("rawcall requires at least 2 parameters (callee and this)");
            }
            nargs -= 2; // 移除 callee 与 this
        }
        for _ in 0..(nargs - 1) {
            self.fs().pop_target();
        }
        let stackbase = self.fs().pop_target();
        let closure = self.fs().pop_target();
        let trg = self.fs().push_target(-1)?;
        self.fs().add_instruction_raw(Op::Call, trg, closure, stackbase, nargs)?;
        // 调用后缀 {..} 具名参数块
        if self.token == b'{' as Token {
            let retval = self.fs().top_target();
            // nkeys 供 C++ 的 SetInstructionParam 使用（Rust 在表分支里用）
            #[allow(unused_assignments, unused_variables)]
            let mut nkeys = 0;
            self.lex_token()?;
            while self.token != b'}' as Token {
                match self.token {
                    t if t == b'[' as Token => {
                        self.lex_token()?;
                        self.comma_expr()?;
                        self.expect(b']' as Token)?;
                        self.expect(b'=' as Token)?;
                        self.expression()?;
                    }
                    _ => {
                        let id = self.expect(TK_IDENTIFIER)?;
                        let lit = self.fs().get_constant(&id)?;
                        let trg = self.fs().push_target(-1)?;
                        self.fs().add_instruction_raw(Op::Load, trg, lit, 0, 0)?;
                        self.expect(b'=' as Token)?;
                        self.expression()?;
                    }
                }
                if self.token == b',' as Token {
                    self.lex_token()?;
                }
                #[allow(unused_assignments)]
                {
                    nkeys += 1;
                }
                let val = self.fs().pop_target();
                let key = self.fs().pop_target();
                self.fs().add_instruction_raw(Op::Set, 0xFF, retval, key, val)?;
            }
            self.lex_token()?;
        }
        Ok(())
    }

    /// ↔ ParseTableOrClass
    fn parse_table_or_class(&mut self, separator: Token, terminator: Token) -> Result<(), CompileError> {
        let tpos = self.fs().get_current_pos();
        let mut nkeys = 0;
        while self.token != terminator {
            let mut hasattrs = false;
            let mut isstatic = false;
            if separator == b';' as Token {
                if self.token == TK_ATTR_OPEN {
                    let trg = self.fs().push_target(-1)?;
                    self.fs().add_instruction_raw(Op::NewObj, trg, 0, 0, NOT_TABLE)?;
                    self.lex_token()?;
                    self.parse_table_or_class(b',' as Token, TK_ATTR_CLOSE)?;
                    hasattrs = true;
                }
                if self.token == TK_STATIC {
                    isstatic = true;
                    self.lex_token()?;
                }
            }
            let mut is_json_string = false;
            match self.token {
                TK_FUNCTION | TK_CONSTRUCTOR => {
                    let tk = self.token;
                    self.lex_token()?;
                    let id = if tk == TK_FUNCTION {
                        self.expect(TK_IDENTIFIER)?
                    } else {
                        self.fs().create_string(b"constructor")
                    };
                    let lit = self.fs().get_constant(&id)?;
                    let trg = self.fs().push_target(-1)?;
                    self.fs().add_instruction_raw(Op::Load, trg, lit, 0, 0)?;
                    let mut boundtarget = 0xFF;
                    if self.token == b'[' as Token {
                        boundtarget = self.parse_bind_env()?;
                    }
                    self.expect(b'(' as Token)?;
                    self.create_function(&id, boundtarget, false)?;
                    let fidx = self.fs().functions.len() as SQInteger - 1;
                    let trg = self.fs().push_target(-1)?;
                    self.fs().add_instruction_raw(Op::Closure, trg, fidx, boundtarget, 0)?;
                }
                t if t == b'[' as Token => {
                    self.lex_token()?;
                    self.comma_expr()?;
                    self.expect(b']' as Token)?;
                    self.expect(b'=' as Token)?;
                    self.expression()?;
                }
                TK_STRING_LITERAL if separator == b',' as Token => {
                    // JSON 风格字符串键（仅表）
                    is_json_string = true;
                    let s = self.expect(TK_STRING_LITERAL)?;
                    let lit = self.fs().get_constant(&s)?;
                    let trg = self.fs().push_target(-1)?;
                    self.fs().add_instruction_raw(Op::Load, trg, lit, 0, 0)?;
                    self.expect(b':' as Token)?;
                    self.expression()?;
                }
                _ => {
                    let id = self.expect(TK_IDENTIFIER)?;
                    let lit = self.fs().get_constant(&id)?;
                    let trg = self.fs().push_target(-1)?;
                    self.fs().add_instruction_raw(Op::Load, trg, lit, 0, 0)?;
                    self.expect(b'=' as Token)?;
                    self.expression()?;
                }
            }
            let _ = is_json_string;
            if self.token == separator {
                self.lex_token()?;
            }
            nkeys += 1;
            let val = self.fs().pop_target();
            let key = self.fs().pop_target();
            if hasattrs {
                self.fs().pop_target(); // attrs（值在 key-1，与 C++ 断言一致）
            }
            let flags: SQInteger = (if hasattrs { NEW_SLOT_ATTRIBUTES_FLAG } else { 0 } as SQInteger)
                | (if isstatic { NEW_SLOT_STATIC_FLAG } else { 0 } as SQInteger);
            let table = self.fs().top_target();
            if separator == b',' as Token {
                self.fs().add_instruction_raw(Op::NewSlot, 0xFF, table, key, val)?;
            } else {
                self.fs().add_instruction_raw(Op::NewSlotA, flags, table, key, val)?;
            }
        }
        if separator == b',' as Token {
            self.fs().set_instruction_param(tpos, 1, nkeys);
        }
        self.lex_token()?;
        Ok(())
    }

    /// ↔ LocalDeclStatement
    fn local_decl_statement(&mut self) -> Result<(), CompileError> {
        self.lex_token()?;
        if self.token == TK_FUNCTION {
            let mut boundtarget = 0xFF;
            self.lex_token()?;
            let varname = self.expect(TK_IDENTIFIER)?;
            if self.token == b'[' as Token {
                boundtarget = self.parse_bind_env()?;
            }
            self.expect(b'(' as Token)?;
            self.create_function(&varname, 0xFF, false)?;
            let fidx = self.fs().functions.len() as SQInteger - 1;
            let trg = self.fs().push_target(-1)?;
            self.fs().add_instruction_raw(Op::Closure, trg, fidx, boundtarget, 0)?;
            self.fs().pop_target();
            self.fs().push_local_variable(varname);
            return Ok(());
        }

        loop {
            let varname = self.expect(TK_IDENTIFIER)?;
            if self.token == b'=' as Token {
                self.lex_token()?;
                self.expression()?;
                let src = self.fs().pop_target();
                let dest = self.fs().push_target(-1)?;
                if dest != src {
                    if self.fs().is_local(src as SQUnsignedInteger) {
                        self.fs().snooze_opt();
                    }
                    self.fs().add_instruction_raw(Op::Move, dest, src, 0, 0)?;
                }
            } else {
                let trg = self.fs().push_target(-1)?;
                self.fs().add_instruction_raw(Op::LoadNulls, trg, 1, 0, 0)?;
            }
            self.fs().pop_target();
            self.fs().push_local_variable(varname);
            if self.token == b',' as Token {
                self.lex_token()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// ↔ IfBlock
    fn if_block(&mut self) -> Result<(), CompileError> {
        if self.token == b'{' as Token {
            let oldscope = self.begin_scope();
            self.lex_token()?;
            self.statements()?;
            self.expect(b'}' as Token)?;
            self.end_scope(oldscope)?;
        } else {
            self.statement(true)?;
            if self.lex.prevtoken != b'}' as Token && self.lex.prevtoken != b';' as Token {
                self.optional_semicolon()?;
            }
        }
        Ok(())
    }

    /// ↔ IfStatement
    fn if_statement(&mut self) -> Result<(), CompileError> {
        #[allow(unused_assignments)]
        let mut jmppos = 0;
        let mut haselse = false;
        self.lex_token()?;
        self.expect(b'(' as Token)?;
        self.comma_expr()?;
        self.expect(b')' as Token)?;
        let popped = self.fs().pop_target();
        self.fs().add_instruction_raw(Op::JZ, popped, 0, 0, 0)?;
        let jnepos = self.fs().get_current_pos();

        self.if_block()?;

        let endifblock = self.fs().get_current_pos();
        if self.token == TK_ELSE {
            haselse = true;
            self.fs().add_instruction_raw(Op::Jmp, 0, 0, 0, 0)?;
            jmppos = self.fs().get_current_pos();
            self.lex_token()?;
            self.if_block()?;
            let cur = self.fs().get_current_pos();
            self.fs().set_instruction_param(jmppos, 1, cur - jmppos);
        }
        self.fs()
            .set_instruction_param(jnepos, 1, endifblock - jnepos + if haselse { 1 } else { 0 });
        Ok(())
    }

    /// ↔ WhileStatement
    fn while_statement(&mut self) -> Result<(), CompileError> {
        let jmppos = self.fs().get_current_pos();
        self.lex_token()?;
        self.expect(b'(' as Token)?;
        self.comma_expr()?;
        self.expect(b')' as Token)?;

        // ↔ BEGIN_BREAKBLE_BLOCK()
        let nbreaks = self.fs().unresolvedbreaks.len();
        let ncontinues = self.fs().unresolvedcontinues.len();
        self.fs().breaktargets.push(0);
        self.fs().continuetargets.push(0);

        let popped = self.fs().pop_target();
        self.fs().add_instruction_raw(Op::JZ, popped, 0, 0, 0)?;
        let jzpos = self.fs().get_current_pos();
        let oldscope = self.begin_scope();

        self.statement(true)?;

        self.end_scope(oldscope)?;
        let cur = self.fs().get_current_pos();
        self.fs().add_instruction_raw(Op::Jmp, 0, jmppos - cur - 1, 0, 0)?;
        let cur2 = self.fs().get_current_pos();
        self.fs().set_instruction_param(jzpos, 1, cur2 - jzpos);

        // ↔ END_BREAKBLE_BLOCK(jmppos)
        self.end_breakble_block(nbreaks, ncontinues, jmppos)?;
        Ok(())
    }

    /// ↔ END_BREAKBLE_BLOCK 宏
    fn end_breakble_block(
        &mut self,
        nbreaks: usize,
        ncontinues: usize,
        continue_target: SQInteger,
    ) -> Result<(), CompileError> {
        let nbreaks = self.fs().unresolvedbreaks.len() - nbreaks;
        let ncontinues = self.fs().unresolvedcontinues.len() - ncontinues;
        if ncontinues > 0 {
            self.resolve_continues(ncontinues as SQInteger, continue_target);
        }
        if nbreaks > 0 {
            self.resolve_breaks(nbreaks as SQInteger);
        }
        self.fs().breaktargets.pop();
        self.fs().continuetargets.pop();
        Ok(())
    }

    /// ↔ DoWhileStatement
    fn do_while_statement(&mut self) -> Result<(), CompileError> {
        self.lex_token()?;
        let jmptrg = self.fs().get_current_pos();
        let nbreaks = self.fs().unresolvedbreaks.len();
        let ncontinues = self.fs().unresolvedcontinues.len();
        self.fs().breaktargets.push(0);
        self.fs().continuetargets.push(0);
        let oldscope = self.begin_scope();
        self.statement(true)?;
        self.end_scope(oldscope)?;
        self.expect(TK_WHILE)?;
        let continuetrg = self.fs().get_current_pos();
        self.expect(b'(' as Token)?;
        self.comma_expr()?;
        self.expect(b')' as Token)?;
        let popped = self.fs().pop_target();
        self.fs().add_instruction_raw(Op::JZ, popped, 1, 0, 0)?;
        let cur = self.fs().get_current_pos();
        self.fs().add_instruction_raw(Op::Jmp, 0, jmptrg - cur - 1, 0, 0)?;
        self.end_breakble_block(nbreaks, ncontinues, continuetrg)?;
        Ok(())
    }

    /// ↔ ForStatement
    fn for_statement(&mut self) -> Result<(), CompileError> {
        self.lex_token()?;
        let oldscope = self.begin_scope();
        self.expect(b'(' as Token)?;
        if self.token == TK_LOCAL {
            self.local_decl_statement()?;
        } else if self.token != b';' as Token {
            self.comma_expr()?;
            self.fs().pop_target();
        }
        self.expect(b';' as Token)?;
        self.fs().snooze_opt();
        let jmppos = self.fs().get_current_pos();
        let mut jzpos = -1;
        if self.token != b';' as Token {
            self.comma_expr()?;
            let popped = self.fs().pop_target();
            self.fs().add_instruction_raw(Op::JZ, popped, 0, 0, 0)?;
            jzpos = self.fs().get_current_pos();
        }
        self.expect(b';' as Token)?;
        self.fs().snooze_opt();
        let expstart = self.fs().get_current_pos() + 1;
        if self.token != b')' as Token {
            self.comma_expr()?;
            self.fs().pop_target();
        }
        self.expect(b')' as Token)?;
        self.fs().snooze_opt();
        let expend = self.fs().get_current_pos();
        let expsize = (expend - expstart) + 1;
        let mut exp: Vec<SQInstruction> = Vec::new();
        if expsize > 0 {
            for i in 0..expsize {
                exp.push(self.fs().get_instruction(expstart + i));
            }
            self.fs().pop_instructions(expsize);
        }
        let nbreaks = self.fs().unresolvedbreaks.len();
        let ncontinues = self.fs().unresolvedcontinues.len();
        self.fs().breaktargets.push(0);
        self.fs().continuetargets.push(0);
        self.statement(true)?;
        let continuetrg = self.fs().get_current_pos();
        if expsize > 0 {
            for i in 0..expsize {
                let instr = exp[i as usize];
                self.fs().add_instruction(instr)?;
            }
        }
        let cur = self.fs().get_current_pos();
        self.fs().add_instruction_raw(Op::Jmp, 0, jmppos - cur - 1, 0, 0)?;
        if jzpos > 0 {
            let cur2 = self.fs().get_current_pos();
            self.fs().set_instruction_param(jzpos, 1, cur2 - jzpos);
        }
        self.end_breakble_block(nbreaks, ncontinues, continuetrg)?;
        self.end_scope(oldscope)?;
        Ok(())
    }

    /// ↔ ForEachStatement
    fn for_each_statement(&mut self) -> Result<(), CompileError> {
        self.lex_token()?;
        self.expect(b'(' as Token)?;
        let mut valname = self.expect(TK_IDENTIFIER)?;
        let idxname;
        if self.token == b',' as Token {
            idxname = valname;
            self.lex_token()?;
            valname = self.expect(TK_IDENTIFIER)?;
        } else {
            idxname = self.fs().create_string(b"@INDEX@");
        }
        self.expect(TK_IN)?;

        let oldscope = self.begin_scope();
        self.expression()?;
        self.expect(b')' as Token)?;
        let container = self.fs().top_target();
        // 索引局部变量
        let indexpos = self.fs().push_local_variable(idxname);
        self.fs().add_instruction_raw(Op::LoadNulls, indexpos, 1, 0, 0)?;
        // 值局部变量
        let valuepos = self.fs().push_local_variable(valname);
        self.fs().add_instruction_raw(Op::LoadNulls, valuepos, 1, 0, 0)?;
        // 引用索引（不可访问的内部名）
        let itrname = self.fs().create_string(b"@ITERATOR@");
        let itrpos = self.fs().push_local_variable(itrname);
        self.fs().add_instruction_raw(Op::LoadNulls, itrpos, 1, 0, 0)?;
        let jmppos = self.fs().get_current_pos();
        self.fs().add_instruction_raw(Op::Foreach, container, 0, indexpos, 0)?;
        let foreachpos = self.fs().get_current_pos();
        self.fs().add_instruction_raw(Op::PostForeach, container, 0, indexpos, 0)?;

        let nbreaks = self.fs().unresolvedbreaks.len();
        let ncontinues = self.fs().unresolvedcontinues.len();
        self.fs().breaktargets.push(0);
        self.fs().continuetargets.push(0);
        self.statement(true)?;
        let cur = self.fs().get_current_pos();
        self.fs().add_instruction_raw(Op::Jmp, 0, jmppos - cur - 1, 0, 0)?;
        let cur2 = self.fs().get_current_pos();
        self.fs().set_instruction_param(foreachpos, 1, cur2 - foreachpos);
        self.fs().set_instruction_param(foreachpos + 1, 1, cur2 - foreachpos);
        self.end_breakble_block(nbreaks, ncontinues, foreachpos - 1)?;
        self.fs().pop_target();
        self.end_scope(oldscope)?;
        Ok(())
    }

    /// ↔ SwitchStatement
    fn switch_statement(&mut self) -> Result<(), CompileError> {
        self.lex_token()?;
        self.expect(b'(' as Token)?;
        self.comma_expr()?;
        self.expect(b')' as Token)?;
        self.expect(b'{' as Token)?;
        let expr = self.fs().top_target();
        let mut bfirst = true;
        let mut tonextcondjmp = -1;
        let mut skipcondjmp = -1;
        let nbreaks = self.fs().unresolvedbreaks.len();
        self.fs().breaktargets.push(0);
        while self.token == TK_CASE {
            if !bfirst {
                self.fs().add_instruction_raw(Op::Jmp, 0, 0, 0, 0)?;
                skipcondjmp = self.fs().get_current_pos();
                self.fs().set_instruction_param(tonextcondjmp, 1, skipcondjmp - tonextcondjmp);
            }
            // condition
            self.lex_token()?;
            self.expression()?;
            self.expect(b':' as Token)?;
            let trg = self.fs().pop_target();
            let mut eqtarget = trg;
            let local = self.fs().is_local(trg as SQUnsignedInteger);
            if local {
                eqtarget = self.fs().push_target(-1)?;
            }
            self.fs().add_instruction_raw(Op::Eq, eqtarget, trg, expr, 0)?;
            self.fs().add_instruction_raw(Op::JZ, eqtarget, 0, 0, 0)?;
            if local {
                self.fs().pop_target();
            }
            // end condition
            if skipcondjmp != -1 {
                let cur = self.fs().get_current_pos();
                self.fs().set_instruction_param(skipcondjmp, 1, cur - skipcondjmp);
            }
            tonextcondjmp = self.fs().get_current_pos();
            let oldscope = self.begin_scope();
            self.statements()?;
            self.end_scope(oldscope)?;
            bfirst = false;
        }
        if tonextcondjmp != -1 {
            let cur = self.fs().get_current_pos();
            self.fs().set_instruction_param(tonextcondjmp, 1, cur - tonextcondjmp);
        }
        if self.token == TK_DEFAULT {
            self.lex_token()?;
            self.expect(b':' as Token)?;
            let oldscope = self.begin_scope();
            self.statements()?;
            self.end_scope(oldscope)?;
        }
        self.expect(b'}' as Token)?;
        self.fs().pop_target();
        let nbreaks = self.fs().unresolvedbreaks.len() - nbreaks;
        if nbreaks > 0 {
            self.resolve_breaks(nbreaks as SQInteger);
        }
        self.fs().breaktargets.pop();
        Ok(())
    }

    /// ↔ FunctionStatement
    fn function_statement(&mut self) -> Result<(), CompileError> {
        self.lex_token()?;
        let mut id = self.expect(TK_IDENTIFIER)?;
        let _ = self.fs().push_target(0);
        let lit = self.fs().get_constant(&id)?;
        let trg = self.fs().push_target(-1)?;
        self.fs().add_instruction_raw(Op::Load, trg, lit, 0, 0)?;
        if self.token == TK_DOUBLE_COLON {
            self.emit_2args_op(Op::Get, 0)?;
        }
        while self.token == TK_DOUBLE_COLON {
            self.lex_token()?;
            id = self.expect(TK_IDENTIFIER)?;
            let lit = self.fs().get_constant(&id)?;
            let trg = self.fs().push_target(-1)?;
            self.fs().add_instruction_raw(Op::Load, trg, lit, 0, 0)?;
            if self.token == TK_DOUBLE_COLON {
                self.emit_2args_op(Op::Get, 0)?;
            }
        }
        let mut boundtarget = 0xFF;
        if self.token == b'[' as Token {
            boundtarget = self.parse_bind_env()?;
        }
        self.expect(b'(' as Token)?;
        self.create_function(&id, boundtarget, false)?;
        let fidx = self.fs().functions.len() as SQInteger - 1;
        let trg = self.fs().push_target(-1)?;
        self.fs().add_instruction_raw(Op::Closure, trg, fidx, boundtarget, 0)?;
        self.emit_deref_op(Op::NewSlot)?;
        self.fs().pop_target();
        Ok(())
    }

    /// ↔ ClassStatement
    fn class_statement(&mut self) -> Result<(), CompileError> {
        self.lex_token()?;
        let es = self.es;
        self.es.donot_get = true;
        self.prefixed_expr()?;
        if self.es.etype == EXPR {
            return cerr("invalid class name");
        } else if self.es.etype == OBJECT || self.es.etype == BASE {
            self.class_exp()?;
            self.emit_deref_op(Op::NewSlot)?;
            self.fs().pop_target();
        } else {
            return cerr("cannot create a class in a local with the syntax(class <local>)");
        }
        self.es = es;
        Ok(())
    }

    /// ↔ ExpectScalar
    fn expect_scalar(&mut self) -> Result<Value, CompileError> {
        let val = match self.token {
            TK_INTEGER => Value::Integer(self.lex.nvalue),
            TK_FLOAT => Value::Float(self.lex.fvalue),
            TK_STRING_LITERAL => {
                let sv = self.lex.svalue().to_vec();
                self.fs().create_string(&sv)
            }
            TK_TRUE | TK_FALSE => Value::Bool(self.token == TK_TRUE),
            t if t == b'-' as Token => {
                self.lex_token()?;
                match self.token {
                    TK_INTEGER => Value::Integer(-self.lex.nvalue),
                    TK_FLOAT => Value::Float(-self.lex.fvalue),
                    _ => return cerr("scalar expected : integer, float"),
                }
            }
            _ => return cerr("scalar expected : integer, float, or string"),
        };
        self.lex_token()?;
        Ok(val)
    }

    /// ↔ EnumStatement
    fn enum_statement(&mut self) -> Result<(), CompileError> {
        self.lex_token()?;
        let id = self.expect(TK_IDENTIFIER)?;
        self.expect(b'{' as Token)?;

        let table = self.fs().create_table();
        let mut nval = 0;
        while self.token != b'}' as Token {
            let key = self.expect(TK_IDENTIFIER)?;
            let val;
            if self.token == b'=' as Token {
                self.lex_token()?;
                val = self.expect_scalar()?;
            } else {
                val = Value::Integer(nval);
                nval += 1;
            }
            if let Value::Table(t) = &table {
                t.borrow_mut().new_slot(&key, &val);
            }
            if self.token == b',' as Token {
                self.lex_token()?;
            }
        }
        if let Value::Table(enums) = &*self.ss.consts.borrow() {
            enums.borrow_mut().new_slot(&id, &table);
        }
        self.lex_token()?;
        Ok(())
    }

    /// ↔ TryCatchStatement
    fn try_catch_statement(&mut self) -> Result<(), CompileError> {
        self.lex_token()?;
        self.fs().add_instruction_raw(Op::PushTrap, 0, 0, 0, 0)?;
        self.fs().traps += 1;
        if !self.fs().breaktargets.is_empty() {
            let n = self.fs().breaktargets.len() - 1;
            self.fs().breaktargets[n] += 1;
        }
        if !self.fs().continuetargets.is_empty() {
            let n = self.fs().continuetargets.len() - 1;
            self.fs().continuetargets[n] += 1;
        }
        let trappos = self.fs().get_current_pos();
        {
            let oldscope = self.begin_scope();
            self.statement(true)?;
            self.end_scope(oldscope)?;
        }
        self.fs().traps -= 1;
        self.fs().add_instruction_raw(Op::PopTrap, 1, 0, 0, 0)?;
        if !self.fs().breaktargets.is_empty() {
            let n = self.fs().breaktargets.len() - 1;
            self.fs().breaktargets[n] -= 1;
        }
        if !self.fs().continuetargets.is_empty() {
            let n = self.fs().continuetargets.len() - 1;
            self.fs().continuetargets[n] -= 1;
        }
        self.fs().add_instruction_raw(Op::Jmp, 0, 0, 0, 0)?;
        let jmppos = self.fs().get_current_pos();
        let cur = self.fs().get_current_pos();
        self.fs().set_instruction_param(trappos, 1, cur - trappos);
        self.expect(TK_CATCH)?;
        self.expect(b'(' as Token)?;
        let exid = self.expect(TK_IDENTIFIER)?;
        self.expect(b')' as Token)?;
        {
            let oldscope = self.begin_scope();
            let ex_target = self.fs().push_local_variable(exid);
            self.fs().set_instruction_param(trappos, 0, ex_target);
            self.statement(true)?;
            let cur = self.fs().get_current_pos();
            self.fs().set_instruction_params(jmppos, 0, cur - jmppos, 0, 0);
            self.end_scope(oldscope)?;
        }
        Ok(())
    }

    /// ↔ ParseBindEnv：function f[env]() 的环境绑定
    fn parse_bind_env(&mut self) -> Result<SQInteger, CompileError> {
        self.lex_token()?;
        self.expression()?;
        let boundtarget = self.fs().top_target();
        self.expect(b']' as Token)?;
        Ok(boundtarget)
    }

    /// ↔ FunctionExp
    fn function_exp(&mut self, lambda: bool) -> Result<(), CompileError> {
        self.lex_token()?;
        let mut boundtarget = 0xFF;
        if self.token == b'[' as Token {
            boundtarget = self.parse_bind_env()?;
        }
        self.expect(b'(' as Token)?;
        let dummy = Value::Null;
        self.create_function(&dummy, boundtarget, lambda)?;
        let fidx = self.fs().functions.len() as SQInteger - 1;
        let trg = self.fs().push_target(-1)?;
        self.fs().add_instruction_raw(Op::Closure, trg, fidx, boundtarget, 0)?;
        Ok(())
    }

    /// ↔ ClassExp
    fn class_exp(&mut self) -> Result<(), CompileError> {
        let mut base = -1;
        let mut attrs = -1;
        if self.token == TK_EXTENDS {
            self.lex_token()?;
            self.expression()?;
            base = self.fs().top_target();
        }
        if self.token == TK_ATTR_OPEN {
            self.lex_token()?;
            let trg = self.fs().push_target(-1)?;
            self.fs().add_instruction_raw(Op::NewObj, trg, 0, 0, NOT_TABLE)?;
            self.parse_table_or_class(b',' as Token, TK_ATTR_CLOSE)?;
            attrs = self.fs().top_target();
        }
        self.expect(b'{' as Token)?;
        if attrs != -1 {
            self.fs().pop_target();
        }
        if base != -1 {
            self.fs().pop_target();
        }
        let trg = self.fs().push_target(-1)?;
        self.fs().add_instruction_raw(Op::NewObj, trg, base, attrs, NOT_CLASS)?;
        self.parse_table_or_class(b';' as Token, b'}' as Token)?;
        Ok(())
    }

    /// ↔ DeleteExpr
    fn delete_expr(&mut self) -> Result<(), CompileError> {
        self.lex_token()?;
        let es = self.es;
        self.es.donot_get = true;
        self.prefixed_expr()?;
        if self.es.etype == EXPR {
            return cerr("can't delete an expression");
        }
        if self.es.etype == BASE {
            return cerr("can't delete 'base'");
        }
        if self.es.etype == OBJECT {
            self.emit_2args_op(Op::Delete, 0)?;
        } else {
            return cerr("cannot delete an (outer) local");
        }
        self.es = es;
        Ok(())
    }

    /// ↔ PrefixIncDec
    fn prefix_inc_dec(&mut self, token: Token) -> Result<(), CompileError> {
        let diff: SQInteger = if token == TK_MINUSMINUS { -1 } else { 1 };
        self.lex_token()?;
        let es = self.es;
        self.es.donot_get = true;
        self.prefixed_expr()?;
        if self.es.etype == EXPR {
            return cerr("can't '++' or '--' an expression");
        } else if self.es.etype == BASE {
            return cerr("can't '++' or '--' a base");
        } else if self.es.etype == OBJECT {
            self.emit_2args_op(Op::Inc, diff)?;
        } else if self.es.etype == LOCAL {
            let src = self.fs().top_target();
            self.fs().add_instruction_raw(Op::IncL, src, src, 0, diff)?;
        } else if self.es.etype == OUTER {
            let epos = self.es.epos;
            let tmp = self.fs().push_target(-1)?;
            self.fs().add_instruction_raw(Op::GetOuter, tmp, epos, 0, 0)?;
            self.fs().add_instruction_raw(Op::IncL, tmp, tmp, 0, diff)?;
            self.fs().add_instruction_raw(Op::SetOuter, tmp, epos, tmp, 0)?;
        }
        self.es = es;
        Ok(())
    }

    /// ↔ CreateFunction
    fn create_function(&mut self, name: &Value, boundtarget: SQInteger, lambda: bool) -> Result<(), CompileError> {
        // ↔ PushChildState（先在局部构造 child；参数解析期间当前 fs 仍是父函数，
        //    与 C++ 在参数解析后才切换 _fs 的时机一致）
        let mut child = FuncState::new(&self.ss, false);
        // ↔ funcstate->_name = name（名字设在子函数上，不是父函数！）
        child.name = name.clone();
        // child 的参数构造需要 ss —— 用局部变量累积，最后统一下发
        let mut child_params: Vec<Value> = Vec::new();
        child_params.push(self.fs().create_string(b"this"));
        let mut child_varparams = false;
        let mut defparams = 0;
        while self.token != b')' as Token {
            if self.token == TK_VARPARAMS {
                if defparams > 0 {
                    return cerr(
                        "function with default parameters cannot have variable number of parameters",
                    );
                }
                let vargv = self.fs().create_string(b"vargv");
                child_params.push(vargv);
                child_varparams = true;
                self.lex_token()?;
                if self.token != b')' as Token {
                    return cerr("expected ')'");
                }
                break;
            } else {
                let paramname = self.expect(TK_IDENTIFIER)?;
                child_params.push(paramname);
                if self.token == b'=' as Token {
                    self.lex_token()?;
                    // ↔ Expression() 编译进父函数（寄存器在其栈上）
                    self.expression()?;
                    let dp = self.fs().top_target();
                    defparams += 1;
                    let mut child_dp = self.pending_default_params.clone();
                    child_dp.push(dp);
                    self.pending_default_params = child_dp;
                } else if defparams > 0 {
                    return cerr("expected '='");
                }
                if self.token == b',' as Token {
                    self.lex_token()?;
                } else if self.token != b')' as Token {
                    return cerr("expected ')' or ','");
                }
            }
        }
        self.expect(b')' as Token)?;
        if boundtarget != 0xFF {
            self.fs().pop_target();
        }
        for _ in 0..defparams {
            self.fs().pop_target();
        }

        // ↔ _fs = funcstate（此刻才切换到子函数状态）
        self.fstates.push(child);
        {
            let dps = std::mem::take(&mut self.pending_default_params);
            let srcname = self.sourcename.clone();
            let fs = self.fs();
            for p in child_params {
                fs.add_parameter(p);
            }
            fs.varparams = child_varparams;
            fs.sourcename = srcname;
            for dp in dps {
                fs.add_default_param(dp);
            }
        }
        if lambda {
            self.expression()?;
            let popped = self.fs().pop_target();
            self.fs().add_instruction_raw(Op::Return, 1, popped, 0, 0)?;
        } else {
            self.statement(false)?;
        }
        let line = if self.lex.prevtoken == b'\n' as Token {
            self.lex.lasttokenline
        } else {
            self.lex.currentline
        };
        let li = self.lineinfo;
        self.fs().add_line_infos(line, li, true)?;
        // ↔ AddInstruction(_OP_RETURN, -1)：-1 截断为 u8 即 0xFF
        self.fs().add_instruction_raw(Op::Return, -1, 0, 0, 0)?;
        self.fs().set_stack_size(0);

        let func = self.fs().build_proto()?;
        // ↔ _fs = currchunk; _fs->_functions.push; PopChildState
        let child = self.fstates.pop().unwrap();
        let parent = self.fstates.last_mut().unwrap();
        parent.functions.push(Value::FuncProto(func));
        let _ = child;
        Ok(())
    }

    /// ↔ ResolveBreaks
    fn resolve_breaks(&mut self, mut ntoresolve: SQInteger) {
        while ntoresolve > 0 {
            let pos = self.fs().unresolvedbreaks.pop().unwrap();
            let cur = self.fs().get_current_pos();
            self.fs().set_instruction_params(pos, 0, cur - pos, 0, 0);
            ntoresolve -= 1;
        }
    }

    /// ↔ ResolveContinues
    fn resolve_continues(&mut self, mut ntoresolve: SQInteger, targetpos: SQInteger) {
        while ntoresolve > 0 {
            let pos = self.fs().unresolvedcontinues.pop().unwrap();
            self.fs().set_instruction_params(pos, 0, targetpos - pos, 0, 0);
            ntoresolve -= 1;
        }
    }
}

// ↔ ChooseArithOpByToken
fn choose_arith_op_by_token(tok: Token) -> Op {
    match tok {
        TK_PLUSEQ | 0x2B => Op::Add,  // '+'
        TK_MINUSEQ | 0x2D => Op::Sub, // '-'
        TK_MULEQ | 0x2A => Op::Mul,   // '*'
        TK_DIVEQ | 0x2F => Op::Div,   // '/'
        TK_MODEQ | 0x25 => Op::Mod,   // '%'
        _ => Op::Add,
    }
}

// ↔ ChooseCompArithCharByToken
fn choose_comp_arith_char_by_token(tok: Token) -> SQInteger {
    match tok {
        TK_MINUSEQ => b'-' as SQInteger,
        TK_PLUSEQ => b'+' as SQInteger,
        TK_MULEQ => b'*' as SQInteger,
        TK_DIVEQ => b'/' as SQInteger,
        TK_MODEQ => b'%' as SQInteger,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile_src(src: &str) -> Result<Value, CompileError> {
        // 不依赖 VmData：直接构造编译器跑 run()
        let ss = SharedState::new();
        let mut c = Compiler {
            token: 0,
            fstates: Vec::new(),
            sourcename: ss.intern(b"test"),
            lex: SqlLexer::new({
                let mut pos = 0usize;
                let bytes = src.as_bytes().to_vec();
                Box::new(move || {
                    if pos < bytes.len() {
                        let c = bytes[pos];
                        pos += 1;
                        c as SQInteger
                    } else {
                        0
                    }
                })
            }),
            lineinfo: false,
            raiseerror: false,
            es: ExpState::default(),
            scope: Scope {
                outers: 0,
                stacksize: 0,
            },
            ss,
            pending_default_params: Vec::new(),
        };
        // run() 里错误路径用到 raiseerror/compilererrorhandler——测试里 raiseerror=false 安全
        // 但 run 的错误分支访问 self.ss.compilererrorhandler；此处直接调用内部 run
        c.run_test()
    }

    impl Compiler {
        fn run_test(&mut self) -> Result<Value, CompileError> {
            self.run()
        }
    }

    fn instrs(proto: &Value) -> &Vec<SQInstruction> {
        match proto {
            Value::FuncProto(p) => &p.instructions,
            _ => panic!("not a proto"),
        }
    }

    #[test]
    fn compile_hello_world_like() {
        let p = compile_src("print(\"Hello World!\")").unwrap();
        let ins = instrs(&p);
        // 预期：GETK/print, LOAD "Hello World!", CALL, RETURN
        assert!(ins.len() >= 3);
        assert_eq!(ins[ins.len() - 1].opcode(), Op::Return);
    }

    #[test]
    fn compile_arithmetic_peephole() {
        let p = compile_src("return 1+2").unwrap();
        let ins = instrs(&p);
        // 1+2：LOADINT 1; LOADINT 2; ADD; RETURN 1
        let names: Vec<&str> = ins.iter().map(|i| i.opcode().name()).collect();
        println!("ARITH: {:?}", names);
        assert_eq!(
            names,
            vec!["LOADINT", "LOADINT", "ADD", "RETURN", "RETURN"]
        );
    }

    #[test]
    fn compile_local_and_if() {
        let p = compile_src("local a = 10; if (a > 5) a = a - 1").unwrap();
        let ins = instrs(&p);
        let names: Vec<&str> = ins.iter().map(|i| i.opcode().name()).collect();
        // CMP+JZ 被窥孔合并为 JCMP；a=a-1 的 MOVE 并入 SUB（均为 C++ 原版行为）
        assert!(names.contains(&"JCMP"));
        assert!(names.contains(&"SUB"));
        assert!(!names.contains(&"MOVE"));
    }

    #[test]
    fn compile_while_loop() {
        let p = compile_src("local i=0; while(i<3) i++").unwrap();
        let ins = instrs(&p);
        let names: Vec<&str> = ins.iter().map(|i| i.opcode().name()).collect();
        assert!(names.contains(&"JMP"));
        assert!(names.contains(&"PINCL"));
    }

    #[test]
    fn compile_foreach() {
        let p = compile_src("foreach(k,v in {}) print(k)").unwrap();
        let ins = instrs(&p);
        let names: Vec<&str> = ins.iter().map(|i| i.opcode().name()).collect();
        assert!(names.contains(&"FOREACH"));
        assert!(names.contains(&"POSTFOREACH"));
    }

    #[test]
    fn compile_class_with_methods() {
        let p = compile_src("class A { function foo() { return 1; } }").unwrap();
        let ins = instrs(&p);
        let names: Vec<&str> = ins.iter().map(|i| i.opcode().name()).collect();
        assert!(names.contains(&"NEWOBJ"));
        assert!(names.contains(&"CLOSURE"));
        assert!(names.contains(&"NEWSLOTA"));
        // 子函数存在
        match &p {
            Value::FuncProto(pp) => assert_eq!(pp.functions.len(), 1),
            _ => unreachable!(),
        }
    }

    #[test]
    fn compile_nested_closure_captures_outer() {
        let p = compile_src("local x = 1; local f = function() { return x; }").unwrap();
        match &p {
            Value::FuncProto(pp) => {
                assert_eq!(pp.functions.len(), 1);
                match &pp.functions[0] {
                    Value::FuncProto(child) => {
                        assert_eq!(child.outervalues.len(), 1);
                        assert_eq!(child.outervalues[0].otype, crate::funcproto::SQOuterType::OtLocal);
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn compile_try_catch() {
        let p = compile_src("try { throw \"x\"; } catch(e) { }").unwrap();
        let ins = instrs(&p);
        let names: Vec<&str> = ins.iter().map(|i| i.opcode().name()).collect();
        assert!(names.contains(&"PUSHTRAP"));
        assert!(names.contains(&"POPTRAP"));
        assert!(names.contains(&"THROW"));
    }

    #[test]
    fn compile_switch() {
        let p = compile_src("switch(1) { case 1: break; default: }").unwrap();
        let ins = instrs(&p);
        let names: Vec<&str> = ins.iter().map(|i| i.opcode().name()).collect();
        assert!(names.contains(&"EQ"));
        assert_eq!(names.iter().filter(|n| **n == "JZ").count(), 1);
        assert_eq!(names.iter().filter(|n| **n == "JMP").count(), 1);
    }

    #[test]
    fn syntax_error_is_reported() {
        assert!(compile_src("local = 5").is_err());
        assert!(compile_src("1 +").is_err());
        assert!(compile_src("if (1").is_err());
    }

    #[test]
    fn compile_enum_and_const() {
        let p = compile_src("enum E { a, b = 5, c } const K = 42; return K").unwrap();
        assert!(matches!(&p, Value::FuncProto(_)));
    }
}

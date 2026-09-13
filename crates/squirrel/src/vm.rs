//! ↔ squirrel/sqvm.h + sqvm.cpp + Raise_*（sqdebug.cpp）
//!
//! 执行模型：共享单栈 + CallInfo 帧窗口的寄存器式 VM。
//! C++ 的 `goto exception_trap` 重构为 `Result<_, Value>`（Err 携带错误对象）；
//! `ci` 指针对应 callsstack.last_mut()；`_ip` 对应指令索引。

use std::rc::Rc;

use crate::array::SQArray;
use crate::class::SQClass;
use crate::closure::GeneratorState;
use crate::config::*;
use crate::cformat::format_g_prec;
use crate::object::{
    get_type_name, id_type2name, ClassRef, ClosureRef, FuncProtoRef, GeneratorRef, OuterRef, ThreadRef, Value,
};
use crate::opcodes::*;
use crate::state::SharedState;
use crate::table::SQTable;

pub const GET_FLAG_RAW: SQUnsignedInteger = 0x0000_0001;
pub const GET_FLAG_DO_NOT_RAISE_ERROR: SQUnsignedInteger = 0x0000_0002;

pub const FALLBACK_OK: SQInteger = 0;
pub const FALLBACK_NO_MATCH: SQInteger = 1;
pub const FALLBACK_ERROR: SQInteger = 2;

/// ↔ SQExceptionTrap
#[derive(Clone)]
pub struct SQExceptionTrap {
    pub stackbase: SQInteger,
    pub stacksize: SQInteger,
    pub ip: usize,
    pub extarget: SQInteger,
}

/// ↔ SQVM::CallInfo
#[derive(Clone)]
pub struct CallInfo {
    pub ip: usize,
    pub literals: FuncProtoRef,
    pub closure: Value,
    pub generator: Option<GeneratorRef>,
    pub etraps: SQInt32,
    pub prevstkbase: SQInt32,
    pub prevtop: SQInt32,
    pub target: SQInt32,
    pub ncalls: SQInt32,
    pub root: bool,
}

impl Default for CallInfo {
    fn default() -> Self {
        CallInfo {
            ip: 0,
            literals: SharedState::placeholder_proto(),
            closure: Value::Null,
            generator: None,
            etraps: 0,
            prevstkbase: 0,
            prevtop: 0,
            target: 0,
            ncalls: 0,
            root: false,
        }
    }
}

/// ↔ SQVM
pub struct VmData {
    pub ss: Rc<SharedState>,
    pub stack: Vec<Value>,
    pub top: SQInteger,
    pub stackbase: SQInteger,
    pub openouters: Vec<OuterRef>, // 尾部 == C++ 链表头（语义上无序，close 按阈值过滤）
    pub roottable: Value,
    pub lasterror: Value,
    pub errorhandler: Value,
    pub debughook: bool,
    pub debughook_native: Option<crate::state::DebugHookFn>,
    pub debughook_closure: Value,
    pub temp_reg: Value,
    pub callsstack: Vec<CallInfo>,
    pub etraps: Vec<SQExceptionTrap>,
    pub foreignptr: SQUserPointer,
    pub nnativecalls: SQInteger,
    pub nmetamethodscall: SQInteger,
    /// >0 时给错误处理器/调试钩子额外留栈余量（见 enter_frame）
    pub reserve_stack_headroom: SQInteger,
    /// 打开的 outer 对应的共享单元：栈槽索引 → 单元（↔ `_valptr` 指向的槽）
    pub open_cells: Vec<(usize, std::rc::Rc<std::cell::RefCell<Value>>)>,
    pub releasehook: Option<crate::userdata::ReleaseHook>,
    pub suspended: bool,
    pub suspended_root: bool,
    pub suspended_target: SQInteger,
    pub suspended_traps: SQInteger,
    pub(crate) self_weak: std::cell::RefCell<Option<std::rc::Weak<std::cell::RefCell<VmData>>>>,
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

macro_rules! throw {
    ($vm:expr) => {
        return Err($vm.lasterror.clone())
    };
}

/// ↔ C 的 `fmod`（参考实现走 UCRT 的 `fmod((double)f1,(double)f2)`）。
///
/// 正常输入与 Rust 的 `%` 完全一致（fmod 是精确运算），但**异常输入上的 NaN 符号位**
/// 不同：Rust 的 `%` 会把 `x % 0` 算成负 NaN，UCRT 给的是正 NaN。逐条对齐实测结果
/// （`tools/probes/nan_probe.nut` 覆盖 ±0/±1/2.5/±inf/nan 全组合，两引擎逐行一致）：
///
/// | 输入 | 参考实现 |
/// |---|---|
/// | `x` 是 NaN 或 `y` 是 NaN | 原样传播该 NaN（本实现里的 NaN 都是 `-nan(ind)`） |
/// | `x` 是 ±inf | NaN，符号跟 `x`（`+inf → nan`、`-inf → -nan(ind)`） |
/// | `y` 是 ±0（`x` 有限） | `nan`，与 `x` 的符号无关 |
/// | 其它 | 普通 fmod |
pub fn sq_fmod(x: SQFloat, y: SQFloat) -> SQFloat {
    if x.is_nan() {
        return x;
    }
    if y.is_nan() {
        return y;
    }
    if x.is_infinite() {
        return if x.is_sign_negative() {
            -SQFloat::NAN
        } else {
            SQFloat::NAN
        };
    }
    if y == 0.0 {
        return SQFloat::NAN;
    }
    (x as f64 % y as f64) as SQFloat
}

/// ↔ SQVM::SQVM
impl VmData {
    pub fn new(ss: Rc<SharedState>, stacksize: SQInteger) -> VmData {
        VmData {
            ss,
            stack: vec![Value::Null; stacksize.max(1) as usize],
            top: 0,
            stackbase: 0,
            openouters: Vec::new(),
            roottable: Value::Null,
            lasterror: Value::Null,
            errorhandler: Value::Null,
            debughook: false,
            debughook_native: None,
            debughook_closure: Value::Null,
            temp_reg: Value::Null,
            callsstack: Vec::new(),
            etraps: Vec::new(),
            foreignptr: 0,
            nnativecalls: 0,
            nmetamethodscall: 0,
            reserve_stack_headroom: 0,
            open_cells: Vec::new(),
            releasehook: None,
            suspended: false,
            suspended_root: false,
            suspended_target: -1,
            suspended_traps: -1,
            self_weak: std::cell::RefCell::new(None),
            weakref_slot: std::cell::RefCell::new(None),
        }
    }

    pub fn set_self_ref(&self, r: &ThreadRef) {
        *self.self_weak.borrow_mut() = Some(Rc::downgrade(r));
    }

    pub fn self_ref(&self) -> Option<ThreadRef> {
        self.self_weak.borrow().as_ref().and_then(|w| w.upgrade())
    }

    // ---------- 栈 API ----------

    pub fn ensure_stack(&mut self, idx: usize) {
        if idx >= self.stack.len() {
            self.stack.resize(idx + 1, Value::Null);
        }
    }

    pub fn push(&mut self, o: Value) {
        let t = self.top as usize;
        self.ensure_stack(t);
        self.stack[t] = o;
        self.top += 1;
    }

    pub fn push_null(&mut self) {
        self.push(Value::Null);
    }

    pub fn pop(&mut self) {
        self.top -= 1;
        self.stack[self.top as usize] = Value::Null;
    }

    pub fn pop_n(&mut self, n: SQInteger) {
        for _ in 0..n {
            self.pop();
        }
    }

    pub fn pop_get(&mut self) -> Value {
        self.top -= 1;
        let v = self.stack[self.top as usize].clone();
        self.stack[self.top as usize] = Value::Null;
        v
    }

    pub fn top_val(&self) -> Value {
        self.stack[self.top as usize - 1].clone()
    }

    pub fn get_up(&self, n: SQInteger) -> Value {
        self.stack[(self.top + n) as usize].clone()
    }

    pub fn get_at(&self, n: SQInteger) -> Value {
        self.stack[n as usize].clone()
    }

    /// ↔ stack_get
    pub fn stack_get(&self, idx: SQInteger) -> Value {
        if idx >= 0 {
            self.get_at(idx + self.stackbase - 1)
        } else {
            self.get_up(idx)
        }
    }

    /// ↔ SQVM::Remove
    pub fn remove(&mut self, n: SQInteger) {
        let n = if n >= 0 { n + self.stackbase - 1 } else { self.top + n };
        for i in n..self.top {
            self.stack[i as usize] = self.stack[i as usize + 1].clone();
        }
        self.stack[self.top as usize] = Value::Null;
        self.top -= 1;
    }

    pub fn last_callinfo(&self) -> &CallInfo {
        self.callsstack.last().expect("empty callstack")
    }

    pub fn last_callinfo_mut(&mut self) -> &mut CallInfo {
        self.callsstack.last_mut().expect("empty callstack")
    }

    pub fn stk_public(&mut self, a: SQInteger) -> Value {
        self.stk(a)
    }

    pub fn stk_public_set(&mut self, a: SQInteger, v: Value) {
        self.stk_set(a, v)
    }

    pub fn start_call_pub(
        &mut self,
        closure: &ClosureRef,
        target: SQInteger,
        args: SQInteger,
        stackbase: SQInteger,
        tailcall: bool,
    ) -> Result<(), Value> {
        self.start_call(closure, target, args, stackbase, tailcall)
    }

    fn stk(&self, a: SQInteger) -> Value {
        let i = (self.stackbase + a) as usize;
        // 被闭包捕获（open outer）的局部变量以共享单元为准：可能被别的线程改过
        for (slot, cell) in &self.open_cells {
            if *slot == i {
                return cell.borrow().clone();
            }
        }
        self.stack[i].clone()
    }

    fn stk_set(&mut self, a: SQInteger, v: Value) {
        let i = (self.stackbase + a) as usize;
        self.ensure_stack(i);
        for (slot, cell) in &self.open_cells {
            if *slot == i {
                *cell.borrow_mut() = v.clone();
                break;
            }
        }
        self.stack[i] = v;
    }

    // ---------- 错误（↔ sqdebug.cpp Raise_*） ----------

    /// ↔ Raise_Error(const SQChar *s, ...)（格式化消息）
    pub fn raise_error_str(&mut self, s: impl Into<String>) {
        self.lasterror = self.ss.intern(s.into().as_bytes());
    }

    /// ↔ Raise_Error(const SQObjectPtr &desc)
    pub fn raise_error_obj(&mut self, desc: Value) {
        self.lasterror = desc;
    }

    /// ↔ PrintObjVal
    pub fn print_obj_val(&mut self, o: &Value) -> Value {
        match o {
            Value::String(_) => o.clone(),
            Value::Integer(i) => self.ss.intern(i.to_string().as_bytes()),
            Value::Float(f) => {
                let s = format_g_prec(*f as f64, 14);
                self.ss.intern(s.as_bytes())
            }
            _ => {
                let n = get_type_name(o).unwrap_or("unknown");
                self.ss.intern(n.as_bytes())
            }
        }
    }

    /// ↔ Raise_IdxError
    pub fn raise_idx_error<T>(&mut self, o: &Value) -> Result<T, Value> {
        let oval = self.print_obj_val(o);
        let s = String::from_utf8_lossy(oval.as_bytes()).into_owned();
        // ↔ "%.50s" 截断
        let truncated: String = s.chars().take(50).collect();
        self.raise_error_str(format!("the index '{}' does not exist", truncated));
        throw!(self)
    }

    /// ↔ Raise_CompareError
    fn raise_compare_error<T>(&mut self, o1: &Value, o2: &Value) -> Result<T, Value> {
        let a = self.print_obj_val(o1);
        let b = self.print_obj_val(o2);
        let a = String::from_utf8_lossy(a.as_bytes()).into_owned();
        let b = String::from_utf8_lossy(b.as_bytes()).into_owned();
        self.raise_error_str(format!(
            "comparison between '{}' and '{}'",
            truncate50(&a),
            truncate50(&b)
        ));
        throw!(self)
    }

    /// ↔ Raise_ParamTypeError
    pub fn raise_param_type_error<T>(
        &mut self,
        nparam: SQInteger,
        typemask: SQInteger,
        type_: SQObjectType,
    ) -> Result<T, Value> {
        let mut exptypes = String::new();
        let mut found = 0;
        for i in 0..16 {
            let mask = 1i64 << i;
            if typemask & mask != 0 {
                if found > 0 {
                    exptypes.push('|');
                }
                found += 1;
                exptypes.push_str(id_type2name(mask as SQObjectType).unwrap_or("unknown"));
            }
        }
        self.raise_error_str(format!(
            "parameter {} has an invalid type '{}' ; expected: '{}'",
            nparam,
            id_type2name(type_).unwrap_or("unknown"),
            exptypes
        ));
        throw!(self)
    }

    // ---------- 算术/比较（↔ sqvm.cpp:21-108） ----------

    /// ↔ ARITH_OP
    pub fn arith_op(&mut self, op: u8, o1: &Value, o2: &Value) -> Result<Value, Value> {
        let tmask = o1.type_() | o2.type_();
        match tmask {
            t if t == OT_INTEGER => {
                let i1 = o1.as_integer();
                let i2 = o2.as_integer();
                let res = match op {
                    b'+' => i1.wrapping_add(i2),
                    b'-' => i1.wrapping_sub(i2),
                    b'/' => {
                        if i2 == 0 {
                            self.raise_error_str("division by zero");
                            throw!(self)
                        } else if i2 == -1 && i1 == SQInteger::MIN {
                            self.raise_error_str("integer overflow");
                            throw!(self)
                        } else {
                            i1.wrapping_div(i2)
                        }
                    }
                    b'*' => i1.wrapping_mul(i2),
                    b'%' => {
                        if i2 == 0 {
                            self.raise_error_str("modulo by zero");
                            throw!(self)
                        } else if i2 == -1 && i1 == SQInteger::MIN {
                            0
                        } else {
                            i1.wrapping_rem(i2)
                        }
                    }
                    _ => 0xDEAD_BEEF,
                };
                Ok(Value::Integer(res))
            }
            t if t == OT_FLOAT || t == (OT_FLOAT | OT_INTEGER) => {
                let f1 = o1.as_float();
                let f2 = o2.as_float();
                let res = match op {
                    b'+' => f1 + f2,
                    b'-' => f1 - f2,
                    b'/' => f1 / f2,
                    b'*' => f1 * f2,
                    b'%' => sq_fmod(f1, f2),
                    _ => 0x0f as SQFloat,
                };
                Ok(Value::Float(res))
            }
            _ => {
                if op == b'+' && (tmask & _RT_STRING) != 0 {
                    let mut dest = Value::Null;
                    if !self.string_cat(o1, o2, &mut dest) {
                        throw!(self)
                    }
                    Ok(dest)
                } else {
                    self.arith_meta_method(op, o1, o2)
                }
            }
        }
    }

    /// ↔ ArithMetaMethod
    fn arith_meta_method(&mut self, op: u8, o1: &Value, o2: &Value) -> Result<Value, Value> {
        let mm = match op {
            b'+' => 0,
            b'-' => 1,
            b'/' => 3,
            b'*' => 2,
            b'%' => 5,
            _ => 0,
        };
        if o1.is_delegable() {
            if let Some(closure) = self.get_meta_method_of(o1, mm) {
                self.push(o1.clone());
                self.push(o2.clone());
                return self.call_meta_method(closure, 2);
            }
        }
        self.raise_error_str(format!(
            "arith op {} on between '{}' and '{}'",
            op as char,
            get_type_name(o1).unwrap_or("?"),
            get_type_name(o2).unwrap_or("?")
        ));
        throw!(self)
    }

    /// ↔ BW_OP
    fn bw_op(&mut self, op: SQInteger, o1: &Value, o2: &Value) -> Result<Value, Value> {
        if o1.type_() | o2.type_() == OT_INTEGER {
            let i1 = o1.as_integer();
            let i2 = o2.as_integer();
            let res = match op {
                BW_AND => i1 & i2,
                BW_OR => i1 | i2,
                BW_XOR => i1 ^ i2,
                BW_SHIFTL => {
                    // ↔ C++ 的 `<<`：x86 的硬件移位把计数**掩到 6 位**
                    // （`1 << 2147483647` == `1 << 63` == INT64_MIN，实测参考实现如此）。
                    // Rust 的 wrapping_shl 语义相同：只用计数低 6 位，负数按补码取低位。
                    i1.wrapping_shl(i2 as u32)
                }
                BW_SHIFTR => i1.wrapping_shr(i2 as u32),
                BW_USHIFTR => ((i1 as SQUnsignedInteger).wrapping_shr(i2 as u32)) as SQInteger,
                _ => {
                    self.raise_error_str("internal vm error bitwise op failed");
                    throw!(self)
                }
            };
            Ok(Value::Integer(res))
        } else {
            self.raise_error_str(format!(
                "bitwise op between '{}' and '{}'",
                get_type_name(o1).unwrap_or("?"),
                get_type_name(o2).unwrap_or("?")
            ));
            throw!(self)
        }
    }

    /// ↔ NEG_OP
    fn neg_op(&mut self, o: &Value) -> Result<Value, Value> {
        match o {
            Value::Integer(i) => Ok(Value::Integer(i.wrapping_neg())),
            Value::Float(f) => Ok(Value::Float(-f)),
            Value::Table(_) | Value::Userdata(_) | Value::Instance(_) => {
                if let Some(closure) = self.get_meta_method_of(o, 4 /*MT_UNM*/) {
                    self.push(o.clone());
                    let r = self.call_meta_method(closure, 1)?;
                    return Ok(r);
                }
                self.raise_error_str(format!(
                    "attempt to negate a {}",
                    get_type_name(o).unwrap_or("?")
                ));
                throw!(self)
            }
            _ => {
                self.raise_error_str(format!(
                    "attempt to negate a {}",
                    get_type_name(o).unwrap_or("?")
                ));
                throw!(self)
            }
        }
    }

    /// ↔ ObjCmp
    pub fn obj_cmp(&mut self, o1: &Value, o2: &Value) -> Result<SQInteger, Value> {
        let t1 = o1.type_();
        let t2 = o2.type_();
        if t1 == t2 {
            if crate::object::raw_eq(o1, o2) {
                return Ok(0);
            }
            match t1 {
                OT_STRING => {
                    let a = o1.as_bytes();
                    let b = o2.as_bytes();
                    // ↔ scstrcmp
                    return Ok(match a.cmp(b) {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Greater => 1,
                        std::cmp::Ordering::Equal => 0,
                    });
                }
                OT_INTEGER => return Ok(if o1.as_integer() < o2.as_integer() { -1 } else { 1 }),
                OT_FLOAT => return Ok(if o1.as_float() < o2.as_float() { -1 } else { 1 }),
                OT_BOOL => {
                    // ↔ ObjCmp 的 default 分支：`_userpointer(o)` 读的是 union 里的整数，
                    // 布尔就是 0/1，于是 false < true 恒成立（不是身份地址比较）
                    return Ok(if o1.as_integer() < o2.as_integer() { -1 } else { 1 });
                }
                OT_TABLE | OT_USERDATA | OT_INSTANCE => {
                    if let Some(closure) = self.get_meta_method_of(o1, 10 /*MT_CMP*/) {
                        self.push(o1.clone());
                        self.push(o2.clone());
                        let res = self.call_meta_method(closure, 2)?;
                        if !matches!(res, Value::Integer(_)) {
                            self.raise_error_str("_cmp must return an integer");
                            throw!(self)
                        }
                        return Ok(res.as_integer());
                    }
                    // 无 metamethod：按指针比较（不可复刻，用身份地址）
                    return Ok(if o1.identity() < o2.identity() { -1 } else { 1 });
                }
                _ => {
                    return Ok(if o1.identity() < o2.identity() { -1 } else { 1 });
                }
            }
        } else {
            let n1 = t1 & SQOBJECT_NUMERIC != 0;
            let n2 = t2 & SQOBJECT_NUMERIC != 0;
            if n1 && n2 {
                if t1 == OT_INTEGER && t2 == OT_FLOAT {
                    let a = o1.as_integer() as SQFloat;
                    let b = o2.as_float();
                    if a == b {
                        return Ok(0);
                    } else if a < b {
                        return Ok(-1);
                    }
                    return Ok(1);
                } else {
                    let a = o1.as_float();
                    let b = o2.as_integer() as SQFloat;
                    if a == b {
                        return Ok(0);
                    } else if a < b {
                        return Ok(-1);
                    }
                    return Ok(1);
                }
            } else if t1 == OT_NULL {
                return Ok(-1);
            } else if t2 == OT_NULL {
                return Ok(1);
            } else {
                return self.raise_compare_error(o1, o2);
            }
        }
    }

    /// ↔ CMP_OP
    pub fn cmp_op(&mut self, op: SQInteger, o1: &Value, o2: &Value) -> Result<Value, Value> {
        let r = self.obj_cmp(o1, o2)?;
        Ok(match op {
            CMP_G => Value::Bool(r > 0),
            CMP_GE => Value::Bool(r >= 0),
            CMP_L => Value::Bool(r < 0),
            CMP_LE => Value::Bool(r <= 0),
            CMP_3W => Value::Integer(r),
            _ => Value::Bool(false),
        })
    }

    /// ↔ IsEqual（静态）
    pub fn is_equal(o1: &Value, o2: &Value) -> bool {
        let t1 = o1.type_();
        let t2 = o2.type_();
        if t1 == t2 {
            if t1 == OT_FLOAT {
                o1.as_float() == o2.as_float()
            } else {
                crate::object::raw_eq(o1, o2)
            }
        } else {
            let n1 = t1 & SQOBJECT_NUMERIC != 0;
            let n2 = t2 & SQOBJECT_NUMERIC != 0;
            if n1 && n2 {
                o1.as_float() == o2.as_float()
            } else {
                false
            }
        }
    }

    /// ↔ IsFalse
    pub fn is_false(o: &Value) -> bool {
        match o {
            Value::Null => true,
            Value::Integer(i) => *i == 0,
            Value::Float(f) => *f == 0.0,
            Value::Bool(b) => !*b,
            _ => false,
        }
    }

    // ---------- ToString / TypeOf ----------

    /// ↔ SQVM::ToString
    pub fn to_string_value(&mut self, o: &Value) -> Result<Value, Value> {
        let s: String = match o {
            Value::String(_) => return Ok(o.clone()),
            Value::Float(f) => crate::cformat::format_g(*f as f64),
            Value::Integer(i) => i.to_string(),
            Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            Value::Null => "null".to_string(),
            Value::Table(_) | Value::Userdata(_) | Value::Instance(_) => {
                if let Some(closure) = self.get_meta_method_of(o, 15 /*MT_TOSTRING*/) {
                    self.push(o.clone());
                    let res = self.call_meta_method(closure, 1)?;
                    if let Value::String(_) = &res {
                        return Ok(res);
                    }
                    // metamethod 未返回字符串 → 走默认格式
                    format!(
                        "({} : 0x{:x})",
                        get_type_name(o).unwrap_or("?"),
                        o.identity()
                    )
                } else {
                    format!(
                        "({} : 0x{:x})",
                        get_type_name(o).unwrap_or("?"),
                        o.identity()
                    )
                }
            }
            _ => format!("({} : 0x{:x})", get_type_name(o).unwrap_or("?"), o.identity()),
        };
        Ok(self.ss.intern(s.as_bytes()))
    }

    /// ↔ StringCat
    pub fn string_cat(&mut self, str_: &Value, obj: &Value, dest: &mut Value) -> bool {
        let a = match self.to_string_value(str_) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let b = match self.to_string_value(obj) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let mut bytes = Vec::with_capacity(a.as_bytes().len() + b.as_bytes().len());
        bytes.extend_from_slice(a.as_bytes());
        bytes.extend_from_slice(b.as_bytes());
        *dest = self.ss.intern(&bytes);
        true
    }

    /// ↔ TypeOf
    fn type_of(&mut self, o: &Value) -> Result<Value, Value> {
        if o.is_delegable() {
            if let Some(closure) = self.get_meta_method_of(o, 8 /*MT_TYPEOF*/) {
                self.push(o.clone());
                return self.call_meta_method(closure, 1);
            }
        }
        Ok(self.ss.intern(get_type_name(o).unwrap_or("?").as_bytes()))
    }

    // ---------- metamethod 获取（↔ SQDelegable::GetMetaMethod / SQInstance 特化） ----------

    pub fn get_meta_method_of(&self, o: &Value, mm: usize) -> Option<Value> {
        match o {
            Value::Table(rc) => {
                let t = rc.borrow();
                if let Some(d) = &t.delegate {
                    let mmname = self.ss.metamethods.borrow()[mm].clone();
                    return d.borrow().get(&mmname);
                }
                None
            }
            Value::Userdata(rc) => {
                let t = rc.borrow();
                if let Some(d) = &t.delegate {
                    let mmname = self.ss.metamethods.borrow()[mm].clone();
                    return d.borrow().get(&mmname);
                }
                None
            }
            Value::Instance(rc) => rc.borrow().get_metamethod(mm),
            _ => None,
        }
    }

    // ---------- 帧（↔ EnterFrame/LeaveFrame/CloseOuters） ----------

    /// ↔ SQVM::EnterFrame
    pub fn enter_frame(&mut self, newbase: SQInteger, newtop: SQInteger, tailcall: bool) -> Result<(), Value> {
        if !tailcall {
            let mut nci = CallInfo::default();
            nci.prevstkbase = (newbase - self.stackbase) as SQInt32;
            nci.prevtop = (self.top - self.stackbase) as SQInt32;
            nci.etraps = 0;
            nci.ncalls = 1;
            nci.generator = None;
            nci.root = false;
            self.callsstack.push(nci);
        } else {
            self.last_callinfo_mut().ncalls += 1;
        }
        self.stackbase = newbase;
        self.top = newtop;
        if newtop + MIN_STACK_OVERHEAD > self.stack.len() as SQInteger {
            if self.nmetamethodscall > 0 {
                self.raise_error_str("stack overflow, cannot resize stack while in a metamethod");
                throw!(self)
            }
            // 参考实现这里**没有上限**：`_stack.resize` 会一直涨，非尾递归的无限递归
            // 把宿主内存吃光（实测 32GB 后进程被杀）。按健壮性约定封顶并报可捕获错误：
            // 上限 = MAX_STACK_SLOTS 槽 / MAX_CALL_FRAMES 帧，远超正常脚本的用量
            // （正常深递归只需几千帧），超限时脚本能自己 catch。
            let (slot_cap, frame_cap) = if self.reserve_stack_headroom > 0 {
                (
                    MAX_STACK_SLOTS + STACK_RESERVED_SLOTS,
                    MAX_CALL_FRAMES + STACK_RESERVED_FRAMES,
                )
            } else {
                (MAX_STACK_SLOTS, MAX_CALL_FRAMES)
            };
            if newtop + MIN_STACK_OVERHEAD > slot_cap
                || self.callsstack.len() as SQInteger >= frame_cap
            {
                self.raise_error_str("stack overflow");
                throw!(self)
            }
            self.stack
                .resize((newtop + (MIN_STACK_OVERHEAD << 2)) as usize, Value::Null);
            // Rust 用栈槽索引，无需 RelocateOuters
        }
        Ok(())
    }

    /// ↔ SQVM::LeaveFrame
    pub fn leave_frame(&mut self) {
        let last_top = self.top;
        let last_stackbase = self.stackbase;
        let (prevstkbase, prevtop) = {
            let ci = self.last_callinfo();
            (ci.prevstkbase, ci.prevtop)
        };
        self.callsstack.pop();
        self.stackbase -= prevstkbase as SQInteger;
        self.top = self.stackbase + prevtop as SQInteger;
        if !self.openouters.is_empty() {
            self.close_outers(last_stackbase as usize);
        }
        let mut lt = last_top;
        while lt >= self.top {
            let i = lt as usize;
            if i < self.stack.len() {
                self.stack[i] = Value::Null;
            }
            lt -= 1;
        }
    }

    /// ↔ SQVM::CloseOuters（关闭所有指向 >= stackindex 的 outer）
    pub fn close_outers(&mut self, stackindex: usize) {
        let mut keep = Vec::with_capacity(self.openouters.len());
        let mut closed_slots = Vec::new();
        for o in std::mem::take(&mut self.openouters) {
            let idx = o.borrow().open_idx();
            match idx {
                Some(i) if i >= stackindex => {
                    let mut ob = o.borrow_mut();
                    // ↔ _value = *_valptr（值以共享单元为准，可能是别的线程最后写入的）
                    ob.value = match &ob.open {
                        Some(cell) => cell.borrow().clone(),
                        None => ob.value.clone(),
                    };
                    ob.open = None;
                    closed_slots.push(i);
                }
                _ => keep.push(o),
            }
        }
        self.openouters = keep;
        if !closed_slots.is_empty() {
            self.open_cells.retain(|(slot, _)| !closed_slots.contains(slot));
        }
    }

    /// ↔ FindOuter（无序 Vec + 精确匹配；语义等价）
    pub fn find_outer(&mut self, stackindex: usize) -> OuterRef {
        for o in self.openouters.iter() {
            if o.borrow().open_idx() == Some(stackindex) {
                return o.clone();
            }
        }
        // 值与栈槽共享单元：宿主的局部变量读写走 stk/stk_set（会更新单元），
        // 别的线程读 outer 时只碰单元，不碰宿主 VM
        let cell = std::rc::Rc::new(std::cell::RefCell::new(
            self.stack.get(stackindex).cloned().unwrap_or(Value::Null),
        ));
        let newo = crate::closure::SQOuter::create(&self.ss, cell.clone(), stackindex);
        self.openouters.push(newo.clone());
        self.open_cells.push((stackindex, cell));
        newo
    }

    /// ↔ `_valptr` 解引用：打开时读共享单元（可能是别的线程最后写入的值）
    pub fn outer_read(&self, otr: &OuterRef) -> Value {
        let (open, closed) = {
            let o = otr.borrow();
            (o.open.clone(), o.value.clone())
        };
        match open {
            Some(cell) => cell.borrow().clone(),
            None => closed,
        }
    }

    fn outer_write(&mut self, otr: &OuterRef, v: Value) {
        let open = { otr.borrow().open.clone() };
        match open {
            Some(cell) => {
                *cell.borrow_mut() = v;
            }
            None => {
                otr.borrow_mut().value = v;
            }
        }
    }

    // ---------- Get/Set 回退链（↔ sqvm.cpp:1265-1445） ----------

    /// ↔ SQVM::Get
    pub fn get_value(
        &mut self,
        self_: &Value,
        key: &Value,
        getflags: SQUnsignedInteger,
        selfidx: SQInteger,
    ) -> Result<Value, Value> {
        match self_ {
            Value::Table(rc) => {
                if let Some(v) = rc.borrow().get(key) {
                    return Ok(v);
                }
            }
            Value::Array(rc) => {
                if key.type_() & SQOBJECT_NUMERIC != 0 {
                    let idx = key.as_integer();
                    if let Some(v) = rc.borrow().get(idx) {
                        return Ok(v);
                    }
                    if getflags & GET_FLAG_DO_NOT_RAISE_ERROR == 0 {
                        return self.raise_idx_error(key);
                    }
                    throw!(self)
                }
            }
            Value::Instance(rc) => {
                if let Some(v) = rc.borrow().get(key) {
                    return Ok(v);
                }
            }
            Value::Class(rc) => {
                if let Some(v) = rc.borrow().get(key) {
                    return Ok(v);
                }
            }
            Value::String(s) => {
                if key.type_() & SQOBJECT_NUMERIC != 0 {
                    let mut n = key.as_integer();
                    let len = s.len();
                    if n < 0 {
                        n += len;
                    }
                    if n >= 0 && n < len {
                        // ↔ C++：`_string(o)->_val[idx]` 是 `SQChar`（本构建 = **有符号** char），
                        // 所以高位字节会**符号扩展**：`"\xff"[0]` == -1、`"é"[0]` == -61
                        // （blob 的 `_get` 走 `unsigned char`，是 0..255，两者不同）
                        return Ok(Value::Integer(s.val[n as usize] as i8 as SQInteger));
                    }
                    if getflags & GET_FLAG_DO_NOT_RAISE_ERROR == 0 {
                        return self.raise_idx_error(key);
                    }
                    throw!(self)
                }
            }
            _ => {}
        }
        if getflags & GET_FLAG_RAW == 0 {
            match self.fallback_get(self_, key)? {
                FALLBACK_OK => return Ok(self.temp_reg.clone()),
                FALLBACK_NO_MATCH => {}
                FALLBACK_ERROR => throw!(self),
                _ => unreachable!(),
            }
            if self.invoke_default_delegate(self_, key)? {
                return Ok(self.temp_reg.clone());
            }
        }
        // root 回退
        if selfidx == 0 {
            let root = {
                let ci = self.last_callinfo();
                match &ci.closure {
                    Value::Closure(c) => c
                        .borrow()
                        .root
                        .as_ref()
                        .map(|w| w.borrow().obj.get())
                        .unwrap_or(Value::Null),
                    _ => Value::Null,
                }
            };
            if !root.is_null() {
                if let Ok(v) = self.get_value(&root, key, 0, DONT_FALL_BACK) {
                    return Ok(v);
                }
            }
        }
        if getflags & GET_FLAG_DO_NOT_RAISE_ERROR == 0 {
            return self.raise_idx_error(key);
        }
        throw!(self)
    }

    /// ↔ FallBackGet（结果放入 self.temp_reg）
    fn fallback_get(&mut self, self_: &Value, key: &Value) -> Result<SQInteger, Value> {
        match self_ {
            Value::Table(rc) => {
                let delegate = rc.borrow().delegate.clone();
                if let Some(d) = delegate {
                    if let Ok(v) = self.get_value(&Value::Table(d), key, GET_FLAG_DO_NOT_RAISE_ERROR, DONT_FALL_BACK) {
                        self.temp_reg = v;
                        return Ok(FALLBACK_OK);
                    }
                    // 继续尝试 _get metamethod（C++ 里 fall through 到 INSTANCE 分支）
                    let closure = self.get_meta_method_of(self_, 7 /*MT_GET*/);
                    if let Some(closure) = closure {
                        self.push(self_.clone());
                        self.push(key.clone());
                        self.nmetamethodscall += 1;
                        let r = self.call_value(&closure, 2, self.top - 2, false);
                        self.nmetamethodscall -= 1;
                        self.pop_n(2);
                        match r {
                            Ok(v) => {
                                self.temp_reg = v;
                                return Ok(FALLBACK_OK);
                            }
                            Err(_) => {
                                if !self.lasterror.is_null() {
                                    return Ok(FALLBACK_ERROR);
                                }
                            }
                        }
                    }
                } else {
                    return Ok(FALLBACK_NO_MATCH);
                }
            }
            Value::Userdata(_) => {
                let closure = self.get_meta_method_of(self_, 7 /*MT_GET*/);
                if let Some(closure) = closure {
                    self.push(self_.clone());
                    self.push(key.clone());
                    self.nmetamethodscall += 1;
                    let r = self.call_value(&closure, 2, self.top - 2, false);
                    self.nmetamethodscall -= 1;
                    self.pop_n(2);
                    match r {
                        Ok(v) => {
                            self.temp_reg = v;
                            return Ok(FALLBACK_OK);
                        }
                        Err(_) => {
                            if !self.lasterror.is_null() {
                                return Ok(FALLBACK_ERROR);
                            }
                        }
                    }
                }
            }
            Value::Instance(_) => {
                let closure = self.get_meta_method_of(self_, 7 /*MT_GET*/);
                if let Some(closure) = closure {
                    self.push(self_.clone());
                    self.push(key.clone());
                    self.nmetamethodscall += 1;
                    let r = self.call_value(&closure, 2, self.top - 2, false);
                    self.nmetamethodscall -= 1;
                    self.pop_n(2);
                    match r {
                        Ok(v) => {
                            self.temp_reg = v;
                            return Ok(FALLBACK_OK);
                        }
                        Err(_) => {
                            if !self.lasterror.is_null() {
                                return Ok(FALLBACK_ERROR);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(FALLBACK_NO_MATCH)
    }

    /// ↔ InvokeDefaultDelegate（结果放入 self.temp_reg）
    fn invoke_default_delegate(&mut self, self_: &Value, key: &Value) -> Result<bool, Value> {
        let ddel: Value = match self_ {
            Value::Class(_) => self.ss.class_default_delegate.borrow().clone(),
            Value::Table(_) => self.ss.table_default_delegate.borrow().clone(),
            Value::Array(_) => self.ss.array_default_delegate.borrow().clone(),
            Value::String(_) => self.ss.string_default_delegate.borrow().clone(),
            Value::Instance(_) => self.ss.instance_default_delegate.borrow().clone(),
            Value::Integer(_) | Value::Float(_) | Value::Bool(_) => {
                self.ss.number_default_delegate.borrow().clone()
            }
            Value::Generator(_) => self.ss.generator_default_delegate.borrow().clone(),
            Value::Closure(_) | Value::NativeClosure(_) => {
                self.ss.closure_default_delegate.borrow().clone()
            }
            Value::Thread(_) => self.ss.thread_default_delegate.borrow().clone(),
            Value::WeakRef(_) => self.ss.weakref_default_delegate.borrow().clone(),
            _ => return Ok(false),
        };
        if let Value::Table(t) = &ddel {
            if let Some(v) = t.borrow().get(key) {
                self.temp_reg = v;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// ↔ SQVM::Set
    pub fn set_value(&mut self, self_: &Value, key: &Value, val: &Value, selfidx: SQInteger) -> Result<(), Value> {
        match self_ {
            Value::Table(rc) => {
                if rc.borrow_mut().set(key, val) {
                    return Ok(());
                }
            }
            Value::Instance(rc) => {
                if rc.borrow_mut().set(key, val) {
                    return Ok(());
                }
            }
            Value::Array(rc) => {
                if key.type_() & SQOBJECT_NUMERIC == 0 {
                    self.raise_error_str(format!(
                        "indexing {} with {}",
                        get_type_name(self_).unwrap_or("?"),
                        get_type_name(key).unwrap_or("?")
                    ));
                    throw!(self)
                }
                if !rc.borrow_mut().set(key.as_integer(), val) {
                    return self.raise_idx_error(key);
                }
                return Ok(());
            }
            Value::Userdata(_) => {} // 必须走回退
            _ => {
                self.raise_error_str(format!(
                    "trying to set '{}'",
                    get_type_name(self_).unwrap_or("?")
                ));
                throw!(self)
            }
        }
        match self.fallback_set(self_, key, val)? {
            FALLBACK_OK => return Ok(()),
            FALLBACK_NO_MATCH => {}
            FALLBACK_ERROR => throw!(self),
            _ => unreachable!(),
        }
        if selfidx == 0 {
            if let Value::Table(rt) = &self.roottable {
                if rt.borrow_mut().set(key, val) {
                    return Ok(());
                }
            }
        }
        self.raise_idx_error(key)
    }

    /// ↔ FallBackSet
    fn fallback_set(&mut self, self_: &Value, key: &Value, val: &Value) -> Result<SQInteger, Value> {
        match self_ {
            Value::Table(rc) => {
                let delegate = rc.borrow().delegate.clone();
                if let Some(d) = delegate {
                    if self.set_value(&Value::Table(d.clone()), key, val, DONT_FALL_BACK).is_ok() {
                        return Ok(FALLBACK_OK);
                    }
                }
                let closure = self.get_meta_method_of(self_, 6 /*MT_SET*/);
                if let Some(closure) = closure {
                    self.push(self_.clone());
                    self.push(key.clone());
                    self.push(val.clone());
                    self.nmetamethodscall += 1;
                    let r = self.call_value(&closure, 3, self.top - 3, false);
                    self.nmetamethodscall -= 1;
                    self.pop_n(3);
                    match r {
                        Ok(_) => return Ok(FALLBACK_OK),
                        Err(_) => {
                            if !self.lasterror.is_null() {
                                return Ok(FALLBACK_ERROR);
                            }
                        }
                    }
                }
            }
            Value::Instance(_) | Value::Userdata(_) => {
                let closure = self.get_meta_method_of(self_, 6 /*MT_SET*/);
                if let Some(closure) = closure {
                    self.push(self_.clone());
                    self.push(key.clone());
                    self.push(val.clone());
                    self.nmetamethodscall += 1;
                    let r = self.call_value(&closure, 3, self.top - 3, false);
                    self.nmetamethodscall -= 1;
                    self.pop_n(3);
                    match r {
                        Ok(_) => return Ok(FALLBACK_OK),
                        Err(_) => {
                            if !self.lasterror.is_null() {
                                return Ok(FALLBACK_ERROR);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(FALLBACK_NO_MATCH)
    }

    // ---------- NewSlot/DeleteSlot/Clone ----------

    /// ↔ SQVM::NewSlot
    pub fn new_slot(&mut self, self_: &Value, key: &Value, val: &Value, bstatic: bool) -> Result<(), Value> {
        if key.is_null() {
            self.raise_error_str("null cannot be used as index");
            throw!(self)
        }
        match self_ {
            Value::Table(rc) => {
                let delegate = rc.borrow().delegate.clone();
                let mut rawcall = true;
                if let Some(d) = delegate {
                    let exists = rc.borrow().get(key).is_some();
                    if !exists {
                        let closure = {
                            let mmname = self.ss.metamethods.borrow()[13 /*MT_NEWSLOT*/].clone();
                            d.borrow().get(&mmname)
                        };
                        if let Some(closure) = closure {
                            self.push(self_.clone());
                            self.push(key.clone());
                            self.push(val.clone());
                            let _ = self.call_meta_method(closure, 3)?;
                            rawcall = false;
                        }
                    }
                }
                if rawcall {
                    rc.borrow_mut().new_slot(key, val);
                }
                Ok(())
            }
            Value::Instance(_rc) => {
                let closure = self.get_meta_method_of(self_, 13 /*MT_NEWSLOT*/);
                if let Some(closure) = closure {
                    self.push(self_.clone());
                    self.push(key.clone());
                    self.push(val.clone());
                    let _ = self.call_meta_method(closure, 3)?;
                    return Ok(());
                }
                self.raise_error_str("class instances do not support the new slot operator");
                throw!(self)
            }
            Value::Class(rc) => {
                let ok = rc.borrow_mut().new_slot(&self.ss, key, val, bstatic);
                if !ok {
                    let locked = rc.borrow().locked;
                    if locked {
                        self.raise_error_str("trying to modify a class that has already been instantiated");
                        throw!(self)
                    } else {
                        let oval = self.print_obj_val(key);
                        let s = String::from_utf8_lossy(oval.as_bytes()).into_owned();
                        self.raise_error_str(format!("the property '{}' already exists", s));
                        throw!(self)
                    }
                }
                Ok(())
            }
            _ => {
                self.raise_error_str(format!(
                    "indexing {} with {}",
                    get_type_name(self_).unwrap_or("?"),
                    get_type_name(key).unwrap_or("?")
                ));
                throw!(self)
            }
        }
    }

    /// ↔ SQVM::NewSlotA
    pub fn new_slot_a(
        &mut self,
        self_: &Value,
        key: &Value,
        val: &Value,
        attrs: &Value,
        bstatic: bool,
        raw: bool,
    ) -> Result<(), Value> {
        if !matches!(self_, Value::Class(_)) {
            self.raise_error_str("object must be a class");
            throw!(self)
        }
        let c = match self_ {
            Value::Class(c) => c.clone(),
            _ => unreachable!(),
        };
        if !raw {
            let mm = c.borrow().metamethods[16 /*MT_NEWMEMBER*/].clone();
            if !mm.is_null() {
                self.push(self_.clone());
                self.push(key.clone());
                self.push(val.clone());
                self.push(attrs.clone());
                self.push(Value::Bool(bstatic));
                let _ = self.call_meta_method(mm, 5)?;
                return Ok(());
            }
        }
        self.new_slot(self_, key, val, bstatic)?;
        if !attrs.is_null() {
            c.borrow_mut().set_attributes(key, attrs);
        }
        Ok(())
    }

    /// ↔ SQVM::DeleteSlot
    fn delete_slot(&mut self, self_: &Value, key: &Value) -> Result<Value, Value> {
        match self_ {
            Value::Table(_) | Value::Instance(_) | Value::Userdata(_) => {
                let closure = self.get_meta_method_of(self_, 14 /*MT_DELSLOT*/);
                if let Some(closure) = closure {
                    self.push(self_.clone());
                    self.push(key.clone());
                    return self.call_meta_method(closure, 2);
                } else if let Value::Table(rc) = self_ {
                    let t = rc.borrow().get(key);
                    match t {
                        Some(v) => {
                            rc.borrow_mut().remove(key);
                            Ok(v)
                        }
                        None => self.raise_idx_error(key),
                    }
                } else {
                    self.raise_error_str(format!(
                        "cannot delete a slot from {}",
                        get_type_name(self_).unwrap_or("?")
                    ));
                    throw!(self)
                }
            }
            _ => {
                self.raise_error_str(format!(
                    "attempt to delete a slot from a {}",
                    get_type_name(self_).unwrap_or("?")
                ));
                throw!(self)
            }
        }
    }

    /// ↔ SQVM::Clone
    fn clone_value(&mut self, self_: &Value) -> Result<Value, Value> {
        match self_ {
            Value::Table(rc) => {
                let newobj = rc.borrow().clone_table(&self.ss);
                let closure = {
                    let t = newobj.borrow();
                    if let Some(d) = &t.delegate {
                        let mmname = self.ss.metamethods.borrow()[12 /*MT_CLONED*/].clone();
                        d.borrow().get(&mmname)
                    } else {
                        None
                    }
                };
                if let Some(closure) = closure {
                    self.push(Value::Table(newobj.clone()));
                    self.push(self_.clone());
                    let _ = self.call_meta_method(closure, 2)?;
                }
                Ok(Value::Table(newobj))
            }
            Value::Instance(rc) => {
                let newobj = rc.borrow().clone_instance(&self.ss);
                let closure = self.get_meta_method_of(&Value::Instance(newobj.clone()), 12);
                if let Some(closure) = closure {
                    self.push(Value::Instance(newobj.clone()));
                    self.push(self_.clone());
                    let _ = self.call_meta_method(closure, 2)?;
                }
                Ok(Value::Instance(newobj))
            }
            Value::Array(rc) => Ok(Value::Array(rc.borrow().clone_array(&self.ss))),
            _ => {
                self.raise_error_str(format!("cloning a {}", get_type_name(self_).unwrap_or("?")));
                throw!(self)
            }
        }
    }

    // ---------- 调用机制 ----------

    /// ↔ Execute 的结果版本（Call/MetaMethod 用）
    pub fn execute_value(
        &mut self,
        closure: &Value,
        nargs: SQInteger,
        stackbase: SQInteger,
        raiseerror: bool,
        et: ExecutionType,
    ) -> Result<Value, Value> {
        let mut outres = Value::Null;
        execute(self, closure, nargs, stackbase, &mut outres, raiseerror, et)?;
        Ok(outres)
    }

    /// ↔ SQVM::Call（通用调用：Squirrel 闭包/native/类）
    pub fn call_value(
        &mut self,
        closure: &Value,
        nparams: SQInteger,
        stackbase: SQInteger,
        raiseerror: bool,
    ) -> Result<Value, Value> {
        match closure {
            Value::Closure(_) => self.execute_value(closure, nparams, stackbase, raiseerror, ExecutionType::Call),
            Value::NativeClosure(nc) => {
                let mut suspend = false;
                let mut tailcall = false;
                let mut out = Value::Null;
                self.call_native(nc.clone(), nparams, stackbase, &mut out, -1, &mut suspend, &mut tailcall)?;
                Ok(out)
            }
            Value::Class(c) => {
                let inst = Value::Instance(SQClass::create_instance(&c, &self.ss));
                let constr = c.borrow().get_constructor();
                match constr {
                    Some(cv) if matches!(cv, Value::Closure(_) | Value::NativeClosure(_)) => {
                        let i = stackbase as usize;
                        self.ensure_stack(i);
                        self.stack[i] = inst.clone();
                        self.call_value(&cv, nparams, stackbase, raiseerror)?;
                        Ok(inst)
                    }
                    _ => Ok(inst),
                }
            }
            _ => {
                self.raise_error_str(format!(
                    "attempt to call '{}'",
                    get_type_name(closure).unwrap_or("?")
                ));
                throw!(self)
            }
        }
    }

    /// ↔ CallMetaMethod
    pub fn call_meta_method(&mut self, closure: Value, nparams: SQInteger) -> Result<Value, Value> {
        self.nmetamethodscall += 1;
        let r = self.call_value(&closure, nparams, self.top - nparams, false);
        self.nmetamethodscall -= 1;
        self.pop_n(nparams);
        r
    }

    /// ↔ SQVM::StartCall
    fn start_call(
        &mut self,
        closure: &ClosureRef,
        target: SQInteger,
        args: SQInteger,
        stackbase: SQInteger,
        tailcall: bool,
    ) -> Result<(), Value> {
        let func = closure.borrow().function.clone();
        let mut paramssize = func.parameters.len() as SQInteger;
        let newtop = stackbase + func.stacksize;
        let nargs = args;
        if func.varparams != 0 {
            paramssize -= 1;
            if nargs < paramssize {
                self.raise_error_str(format!(
                    "wrong number of parameters ({} passed, at least {} required)",
                    nargs, paramssize
                ));
                throw!(self)
            }
            let nvargs = nargs - paramssize;
            let arr = SQArray::create(&self.ss, nvargs);
            {
                let mut a = arr.borrow_mut();
                let mut pbase = (stackbase + paramssize) as usize;
                for n in 0..nvargs {
                    // ↔ arr->_values[n] = _stack._vals[pbase]（索引赋值，不是 append）
                    a.values[n as usize] = self.stack[pbase].clone();
                    self.stack[pbase] = Value::Null;
                    pbase += 1;
                }
            }
            let i = (stackbase + paramssize) as usize;
            self.ensure_stack(i);
            self.stack[i] = Value::Array(arr);
        } else if paramssize != nargs {
            let ndef = func.defaultparams.len() as SQInteger;
            if ndef > 0 && nargs < paramssize && (paramssize - nargs) <= ndef {
                let diff = paramssize - nargs;
                let mut nargs2 = nargs;
                let dps = closure.borrow().defaultparams.clone();
                for n in (ndef - diff)..ndef {
                    let i = (stackbase + nargs2) as usize;
                    self.ensure_stack(i);
                    self.stack[i] = dps[n as usize].clone();
                    nargs2 += 1;
                }
            } else {
                self.raise_error_str(format!(
                    "wrong number of parameters ({} passed, {} required)",
                    nargs, paramssize
                ));
                throw!(self)
            }
        }

        let env = closure.borrow().env.clone();
        if let Some(env) = env {
            let o = env.borrow().obj.get();
            let i = stackbase as usize;
            self.ensure_stack(i);
            self.stack[i] = o;
        }

        self.enter_frame(stackbase, newtop, tailcall)?;

        {
            let ci = self.last_callinfo_mut();
            ci.closure = Value::Closure(closure.clone());
            ci.literals = func.clone();
            ci.ip = 0;
            ci.target = target as SQInt32;
        }

        if self.debughook {
            self.call_debug_hook('c' as SQInteger, 0);
        }

        if func.bgenerator {
            let gen = crate::closure::SQGenerator::create(&self.ss, closure);
            let stacksize = func.stacksize;
            let g2 = gen.clone();
            // ↔ gen->Yield(this, f->_stacksize)
            {
                let mut g = g2.borrow_mut();
                let target_size = stacksize;
                if !g.r#yield(self, target_size) {
                    throw!(self)
                }
            }
            let mut temp = Value::Null;
            let isroot = self.do_return(1, target, &mut temp);
            self.stk_set(target as SQInteger, Value::Generator(gen));
            let _ = isroot;
        }

        Ok(())
    }

    /// ↔ SQVM::Return
    fn do_return(&mut self, arg0: SQInteger, arg1: SQInteger, retval: &mut Value) -> bool {
        let (isroot, target, prevstkbase) = {
            let ci = self.last_callinfo();
            (ci.root, ci.target, ci.prevstkbase)
        };
        let callerbase = self.stackbase - prevstkbase as SQInteger;

        if self.debughook {
            let ncalls = self.last_callinfo().ncalls;
            for _ in 0..ncalls {
                self.call_debug_hook('r' as SQInteger, 0);
            }
        }

        if isroot {
            if arg0 != 0xFF {
                *retval = self.stk(arg1);
            } else {
                *retval = Value::Null;
            }
        } else if target != -1 {
            let dest = (callerbase + target as SQInteger) as usize;
            self.ensure_stack(dest);
            if arg0 != 0xFF {
                self.stack[dest] = self.stk(arg1);
            } else {
                self.stack[dest] = Value::Null;
            }
        }
        self.leave_frame();
        isroot
    }

    /// ↔ SQVM::CallNative
    pub fn call_native(
        &mut self,
        nclosure: crate::object::NativeClosureRef,
        nargs: SQInteger,
        newbase: SQInteger,
        retval: &mut Value,
        target: SQInt32,
        suspend: &mut bool,
        tailcall: &mut bool,
    ) -> Result<(), Value> {
        let nparamscheck = nclosure.borrow().nparamscheck;
        let noutervalues = nclosure.borrow().outervalues.len() as SQInteger;
        let newtop = newbase + nargs + noutervalues;

        if self.nnativecalls + 1 > MAX_NATIVE_CALLS {
            self.raise_error_str("Native stack overflow");
            throw!(self)
        }

        if nparamscheck != 0
            && ((nparamscheck > 0 && nparamscheck != nargs)
                || (nparamscheck < 0 && nargs < -nparamscheck))
        {
            self.raise_error_str("wrong number of parameters");
            throw!(self)
        }

        {
            let tc = nclosure.borrow().typecheck.clone();
            let tcs = tc.len() as SQInteger;
            if tcs > 0 {
                for i in 0..nargs.min(tcs) {
                    if tc[i as usize] != -1
                        && (self.stack[(newbase + i) as usize].type_() as SQInteger & tc[i as usize] as SQInteger) == 0
                    {
                        let t = self.stack[(newbase + i) as usize].type_();
                        self.raise_param_type_error(i, tc[i as usize] as SQInteger, t)?;
                    }
                }
            }
        }

        self.enter_frame(newbase, newtop, false)?;
        {
            let ci = self.last_callinfo_mut();
            ci.closure = Value::NativeClosure(nclosure.clone());
            ci.target = target;
        }

        let (outers, env, f) = {
            let nc = nclosure.borrow();
            (nc.outervalues.clone(), nc.env.clone(), nc.function.clone())
        };
        for i in 0..outers.len() {
            let idx = (newbase + nargs + i as SQInteger) as usize;
            self.ensure_stack(idx);
            self.stack[idx] = outers[i].clone();
        }
        if let Some(env) = env {
            let o = env.borrow().obj.get();
            let i = newbase as usize;
            self.ensure_stack(i);
            self.stack[i] = o;
        }

        self.nnativecalls += 1;
        let ret = f(self);
        self.nnativecalls -= 1;

        *suspend = false;
        *tailcall = false;
        if ret == SQ_TAILCALL_FLAG {
            *tailcall = true;
            return Ok(());
        } else if ret == SQ_SUSPEND_FLAG {
            *suspend = true;
        } else if ret < 0 {
            let e = self.lasterror.clone();
            self.leave_frame();
            self.raise_error_obj(e);
            throw!(self)
        }
        // ↔ C++ if(ret)：非零（含负的 SUSPEND/TAILCALL 标志）都取栈顶
        if ret != 0 {
            *retval = self.stack[(self.top - 1) as usize].clone();
        } else {
            *retval = Value::Null;
        }
        self.leave_frame();
        Ok(())
    }

    /// ↔ SQVM::CallErrorHandler
    pub fn call_error_handler(&mut self, error: &Value) {
        if !self.errorhandler.is_null() {
            let eh = self.errorhandler.clone();
            self.push(self.roottable.clone());
            self.push(error.clone());
            // ↔ 参考实现没有栈上限，错误处理器总能跑；我们封了顶，就给处理器留余量，
            // 否则「栈溢出」这个错误本身会因为压不进帧而什么都打印不出来
            self.reserve_stack_headroom += 1;
            let r = self.call_value(&eh, 2, self.top - 2, false);
            self.reserve_stack_headroom -= 1;
            let _ = r;
            self.pop_n(2);
        }
    }

    /// ↔ SQVM::CallDebugHook
    pub fn call_debug_hook(&mut self, t: SQInteger, forcedline: SQInteger) {
        self.reserve_stack_headroom += 1;
        self.debughook = false;
        let (srcname, fname, line) = {
            let ci = self.last_callinfo();
            let proto = &ci.literals;
            let src = proto.sourcename.clone();
            let fname = proto.name.clone();
            let ip = ci.ip;
            let line = if forcedline != 0 {
                forcedline
            } else {
                proto.get_line(ip as SQInteger)
            };
            (src, fname, line)
        };
        if let Some(hook) = self.debughook_native.clone() {
            let src = match &srcname {
                Value::String(s) => String::from_utf8_lossy(&s.val).into_owned(),
                _ => String::new(),
            };
            let fname = match &fname {
                Value::String(s) => String::from_utf8_lossy(&s.val).into_owned(),
                _ => String::new(),
            };
            hook(self, t, &src, line, &fname);
        } else if !self.debughook_closure.is_null() {
            let dh = self.debughook_closure.clone();
            let nparams = 5;
            self.push(self.roottable.clone());
            self.push(Value::Integer(t));
            self.push(srcname);
            self.push(Value::Integer(line));
            self.push(fname);
            let _ = self.call_value(&dh, nparams, self.top - nparams, false);
            self.pop_n(nparams);
        }
        self.debughook = true;
    }

    // ---------- CLOSURE_OP / CLASS_OP / FOREACH_OP ----------

    /// ↔ CLOSURE_OP
    fn closure_op(&mut self, func: FuncProtoRef, boundtarget: SQInteger) -> Result<Value, Value> {
        let root_weak = match &self.roottable {
            Value::Table(t) => Some(Value::Table(t.clone()).get_weakref_as_weakref()),
            _ => None,
        };
        let closure = crate::closure::SQClosure::create(&self.ss, func.clone(), root_weak);
        let nouters = func.outervalues.len();
        if nouters > 0 {
            for i in 0..nouters {
                let (otype, src) = {
                    let v = &func.outervalues[i];
                    (v.otype, v.src.as_integer())
                };
                match otype {
                    crate::funcproto::SQOuterType::OtLocal => {
                        // ↔ CLOSURE_OP：otLOCAL 的 src 是栈槽偏移。畸形/外部字节码里
                        // 它可能越界，必须先查范围再索引（否则宿主进程直接 abort）
                        if src < 0 || self.stackbase + src >= self.stack.len() as SQInteger {
                            self.raise_error_str("invalid outer value index");
                            throw!(self)
                        }
                        let otr = self.find_outer((self.stackbase + src) as usize);
                        closure.borrow_mut().outervalues[i] = Value::Outer(otr);
                    }
                    crate::funcproto::SQOuterType::OtOuter => {
                        let cur = self.last_callinfo().closure.clone();
                        if let Value::Closure(c) = &cur {
                            // 同上：当前闭包的 outervalues 长度可能小于 src
                            let v = match c.borrow().outervalues.get(src as usize) {
                                Some(v) => v.clone(),
                                None => {
                                    self.raise_error_str("invalid outer value index");
                                    throw!(self)
                                }
                            };
                            closure.borrow_mut().outervalues[i] = v;
                        }
                    }
                }
            }
        }
        let ndefparams = func.defaultparams.len();
        if ndefparams > 0 {
            for i in 0..ndefparams {
                let spos = func.defaultparams[i];
                // 默认参数槽同样来自字节码，先查范围
                if spos < 0 || self.stackbase + spos >= self.stack.len() as SQInteger {
                    self.raise_error_str("invalid default parameter index");
                    throw!(self)
                }
                let v = self.stack[(self.stackbase + spos) as usize].clone();
                closure.borrow_mut().defaultparams[i] = v;
            }
        }
        if boundtarget != 0xFF {
            if boundtarget < 0 || self.stackbase + boundtarget >= self.stack.len() as SQInteger {
                self.raise_error_str("invalid bind target");
                throw!(self)
            }
            let val = self.stack[(self.stackbase + boundtarget) as usize].clone();
            let t = val.type_();
            if matches!(t, x if x == OT_TABLE || x == OT_CLASS || x == OT_INSTANCE || x == OT_ARRAY)
            {
                closure.borrow_mut().env = Some(val.get_weakref_as_weakref());
            } else {
                self.raise_error_str(format!(
                    "cannot bind a {} as environment object",
                    id_type2name(t).unwrap_or("?")
                ));
                throw!(self)
            }
        }
        Ok(Value::Closure(closure))
    }

    /// ↔ CLASS_OP
    fn class_op(&mut self, baseclass: SQInteger, attributes: SQInteger) -> Result<Value, Value> {
        let mut base: Option<ClassRef> = None;
        let mut attrs = Value::Null;
        if baseclass != -1 {
            let b = self.stack[(self.stackbase + baseclass) as usize].clone();
            if !matches!(b, Value::Class(_)) {
                self.raise_error_str(format!(
                    "trying to inherit from a {}",
                    get_type_name(&b).unwrap_or("?")
                ));
                throw!(self)
            }
            base = match b {
                Value::Class(c) => Some(c),
                _ => unreachable!(),
            };
        }
        if attributes != MAX_FUNC_STACKSIZE as SQInteger {
            attrs = self.stack[(self.stackbase + attributes) as usize].clone();
        }
        let target = Value::Class(SQClass::create(&self.ss, base));
        let inherited_mm = {
            match &target {
                Value::Class(c) => c.borrow().metamethods[17 /*MT_INHERITED*/].clone(),
                _ => Value::Null,
            }
        };
        if !inherited_mm.is_null() {
            let nparams = 2;
            self.push(target.clone());
            self.push(attrs.clone());
            let r = self.call_value(&inherited_mm, nparams, self.top - nparams, false);
            self.pop_n(nparams);
            r?;
        }
        if let Value::Class(c) = &target {
            c.borrow_mut().attributes = attrs;
        }
        Ok(target)
    }

    /// ↔ FOREACH_OP：返回跳转量
    fn foreach_op(
        &mut self,
        o1: &Value,
        o2_idx: SQInteger,
        o3_idx: SQInteger,
        o4_idx: SQInteger,
        exitpos: SQInteger,
    ) -> Result<SQInteger, Value> {
        let o4 = self.stk(o4_idx);
        match o1 {
            Value::Table(rc) => {
                let r = rc.borrow().next(false, &o4);
                match r {
                    Some((nrefidx, k, v)) => {
                        self.stk_set(o2_idx, k);
                        self.stk_set(o3_idx, v);
                        self.stk_set(o4_idx, Value::Integer(nrefidx));
                        Ok(1)
                    }
                    None => Ok(exitpos),
                }
            }
            Value::Array(rc) => {
                let r = rc.borrow().next(&o4);
                match r {
                    Some((nrefidx, k, v)) => {
                        self.stk_set(o2_idx, k);
                        self.stk_set(o3_idx, v);
                        self.stk_set(o4_idx, Value::Integer(nrefidx));
                        Ok(1)
                    }
                    None => Ok(exitpos),
                }
            }
            Value::String(s) => {
                let r = s.next(&o4);
                match r {
                    Some((nrefidx, k, v)) => {
                        self.stk_set(o2_idx, k);
                        self.stk_set(o3_idx, v);
                        self.stk_set(o4_idx, Value::Integer(nrefidx));
                        Ok(1)
                    }
                    None => Ok(exitpos),
                }
            }
            Value::Class(rc) => {
                let r = rc.borrow().next(&o4);
                match r {
                    Some((nrefidx, k, v)) => {
                        self.stk_set(o2_idx, k);
                        self.stk_set(o3_idx, v);
                        self.stk_set(o4_idx, Value::Integer(nrefidx));
                        Ok(1)
                    }
                    None => Ok(exitpos),
                }
            }
            Value::Userdata(_) | Value::Instance(_) => {
                if self.get_meta_method_of(o1, 9 /*MT_NEXTI*/).is_some() {
                    let closure = self.get_meta_method_of(o1, 9).unwrap();
                    self.push(o1.clone());
                    self.push(o4.clone());
                    let itr = self.call_meta_method(closure, 2)?;
                    self.stk_set(o4_idx, itr.clone());
                    self.stk_set(o2_idx, itr.clone());
                    if itr.is_null() {
                        return Ok(exitpos);
                    }
                    // ↔ FOREACH_OP：取不到值时**覆盖**成 _nexti 的错误（参考实现先
                    // Raise_IdxError 再 Raise_Error("_nexti returned an invalid idx")）
                    let v = match self.get_value(o1, &itr, 0, DONT_FALL_BACK) {
                        Ok(v) => v,
                        Err(_) => {
                            self.raise_error_str("_nexti returned an invalid idx");
                            throw!(self)
                        }
                    };
                    self.stk_set(o3_idx, v);
                    Ok(1)
                } else {
                    self.raise_error_str("_nexti failed");
                    throw!(self)
                }
            }
            Value::Generator(rc) => {
                let state = rc.borrow().state;
                if state == GeneratorState::Dead {
                    return Ok(exitpos);
                }
                if state == GeneratorState::Suspended {
                    let mut idx: SQInteger = 0;
                    if let Value::Integer(i) = &o4 {
                        idx = *i + 1;
                    }
                    self.stk_set(o2_idx, Value::Integer(idx));
                    self.stk_set(o4_idx, Value::Integer(idx));
                    let _vm_rc = self.self_ref().unwrap();
                    let ok = rc.borrow_mut().resume(self, o3_idx, &rc);
                    if !ok {
                        throw!(self)
                    }
                    return Ok(0);
                }
                self.raise_error_str(format!(
                    "cannot iterate {}",
                    get_type_name(o1).unwrap_or("?")
                ));
                throw!(self)
            }
            _ => {
                self.raise_error_str(format!(
                    "cannot iterate {}",
                    get_type_name(o1).unwrap_or("?")
                ));
                throw!(self)
            }
        }
    }

    /// ↔ SQVM::Init（friend 直接以 &mut 传入，避免经 RefCell 重入借用）
    pub fn init(&mut self, friendvm: Option<&mut VmData>, _stacksize: SQInteger) {
        if let Some(fv) = friendvm {
            self.roottable = fv.roottable.clone();
            self.errorhandler = fv.errorhandler.clone();
            self.debughook = fv.debughook;
            self.debughook_native = fv.debughook_native.clone();
            self.debughook_closure = fv.debughook_closure.clone();
        } else {
            self.roottable = Value::Table(SQTable::create(&self.ss, 0));
            crate::baselib::base_register(self);
        }
    }

    /// ↔ SQVM::Finalize
    pub fn finalize(&mut self) {
        if !self.openouters.is_empty() {
            self.close_outers(0);
        }
        self.roottable = Value::Null;
        self.lasterror = Value::Null;
        self.errorhandler = Value::Null;
        self.debughook = false;
        self.debughook_native = None;
        self.debughook_closure = Value::Null;
        self.temp_reg = Value::Null;
        self.callsstack.clear();
        for s in self.stack.iter_mut() {
            *s = Value::Null;
        }
    }

    /// ↔ SQVM::Suspend
    pub fn suspend(&mut self) -> SQInteger {
        if self.suspended {
            self.raise_error_str("cannot suspend an already suspended vm");
            return SQ_ERROR;
        }
        if self.nnativecalls != 2 {
            self.raise_error_str("cannot suspend through native calls/metamethods");
            return SQ_ERROR;
        }
        SQ_SUSPEND_FLAG
    }
}

fn truncate50(s: &str) -> String {
    s.chars().take(50).collect()
}

impl Value {
    /// 取 weakref 单例的 Rc 形式（内部使用）
    pub fn get_weakref_as_weakref(&self) -> crate::object::WeakRefRef {
        match &self.get_weakref() {
            Value::WeakRef(w) => w.clone(),
            _ => unreachable!(),
        }
    }
}

impl SharedState {
    pub(crate) fn placeholder_proto() -> FuncProtoRef {
        thread_local! {
            static PLACEHOLDER: FuncProtoRef = Rc::new(crate::funcproto::SQFunctionProto::empty(Value::Null, Value::Null));
        }
        PLACEHOLDER.with(|p| p.clone())
    }
}

// ==================== Execute 主循环 ====================

#[derive(Clone, Copy, PartialEq)]
pub enum ExecutionType {
    Call,
    ResumeGenerator,
    ResumeVm,
    ResumeThrowVm,
}

enum ExecOutcome {
    /// 正常返回（结果在 temp_reg）
    Returned,
    /// 挂起（resume 信息已保存；clo 在 outres）
    Suspended(Value),
}

/// 单条 CALL 指令后的流向
enum CallFlow {
    Continue,
    #[allow(dead_code)]
    Returned,
    Suspended(Value),
}

/// ↔ SQVM::Execute
pub fn execute(
    vm: &mut VmData,
    closure: &Value,
    nargs: SQInteger,
    stackbase: SQInteger,
    outres: &mut Value,
    raiseerror: bool,
    et: ExecutionType,
) -> Result<(), Value> {
    // ↔ SQVM::Call：非闭包的 Call 入口（原生闭包 / 类构造 / 报错）不走 Execute
    if matches!(et, ExecutionType::Call) && !matches!(closure, Value::Closure(_)) {
        return match vm.call_value(closure, nargs, stackbase, raiseerror) {
            Ok(v) => {
                *outres = v;
                Ok(())
            }
            Err(e) => Err(e),
        };
    }
    let vm_rc = vm.self_ref().expect("vm has no self ref");
    if vm.nnativecalls + 1 > MAX_NATIVE_CALLS {
        vm.raise_error_str("Native stack overflow");
        return Err(vm.lasterror.clone());
    }
    vm.nnativecalls += 1;
    let r = execute_inner(vm, &vm_rc, closure, nargs, stackbase, outres, raiseerror, et);
    vm.nnativecalls -= 1;
    r
}

fn execute_inner(
    vm: &mut VmData,
    vm_rc: &ThreadRef,
    closure: &Value,
    nargs: SQInteger,
    stackbase: SQInteger,
    outres: &mut Value,
    raiseerror: bool,
    et: ExecutionType,
) -> Result<(), Value> {
    let mut traps: SQInteger = 0;
    // ResumeThrowVm 注入的异常（SQ_THROW 语义）：在进入主循环前先记下，循环首轮按抛出处理
    let mut pending_throw = false;

    match et {
        ExecutionType::Call => {
            vm.temp_reg = closure.clone();
            let clo = match &vm.temp_reg {
                Value::Closure(c) => c.clone(),
                // ↔ SQVM::Call 的 default 分支：非闭包入口（宿主 API / thread.call）
                // 必须报 "attempt to call '<type>'"，而不是 panic
                other => {
                    let t = crate::object::get_type_name(other).unwrap_or("?");
                    vm.raise_error_str(format!("attempt to call '{}'", t));
                    return Err(vm.lasterror.clone());
                }
            };
            let target = vm.top - nargs;
            let prevci_len = vm.callsstack.len();
            if vm.start_call(&clo, target, nargs, stackbase, false).is_err() {
                if vm.callsstack.is_empty() {
                    let e = vm.lasterror.clone();
                    vm.call_error_handler(&e);
                }
                return Err(vm.lasterror.clone());
            }
            // ↔ if(ci == prevci)：生成器函数的 StartCall 立即弹出帧
            if vm.callsstack.len() == prevci_len {
                let v = vm.stk(vm.top - nargs);
                *outres = v;
                return Ok(());
            }
            vm.last_callinfo_mut().root = true;
        }
        ExecutionType::ResumeGenerator => {
            let gen = match closure {
                Value::Generator(g) => g.clone(),
                _ => unreachable!(),
            };
            let target = vm.stackbase + outres_slot_target(vm);
            if !gen.borrow_mut().resume(vm, target, &gen) {
                return Err(vm.lasterror.clone());
            }
            vm.last_callinfo_mut().root = true;
            traps += vm.last_callinfo().etraps as SQInteger;
        }
        ExecutionType::ResumeVm | ExecutionType::ResumeThrowVm => {
            traps = vm.suspended_traps;
            vm.last_callinfo_mut().root = vm.suspended_root;
            vm.suspended = false;
            if et == ExecutionType::ResumeThrowVm {
                // ↔ SQ_THROW()：注入的异常必须像本帧内的 throw 一样走 exception_trap
                // （否则线程里的 try/catch 抓不到，异常会冒到唤醒方）
                pending_throw = true;
            }
        }
    }

    // ---------------- 主循环（exception_restore 入口） ----------------
    'exception_restore: loop {
        let step = if pending_throw {
            pending_throw = false;
            Err(vm.lasterror.clone())
        } else {
            run_instructions(vm, vm_rc, &mut traps, outres, raiseerror)
        };
        match step {
            Ok(outcome) => match outcome {
                ExecOutcome::Returned => return Ok(()),
                ExecOutcome::Suspended(clo) => {
                    *outres = clo;
                    return Ok(());
                }
            },
            Err(currerror) => {
                // ---------------- exception_trap ----------------
                if std::env::var_os("SQ_DEBUG_TRAP").is_some() {
                    eprintln!(
                        "TRAP: err={:?} traps={} raiseerror={} notify={} cssize={}",
                        String::from_utf8_lossy(currerror.as_bytes()),
                        traps,
                        raiseerror,
                        vm.ss.notifyallexceptions.get(),
                        vm.callsstack.len()
                    );
                }
                vm.lasterror = currerror.clone();
                let last_top = vm.top;
                if vm.ss.notifyallexceptions.get() || (traps == 0 && raiseerror) {
                    vm.call_error_handler(&currerror);
                }
                while !vm.callsstack.is_empty() {
                    let has_traps = vm.last_callinfo().etraps > 0 && !vm.etraps.is_empty();
                    if has_traps {
                        // 上面已确认两侧都非空：trap 栈与 ci.etraps 失配（畸形字节码）时
                        // 退化为「无 trap」继续出栈，而不是 panic
                        let et_trap = match vm.etraps.last() {
                            Some(t) => t.clone(),
                            None => break,
                        };
                        {
                            let ci = vm.last_callinfo_mut();
                            ci.ip = et_trap.ip;
                        }
                        vm.top = et_trap.stacksize;
                        vm.stackbase = et_trap.stackbase;
                        let slot = (vm.stackbase + et_trap.extarget) as usize;
                        vm.ensure_stack(slot);
                        vm.stack[slot] = currerror.clone();
                        vm.etraps.pop();
                        traps -= 1;
                        vm.last_callinfo_mut().etraps -= 1;
                        let mut lt = last_top;
                        while lt >= vm.top {
                            let i = lt as usize;
                            if i < vm.stack.len() {
                                vm.stack[i] = Value::Null;
                            }
                            lt -= 1;
                        }
                        continue 'exception_restore; // ↔ goto exception_restore
                    } else if vm.debughook {
                        let ncalls = vm.last_callinfo().ncalls;
                        for _ in 0..ncalls {
                            vm.call_debug_hook('r' as SQInteger, 0);
                        }
                    }
                    if let Some(g) = &vm.last_callinfo().generator {
                        g.borrow_mut().kill();
                    }
                    let mustbreak = vm.last_callinfo().root;
                    vm.leave_frame();
                    if mustbreak {
                        break;
                    }
                }
                vm.lasterror = currerror;
                return Err(vm.lasterror.clone());
            }
        }
    }
}

fn outres_slot_target(_vm: &VmData) -> SQInteger {
    // Resume 的 target 在调用方计算（对应 C++ &dest - &stack[stackbase]）
    0
}

/// 单轮指令执行；Err = SQ_THROW（错误对象）
fn run_instructions(
    vm: &mut VmData,
    vm_rc: &ThreadRef,
    traps: &mut SQInteger,
    outres: &mut Value,
    _raiseerror: bool,
) -> Result<ExecOutcome, Value> {
    loop {
        // 取指（ci->_ip++）
        let (instr, proto) = {
            let ci = vm.last_callinfo();
            let proto = ci.literals.clone();
            let instr = proto.instructions[ci.ip];
            vm.last_callinfo_mut().ip += 1;
            (instr, proto)
        };
        let arg0 = instr.arg0 as SQInteger;
        let sarg0 = instr.arg0 as i8 as SQInteger;
        let arg1 = instr.arg1 as SQInteger;
        let sarg1 = instr.arg1 as SQInteger;
        let arg2 = instr.arg2 as SQInteger;
        let arg3 = instr.arg3 as SQInteger;
        let sarg3 = instr.arg3 as i8 as SQInteger;

        match instr.opcode() {
            Op::Line => {
                if vm.debughook {
                    vm.call_debug_hook('l' as SQInteger, arg1);
                }
            }
            Op::Load => {
                let v = proto.literals[arg1 as usize].clone();
                vm.stk_set(arg0, v);
            }
            Op::LoadInt => {
                // _SQ64：经 (SQInt32) 符号扩展
                vm.stk_set(arg0, Value::Integer(arg1 as SQInt32 as SQInteger));
            }
            Op::LoadFloat => {
                vm.stk_set(arg0, Value::Float(instr.farg1()));
            }
            Op::DLoad => {
                let a = proto.literals[arg1 as usize].clone();
                let b = proto.literals[arg3 as usize].clone();
                vm.stk_set(arg0, a);
                vm.stk_set(arg2, b);
            }
            Op::TailCall => {
                let t = vm.stk(arg1);
                let is_plain_closure = match &t {
                    Value::Closure(c) => !c.borrow().function.bgenerator,
                    _ => false,
                };
                if is_plain_closure {
                    let clo = match &t {
                        Value::Closure(c) => c.clone(),
                        _ => unreachable!(),
                    };
                    let last_top = vm.top;
                    if !vm.openouters.is_empty() {
                        vm.close_outers(vm.stackbase as usize);
                    }
                    for i in 0..arg3 {
                        let v = vm.stk(arg2 + i);
                        vm.stk_set(i, v);
                    }
                    let target = vm.last_callinfo().target as SQInteger;
                    vm.start_call(&clo, target, arg3, vm.stackbase, true)?;
                    if last_top >= vm.top {
                        vm.top = last_top;
                    }
                    continue;
                }
                // 非闭包目标：落入 CALL 逻辑
                match call_fallthrough(vm, vm_rc, traps, outres, arg0, sarg0, arg1, arg2, arg3)? {
                    CallFlow::Continue => {}
                    CallFlow::Returned => return Ok(ExecOutcome::Returned),
                    CallFlow::Suspended(c) => return Ok(ExecOutcome::Suspended(c)),
                }
            }
            Op::Call => {
                match call_fallthrough(vm, vm_rc, traps, outres, arg0, sarg0, arg1, arg2, arg3)? {
                    CallFlow::Continue => {}
                    CallFlow::Returned => return Ok(ExecOutcome::Returned),
                    CallFlow::Suspended(c) => return Ok(ExecOutcome::Suspended(c)),
                }
            }
            Op::PrepCall | Op::PrepCallK => {
                let key = if instr.opcode() == Op::PrepCallK {
                    proto.literals[arg1 as usize].clone()
                } else {
                    vm.stk(arg1)
                };
                let o = vm.stk(arg2);
                let v = vm.get_value(&o, &key, 0, arg2)?;
                vm.temp_reg = v;
                vm.stk_set(arg3, o);
                let t = std::mem::replace(&mut vm.temp_reg, Value::Null);
                vm.stk_set(arg0, t);
            }
            Op::GetK => {
                let self_ = vm.stk(arg2);
                let key = proto.literals[arg1 as usize].clone();
                let v = vm.get_value(&self_, &key, 0, arg2)?;
                vm.stk_set(arg0, v);
            }
            Op::Move => {
                let v = vm.stk(arg1);
                vm.stk_set(arg0, v);
            }
            Op::NewSlot => {
                let self_ = vm.stk(arg1);
                let key = vm.stk(arg2);
                let val = vm.stk(arg3);
                vm.new_slot(&self_, &key, &val, false)?;
                if arg0 != 0xFF {
                    let v = vm.stk(arg3);
                    vm.stk_set(arg0, v);
                }
            }
            Op::Delete => {
                let self_ = vm.stk(arg1);
                let key = vm.stk(arg2);
                let res = vm.delete_slot(&self_, &key)?;
                vm.stk_set(arg0, res);
            }
            Op::Set => {
                let self_ = vm.stk(arg1);
                let key = vm.stk(arg2);
                let val = vm.stk(arg3);
                vm.set_value(&self_, &key, &val, arg1)?;
                if arg0 != 0xFF {
                    let v = vm.stk(arg3);
                    vm.stk_set(arg0, v);
                }
            }
            Op::Get => {
                let self_ = vm.stk(arg1);
                let key = vm.stk(arg2);
                let v = vm.get_value(&self_, &key, 0, arg1)?;
                vm.stk_set(arg0, v);
            }
            Op::Eq => {
                let lit = if arg3 != 0 {
                    proto.literals[arg1 as usize].clone()
                } else {
                    vm.stk(arg1)
                };
                let lhs = vm.stk(arg2);
                let res = VmData::is_equal(&lhs, &lit);
                vm.stk_set(arg0, Value::Bool(res));
            }
            Op::Ne => {
                let lit = if arg3 != 0 {
                    proto.literals[arg1 as usize].clone()
                } else {
                    vm.stk(arg1)
                };
                let lhs = vm.stk(arg2);
                let res = VmData::is_equal(&lhs, &lit);
                vm.stk_set(arg0, Value::Bool(!res));
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div => {
                // ↔ _ARITH_ / _ARITH_NOZERO 宏展开（MOD 不走宏，见下）
                let opch = match instr.opcode() {
                    Op::Add => b'+',
                    Op::Sub => b'-',
                    Op::Mul => b'*',
                    _ => b'/',
                };
                let o1 = vm.stk(arg2);
                let o2 = vm.stk(arg1);
                let tmask = o1.type_() | o2.type_();
                let res = match tmask {
                    t if t == OT_INTEGER => {
                        let i1 = o1.as_integer();
                        let i2 = o2.as_integer();
                        match opch {
                            b'+' => Value::Integer(i1.wrapping_add(i2)),
                            b'-' => Value::Integer(i1.wrapping_sub(i2)),
                            b'*' => Value::Integer(i1.wrapping_mul(i2)),
                            _ => {
                                // _ARITH_NOZERO：仅检查除零
                                if i2 == 0 {
                                    vm.raise_error_str("division by zero");
                                    throw!(vm)
                                }
                                Value::Integer(i1.wrapping_div(i2))
                            }
                        }
                    }
                    t if t == OT_FLOAT || t == (OT_FLOAT | OT_INTEGER) => {
                        let f1 = o1.as_float();
                        let f2 = o2.as_float();
                        match opch {
                            b'+' => Value::Float(f1 + f2),
                            b'-' => Value::Float(f1 - f2),
                            b'*' => Value::Float(f1 * f2),
                            _ => Value::Float(f1 / f2),
                        }
                    }
                    _ => vm.arith_op(opch, &o1, &o2)?,
                };
                vm.stk_set(arg0, res);
            }
            Op::Mod => {
                // ↔ sqvm.cpp:881 _OP_MOD 无 _GUARD：ARITH_OP 失败被静默吞掉
                //   （lasterror 已设但不抛出；目标槽保持原值——通常是 null）
                let o1 = vm.stk(arg2);
                let o2 = vm.stk(arg1);
                match vm.arith_op(b'%', &o1, &o2) {
                    Ok(res) => vm.stk_set(arg0, res),
                    Err(_) => { /* 静默：复刻 C++ 怪癖 */ }
                }
            }
            Op::BitW => {
                let o1 = vm.stk(arg2);
                let o2 = vm.stk(arg1);
                let res = vm.bw_op(arg3, &o1, &o2)?;
                vm.stk_set(arg0, res);
            }
            Op::Return => {
                if let Some(g) = &vm.last_callinfo().generator {
                    g.borrow_mut().kill();
                }
                let mut temp = Value::Null;
                let isroot = vm.do_return(arg0, arg1, &mut temp);
                if isroot {
                    *outres = temp;
                    return Ok(ExecOutcome::Returned);
                }
            }
            Op::LoadNulls => {
                for n in 0..arg1 {
                    vm.stk_set(arg0 + n, Value::Null);
                }
            }
            Op::LoadRoot => {
                let w = {
                    let ci = vm.last_callinfo();
                    match &ci.closure {
                        Value::Closure(c) => {
                            let cb = c.borrow();
                            cb.root
                                .as_ref()
                                .map(|w| w.borrow().obj.get())
                                .unwrap_or(Value::Null)
                        }
                        _ => Value::Null,
                    }
                };
                if !w.is_null() {
                    vm.stk_set(arg0, w);
                } else {
                    let rt = vm.roottable.clone();
                    vm.stk_set(arg0, rt);
                }
            }
            Op::LoadBool => {
                vm.stk_set(arg0, Value::Bool(arg1 != 0));
            }
            Op::DMove => {
                let a = vm.stk(arg1);
                let b = vm.stk(arg3);
                vm.stk_set(arg0, a);
                vm.stk_set(arg2, b);
            }
            Op::Jmp => {
                let ci = vm.last_callinfo_mut();
                ci.ip = (ci.ip as SQInteger + sarg1) as usize;
            }
            Op::JCmp => {
                let o1 = vm.stk(arg2);
                let o2 = vm.stk(arg0);
                let res = vm.cmp_op(arg3, &o1, &o2)?;
                if VmData::is_false(&res) {
                    let ci = vm.last_callinfo_mut();
                    ci.ip = (ci.ip as SQInteger + sarg1) as usize;
                }
            }
            Op::JZ => {
                let v = vm.stk(arg0);
                if VmData::is_false(&v) {
                    let ci = vm.last_callinfo_mut();
                    ci.ip = (ci.ip as SQInteger + sarg1) as usize;
                }
            }
            Op::GetOuter => {
                let otr = {
                    let ci = vm.last_callinfo();
                    match &ci.closure {
                        Value::Closure(c) => {
                            let cb = c.borrow();
                            match &cb.outervalues[arg1 as usize] {
                                Value::Outer(o) => o.clone(),
                                _ => unreachable!(),
                            }
                        }
                        _ => unreachable!(),
                    }
                };
                let v = vm.outer_read(&otr);
                vm.stk_set(arg0, v);
            }
            Op::SetOuter => {
                let otr = {
                    let ci = vm.last_callinfo();
                    match &ci.closure {
                        Value::Closure(c) => {
                            let cb = c.borrow();
                            match &cb.outervalues[arg1 as usize] {
                                Value::Outer(o) => o.clone(),
                                _ => unreachable!(),
                            }
                        }
                        _ => unreachable!(),
                    }
                };
                let v = vm.stk(arg2);
                vm.outer_write(&otr, v);
                if arg0 != 0xFF {
                    let v = vm.stk(arg2);
                    vm.stk_set(arg0, v);
                }
            }
            Op::NewObj => match arg3 as u8 {
                NOT_TABLE_U8 => {
                    let t = SQTable::create(&vm.ss, arg1);
                    vm.stk_set(arg0, Value::Table(t));
                }
                NOT_ARRAY_U8 => {
                    // ↔ Reserve(arg1)：仅容量提示，不可观测
                    let a = SQArray::create(&vm.ss, 0);
                    vm.stk_set(arg0, Value::Array(a));
                }
                NOT_CLASS_U8 => {
                    let c = vm.class_op(arg1, arg2)?;
                    vm.stk_set(arg0, c);
                }
                _ => {}
            },
            Op::AppendArray => {
                let val: Value = match arg2 as u8 {
                    AAT_STACK => vm.stk(arg1),
                    AAT_LITERAL => proto.literals[arg1 as usize].clone(),
                    AAT_INT => Value::Integer(arg1 as SQInt32 as SQInteger),
                    AAT_FLOAT => Value::Float(instr.farg1()),
                    AAT_BOOL => Value::Bool(arg1 != 0),
                    _ => Value::Integer(arg1 as SQInt32 as SQInteger),
                };
                let arr = vm.stk(arg0);
                if let Value::Array(a) = &arr {
                    a.borrow_mut().append(&val);
                }
            }
            Op::Comparith => {
                let selfidx = ((arg1 as SQUnsignedInteger) & 0xFFFF_0000) >> 16;
                let self_ = vm.stk(selfidx as SQInteger);
                let key = vm.stk(arg2);
                let incr = vm.stk(arg1 & 0x0000_FFFF);
                // ↔ DerefInc(..., false, selfidx)
                let tmp = vm.get_value(&self_, &key, 0, selfidx as SQInteger)?;
                let target = vm.arith_op(arg3 as u8, &tmp, &incr)?;
                vm.set_value(&self_, &key, &target, selfidx as SQInteger)?;
                vm.stk_set(arg0, target);
            }
            Op::Inc => {
                let self_ = vm.stk(arg1);
                let key = vm.stk(arg2);
                let incr = Value::Integer(sarg3);
                let tmp = vm.get_value(&self_, &key, 0, arg1)?;
                let target = vm.arith_op(b'+', &tmp, &incr)?;
                vm.set_value(&self_, &key, &target, arg1)?;
                vm.stk_set(arg0, target);
            }
            Op::IncL => {
                let a = vm.stk(arg1);
                if let Value::Integer(i) = &a {
                    vm.stk_set(arg1, Value::Integer(i.wrapping_add(sarg3)));
                } else {
                    let o = Value::Integer(sarg3);
                    let res = vm.arith_op(b'+', &a, &o)?;
                    vm.stk_set(arg1, res);
                }
            }
            Op::PInc => {
                let self_ = vm.stk(arg1);
                let key = vm.stk(arg2);
                let incr = Value::Integer(sarg3);
                let tmp = vm.get_value(&self_, &key, 0, arg1)?;
                let target = vm.arith_op(b'+', &tmp, &incr)?;
                vm.set_value(&self_, &key, &target, arg1)?;
                vm.stk_set(arg0, tmp); // postfix：返回旧值
            }
            Op::PIncL => {
                let a = vm.stk(arg1);
                if let Value::Integer(i) = &a {
                    vm.stk_set(arg0, a.clone());
                    vm.stk_set(arg1, Value::Integer(i.wrapping_add(sarg3)));
                } else {
                    let o = Value::Integer(sarg3);
                    // ↔ PLOCAL_INC
                    let trg = vm.arith_op(b'+', &a, &o)?;
                    vm.stk_set(arg0, a);
                    vm.stk_set(arg1, trg);
                }
            }
            Op::Cmp => {
                let o1 = vm.stk(arg2);
                let o2 = vm.stk(arg1);
                let res = vm.cmp_op(arg3, &o1, &o2)?;
                vm.stk_set(arg0, res);
            }
            Op::Exists => {
                let self_ = vm.stk(arg1);
                let key = vm.stk(arg2);
                let r = vm.get_value(
                    &self_,
                    &key,
                    GET_FLAG_DO_NOT_RAISE_ERROR | GET_FLAG_RAW,
                    DONT_FALL_BACK,
                );
                vm.stk_set(arg0, Value::Bool(r.is_ok()));
            }
            Op::InstanceOf => {
                let o1 = vm.stk(arg1);
                let o2 = vm.stk(arg2);
                if !matches!(o1, Value::Class(_)) {
                    vm.raise_error_str(format!(
                        "cannot apply instanceof between a {} and a {}",
                        get_type_name(&o1).unwrap_or("?"),
                        get_type_name(&o2).unwrap_or("?")
                    ));
                    throw!(vm)
                }
                let res = match (&o1, &o2) {
                    (Value::Class(c), Value::Instance(i)) => Value::Bool(i.borrow().instance_of(c)),
                    _ => Value::Bool(false),
                };
                vm.stk_set(arg0, res);
            }
            Op::And => {
                let v = vm.stk(arg2);
                if VmData::is_false(&v) {
                    vm.stk_set(arg0, v);
                    let ci = vm.last_callinfo_mut();
                    ci.ip = (ci.ip as SQInteger + sarg1) as usize;
                }
            }
            Op::Or => {
                let v = vm.stk(arg2);
                if !VmData::is_false(&v) {
                    vm.stk_set(arg0, v);
                    let ci = vm.last_callinfo_mut();
                    ci.ip = (ci.ip as SQInteger + sarg1) as usize;
                }
            }
            Op::Neg => {
                let v = vm.stk(arg1);
                let res = vm.neg_op(&v)?;
                vm.stk_set(arg0, res);
            }
            Op::Not => {
                let v = vm.stk(arg1);
                vm.stk_set(arg0, Value::Bool(VmData::is_false(&v)));
            }
            Op::BwNot => {
                let v = vm.stk(arg1);
                if let Value::Integer(i) = &v {
                    vm.stk_set(arg0, Value::Integer(!i));
                } else {
                    vm.raise_error_str(format!(
                        "attempt to perform a bitwise op on a {}",
                        get_type_name(&v).unwrap_or("?")
                    ));
                    throw!(vm)
                }
            }
            Op::Closure => {
                let fp = {
                    let ci = vm.last_callinfo();
                    match &ci.closure {
                        Value::Closure(c) => match &c.borrow().function.functions[arg1 as usize] {
                            Value::FuncProto(f) => f.clone(),
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    }
                };
                let c = vm.closure_op(fp, arg2)?;
                vm.stk_set(arg0, c);
            }
            Op::Yield => {
                let has_gen = vm.last_callinfo().generator.is_some();
                if has_gen {
                    if sarg1 != MAX_FUNC_STACKSIZE as SQInteger {
                        vm.temp_reg = vm.stk(arg1);
                    }
                    if !vm.openouters.is_empty() {
                        vm.close_outers(vm.stackbase as usize);
                    }
                    // 帧上没有生成器（畸形字节码 / 伪造 bgenerator 标志）时按普通 yield 报错
                    let gen = match vm.last_callinfo().generator.clone() {
                        Some(g) => g,
                        None => {
                            vm.raise_error_str("trying to yield a 'null',only genenerator can be yielded");
                            throw!(vm)
                        }
                    };
                    let ok = gen.borrow_mut().r#yield(vm, arg2);
                    if !ok {
                        throw!(vm)
                    }
                    *traps -= vm.last_callinfo().etraps as SQInteger;
                    if sarg1 != MAX_FUNC_STACKSIZE as SQInteger {
                        let t = vm.temp_reg.clone();
                        vm.stk_set(arg1, t);
                    }
                } else {
                    vm.raise_error_str("trying to yield a 'null',only genenerator can be yielded");
                    throw!(vm)
                }
                let mut temp = Value::Null;
                let isroot = vm.do_return(arg0, arg1, &mut temp);
                if isroot {
                    *outres = temp;
                    return Ok(ExecOutcome::Returned);
                }
            }
            Op::Resume => {
                let g = vm.stk(arg1);
                if !matches!(g, Value::Generator(_)) {
                    vm.raise_error_str(format!(
                        "trying to resume a '{}',only genenerator can be resumed",
                        get_type_name(&g).unwrap_or("?")
                    ));
                    throw!(vm)
                }
                let gr = match &g {
                    Value::Generator(g) => g.clone(),
                    _ => unreachable!(),
                };
                let ok = gr.borrow_mut().resume(vm, arg0, &gr);
                if !ok {
                    throw!(vm)
                }
                *traps += vm.last_callinfo().etraps as SQInteger;
            }
            Op::Foreach => {
                let o1 = vm.stk(arg0);
                let jump = vm.foreach_op(&o1, arg2, arg2 + 1, arg2 + 2, sarg1)?;
                let ci = vm.last_callinfo_mut();
                ci.ip = (ci.ip as SQInteger + jump) as usize;
            }
            Op::PostForeach => {
                let o1 = vm.stk(arg0);
                if let Value::Generator(g) = &o1 {
                    if g.borrow().state == GeneratorState::Dead {
                        let ci = vm.last_callinfo_mut();
                        ci.ip = (ci.ip as SQInteger + sarg1 - 1) as usize;
                    }
                }
            }
            Op::Clone => {
                let v = vm.stk(arg1);
                let c = vm.clone_value(&v)?;
                vm.stk_set(arg0, c);
            }
            Op::TypeOf => {
                let v = vm.stk(arg1);
                let t = vm.type_of(&v)?;
                vm.stk_set(arg0, t);
            }
            Op::PushTrap => {
                let target_ip = {
                    let ci = vm.last_callinfo();
                    (ci.ip as SQInteger + arg1) as usize
                };
                vm.etraps.push(SQExceptionTrap {
                    stacksize: vm.top,
                    stackbase: vm.stackbase,
                    ip: target_ip,
                    extarget: arg0,
                });
                *traps += 1;
                vm.last_callinfo_mut().etraps += 1;
            }
            Op::PopTrap => {
                for _ in 0..arg0 {
                    vm.etraps.pop();
                    *traps -= 1;
                    vm.last_callinfo_mut().etraps -= 1;
                }
            }
            Op::Throw => {
                let v = vm.stk(arg0);
                vm.raise_error_obj(v);
                throw!(vm)
            }
            Op::NewSlotA => {
                let self_ = vm.stk(arg1);
                let key = vm.stk(arg2);
                let val = vm.stk(arg3);
                let attrs = if arg0 as u8 & NEW_SLOT_ATTRIBUTES_FLAG != 0 {
                    vm.stk(arg2 - 1)
                } else {
                    Value::Null
                };
                let bstatic = arg0 as u8 & NEW_SLOT_STATIC_FLAG != 0;
                vm.new_slot_a(&self_, &key, &val, &attrs, bstatic, false)?;
            }
            Op::GetBase => {
                let base = {
                    let ci = vm.last_callinfo();
                    match &ci.closure {
                        Value::Closure(c) => {
                            let cb = c.borrow();
                            cb.base.clone().map(Value::Class).unwrap_or(Value::Null)
                        }
                        _ => Value::Null,
                    }
                };
                vm.stk_set(arg0, base);
            }
            Op::Close => {
                if !vm.openouters.is_empty() {
                    let base = (vm.stackbase + arg1) as usize;
                    vm.close_outers(base);
                }
            }
        }
    }
}

const NOT_TABLE_U8: u8 = NOT_TABLE as u8;
const NOT_ARRAY_U8: u8 = NOT_ARRAY as u8;
const NOT_CLASS_U8: u8 = NOT_CLASS as u8;

/// ↔ _OP_CALL / _OP_TAILCALL 落入的调用逻辑。
fn call_fallthrough(
    vm: &mut VmData,
    vm_rc: &ThreadRef,
    traps: &mut SQInteger,
    _outres: &mut Value,
    arg0: SQInteger,
    sarg0: SQInteger,
    arg1: SQInteger,
    arg2: SQInteger,
    arg3: SQInteger,
) -> Result<CallFlow, Value> {
    let clo = vm.stk(arg1);
    match &clo {
        Value::Closure(_) => {
            let c = match &clo {
                Value::Closure(c) => c.clone(),
                _ => unreachable!(),
            };
            vm.start_call(&c, sarg0, arg3, vm.stackbase + arg2, false)?;
        }
        Value::NativeClosure(nc) => {
            let nc = nc.clone();
            let mut suspend = false;
            let mut tailcall = false;
            let mut retval = Value::Null;
            vm.call_native(nc, arg3, vm.stackbase + arg2, &mut retval, sarg0 as SQInt32, &mut suspend, &mut tailcall)?;
            if suspend {
                vm.suspended = true;
                vm.suspended_target = sarg0;
                vm.suspended_root = vm.last_callinfo().root;
                vm.suspended_traps = *traps;
                // ↔ C++ 把 clo 变量复用为 CallNative 的 retval 输出——
                // 挂起时返回的是 native 的返回值（suspend 的参数）
                return Ok(CallFlow::Suspended(retval));
            }
            if sarg0 != -1 && !tailcall {
                vm.stk_set(arg0, retval);
            }
        }
        Value::Class(c) => {
            let c = c.clone();
            // ↔ CreateClassInstance
            let inst = Value::Instance(SQClass::create_instance(&c, &vm.ss));
            let ctor = c.borrow().get_constructor();
            if sarg0 != -1 {
                vm.stk_set(arg0, inst.clone());
            }
            match ctor {
                Some(cv) => match &cv {
                    Value::Closure(_) => {
                        let stkbase = vm.stackbase + arg2;
                        let i = stkbase as usize;
                        vm.ensure_stack(i);
                        vm.stack[i] = inst.clone();
                        let cc = match &cv {
                            Value::Closure(cc) => cc.clone(),
                            _ => unreachable!(),
                        };
                        vm.start_call(&cc, -1, arg3, stkbase, false)?;
                    }
                    Value::NativeClosure(nc) => {
                        let nc = nc.clone();
                        let stkbase = vm.stackbase + arg2;
                        let i = stkbase as usize;
                        vm.ensure_stack(i);
                        vm.stack[i] = inst.clone();
                        let mut dummy1 = false;
                        let mut dummy2 = false;
                        let mut retval = Value::Null;
                        vm.call_native(nc, arg3, stkbase, &mut retval, -1, &mut dummy1, &mut dummy2)?;
                    }
                    _ => {}
                },
                None => {}
            }
        }
        Value::Table(_) | Value::Userdata(_) | Value::Instance(_) => {
            let closure = vm.get_meta_method_of(&clo, 11 /*MT_CALL*/);
            if let Some(closure) = closure {
                vm.push(clo.clone());
                for i in 0..arg3 {
                    let v = vm.stk(arg2 + i);
                    vm.push(v);
                }
                let res = vm.call_meta_method(closure, arg3 + 1)?;
                if sarg0 != -1 {
                    vm.stk_set(arg0, res);
                }
            } else {
                vm.raise_error_str(format!(
                    "attempt to call '{}'",
                    get_type_name(&clo).unwrap_or("?")
                ));
                throw!(vm)
            }
        }
        _ => {
            vm.raise_error_str(format!(
                "attempt to call '{}'",
                get_type_name(&clo).unwrap_or("?")
            ));
            throw!(vm)
        }
    }
    let _ = (vm_rc, _outres);
    Ok(CallFlow::Continue)
}

/// ↔ sq_wakeupvm 的执行入口（baselib 线程唤醒用）
pub fn execute_resume(
    vm: &mut VmData,
    vm_rc: &ThreadRef,
    outres: &mut Value,
    et: ExecutionType,
) -> bool {
    execute_inner(vm, vm_rc, &Value::Null, 0, 0, outres, true, et).is_ok()
}

#[cfg(test)]
mod tests {
    use super::sq_fmod;

    /// ↔ UCRT mod 的 NaN 符号位（实测见 tools/probes/nan_probe.nut 全组合）
    #[test]
    fn fmod_matches_msvc_nan_signs() {
        // x % 0（x 有限）→ 正 NaN，与 x 的符号无关
        assert_eq!(sq_fmod(1.0, 0.0).to_bits(), f32::NAN.to_bits());
        assert_eq!(sq_fmod(-1.0, 0.0).to_bits(), f32::NAN.to_bits());
        assert_eq!(sq_fmod(-0.0, 0.0).to_bits(), f32::NAN.to_bits());
        // x 是 inf → NaN，符号跟 x
        assert_eq!(sq_fmod(f32::INFINITY, 1.0).to_bits(), f32::NAN.to_bits());
        let n = sq_fmod(f32::NEG_INFINITY, 1.0);
        assert!(n.is_nan() && n.is_sign_negative());
        // NaN 操作数原样传播
        let q = f32::from_bits(0xFFC0_0000);
        assert_eq!(sq_fmod(q, 1.0).to_bits(), q.to_bits());
        assert_eq!(sq_fmod(1.0, q).to_bits(), q.to_bits());
        // 正常值仍然是普通 fmod
        assert_eq!(sq_fmod(7.0, 2.0), 1.0);
        assert_eq!(sq_fmod(-7.0, 2.0), -1.0);
        assert_eq!(sq_fmod(0.0, 2.0), 0.0);
        assert_eq!(sq_fmod(-0.0, 2.0).to_bits(), (-0.0f32).to_bits());
    }
}

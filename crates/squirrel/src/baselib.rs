//! ↔ squirrel/sqbaselib.cpp —— 内建 base 库 + 11 张默认委托表。
//!
//! 原生函数用 Rust 闭包（NativeFn = Rc<dyn Fn(&mut VmData) -> SQInteger>）；
//! C++ 经 sq_* C API 操作栈，这里直接用 VmData 的栈方法，语义一致。

use std::rc::Rc;

use crate::array::SQArray;
use crate::closure::{GeneratorState, NativeFn};
use crate::config::*;
use crate::object::{get_type_name, ArrayRef, TableRef, Value};
use crate::state::{compile_typemask, SharedState};
use crate::table::SQTable;
use crate::vm::VmData;

type NativeResult = SQInteger;

fn ok0() -> NativeResult {
    0
}

/// ↔ sq_throwerror：设置 lasterror 并返回 SQ_ERROR
fn throw_error(vm: &mut VmData, msg: &str) -> NativeResult {
    vm.raise_error_str(msg);
    SQ_ERROR
}

/// ↔ str2num
fn str2num(s: &[u8], base: SQInteger) -> Option<Value> {
    let iseintbase = base > 13;
    let mut isfloat = false;
    for &c in s {
        if c == b'.' || (!iseintbase && (c == b'E' || c == b'e')) {
            isfloat = true;
            break;
        }
    }
    let txt = String::from_utf8_lossy(s);
    if isfloat {
        let t = txt.trim_start();
        // CRT strtod 接受前导空白与正负号
        match parse_c_strtod(t) {
            Some(f) => Some(Value::Float(f as SQFloat)),
            None => None,
        }
    } else {
        let t = txt.trim_start();
        let (neg, digits) = match t.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, t.strip_prefix('+').unwrap_or(t)),
        };
        // ↔ strtol(base=16)：跳过可选的 0x/0X 前缀
        let digits = if base == 16 {
            digits.strip_prefix("0x").or_else(|| digits.strip_prefix("0X")).unwrap_or(digits)
        } else {
            digits
        };
        let mut r: SQInteger = 0;
        let mut any = false;
        for c in digits.bytes() {
            let v = match c {
                b'0'..=b'9' => (c - b'0') as SQInteger,
                b'a'..=b'f' | b'A'..=b'F' if base > 10 => {
                    (c.to_ascii_lowercase() - b'a' + 10) as SQInteger
                }
                _ => break, // ↔ strtol：遇非法字符停止（前缀语义，非失败）
            };
            if v >= base {
                break;
            }
            r = r.wrapping_mul(base).wrapping_add(v);
            any = true;
        }
        if !any {
            return None;
        }
        Some(Value::Integer(if neg { r.wrapping_neg() } else { r }))
    }
}

/// CRT strtod 的近似（前后空白已去除；支持 1.5e3 形式）
fn parse_c_strtod(t: &str) -> Option<f64> {
    let mut normalized = String::new();
    let b = t.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        normalized.push(b[i] as char);
        i += 1;
    }
    let mut any = false;
    while i < b.len() && b[i].is_ascii_digit() {
        normalized.push(b[i] as char);
        i += 1;
        any = true;
    }
    if i < b.len() && b[i] == b'.' {
        normalized.push('.');
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            normalized.push(b[i] as char);
            i += 1;
            any = true;
        }
    }
    if !any {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut exp = String::from("e");
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            exp.push(b[i] as char);
            i += 1;
        }
        let mut anye = false;
        while i < b.len() && b[i].is_ascii_digit() {
            exp.push(b[i] as char);
            i += 1;
            anye = true;
        }
        if anye {
            normalized.push_str(&exp);
        }
    }
    normalized.parse::<f64>().ok()
}

/// 注册一个原生函数到表（↔ CreateDefaultDelegate 的一步）
fn reg(
    ss: &Rc<SharedState>,
    t: &TableRef,
    name: &str,
    f: NativeFn,
    nparamscheck: SQInteger,
    typemask: Option<&str>,
) {
    let nc = crate::closure::SQNativeClosure::create(ss, f, 0);
    {
        let mut n = nc.borrow_mut();
        n.nparamscheck = nparamscheck;
        n.name = ss.intern(name.as_bytes());
        if let Some(tm) = typemask {
            compile_typemask(&mut n.typecheck, tm.as_bytes());
        }
    }
    let key = ss.intern(name.as_bytes());
    t.borrow_mut().new_slot(&key, &Value::NativeClosure(nc));
}

// ---------------------------------------------------------------------------
// base 库（root table 注册）
// ---------------------------------------------------------------------------

/// ↔ sq_base_register
pub fn base_register(vm: &mut VmData) {
    let ss = vm.ss.clone();
    let root = match &vm.roottable {
        Value::Table(t) => t.clone(),
        _ => return,
    };

    macro_rules! simple {
        ($name:expr, $npc:expr, $tm:expr, $f:expr) => {
            reg(&ss, &root, $name, Rc::new($f), $npc, $tm);
        };
    }

    simple!("seterrorhandler", 2, None, |v: &mut VmData| {
        // ↔ base_seterrorhandler：栈顶闭包设为 errorhandler
        let top = v.pop_get();
        v.errorhandler = top;
        ok0()
    });
    simple!("setdebughook", 2, None, |v: &mut VmData| {
        let top = v.pop_get();
        set_debug_hook(v, top);
        ok0()
    });
    simple!("enabledebuginfo", 2, None, |v: &mut VmData| {
        let o = v.stack_get(2);
        v.ss.debuginfo.set(!VmData::is_false(&o));
        ok0()
    });
    simple!("getstackinfos", 2, Some(".n"), base_getstackinfos);
    simple!("getroottable", 1, None, |v: &mut VmData| {
        let rt = v.roottable.clone();
        v.push(rt);
        1
    });
    simple!("setroottable", 2, None, |v: &mut VmData| {
        let old = v.roottable.clone();
        let top = v.pop_get();
        v.roottable = top;
        v.push(old);
        1
    });
    simple!("getconsttable", 1, None, |v: &mut VmData| {
        let c = v.ss.consts.borrow().clone();
        v.push(c);
        1
    });
    simple!("setconsttable", 2, None, |v: &mut VmData| {
        let old = v.ss.consts.borrow().clone();
        let top = v.pop_get();
        *v.ss.consts.borrow_mut() = top;
        v.push(old);
        1
    });
    simple!("assert", -2, None, base_assert);
    simple!("print", 2, None, base_print);
    simple!("error", 2, None, base_error);
    simple!("compilestring", -2, Some(".ss"), base_compilestring);
    simple!("newthread", 2, Some(".c"), base_newthread);
    simple!("suspend", -1, None, base_suspend);
    simple!("array", -2, Some(".n"), base_array);
    simple!("type", 2, None, |v: &mut VmData| {
        let o = v.stack_get(2);
        let name = get_type_name(&o).unwrap_or("?");
        let s = v.ss.intern(name.as_bytes());
        v.push(s);
        1
    });
    simple!("callee", 0, None, |v: &mut VmData| {
        if v.callsstack.len() > 1 {
            let c = v.callsstack[v.callsstack.len() - 2].closure.clone();
            v.push(c);
            1
        } else {
            throw_error(v, "no closure in the calls stack")
        }
    });
    simple!("dummy", 0, None, |_v: &mut VmData| ok0());
    simple!("collectgarbage", 0, None, |v: &mut VmData| {
        let ss = v.ss.clone();
        let n = ss.collect_garbage(v);
        v.push(Value::Integer(n));
        1
    });
    simple!("resurrectunreachable", 0, None, |v: &mut VmData| {
        let ss = v.ss.clone();
        // ↔ 没有不可达对象时压 null（参考实现同样如此）
        let (_n, arr) = ss.resurrect_unreachable(v);
        match arr {
            Some(a) => v.push(Value::Array(a)),
            None => v.push_null(),
        }
        1
    });

    // 版本常量
    let mut slots: Vec<(&str, Value)> = vec![
        ("_versionnumber_", Value::Integer(SQUIRREL_VERSION_NUMBER)),
        ("_version_", ss.intern(SQUIRREL_VERSION.as_bytes())),
        ("_charsize_", Value::Integer(1)),
        ("_intsize_", Value::Integer(8)),
        ("_floatsize_", Value::Integer(4)),
    ];
    for (name, val) in slots.drain(..) {
        let key = ss.intern(name.as_bytes());
        root.borrow_mut().new_slot(&key, &val);
    }

    // 默认委托表内容（↔ SQSharedState::Init 的 CreateDefaultDelegate）
    fill_table_ddel(&ss);
    fill_array_ddel(&ss);
    fill_string_ddel(&ss);
    fill_number_ddel(&ss);
    fill_generator_ddel(&ss);
    fill_closure_ddel(&ss);
    fill_thread_ddel(&ss);
    fill_class_ddel(&ss);
    fill_instance_ddel(&ss);
    fill_weakref_ddel(&ss);
}

fn set_debug_hook(v: &mut VmData, hook: Value) {
    match &hook {
        Value::Closure(_) => {
            v.debughook = true;
            v.debughook_native = None;
            v.debughook_closure = hook;
        }
        Value::NativeClosure(nc) => {
            v.debughook = true;
            v.debughook_native = None;
            let nc2 = nc.clone();
            v.debughook_closure = Value::NativeClosure(nc2);
        }
        _ => {
            v.debughook = false;
            v.debughook_native = None;
            v.debughook_closure = Value::Null;
        }
    }
}

/// ↔ `_printfunc(v, "%s", str)`：C 的 `%s` 在第一个 NUL 处截断。
/// 例如 `print((0).tochar())` 在原版里什么都不输出（整行都被截掉），必须一致。
pub fn c_string_prefix(s: &[u8]) -> &[u8] {
    match s.iter().position(|b| *b == 0) {
        Some(i) => &s[..i],
        None => s,
    }
}

fn base_print(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(2);
    match v.to_string_value(&o) {
        Ok(s) => {
            if let Some(pf) = v.ss.printfunc.borrow().clone() {
                pf(v, &String::from_utf8_lossy(c_string_prefix(s.as_bytes())));
            }
            ok0()
        }
        Err(_) => SQ_ERROR,
    }
}

fn base_error(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(2);
    match v.to_string_value(&o) {
        Ok(s) => {
            if let Some(ef) = v.ss.errorfunc.borrow().clone() {
                ef(v, &String::from_utf8_lossy(c_string_prefix(s.as_bytes())));
            }
            ok0()
        }
        Err(_) => SQ_ERROR,
    }
}

fn base_assert(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(2);
    if VmData::is_false(&o) {
        let top = v.top;
        if top > 2 + v.stackbase {
            let o3 = v.stack_get(3);
            if let Ok(s) = v.to_string_value(&o3) {
                return throw_error(v, &String::from_utf8_lossy(s.as_bytes()));
            }
        }
        return throw_error(v, "assertion failed");
    }
    ok0()
}

fn base_compilestring(v: &mut VmData) -> NativeResult {
    let src = v.stack_get(2);
    let name = if v.top - v.stackbase > 2 {
        v.stack_get(3)
    } else {
        v.ss.intern(b"unnamedbuffer")
    };
    let srcbytes = src.as_bytes().to_vec();
    let namebytes = name.as_bytes().to_vec();
    let mut pos = 0usize;
    let readf = Box::new(move || {
        if pos < srcbytes.len() {
            let c = srcbytes[pos];
            pos += 1;
            c as SQInteger
        } else {
            0
        }
    });
    match crate::compiler::compile(v, readf, &namebytes, false, false) {
        Ok(proto) => {
            // ↔ sq_compile：包装为闭包
            let root_weak = match &v.roottable {
                Value::Table(t) => Some(Value::Table(t.clone()).get_weakref_as_weakref()),
                _ => None,
            };
            let fp = match &proto {
                Value::FuncProto(f) => f.clone(),
                _ => unreachable!(),
            };
            let clo = crate::closure::SQClosure::create(&v.ss, fp, root_weak);
            v.push(Value::Closure(clo));
            1
        }
        Err(()) => SQ_ERROR,
    }
}

fn base_newthread(v: &mut VmData) -> NativeResult {
    let func = v.stack_get(2);
    let stksize = match &func {
        Value::Closure(c) => (c.borrow().function.stacksize << 1) + 2,
        _ => MIN_STACK_OVERHEAD + 2,
    };
    let stk = if stksize < MIN_STACK_OVERHEAD + 2 {
        MIN_STACK_OVERHEAD + 2
    } else {
        stksize
    };
    let newv = create_thread(v, stk);
    // ↔ sq_move(newv, v, -2)：把 func 移入新线程栈
    let f = v.pop_get();
    if let Value::Thread(t) = &newv {
        let mut tv = t.borrow_mut();
        tv.push(f);
    }
    v.push(newv);
    1
}

/// ↔ sq_newthread
pub fn create_thread(friend: &mut VmData, stacksize: SQInteger) -> Value {
    let ss = friend.ss.clone();
    let rc = crate::object::ThreadRef::new(std::cell::RefCell::new(VmData::new(ss.clone(), stacksize)));
    rc.borrow().set_self_ref(&rc);
    {
        let mut nv = rc.borrow_mut();
        nv.init(Some(friend), stacksize);
    }
    let entry = crate::state::GcEntry::Thread(Rc::downgrade(&rc));
    ss.gc_add(entry);
    Value::Thread(rc)
}

fn base_suspend(v: &mut VmData) -> NativeResult {
    v.suspend()
}

fn base_array(v: &mut VmData) -> NativeResult {
    let size = v.stack_get(2);
    let a = if v.top - v.stackbase > 2 {
        let fill = v.stack_get(3);
        let arr = SQArray::create(&v.ss, 0);
        arr.borrow_mut().resize(size.as_integer(), &fill);
        arr
    } else {
        SQArray::create(&v.ss, size.as_integer())
    };
    v.push(Value::Array(a));
    1
}

fn base_getstackinfos(v: &mut VmData) -> NativeResult {
    let level = v.stack_get(2).as_integer();
    match get_callstack_infos(v, level) {
        Some(_) => 1, // 结果已在栈顶
        None => 0,
    }
}

/// ↔ __getcallstackinfos（结果表压栈）
pub fn get_callstack_infos(v: &mut VmData, level: SQInteger) -> Option<()> {
    let cssize = v.callsstack.len() as SQInteger;
    // 负 level 在 C++ 里是越界读（UB）；这里返回 null，不 panic
    if level >= 0 && cssize > level {
        let ci_idx = (cssize - level - 1) as usize;
        let (fn_name, src, line) = match &v.callsstack[ci_idx].closure {
            Value::Closure(c) => {
                let proto = c.borrow().function.clone();
                let line = proto.get_line(v.callsstack[ci_idx].ip as SQInteger);
                (
                    string_or(&proto.name, "unknown"),
                    string_or(&proto.sourcename, "unknown"),
                    line,
                )
            }
            Value::NativeClosure(nc) => {
                let n = nc.borrow().name.clone();
                (string_or(&n, "unknown"), "NATIVE".to_string(), -1)
            }
            _ => ("unknown".to_string(), "unknown".to_string(), -1),
        };
        let t = SQTable::create(&v.ss, 4);
        {
            let mut tt = t.borrow_mut();
            let k = v.ss.intern(b"func");
            let val = v.ss.intern(fn_name.as_bytes());
            tt.new_slot(&k, &val);
            let k = v.ss.intern(b"src");
            let val = v.ss.intern(src.as_bytes());
            tt.new_slot(&k, &val);
            let k = v.ss.intern(b"line");
            tt.new_slot(&k, &Value::Integer(line));
            let k = v.ss.intern(b"locals");
            let lt = SQTable::create(&v.ss, 0);
            tt.new_slot(&k, &Value::Table(lt.clone()));
        }
        // 局部变量（↔ sq_getlocal 循环，seq 从 0 递增直到返回 NULL）
        let mut seq = 0;
        loop {
            let name = get_local_at(v, level, seq);
            match name {
                Some(nm) => {
                    // 栈顶已是局部变量值（get_local_at 压栈）
                    let val = v.pop_get();
                    let locals_key = v.ss.intern(b"locals");
                    let locals = t.borrow().get(&locals_key);
                    if let Some(Value::Table(lt)) = locals {
                        let k = v.ss.intern(nm.as_bytes());
                        lt.borrow_mut().new_slot(&k, &val);
                    }
                    seq += 1;
                }
                None => break,
            }
        }
        v.push(Value::Table(t));
        Some(())
    } else {
        None
    }
}

fn string_or(v: &Value, default: &str) -> String {
    match v {
        Value::String(s) => String::from_utf8_lossy(&s.val).into_owned(),
        _ => default.to_string(),
    }
}

/// ↔ sq_getlocal：level 0 = 当前（原生）帧，1 = 其调用者……
/// 顺序为「先 outer 变量，再 localvarinfos（声明序）」；值压栈并返回变量名。
pub fn get_local_at(v: &mut VmData, level: SQInteger, nseq: SQInteger) -> Option<String> {
    let cstksize = v.callsstack.len() as SQInteger;
    let lvl = cstksize - level - 1;
    if level < 0 || lvl < 0 || lvl >= cstksize {
        return None;
    }
    // ↔ stackbase -= ci._prevstkbase 链（prevstkbase 是相对增量）
    let mut stackbase = v.stackbase;
    for i in 0..level {
        let ci = &v.callsstack[(cstksize - i - 1) as usize];
        stackbase -= ci.prevstkbase as SQInteger;
    }
    let fi = lvl as usize;
    let c = match &v.callsstack[fi].closure {
        Value::Closure(c) => c.clone(),
        _ => return None, // 原生帧没有局部变量信息
    };
    let proto = c.borrow().function.clone();
    // ↔ if(_noutervalues > idx)：outer 变量排在局部变量之前
    let nouter = proto.outervalues.len() as SQInteger;
    if nouter > nseq {
        let outer = match c.borrow().outervalues.get(nseq as usize) {
            Some(Value::Outer(o)) => o.clone(),
            _ => return None,
        };
        // ↔ *_outer(...)->_valptr：打开的 outer 取栈槽，已关闭的取自身 _value
        let (open_idx, closed) = {
            let o = outer.borrow();
            (o.open_idx, o.value.clone())
        };
        let val = match open_idx {
            Some(i) => v.stack[i].clone(),
            None => closed,
        };
        v.push(val);
        return match proto.outervalues.get(nseq as usize).map(|ov| &ov.name) {
            Some(Value::String(s)) => Some(String::from_utf8_lossy(&s.val).into_owned()),
            _ => None,
        };
    }
    // ↔ (ci._ip - func->_instructions) - 1：取指后 ip 已自增，故减一
    let nop = (v.callsstack[fi].ip as SQInteger - 1).max(0) as SQUnsignedInteger;
    proto.get_local(
        v,
        stackbase as SQUnsignedInteger,
        (nseq - nouter) as SQUnsignedInteger,
        nop,
    )
}

// ---------------------------------------------------------------------------
// 通用默认委托函数
// ---------------------------------------------------------------------------

fn default_delegate_len(v: &mut VmData) -> NativeResult {
    let n = get_size_of(v, 1);
    v.push(Value::Integer(n));
    1
}

/// ↔ sq_getsize
pub fn get_size_of(v: &VmData, idx: SQInteger) -> SQInteger {
    let o = v.stack_get(idx);
    match &o {
        Value::String(s) => s.len(),
        Value::Table(t) => t.borrow().count_used(),
        Value::Array(a) => a.borrow().size(),
        Value::Class(c) => c.borrow().defaultvalues.len() as SQInteger,
        Value::Instance(i) => i
            .borrow()
            .class
            .as_ref()
            .map(|c| c.borrow().defaultvalues.len() as SQInteger)
            .unwrap_or(0),
        Value::Generator(g) => match g.borrow().state {
            GeneratorState::Dead => -1,
            _ => 0,
        },
        _ => 0,
    }
}

fn default_delegate_tofloat(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    match &o {
        Value::String(_) => {
            let res = str2num(o.as_bytes(), 10);
            match res {
                Some(Value::Integer(i)) => v.push(Value::Integer(i as SQFloat as SQInteger).float_value()),
                Some(Value::Float(f)) => v.push(Value::Float(f)),
                _ => return throw_error(v, "cannot convert the string"),
            }
        }
        Value::Integer(_) | Value::Float(_) => {
            v.push(Value::Float(o.as_float()));
        }
        Value::Bool(b) => {
            v.push(Value::Float(if *b { 1.0 } else { 0.0 }));
        }
        _ => v.push_null(),
    }
    1
}

impl Value {
    fn float_value(self) -> Value {
        Value::Float(self.as_float())
    }
}

fn default_delegate_tointeger(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let base = if v.top - v.stackbase > 1 {
        v.stack_get(2).as_integer()
    } else {
        10
    };
    match &o {
        Value::String(_) => {
            let res = str2num(o.as_bytes(), base);
            match res {
                Some(Value::Integer(i)) => v.push(Value::Integer(i)),
                Some(Value::Float(f)) => v.push(Value::Integer(c_f32_to_i64(f))),
                _ => return throw_error(v, "cannot convert the string"),
            }
        }
        Value::Integer(_) | Value::Float(_) => {
            v.push(Value::Integer(o.as_integer()));
        }
        Value::Bool(b) => {
            v.push(Value::Integer(if *b { 1 } else { 0 }));
        }
        _ => v.push_null(),
    }
    1
}

fn default_delegate_tostring(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    match v.to_string_value(&o) {
        Ok(s) => {
            v.push(s);
            1
        }
        Err(_) => SQ_ERROR,
    }
}

fn obj_delegate_weakref(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let w = o.get_weakref();
    v.push(w);
    1
}

fn obj_clear(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    match &o {
        Value::Table(t) => {
            t.borrow_mut().clear();
            1
        }
        Value::Array(a) => {
            let mut ab = a.borrow_mut();
            ab.resize(0, &Value::Null);
            1
        }
        _ => SQ_ERROR,
    }
}

fn number_delegate_tochar(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let c = o.as_integer() as u8;
    let s = v.ss.intern(&[c]);
    v.push(s);
    1
}

/// ↔ get_slice_params
fn get_slice_params(v: &mut VmData) -> Option<(SQInteger, SQInteger, Value)> {
    let top = v.top - v.stackbase;
    let mut sidx = 0;
    let mut eidx = 0;
    let o = v.stack_get(1);
    if top > 1 {
        let start = v.stack_get(2);
        if !start.is_null() && start.type_() & SQOBJECT_NUMERIC != 0 {
            sidx = start.as_integer();
        }
    }
    if top > 2 {
        let end = v.stack_get(3);
        if end.type_() & SQOBJECT_NUMERIC != 0 {
            eidx = end.as_integer();
        }
    } else {
        eidx = get_size_of(v, 1);
    }
    Some((sidx, eidx, o))
}

// ---------------------------------------------------------------------------
// 各默认委托表
// ---------------------------------------------------------------------------

fn fill_table_ddel(ss: &Rc<SharedState>) {
    let t = match &*ss.table_default_delegate.borrow() {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    reg(ss, &t, "len", Rc::new(default_delegate_len), 1, Some("t"));
    reg(ss, &t, "rawget", Rc::new(container_rawget), 2, Some("t"));
    reg(ss, &t, "rawset", Rc::new(container_rawset), 3, Some("t"));
    reg(ss, &t, "rawdelete", Rc::new(table_rawdelete), 2, Some("t"));
    reg(ss, &t, "rawin", Rc::new(container_rawexists), 2, Some("t"));
    reg(ss, &t, "weakref", Rc::new(obj_delegate_weakref), 1, None);
    reg(ss, &t, "tostring", Rc::new(default_delegate_tostring), 1, Some("."));
    reg(ss, &t, "clear", Rc::new(obj_clear), 1, Some("."));
    reg(ss, &t, "setdelegate", Rc::new(table_setdelegate), 2, Some(".t|o"));
    reg(ss, &t, "getdelegate", Rc::new(table_getdelegate), 1, Some("."));
    reg(ss, &t, "filter", Rc::new(table_filter), 2, Some("tc"));
    reg(ss, &t, "map", Rc::new(table_map), 2, Some("tc"));
    reg(ss, &t, "keys", Rc::new(table_keys), 1, Some("t"));
    reg(ss, &t, "values", Rc::new(table_values), 1, Some("t"));
}

fn container_rawget(v: &mut VmData) -> NativeResult {
    // ↔ sq_rawget(v,-2)：self 在 -2，key 在 -1
    let self_ = v.get_up(-2);
    let key = v.get_up(-1);
    match raw_get(v, &self_, &key) {
        Some(val) => {
            v.pop();
            v.push(val);
            1
        }
        None => {
            v.pop();
            // ↔ sq_rawget 的失败分支：只有数组的非法索引类型是另一条消息
            let msg = match &self_ {
                Value::Array(_) if !matches!(key, Value::Integer(_) | Value::Float(_)) => {
                    "invalid index type for an array"
                }
                Value::Table(_) | Value::Class(_) | Value::Instance(_) | Value::Array(_) => {
                    "the index doesn't exist"
                }
                _ => "rawget works only on array/table/instance and class",
            };
            v.raise_error_str(msg);
            SQ_ERROR
        }
    }
}

fn raw_get(_v: &mut VmData, self_: &Value, key: &Value) -> Option<Value> {
    match self_ {
        Value::Table(t) => t.borrow().get(key),
        Value::Class(c) => c.borrow().get(key),
        Value::Instance(i) => i.borrow().get(key),
        Value::Array(a) => {
            if matches!(key, Value::Integer(_) | Value::Float(_)) {
                a.borrow().get(key.as_integer())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn container_rawset(v: &mut VmData) -> NativeResult {
    let self_ = v.get_up(-3);
    let key = v.get_up(-2);
    let val = v.get_up(-1);
    match &self_ {
        Value::Table(t) => {
            t.borrow_mut().new_slot(&key, &val);
            // sq_rawset：弹 key/val，保留 self 在栈顶
            v.pop_n(2);
            1
        }
        Value::Class(c) => {
            let ok = c.borrow_mut().new_slot(&v.ss, &key, &val, false);
            v.pop_n(2);
            if ok {
                1
            } else {
                SQ_ERROR
            }
        }
        Value::Instance(i) => {
            let ok = i.borrow_mut().set(&key, &val);
            v.pop_n(2);
            if ok {
                1
            } else {
                SQ_ERROR
            }
        }
        _ => SQ_ERROR,
    }
}

fn table_rawdelete(v: &mut VmData) -> NativeResult {
    let self_ = v.stack_get(1);
    let key = v.stack_get(2);
    if let Value::Table(t) = &self_ {
        let old = t.borrow().get(&key);
        t.borrow_mut().remove(&key);
        match old {
            Some(o) => {
                v.push(o);
                1
            }
            None => {
                v.push_null();
                1
            }
        }
    } else {
        SQ_ERROR
    }
}

fn container_rawexists(v: &mut VmData) -> NativeResult {
    let self_ = v.stack_get(1);
    let key = v.stack_get(2);
    let r = raw_get(v, &self_, &key);
    v.push(Value::Bool(r.is_some()));
    1
}

/// ↔ sq_setdelegate + table_setdelegate：
/// table + table → 环检测失败报 "delegate cycle"；table + null → 清除委托；
/// 其它组合 → "unexpected type <类型名>"。
fn table_setdelegate(v: &mut VmData) -> NativeResult {
    let self_ = v.get_up(-2);
    let mt = v.pop_get();
    match &self_ {
        Value::Table(t) => match &mt {
            Value::Table(d) => {
                // 身份用 RefCell 分配地址（与 Value::identity() 一致），
                // 这样自环/间接环的检测才和 C++ 的指针比较等价
                let identity = std::rc::Rc::as_ptr(t) as usize;
                let ok = {
                    let mut tb = t.borrow_mut();
                    crate::object::set_delegate_checked(&mut tb.delegate, identity, Some(d.clone()))
                };
                if !ok {
                    return throw_error(v, "delegate cycle");
                }
            }
            Value::Null => {
                t.borrow_mut().delegate = None;
            }
            other => {
                let name = crate::object::get_type_name(other).unwrap_or("unknown");
                return throw_error(v, &format!("unexpected type {}", name));
            }
        },
        other => {
            let name = crate::object::get_type_name(other).unwrap_or("unknown");
            return throw_error(v, &format!("unexpected type {}", name));
        }
    }
    v.push(self_);
    1
}

fn table_getdelegate(v: &mut VmData) -> NativeResult {
    let self_ = v.get_up(-1);
    if let Value::Table(t) = &self_ {
        let d = t.borrow().delegate.clone();
        match d {
            Some(d) => v.push(Value::Table(d)),
            None => v.push_null(),
        }
        1
    } else {
        SQ_ERROR
    }
}

fn table_filter(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let closure = v.stack_get(2);
    let ret = SQTable::create(&v.ss, 0);
    if let Value::Table(tbl) = &o {
        let mut refpos = Value::Null;
        loop {
            let nxt = tbl.borrow().next(false, &refpos);
            let (nitr, key, val) = match nxt {
                Some(x) => x,
                None => break,
            };
            refpos = Value::Integer(nitr);
            v.push(o.clone());
            v.push(key.clone());
            v.push(val.clone());
            match v.call_value(&closure, 3, v.top - 3, false) {
                Ok(r) => {
                    if !VmData::is_false(&r) {
                        ret.borrow_mut().new_slot(&key, &val);
                    }
                    v.pop();
                }
                Err(_) => return SQ_ERROR,
            }
        }
    }
    v.push(Value::Table(ret));
    1
}

fn table_map(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let closure = v.stack_get(2);
    let nitems = match &o {
        Value::Table(t) => t.borrow().count_used(),
        _ => 0,
    };
    let ret = SQArray::create(&v.ss, nitems);
    {
        let mut ab = ret.borrow_mut();
        ab.resize(nitems, &Value::Null);
    }
    if let Value::Table(tbl) = &o {
        let mut n = 0;
        let mut refpos = Value::Null;
        loop {
            let nxt = tbl.borrow().next(false, &refpos);
            let (nitr, key, val) = match nxt {
                Some(x) => x,
                None => break,
            };
            refpos = Value::Integer(nitr);
            v.push(o.clone());
            v.push(key.clone());
            v.push(val.clone());
            match v.call_value(&closure, 3, v.top - 3, false) {
                Ok(r) => {
                    ret.borrow_mut().set(n, &r);
                    v.pop();
                }
                Err(_) => return SQ_ERROR,
            }
            n += 1;
        }
    }
    v.push(Value::Array(ret));
    1
}

fn table_keys_values(v: &mut VmData, keys: bool) -> NativeResult {
    let o = v.stack_get(1);
    let nitems = match &o {
        Value::Table(t) => t.borrow().count_used(),
        _ => 0,
    };
    let a = SQArray::create(&v.ss, nitems);
    {
        let mut ab = a.borrow_mut();
        ab.resize(nitems, &Value::Null);
    }
    if let Value::Table(t) = &o {
        let mut n = 0;
        let mut refpos = Value::Null;
        while let Some((nitr, key, val)) = t.borrow().next(false, &refpos) {
            refpos = Value::Integer(nitr);
            let x = if keys { key } else { val };
            a.borrow_mut().set(n, &x);
            n += 1;
        }
    }
    v.push(Value::Array(a));
    1
}

fn table_keys(v: &mut VmData) -> NativeResult {
    table_keys_values(v, true)
}

fn table_values(v: &mut VmData) -> NativeResult {
    table_keys_values(v, false)
}

fn fill_array_ddel(ss: &Rc<SharedState>) {
    let t = match &*ss.array_default_delegate.borrow() {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    reg(ss, &t, "len", Rc::new(default_delegate_len), 1, Some("a"));
    reg(ss, &t, "append", Rc::new(array_append), 2, Some("a"));
    reg(ss, &t, "extend", Rc::new(array_extend), 2, Some("aa"));
    reg(ss, &t, "push", Rc::new(array_append), 2, Some("a"));
    reg(ss, &t, "pop", Rc::new(array_pop), 1, Some("a"));
    reg(ss, &t, "top", Rc::new(array_top), 1, Some("a"));
    reg(ss, &t, "insert", Rc::new(array_insert), 3, Some("an"));
    reg(ss, &t, "remove", Rc::new(array_remove), 2, Some("an"));
    reg(ss, &t, "resize", Rc::new(array_resize), -2, Some("an"));
    reg(ss, &t, "reverse", Rc::new(array_reverse), 1, Some("a"));
    reg(ss, &t, "sort", Rc::new(array_sort), -1, Some("ac"));
    reg(ss, &t, "slice", Rc::new(array_slice), -1, Some("ann"));
    reg(ss, &t, "weakref", Rc::new(obj_delegate_weakref), 1, None);
    reg(ss, &t, "tostring", Rc::new(default_delegate_tostring), 1, Some("."));
    reg(ss, &t, "clear", Rc::new(obj_clear), 1, Some("."));
    reg(ss, &t, "map", Rc::new(array_map), 2, Some("ac"));
    reg(ss, &t, "apply", Rc::new(array_apply), 2, Some("ac"));
    reg(ss, &t, "reduce", Rc::new(array_reduce), -2, Some("ac."));
    reg(ss, &t, "filter", Rc::new(array_filter), 2, Some("ac"));
    reg(ss, &t, "find", Rc::new(array_find), 2, Some("a."));
}

fn array_append(v: &mut VmData) -> NativeResult {
    let self_ = v.get_up(-2);
    let val = v.get_up(-1);
    if let Value::Array(a) = &self_ {
        a.borrow_mut().append(&val);
        v.pop();
        1
    } else {
        SQ_ERROR
    }
}

fn array_extend(v: &mut VmData) -> NativeResult {
    let self_ = v.stack_get(1);
    let other = v.stack_get(2);
    if let (Value::Array(a), Value::Array(b)) = (&self_, &other) {
        let ext = b.borrow().values.clone();
        a.borrow_mut().values.extend(ext.iter().cloned());
    }
    v.pop();
    1
}

fn array_pop(v: &mut VmData) -> NativeResult {
    let self_ = v.stack_get(1);
    if let Value::Array(a) = &self_ {
        let mut ab = a.borrow_mut();
        // 先判空再取（↔ C++ Pop 前的 size 检查）
        if let Some(t) = ab.values.pop() {
            drop(ab);
            v.push(t);
            return 1;
        }
    }
    throw_error(v, "empty array")
}

fn array_top(v: &mut VmData) -> NativeResult {
    let self_ = v.stack_get(1);
    if let Value::Array(a) = &self_ {
        let ab = a.borrow();
        if ab.size() > 0 {
            let t = ab.top();
            drop(ab);
            v.push(t);
            return 1;
        }
    }
    throw_error(v, "top() on a empty array")
}

fn array_insert(v: &mut VmData) -> NativeResult {
    let self_ = v.stack_get(1);
    let idx = v.stack_get(2);
    let val = v.stack_get(3);
    if let Value::Array(a) = &self_ {
        let ok = a.borrow_mut().insert(idx.as_integer(), &val);
        if !ok {
            return throw_error(v, "index out of range");
        }
        v.pop_n(2);
        1
    } else {
        SQ_ERROR
    }
}

fn array_remove(v: &mut VmData) -> NativeResult {
    let self_ = v.stack_get(1);
    let idx = v.stack_get(2);
    if idx.type_() & SQOBJECT_NUMERIC == 0 {
        return throw_error(v, "wrong type");
    }
    if let Value::Array(a) = &self_ {
        let val = a.borrow().get(idx.as_integer());
        match val {
            Some(val) => {
                a.borrow_mut().remove(idx.as_integer());
                v.push(val);
                1
            }
            None => throw_error(v, "idx out of range"),
        }
    } else {
        SQ_ERROR
    }
}

fn array_resize(v: &mut VmData) -> NativeResult {
    let self_ = v.stack_get(1);
    let nsize = v.stack_get(2);
    if nsize.type_() & SQOBJECT_NUMERIC != 0 {
        let sz = nsize.as_integer();
        if sz < 0 {
            return throw_error(v, "resizing to negative length");
        }
        let fill = if v.top - v.stackbase > 2 {
            v.stack_get(3)
        } else {
            Value::Null
        };
        if let Value::Array(a) = &self_ {
            a.borrow_mut().resize(sz, &fill);
        }
        // sq_settop(v,1)
        let base = v.stackbase;
        v.top = base + 1;
        return 1;
    }
    throw_error(v, "size must be a number")
}

fn array_reverse(v: &mut VmData) -> NativeResult {
    let self_ = v.get_up(-1);
    if let Value::Array(a) = &self_ {
        a.borrow_mut().values.reverse();
        1
    } else {
        SQ_ERROR
    }
}

fn array_map(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let size = match &o {
        Value::Array(a) => a.borrow().size(),
        _ => 0,
    };
    let ret = SQArray::create(&v.ss, size);
    {
        let mut rb = ret.borrow_mut();
        rb.resize(size, &Value::Null);
    }
    if map_array(v, &ret, &o).is_err() {
        return SQ_ERROR;
    }
    v.push(Value::Array(ret));
    1
}

fn array_apply(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    if let Value::Array(a) = &o {
        if map_array(v, &a.clone(), &o).is_err() {
            return SQ_ERROR;
        }
    }
    v.pop();
    1
}

/// ↔ __map_array
fn map_array(v: &mut VmData, dest: &ArrayRef, src: &Value) -> Result<(), ()> {
    let closure = v.stack_get(2);
    let size = match src {
        Value::Array(a) => a.borrow().size(),
        _ => 0,
    };
    v.push(closure.clone());
    let nargs = match &closure {
        Value::Closure(c) => c.borrow().function.parameters.len() as SQInteger,
        Value::NativeClosure(nc) => {
            let n = nc.borrow().nparamscheck;
            if n > 0 {
                n
            } else {
                4
            }
        }
        _ => 4,
    };
    for n in 0..size {
        let temp = match src {
            Value::Array(a) => a.borrow().get(n).unwrap_or(Value::Null),
            _ => Value::Null,
        };
        v.push(src.clone());
        v.push(temp);
        if nargs >= 3 {
            v.push(Value::Integer(n));
        }
        if nargs >= 4 {
            v.push(src.clone());
        }
        match v.call_value(&closure, nargs, v.top - nargs, false) {
            Ok(r) => {
                dest.borrow_mut().set(n, &r);
                v.pop();
            }
            Err(_) => {
                return Err(());
            }
        }
    }
    v.pop();
    Ok(())
}

fn array_reduce(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let closure = v.stack_get(2);
    let size = match &o {
        Value::Array(a) => a.borrow().size(),
        _ => 0,
    };
    #[allow(unused_assignments)]
    let mut res = Value::Null;
    let iter_start;
    if v.top - v.stackbase > 2 {
        res = v.stack_get(3);
        iter_start = 0;
    } else if size == 0 {
        return 0;
    } else {
        res = match &o {
            Value::Array(a) => a.borrow().get(0).unwrap_or(Value::Null),
            _ => Value::Null,
        };
        iter_start = 1;
    }
    if size > iter_start {
        v.push(closure.clone());
        for n in iter_start..size {
            let other = match &o {
                Value::Array(a) => a.borrow().get(n).unwrap_or(Value::Null),
                _ => Value::Null,
            };
            v.push(o.clone());
            v.push(res.clone());
            v.push(other);
            match v.call_value(&closure, 3, v.top - 3, false) {
                Ok(r) => {
                    res = r;
                    v.pop();
                }
                Err(_) => return SQ_ERROR,
            }
        }
        v.pop();
    }
    v.push(res);
    1
}

fn array_filter(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let closure = v.stack_get(2);
    let ret = SQArray::create(&v.ss, 0);
    let size = match &o {
        Value::Array(a) => a.borrow().size(),
        _ => 0,
    };
    for n in 0..size {
        let val = match &o {
            Value::Array(a) => a.borrow().get(n).unwrap_or(Value::Null),
            _ => Value::Null,
        };
        v.push(o.clone());
        v.push(Value::Integer(n));
        v.push(val.clone());
        match v.call_value(&closure, 3, v.top - 3, false) {
            Ok(r) => {
                if !VmData::is_false(&r) {
                    ret.borrow_mut().append(&val);
                }
                v.pop();
            }
            Err(_) => return SQ_ERROR,
        }
    }
    v.push(Value::Array(ret));
    1
}

fn array_find(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let val = v.stack_get(2);
    let size = match &o {
        Value::Array(a) => a.borrow().size(),
        _ => 0,
    };
    for n in 0..size {
        let temp = match &o {
            Value::Array(a) => a.borrow().get(n).unwrap_or(Value::Null),
            _ => Value::Null,
        };
        if VmData::is_equal(&temp, &val) {
            v.push(Value::Integer(n));
            return 1;
        }
    }
    0
}

fn array_sort(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let use_cmp = v.top - v.stackbase == 2;
    let size = match &o {
        Value::Array(a) => a.borrow().size(),
        _ => 0,
    };
    if size > 1 {
        if !hsort(v, &o, use_cmp) {
            return SQ_ERROR;
        }
    }
    // sq_settop(v,1)
    let base = v.stackbase;
    v.top = base + 1;
    1
}

/// ↔ _hsort / _hsort_sift_down（迭代 sift-down 保持比较次序一致）
fn hsort(v: &mut VmData, arr: &Value, use_cmp: bool) -> bool {
    let cmp_fn_idx = if use_cmp { Some(v.stackbase + 2) } else { None };
    let a = match arr {
        Value::Array(a) => a.clone(),
        _ => return false,
    };
    let size = a.borrow().size();
    let closure = if use_cmp {
        Some(v.get_at(2 + v.stackbase - 1))
    } else {
        None
    };

    // ↔ _sort_compare
    fn sort_compare(
        v: &mut VmData,
        closure: &Option<Value>,
        x: &Value,
        y: &Value,
    ) -> Result<SQInteger, ()> {
        match closure {
            None => v.obj_cmp(x, y).map_err(|_| ()),
            Some(c) => {
                let top = v.top;
                v.push(c.clone());
                v.push(v.roottable.clone());
                v.push(x.clone());
                v.push(y.clone());
                match v.call_value(c, 3, v.top - 3, false) {
                    Ok(r) => {
                        if !matches!(r, Value::Integer(_)) {
                            v.top = top;
                            v.raise_error_str("numeric value expected as return value of the compare function");
                            return Err(());
                        }
                        let ret = r.as_integer();
                        v.top = top;
                        Ok(ret)
                    }
                    Err(_) => {
                        v.top = top;
                        if !matches!(v.lasterror, Value::String(_)) {
                            v.raise_error_str("compare func failed");
                        }
                        Err(())
                    }
                }
            }
        }
    }

    // 堆排序（与 C++ 相同的 sift-down 次序）
    let mut i = size / 2;
    while i >= 0 {
        if !sift_down(v, &a, i, size - 1, &closure, &mut sort_compare) {
            return false;
        }
        i -= 1;
    }
    let mut i = size - 1;
    while i >= 1 {
        a.borrow_mut().values.swap(0, i as usize);
        if !sift_down(v, &a, 0, i - 1, &closure, &mut sort_compare) {
            return false;
        }
        i -= 1;
    }
    let _ = cmp_fn_idx;
    true
}

fn sift_down(
    v: &mut VmData,
    arr: &ArrayRef,
    mut root: SQInteger,
    bottom: SQInteger,
    closure: &Option<Value>,
    sort_compare: &mut dyn FnMut(&mut VmData, &Option<Value>, &Value, &Value) -> Result<SQInteger, ()>,
) -> bool {
    loop {
        let root2 = root * 2;
        if root2 > bottom {
            break;
        }
        let max_child;
        if root2 == bottom {
            max_child = root2;
        } else {
            let (a, b) = {
                let ab = arr.borrow();
                (ab.values[root2 as usize].clone(), ab.values[root2 as usize + 1].clone())
            };
            let ret = match sort_compare(v, closure, &a, &b) {
                Ok(r) => r,
                Err(_) => return false,
            };
            if ret > 0 {
                max_child = root2;
            } else {
                max_child = root2 + 1;
            }
        }
        let (rv, mv) = {
            let ab = arr.borrow();
            (ab.values[root as usize].clone(), ab.values[max_child as usize].clone())
        };
        let ret = match sort_compare(v, closure, &rv, &mv) {
            Ok(r) => r,
            Err(_) => return false,
        };
        if ret < 0 {
            if root == max_child {
                v.raise_error_str("inconsistent compare function");
                return false;
            }
            arr.borrow_mut().values.swap(root as usize, max_child as usize);
            root = max_child;
        } else {
            break;
        }
    }
    true
}

fn array_slice(v: &mut VmData) -> NativeResult {
    let (mut sidx, mut eidx, o) = match get_slice_params(v) {
        Some(x) => x,
        None => return -1,
    };
    let alen = match &o {
        Value::Array(a) => a.borrow().size(),
        _ => 0,
    };
    if sidx < 0 {
        sidx = alen + sidx;
    }
    if eidx < 0 {
        eidx = alen + eidx;
    }
    if eidx < sidx {
        return throw_error(v, "wrong indexes");
    }
    if eidx > alen || sidx < 0 {
        return throw_error(v, "slice out of range");
    }
    let arr = SQArray::create(&v.ss, eidx - sidx);
    {
        let mut ab = arr.borrow_mut();
        let mut count = 0;
        for i in sidx..eidx {
            let t = match &o {
                Value::Array(a) => a.borrow().get(i).unwrap_or(Value::Null),
                _ => Value::Null,
            };
            ab.set(count, &t);
            count += 1;
        }
    }
    v.push(Value::Array(arr));
    1
}

fn fill_string_ddel(ss: &Rc<SharedState>) {
    let t = match &*ss.string_default_delegate.borrow() {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    reg(ss, &t, "len", Rc::new(default_delegate_len), 1, Some("s"));
    reg(ss, &t, "tointeger", Rc::new(default_delegate_tointeger), -1, Some("sn"));
    reg(ss, &t, "tofloat", Rc::new(default_delegate_tofloat), 1, Some("s"));
    reg(ss, &t, "tostring", Rc::new(default_delegate_tostring), 1, Some("."));
    reg(ss, &t, "slice", Rc::new(string_slice), -1, Some("s n  n"));
    reg(ss, &t, "find", Rc::new(string_find), -2, Some("s s n"));
    reg(ss, &t, "tolower", Rc::new(|v| string_tofunc(v, false)), -1, Some("s n n"));
    reg(ss, &t, "toupper", Rc::new(|v| string_tofunc(v, true)), -1, Some("s n n"));
    reg(ss, &t, "weakref", Rc::new(obj_delegate_weakref), 1, None);
}

fn string_slice(v: &mut VmData) -> NativeResult {
    let (mut sidx, mut eidx, o) = match get_slice_params(v) {
        Some(x) => x,
        None => return -1,
    };
    let slen = o.as_bytes().len() as SQInteger;
    if sidx < 0 {
        sidx = slen + sidx;
    }
    if eidx < 0 {
        eidx = slen + eidx;
    }
    if eidx < sidx {
        return throw_error(v, "wrong indexes");
    }
    if eidx > slen || sidx < 0 {
        return throw_error(v, "slice out of range");
    }
    let sub = &o.as_bytes()[sidx as usize..eidx as usize];
    let s = v.ss.intern(sub);
    v.push(s);
    1
}

fn string_find(v: &mut VmData) -> NativeResult {
    let top = v.top - v.stackbase;
    let s = v.stack_get(1);
    let sub = v.stack_get(2);
    let mut start_idx = 0;
    if top > 2 {
        start_idx = v.stack_get(3).as_integer();
    }
    let hay = s.as_bytes();
    let needle = sub.as_bytes();
    if hay.len() as SQInteger > start_idx && start_idx >= 0 {
        if needle.is_empty() {
            v.push(Value::Integer(start_idx));
            return 1;
        }
        let start = start_idx as usize;
        if let Some(pos) = find_sub(&hay[start..], needle) {
            v.push(Value::Integer(start as SQInteger + pos));
            return 1;
        }
    }
    0
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<SQInteger> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    for i in 0..=(hay.len() - needle.len()) {
        if &hay[i..i + needle.len()] == needle {
            return Some(i as SQInteger);
        }
    }
    None
}

fn string_tofunc(v: &mut VmData, upper: bool) -> NativeResult {
    let (mut sidx, mut eidx, str_) = match get_slice_params(v) {
        Some(x) => x,
        None => return -1,
    };
    let bytes = str_.as_bytes().to_vec();
    let slen = bytes.len() as SQInteger;
    if sidx < 0 {
        sidx = slen + sidx;
    }
    if eidx < 0 {
        eidx = slen + eidx;
    }
    if eidx < sidx {
        return throw_error(v, "wrong indexes");
    }
    if eidx > slen || sidx < 0 {
        return throw_error(v, "slice out of range");
    }
    let mut out = bytes.clone();
    for i in sidx as usize..eidx as usize {
        out[i] = if upper {
            bytes[i].to_ascii_uppercase()
        } else {
            bytes[i].to_ascii_lowercase()
        };
    }
    let s = v.ss.intern(&out);
    v.push(s);
    1
}

fn fill_number_ddel(ss: &Rc<SharedState>) {
    let t = match &*ss.number_default_delegate.borrow() {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    reg(ss, &t, "tointeger", Rc::new(default_delegate_tointeger), 1, Some("n|b"));
    reg(ss, &t, "tofloat", Rc::new(default_delegate_tofloat), 1, Some("n|b"));
    reg(ss, &t, "tostring", Rc::new(default_delegate_tostring), 1, Some("."));
    reg(ss, &t, "tochar", Rc::new(number_delegate_tochar), 1, Some("n|b"));
    reg(ss, &t, "weakref", Rc::new(obj_delegate_weakref), 1, None);
}

fn fill_generator_ddel(ss: &Rc<SharedState>) {
    let t = match &*ss.generator_default_delegate.borrow() {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    reg(ss, &t, "getstatus", Rc::new(generator_getstatus), 1, Some("g"));
    reg(ss, &t, "weakref", Rc::new(obj_delegate_weakref), 1, None);
    reg(ss, &t, "tostring", Rc::new(default_delegate_tostring), 1, Some("."));
}

fn generator_getstatus(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let s = match &o {
        Value::Generator(g) => match g.borrow().state {
            GeneratorState::Suspended => "suspended",
            GeneratorState::Running => "running",
            GeneratorState::Dead => "dead",
        },
        _ => "dead",
    };
    let sv = v.ss.intern(s.as_bytes());
    v.push(sv);
    1
}

fn fill_closure_ddel(ss: &Rc<SharedState>) {
    let t = match &*ss.closure_default_delegate.borrow() {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    reg(ss, &t, "call", Rc::new(closure_call), -1, Some("c"));
    reg(ss, &t, "pcall", Rc::new(closure_pcall), -1, Some("c"));
    reg(ss, &t, "acall", Rc::new(closure_acall), 2, Some("ca"));
    reg(ss, &t, "pacall", Rc::new(closure_pacall), 2, Some("ca"));
    reg(ss, &t, "weakref", Rc::new(obj_delegate_weakref), 1, None);
    reg(ss, &t, "tostring", Rc::new(default_delegate_tostring), 1, Some("."));
    reg(ss, &t, "bindenv", Rc::new(closure_bindenv), 2, Some("c x|y|t"));
    reg(ss, &t, "getinfos", Rc::new(closure_getinfos), 1, Some("c"));
    reg(ss, &t, "getroot", Rc::new(closure_getroot), 1, Some("c"));
    reg(ss, &t, "setroot", Rc::new(closure_setroot), 2, Some("ct"));
}

fn closure_pcall(v: &mut VmData) -> NativeResult {
    let n = v.top - v.stackbase - 1;
    let closure = v.stack_get(1);
    match v.call_value(&closure, n, v.stackbase + 1, false) {
        Ok(r) => {
            v.push(r);
            1
        }
        Err(_) => SQ_ERROR,
    }
}

fn closure_call(v: &mut VmData) -> NativeResult {
    let c = v.stack_get(1);
    let is_plain = match &c {
        Value::Closure(cl) => !cl.borrow().function.bgenerator,
        _ => false,
    };
    let n = v.top - v.stackbase - 1;
    if is_plain {
        // ↔ sq_tailcall
        let cl = match &c {
            Value::Closure(cl) => cl.clone(),
            _ => unreachable!(),
        };
        let last_top = v.top;
        if !v.openouters.is_empty() {
            v.close_outers(v.stackbase as usize);
        }
        let target = v.last_callinfo().target as SQInteger;
        for i in 0..n {
            let val = v.stk_public(1 + i);
            v.stk_public_set(i, val);
        }
        if v.start_call_pub(&cl, target, n, v.stackbase, true).is_err() {
            return SQ_ERROR;
        }
        if last_top >= v.top {
            v.top = last_top;
        }
        SQ_TAILCALL_FLAG
    } else {
        match v.call_value(&c, n, v.stackbase + 1, true) {
            Ok(r) => {
                v.push(r);
                1
            }
            Err(_) => SQ_ERROR,
        }
    }
}

fn closure_acall_impl(v: &mut VmData, raiseerror: bool) -> NativeResult {
    let closure = v.stack_get(1);
    let aparams = v.stack_get(2);
    let nparams = match &aparams {
        Value::Array(a) => a.borrow().size(),
        _ => 0,
    };
    v.push(closure.clone());
    if let Value::Array(a) = &aparams {
        let vals = a.borrow().values.clone();
        for x in vals {
            v.push(x);
        }
    }
    match v.call_value(&closure, nparams, v.top - nparams, raiseerror) {
        Ok(r) => {
            v.push(r);
            1
        }
        Err(_) => SQ_ERROR,
    }
}

fn closure_acall(v: &mut VmData) -> NativeResult {
    closure_acall_impl(v, true)
}

fn closure_pacall(v: &mut VmData) -> NativeResult {
    closure_acall_impl(v, false)
}

fn closure_bindenv(v: &mut VmData) -> NativeResult {
    let closure = v.stack_get(1);
    let env = v.stack_get(2);
    let t = env.type_();
    if !(t == OT_TABLE || t == OT_CLASS || t == OT_INSTANCE || t == OT_ARRAY) {
        return throw_error(
            v,
            &format!(
                "cannot bind a {} as environment object",
                crate::object::id_type2name(t).unwrap_or("?")
            ),
        );
    }
    match &closure {
        Value::Closure(c) => {
            let cloned = c.borrow().clone_closure(&v.ss);
            cloned.borrow_mut().env = Some(env.get_weakref_as_weakref());
            v.push(Value::Closure(cloned));
            1
        }
        Value::NativeClosure(nc) => {
            let cloned = nc.borrow().clone_native(&v.ss);
            cloned.borrow_mut().env = Some(env.get_weakref_as_weakref());
            v.push(Value::NativeClosure(cloned));
            1
        }
        _ => SQ_ERROR,
    }
}

fn closure_getroot(v: &mut VmData) -> NativeResult {
    let closure = v.stack_get(1);
    match &closure {
        Value::Closure(c) => {
            let w = c.borrow().root.clone();
            let obj = w.map(|w| w.borrow().obj.get()).unwrap_or(Value::Null);
            v.push(obj);
            1
        }
        _ => SQ_ERROR,
    }
}

fn closure_setroot(v: &mut VmData) -> NativeResult {
    let closure = v.get_up(-2);
    let root = v.get_up(-1);
    if let Value::Closure(c) = &closure {
        let w = match &root {
            Value::Table(t) => Some(Value::Table(t.clone()).get_weakref_as_weakref()),
            _ => None,
        };
        c.borrow_mut().set_root(w);
        v.push(closure);
        1
    } else {
        SQ_ERROR
    }
}

fn closure_getinfos(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let res = SQTable::create(&v.ss, 4);
    match &o {
        Value::Closure(c) => {
            let f = c.borrow().function.clone();
            let nparams = f.parameters.len() as SQInteger + if f.varparams != 0 { 1 } else { 0 };
            let params = SQArray::create(&v.ss, nparams);
            {
                let mut pb = params.borrow_mut();
                pb.resize(nparams, &Value::Null);
                for n in 0..f.parameters.len() {
                    pb.set(n as SQInteger, &f.parameters[n]);
                }
                if f.varparams != 0 {
                    pb.set(nparams - 1, &v.ss.intern(b"..."));
                }
            }
            let defparams = SQArray::create(&v.ss, f.defaultparams.len() as SQInteger);
            {
                let mut db = defparams.borrow_mut();
                let dps = c.borrow().defaultparams.clone();
                for j in 0..dps.len() {
                    db.set(j as SQInteger, &dps[j]);
                }
            }
            let mut rt = res.borrow_mut();
            rt.new_slot(&v.ss.intern(b"native"), &Value::Bool(false));
            rt.new_slot(&v.ss.intern(b"name"), &f.name);
            rt.new_slot(&v.ss.intern(b"src"), &f.sourcename);
            rt.new_slot(&v.ss.intern(b"parameters"), &Value::Array(params));
            rt.new_slot(&v.ss.intern(b"varargs"), &Value::Bool(f.varparams != 0));
            rt.new_slot(&v.ss.intern(b"defparams"), &Value::Array(defparams));
        }
        Value::NativeClosure(nc) => {
            let (name, nparamscheck, typecheck) = {
                let n = nc.borrow();
                (n.name.clone(), n.nparamscheck, n.typecheck.clone())
            };
            let mut rt = res.borrow_mut();
            rt.new_slot(&v.ss.intern(b"native"), &Value::Bool(true));
            rt.new_slot(&v.ss.intern(b"name"), &name);
            rt.new_slot(&v.ss.intern(b"paramscheck"), &Value::Integer(nparamscheck));
            let tcarr = SQArray::create(&v.ss, typecheck.len() as SQInteger);
            {
                let mut tb = tcarr.borrow_mut();
                tb.resize(typecheck.len() as SQInteger, &Value::Null);
                for n in 0..typecheck.len() {
                    tb.set(n as SQInteger, &Value::Integer(typecheck[n] as SQInteger));
                }
            }
            rt.new_slot(&v.ss.intern(b"typecheck"), &Value::Array(tcarr));
        }
        _ => return SQ_ERROR,
    }
    v.push(Value::Table(res));
    1
}

fn fill_thread_ddel(ss: &Rc<SharedState>) {
    let t = match &*ss.thread_default_delegate.borrow() {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    reg(ss, &t, "call", Rc::new(thread_call), -1, Some("v"));
    reg(ss, &t, "wakeup", Rc::new(thread_wakeup), -1, Some("v"));
    reg(ss, &t, "wakeupthrow", Rc::new(thread_wakeupthrow), -2, Some("v.b"));
    reg(ss, &t, "getstatus", Rc::new(thread_getstatus), 1, Some("v"));
    reg(ss, &t, "weakref", Rc::new(obj_delegate_weakref), 1, None);
    reg(ss, &t, "getstackinfos", Rc::new(thread_getstackinfos), 2, Some("vn"));
    reg(ss, &t, "tostring", Rc::new(default_delegate_tostring), 1, Some("."));
}

fn thread_getstatus_of(_v: &VmData, o: &Value) -> SQInteger {
    if let Value::Thread(t) = o {
        let tv = t.borrow();
        if tv.suspended {
            SQ_VMSTATE_SUSPENDED
        } else if !tv.callsstack.is_empty() {
            SQ_VMSTATE_RUNNING
        } else {
            SQ_VMSTATE_IDLE
        }
    } else {
        SQ_VMSTATE_IDLE
    }
}

fn thread_call(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    if let Value::Thread(t) = &o {
        let nparams = v.top - v.stackbase;
        let root = t.borrow().roottable.clone();
        t.borrow_mut().push(root);
        for i in 2..(nparams + 1) {
            let val = v.get_at(i + v.stackbase - 1);
            t.borrow_mut().push(val);
        }
        // ↔ sq_call(thread, nparams,…)：闭包取 GetUp(-(nparams+1))，
        // 而不是固定槽位（挂起的线程栈顶是 suspend 现场）
        let closure = {
            let tv = t.borrow();
            tv.get_up(-(nparams + 1))
        };
        let base = t.borrow().top - nparams;
        let mut out = Value::Null;
        let ok = {
            let mut tv = t.borrow_mut();
            match crate::vm::execute(&mut tv, &closure, nparams, base, &mut out, true, crate::vm::ExecutionType::Call) {
                Ok(()) => true,
                Err(_) => false,
            }
        };
        if ok {
            t.borrow_mut().pop();
            v.push(out);
            1
        } else {
            let e = t.borrow().lasterror.clone();
            v.raise_error_obj(e);
            SQ_ERROR
        }
    } else {
        throw_error(v, "wrong parameter")
    }
}

fn thread_wakeup(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    if let Value::Thread(t) = &o {
        let state = thread_getstatus_of(v, &o);
        if state != SQ_VMSTATE_SUSPENDED {
            return match state {
                SQ_VMSTATE_IDLE => throw_error(v, "cannot wakeup a idle thread"),
                _ => throw_error(v, "cannot wakeup a running thread"),
            };
        }
        let wakeupret = v.top - v.stackbase > 1;
        if wakeupret {
            let val = v.stack_get(2);
            t.borrow_mut().push(val);
        }
        let mut out = Value::Null;
        let ok = {
            let mut tv = t.borrow_mut();
            wakeup_vm(&mut tv, wakeupret, true, true, false, &mut out)
        };
        if ok {
            // ↔ sq_wakeupvm(retval=true)：结果压回线程栈顶，thread_wakeup 取走
            t.borrow_mut().push(out.clone());
            let st = thread_getstatus_of(v, &o);
            if st == SQ_VMSTATE_IDLE {
                t.borrow_mut().top = 1;
            } else {
                t.borrow_mut().pop();
            }
            v.push(out);
            1
        } else {
            t.borrow_mut().top = 1;
            let e = t.borrow().lasterror.clone();
            v.raise_error_obj(e);
            SQ_ERROR
        }
    } else {
        throw_error(v, "wrong parameter")
    }
}

fn thread_wakeupthrow(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    if let Value::Thread(t) = &o {
        let state = thread_getstatus_of(v, &o);
        if state != SQ_VMSTATE_SUSPENDED {
            return match state {
                SQ_VMSTATE_IDLE => throw_error(v, "cannot wakeup a idle thread"),
                _ => throw_error(v, "cannot wakeup a running thread"),
            };
        }
        let val = v.stack_get(2);
        t.borrow_mut().push(val);
        let rethrow_error = if v.top - v.stackbase > 2 {
            v.stack_get(3).as_bool()
        } else {
            true
        };
        let mut out = Value::Null;
        let ok = {
            let mut tv = t.borrow_mut();
            // ↔ sq_throwobject(thread)：以栈顶对象为错误并弹出，再以 throw 恢复
            let e = tv.get_up(-1);
            tv.raise_error_obj(e);
            tv.pop();
            wakeup_vm(&mut tv, false, true, true, true, &mut out)
        };
        if ok {
            t.borrow_mut().push(out.clone());
            let st = thread_getstatus_of(v, &o);
            if st == SQ_VMSTATE_IDLE {
                t.borrow_mut().top = 1;
            } else {
                t.borrow_mut().pop();
            }
            v.push(out);
            1
        } else {
            t.borrow_mut().top = 1;
            if rethrow_error {
                let e = t.borrow().lasterror.clone();
                v.raise_error_obj(e);
                SQ_ERROR
            } else {
                SQ_OK
            }
        }
    } else {
        throw_error(v, "wrong parameter")
    }
}

fn thread_getstatus(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    let s = match thread_getstatus_of(v, &o) {
        SQ_VMSTATE_IDLE => "idle",
        SQ_VMSTATE_RUNNING => "running",
        SQ_VMSTATE_SUSPENDED => "suspended",
        _ => return throw_error(v, "internal VM error"),
    };
    let sv = v.ss.intern(s.as_bytes());
    v.push(sv);
    1
}

fn thread_getstackinfos(v: &mut VmData) -> NativeResult {
    let o = v.stack_get(1);
    if let Value::Thread(t) = &o {
        let level = v.stack_get(2).as_integer();
        let res = {
            let mut tv = t.borrow_mut();
            get_callstack_infos(&mut tv, level)
        };
        match res {
            Some(()) => {
                // 结果在新线程栈顶 → 移到当前线程
                let val = t.borrow_mut().pop_get();
                v.push(val);
                1
            }
            None => 0,
        }
    } else {
        throw_error(v, "wrong parameter")
    }
}

/// ↔ sq_wakeupvm（线程唤醒的完整语义）
fn wakeup_vm(
    vm: &mut VmData,
    resumedret: bool,
    _retval: bool,
    raiseerror: bool,
    throwerror: bool,
    out: &mut Value,
) -> bool {
    if !vm.suspended {
        vm.raise_error_str("cannot resume an idle vm");
        return false;
    }
    let target = vm.suspended_target;
    if resumedret {
        if target != -1 {
            let val = vm.get_up(-1);
            let slot = (vm.stackbase + target) as usize;
            vm.ensure_stack(slot);
            vm.stack[slot] = val;
        }
        vm.pop();
    }
    let et = if throwerror {
        crate::vm::ExecutionType::ResumeThrowVm
    } else {
        crate::vm::ExecutionType::ResumeVm
    };
    // ↔ v->Execute(dummy,...)：走完整 execute（含 nnativecalls 计数）
    let dummy = Value::Null;
    crate::vm::execute(vm, &dummy, -1, -1, out, raiseerror, et).is_ok()
}

fn fill_class_ddel(ss: &Rc<SharedState>) {
    let t = match &*ss.class_default_delegate.borrow() {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    reg(ss, &t, "getattributes", Rc::new(class_getattributes), 2, Some("y."));
    reg(ss, &t, "setattributes", Rc::new(class_setattributes), 3, Some("y.."));
    reg(ss, &t, "rawget", Rc::new(container_rawget), 2, Some("y"));
    reg(ss, &t, "rawset", Rc::new(container_rawset), 3, Some("y"));
    reg(ss, &t, "rawin", Rc::new(container_rawexists), 2, Some("y"));
    reg(ss, &t, "weakref", Rc::new(obj_delegate_weakref), 1, None);
    reg(ss, &t, "tostring", Rc::new(default_delegate_tostring), 1, Some("."));
    reg(ss, &t, "instance", Rc::new(class_instance), 1, Some("y"));
    reg(ss, &t, "getbase", Rc::new(class_getbase), 1, Some("y"));
    reg(ss, &t, "newmember", Rc::new(class_newmember), -3, Some("y"));
    reg(ss, &t, "rawnewmember", Rc::new(class_rawnewmember), -3, Some("y"));
}

fn class_getattributes(v: &mut VmData) -> NativeResult {
    let self_ = v.get_up(-2);
    let key = v.get_up(-1);
    if let Value::Class(c) = &self_ {
        let r = c.borrow().get_attributes(&key);
        match r {
            Some(a) => {
                v.pop();
                v.push(a);
                1
            }
            None => {
                v.pop();
                throw_error(v, "wrong index")
            }
        }
    } else {
        SQ_ERROR
    }
}

fn class_setattributes(v: &mut VmData) -> NativeResult {
    let self_ = v.get_up(-3);
    let key = v.get_up(-2);
    let val = v.get_up(-1);
    if let Value::Class(c) = &self_ {
        let old = c.borrow().get_attributes(&key);
        match old {
            Some(old) => {
                c.borrow_mut().set_attributes(&key, &val);
                v.pop();
                v.push(old);
                1
            }
            None => {
                v.pop();
                throw_error(v, "wrong index")
            }
        }
    } else {
        SQ_ERROR
    }
}

fn class_instance(v: &mut VmData) -> NativeResult {
    let self_ = v.get_up(-1);
    if let Value::Class(c) = &self_ {
        let inst = crate::class::SQClass::create_instance(c, &v.ss);
        v.push(Value::Instance(inst));
        1
    } else {
        SQ_ERROR
    }
}

fn class_getbase(v: &mut VmData) -> NativeResult {
    let self_ = v.get_up(-1);
    if let Value::Class(c) = &self_ {
        let b = c.borrow().base.clone();
        match b {
            Some(b) => v.push(Value::Class(b)),
            None => v.push_null(),
        }
        1
    } else {
        SQ_ERROR
    }
}

fn class_newmember_impl(v: &mut VmData, raw: bool) -> NativeResult {
    let top = v.top - v.stackbase;
    let mut bstatic = false;
    let _key: SQInteger;
    if top == 5 {
        bstatic = v.stack_get(5).as_bool();
        v.pop();
    }
    if top < 4 {
        v.push_null();
    }
    let self_ = v.get_up(-4);
    let keyv = v.get_up(-3);
    let val = v.get_up(-2);
    let attrs = v.get_up(-1);
    match &self_ {
        Value::Class(c) => {
            let r = v.new_slot_a(&Value::Class(c.clone()), &keyv, &val, &attrs, bstatic, raw);
            match r {
                Ok(()) => {
                    v.pop_n(3);
                    1
                }
                Err(_) => SQ_ERROR,
            }
        }
        _ => SQ_ERROR,
    }
}

fn class_newmember(v: &mut VmData) -> NativeResult {
    class_newmember_impl(v, false)
}

fn class_rawnewmember(v: &mut VmData) -> NativeResult {
    class_newmember_impl(v, true)
}

fn fill_instance_ddel(ss: &Rc<SharedState>) {
    let t = match &*ss.instance_default_delegate.borrow() {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    reg(ss, &t, "getclass", Rc::new(instance_getclass), 1, Some("x"));
    reg(ss, &t, "rawget", Rc::new(container_rawget), 2, Some("x"));
    reg(ss, &t, "rawset", Rc::new(container_rawset), 3, Some("x"));
    reg(ss, &t, "rawin", Rc::new(container_rawexists), 2, Some("x"));
    reg(ss, &t, "weakref", Rc::new(obj_delegate_weakref), 1, None);
    reg(ss, &t, "tostring", Rc::new(default_delegate_tostring), 1, Some("."));
}

fn instance_getclass(v: &mut VmData) -> NativeResult {
    let self_ = v.stack_get(1);
    if let Value::Instance(i) = &self_ {
        let c = i.borrow().class.clone();
        match c {
            Some(c) => v.push(Value::Class(c)),
            None => v.push_null(),
        }
        1
    } else {
        SQ_ERROR
    }
}

fn fill_weakref_ddel(ss: &Rc<SharedState>) {
    let t = match &*ss.weakref_default_delegate.borrow() {
        Value::Table(t) => t.clone(),
        _ => return,
    };
    reg(ss, &t, "ref", Rc::new(weakref_ref), 1, Some("r"));
    reg(ss, &t, "weakref", Rc::new(obj_delegate_weakref), 1, None);
    reg(ss, &t, "tostring", Rc::new(default_delegate_tostring), 1, Some("."));
}

fn weakref_ref(v: &mut VmData) -> NativeResult {
    let self_ = v.stack_get(1);
    let o = self_.weakref_target();
    v.push(o);
    1
}

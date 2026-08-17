//! Squirrel 标准库（↔ sqstdlib/*.cpp）：math / system / io / blob / string / aux。
//!
//! 原生函数直接操作 `&mut VmData`（与 baselib 相同的方式）。

pub mod regex;

use std::rc::Rc;

use squirrel::config::*;
use squirrel::object::{TableRef, Value};
use squirrel::state::SharedState;
use squirrel::vm::VmData;

/// 注册原生函数到 root table 的辅助
pub fn reg_global(
    ss: &Rc<SharedState>,
    root: &TableRef,
    name: &str,
    f: squirrel::closure::NativeFn,
    nparamscheck: SQInteger,
    typemask: Option<&str>,
) {
    let nc = squirrel::closure::SQNativeClosure::create(ss, f, 0);
    {
        let mut n = nc.borrow_mut();
        n.nparamscheck = nparamscheck;
        n.name = ss.intern(name.as_bytes());
        if let Some(tm) = typemask {
            squirrel::state::compile_typemask(&mut n.typecheck, tm.as_bytes());
        }
    }
    let key = ss.intern(name.as_bytes());
    root.borrow_mut().new_slot(&key, &Value::NativeClosure(nc));
}

fn throw_error(vm: &mut VmData, msg: &str) -> SQInteger {
    vm.raise_error_str(msg);
    SQ_ERROR
}

// ===========================================================================
// math（↔ sqstdmath.cpp）
// ===========================================================================

pub fn register_mathlib(vm: &mut VmData) {
    let ss = vm.ss.clone();
    let root = match &vm.roottable {
        Value::Table(t) => t.clone(),
        _ => return,
    };

    macro_rules! m1 {
        ($name:expr, $tm:expr, $f:expr) => {
            reg_global(&ss, &root, $name, Rc::new($f), 2, Some($tm));
        };
    }

    m1!("sqrt", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.sqrt()));
        1
    });
    m1!("sin", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.sin()));
        1
    });
    m1!("cos", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.cos()));
        1
    });
    m1!("asin", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.asin()));
        1
    });
    m1!("acos", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.acos()));
        1
    });
    m1!("log", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.ln()));
        1
    });
    m1!("log10", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.log10()));
        1
    });
    m1!("tan", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.tan()));
        1
    });
    m1!("atan", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.atan()));
        1
    });
    m1!("atan2", "nn", |v: &mut VmData| {
        let y = v.stack_get(2).as_float();
        let x = v.stack_get(3).as_float();
        v.push(Value::Float(y.atan2(x)));
        1
    });
    m1!("pow", "nn", |v: &mut VmData| {
        let y = v.stack_get(2).as_float();
        let x = v.stack_get(3).as_float();
        v.push(Value::Float(y.powf(x)));
        1
    });
    m1!("floor", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        // ↔ (SQInteger)floor(f)：MSVC 截断语义（floor 后已在 int 范围）
        v.push(Value::Float(x.floor()));
        1
    });
    m1!("ceil", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.ceil()));
        1
    });
    m1!("exp", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.exp()));
        1
    });
    m1!("srand", "n", |v: &mut VmData| {
        let seed = v.stack_get(2).as_integer() as u32;
        SQ_RNG.with(|r| r.borrow_mut().srand(seed));
        0
    });
    m1!("rand", "n", |v: &mut VmData| {
        let r = SQ_RNG.with(|r| r.borrow_mut().rand());
        v.push(Value::Integer(r));
        1
    });
    m1!("fabs", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.abs()));
        1
    });
    m1!("abs", "n", |v: &mut VmData| {
        let o = v.stack_get(2);
        if matches!(o, Value::Integer(_)) {
            let i = o.as_integer();
            v.push(Value::Integer(if i < 0 { i.wrapping_neg() } else { i }));
        } else {
            v.push(Value::Float(o.as_float().abs()));
        }
        1
    });

    // 常量
    let c1 = ss.intern(b"RAND_MAX");
    root.borrow_mut().new_slot(&c1, &Value::Integer(0x7FFF)); // CRT RAND_MAX
    let c2 = ss.intern(b"PI");
    root.borrow_mut().new_slot(&c2, &Value::Float(std::f32::consts::PI));
}

/// CRT 兼容的 rand：与 MSVC rand() 相同的 LCG
pub struct CrtRand {
    seed: u32,
}

impl CrtRand {
    fn srand(&mut self, s: u32) {
        self.seed = s;
    }
    fn rand(&mut self) -> SQInteger {
        // MSVC: seed = seed * 214013 + 2531011; return (seed >> 16) & 0x7fff
        self.seed = self.seed.wrapping_mul(214013).wrapping_add(2531011);
        ((self.seed >> 16) & 0x7FFF) as SQInteger
    }
}

thread_local! {
    static SQ_RNG: std::cell::RefCell<CrtRand> = std::cell::RefCell::new(CrtRand { seed: 1 });
}

// ===========================================================================
// system（↔ sqstdsystem.cpp）
// ===========================================================================

pub fn register_systemlib(vm: &mut VmData) {
    let ss = vm.ss.clone();
    let root = match &vm.roottable {
        Value::Table(t) => t.clone(),
        _ => return,
    };

    reg_global(&ss, &root, "getenv", Rc::new(|v: &mut VmData| {
        let name = v.stack_get(2);
        match std::env::var(String::from_utf8_lossy(name.as_bytes()).as_ref()) {
            Ok(val) => {
                let s = v.ss.intern(val.as_bytes());
                v.push(s);
            }
            Err(_) => v.push_null(),
        }
        1
    }), 2, Some(".s"));

    reg_global(&ss, &root, "system", Rc::new(|v: &mut VmData| {
        let cmd = v.stack_get(2);
        let out = std::process::Command::new("cmd")
            .args(["/C", &String::from_utf8_lossy(cmd.as_bytes())])
            .output();
        match out {
            Ok(o) => v.push(Value::Integer(o.status.code().unwrap_or(-1) as SQInteger)),
            Err(_) => v.push(Value::Integer(-1)),
        }
        1
    }), 2, Some(".s"));

    reg_global(&ss, &root, "clock", Rc::new(|v: &mut VmData| {
        // ↔ clock()/CLOCKS_PER_SEC
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        v.push(Value::Float(t as SQFloat));
        1
    }), 1, None);

    reg_global(&ss, &root, "time", Rc::new(|v: &mut VmData| {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as SQInteger)
            .unwrap_or(0);
        v.push(Value::Integer(t));
        1
    }), 1, None);

    reg_global(&ss, &root, "remove", Rc::new(|v: &mut VmData| {
        let p = v.stack_get(2);
        let r = std::fs::remove_file(String::from_utf8_lossy(p.as_bytes()).as_ref())
            .map(|_| 0)
            .unwrap_or(-1);
        v.push(Value::Integer(r));
        1
    }), 2, Some(".s"));

    reg_global(&ss, &root, "rename", Rc::new(|v: &mut VmData| {
        let a = v.stack_get(2);
        let b = v.stack_get(3);
        let r = std::fs::rename(
            String::from_utf8_lossy(a.as_bytes()).as_ref(),
            String::from_utf8_lossy(b.as_bytes()).as_ref(),
        )
        .map(|_| 0)
        .unwrap_or(-1);
        v.push(Value::Integer(r));
        1
    }), 3, Some(".ss"));

    reg_global(&ss, &root, "date", Rc::new(date_fn), -1, Some(".nn"));
}

/// ↔ sqstd_system_date：返回 {sec,min,hour,day,month,year,wday,yday}
fn date_fn(v: &mut VmData) -> SQInteger {
    let top = v.top - v.stackbase;
    let mut t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    if top > 1 {
        t = v.stack_get(2).as_integer();
    }
    // 简易 UTC 日期换算（localtime 与 UTC 的差异按环境而定；用 UTC）
    let days = t.div_euclid(86400);
    let secs = t.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    let wday = ((days + 4).rem_euclid(7)) as SQInteger; // 1970-01-01 是周四
    let yday = day_of_year(y, m, d);
    let t2 = SQ_TABLE_CREATE!(v);
    {
        let tt = match &t2 {
            Value::Table(tt) => tt.clone(),
            _ => unreachable!(),
        };
        let mut b = tt.borrow_mut();
        b.new_slot(&v.ss.intern(b"sec"), &Value::Integer(secs % 60));
        b.new_slot(&v.ss.intern(b"min"), &Value::Integer((secs / 60) % 60));
        b.new_slot(&v.ss.intern(b"hour"), &Value::Integer(secs / 3600));
        b.new_slot(&v.ss.intern(b"day"), &Value::Integer(d));
        b.new_slot(&v.ss.intern(b"month"), &Value::Integer(m));
        b.new_slot(&v.ss.intern(b"year"), &Value::Integer(y));
        b.new_slot(&v.ss.intern(b"wday"), &Value::Integer(wday));
        b.new_slot(&v.ss.intern(b"yday"), &Value::Integer(yday));
    }
    v.push(t2);
    1
}

macro_rules! SQ_TABLE_CREATE {
    ($v:expr) => {
        Value::Table(squirrel::table::SQTable::create(&$v.ss.clone(), 8))
    };
}
use SQ_TABLE_CREATE;

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn day_of_year(y: i64, m: i64, d: i64) -> SQInteger {
    const CUM: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let mut yday = CUM[(m - 1) as usize] + d;
    if m > 2 && leap {
        yday += 1;
    }
    yday - 1
}

// ===========================================================================
// io（↔ sqstdio.cpp：loadfile/dofile/writeclosuretofile + file 类）
// ===========================================================================

pub fn register_iolib(vm: &mut VmData) {
    let ss = vm.ss.clone();
    let root = match &vm.roottable {
        Value::Table(t) => t.clone(),
        _ => return,
    };

    reg_global(&ss, &root, "loadfile", Rc::new(io_loadfile), -2, Some(".sb"));
    reg_global(&ss, &root, "dofile", Rc::new(io_dofile), -2, Some(".sb"));
    reg_global(&ss, &root, "writeclosuretofile", Rc::new(io_writeclosuretofile), 3, Some("cs"));

    // file 类（基于 userdata 持有 File 句柄）
    let file_class = make_file_class(&ss);
    let key = ss.intern(b"file");
    root.borrow_mut().new_slot(&key, &file_class);

    // stdout/stderr/stdin
    for (name, fd) in [("stdout", 1u8), ("stderr", 2u8), ("stdin", 0u8)] {
        let inst = make_stream_instance(&ss, &file_class, StreamHandle::Fd(fd));
        let key = ss.intern(name.as_bytes());
        root.borrow_mut().new_slot(&key, &inst);
    }

    // stream 基类（读 blob/readn 等）
    let stream_class = make_stream_class(&ss);
    let key = ss.intern(b"stream");
    root.borrow_mut().new_slot(&key, &stream_class);
    STREAM_CLASS.with(|c| *c.borrow_mut() = Some(match &stream_class {
        Value::Table(t) => t.clone(),
        _ => unreachable!(),
    }));
}

thread_local! {
    static STREAM_CLASS: std::cell::RefCell<Option<TableRef>> = const { std::cell::RefCell::new(None) };
}

/// ↔ sqstd_loadfile：读文件 → 嗅探（.cnut/BOM 暂不支持）→ 编译 → 压栈闭包
fn io_loadfile(v: &mut VmData) -> SQInteger {
    let path = v.stack_get(2);
    let filename = String::from_utf8_lossy(path.as_bytes()).into_owned();
    let data = match std::fs::read(&filename) {
        Ok(d) => d,
        Err(e) => return throw_error(v, &format!("cannot open file '{} ({})'", filename, e)),
    };
    let ss = v.ss.clone();
    // ↔ 嗅探：前 2 字节 0xFAFA → .cnut 字节码
    if data.len() >= 2 && data[0] == 0xFA && data[1] == 0xFA {
        return match squirrel::serialization::read_closure(v, &data) {
            Ok(clo) => {
                v.push(clo);
                1
            }
            Err(e) => throw_error(v, &e),
        };
    }
    let mut pos = 0usize;
    let src = data.clone();
    let readf = Box::new(move || {
        if pos < src.len() {
            let c = src[pos];
            pos += 1;
            c as SQInteger
        } else {
            0
        }
    });
    match squirrel::compiler::compile(v, readf, path.as_bytes(), true, ss.debuginfo.get()) {
        Ok(proto) => {
            let fp = match &proto {
                Value::FuncProto(f) => f.clone(),
                _ => unreachable!(),
            };
            let root_weak = match &v.roottable {
                Value::Table(t) => Some(Value::Table(t.clone()).get_weakref_as_weakref()),
                _ => None,
            };
            let clo = squirrel::closure::SQClosure::create(&ss, fp, root_weak);
            v.push(Value::Closure(clo));
            1
        }
        Err(()) => SQ_ERROR,
    }
}

/// ↔ sqstd_dofile：loadfile + call + 返回值
fn io_dofile(v: &mut VmData) -> SQInteger {
    let nparams = v.top - v.stackbase;
    // ↔ sqstd_dofile(v, filename, retval, printerror)
    if io_loadfile(v).to_le() == SQ_ERROR.to_le() {
        return SQ_ERROR;
    }
    let clo = v.pop_get();
    // 调用：[closure, this]
    v.push(v.roottable.clone());
    let clo2 = clo.clone();
    match v.call_value(&clo2, 1, v.top - 1, true) {
        Ok(r) => {
            let _ = nparams;
            v.push(r);
            1
        }
        Err(_) => SQ_ERROR,
    }
}

fn io_writeclosuretofile(v: &mut VmData) -> SQInteger {
    // ↔ sqstd_writeclosuretofile：栈 [closure, filename]
    let clo = v.get_up(-2);
    let fname = v.get_up(-1);
    let filename = String::from_utf8_lossy(fname.as_bytes()).into_owned();
    if let Value::Closure(c) = &clo {
        match squirrel::serialization::write_closure(v, c) {
            Ok(bytes) => {
                if std::fs::write(&filename, bytes).is_err() {
                    return throw_error(v, "cannot write closure file");
                }
                v.push(Value::Integer(0));
                1
            }
            Err(_) => SQ_ERROR,
        }
    } else {
        throw_error(v, "closure expected")
    }
}

// ---------------------------------------------------------------------------
// stream/file 基础设施（Rust 用 userdata 持有 StreamHandle）
// ---------------------------------------------------------------------------

pub enum StreamHandle {
    Fd(u8),
    File(std::cell::RefCell<Option<std::fs::File>>),
    Buffer(std::cell::RefCell<std::io::Cursor<Vec<u8>>>),
}

pub struct StreamData {
    pub handle: StreamHandle,
    pub eof: bool,
}

fn make_stream_instance(ss: &Rc<SharedState>, file_class: &Value, handle: StreamHandle) -> Value {
    let ud = squirrel::userdata::SQUserData::create(ss, 0);
    // 用 userdata 的 delegate 挂 file 类方法表；数据放在 typetag 指向的 Rc
    let rc_data: Rc<std::cell::RefCell<StreamData>> = Rc::new(std::cell::RefCell::new(StreamData {
        handle,
        eof: false,
    }));
    STREAMS.with(|s| {
        let mut b = s.borrow_mut();
        b.push(rc_data.clone());
        let id = b.len() - 1;
        ud.borrow_mut().typetag = id;
    });
    if let Value::Table(fc) = file_class {
        let m = fc.borrow().get(&ss.intern(b"__methods"));
        if let Some(Value::Table(mt)) = m {
            ud.borrow_mut().delegate = Some(mt);
        }
    }
    Value::Userdata(ud)
}

thread_local! {
    static STREAMS: std::cell::RefCell<Vec<Rc<std::cell::RefCell<StreamData>>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}


fn make_file_class(ss: &Rc<SharedState>) -> Value {
    let t = squirrel::table::SQTable::create(ss, 8);
    reg_stream_methods(ss, &t);
    Value::Table(t)
}

fn make_stream_class(ss: &Rc<SharedState>) -> Value {
    let t = squirrel::table::SQTable::create(ss, 8);
    reg_stream_methods(ss, &t);
    Value::Table(t)
}

fn reg_stream_methods(ss: &Rc<SharedState>, t: &TableRef) {
    let _ = ss;
    // M9 后续：readblob/readn/writeblob/writen/seek/tell/len/eos/flush
    t.borrow_mut().new_slot(&ss.intern(b"_typeof"), &{
        let f: squirrel::closure::NativeFn = Rc::new(|v: &mut VmData| {
            v.push(v.ss.intern(b"file"));
            1
        });
        Value::NativeClosure(squirrel::closure::SQNativeClosure::create(ss, f, 1))
    });
}

// ===========================================================================
// string（↔ sqstdstring.cpp；regexp 见 regex.rs）
// ===========================================================================

pub fn register_stringlib(vm: &mut VmData) {
    let ss = vm.ss.clone();
    let root = match &vm.roottable {
        Value::Table(t) => t.clone(),
        _ => return,
    };

    reg_global(&ss, &root, "format", Rc::new(string_format), -1, Some("."));
    reg_global(&ss, &root, "printf", Rc::new(|v: &mut VmData| {
        // ↔ printf：format 到 scratchpad 后经 printfunc 输出
        match string_format_impl(v, false) {
            Some(_) => 0,
            None => SQ_ERROR,
        }
    }), -1, Some("."));
    reg_global(&ss, &root, "strip", Rc::new(|v| string_strip_impl(v, Trim::Both)), 2, Some(".s"));
    reg_global(&ss, &root, "lstrip", Rc::new(|v| string_strip_impl(v, Trim::Left)), 2, Some(".s"));
    reg_global(&ss, &root, "rstrip", Rc::new(|v| string_strip_impl(v, Trim::Right)), 2, Some(".s"));
    reg_global(&ss, &root, "split", Rc::new(string_split), -2, Some(".ssb"));
    reg_global(&ss, &root, "escape", Rc::new(string_escape), 2, Some(".s"));
    reg_global(&ss, &root, "startswith", Rc::new(string_startswith), 3, Some(".ss"));
    reg_global(&ss, &root, "endswith", Rc::new(string_endswith), 3, Some(".ss"));

    // regexp 类
    let rx_class = crate::regex::make_regexp_class(&ss);
    let key = ss.intern(b"regexp");
    root.borrow_mut().new_slot(&key, &rx_class);
}

/// ↔ sqstd_format：printf 风格（%s %i %d %c %u %o %x %X %f %g %G %e %E）
fn string_format(v: &mut VmData) -> SQInteger {
    match string_format_impl(v, true) {
        Some(_) => 1,
        None => SQ_ERROR,
    }
}

fn string_format_impl(v: &mut VmData, push_result: bool) -> Option<()> {
    // ↔ sqstd_format：this 在槽 0（newbase），格式串在槽 1（stack_get(2) 语义）
    let nparams = v.top - v.stackbase - 1; // 参数个数（不含 this）
    let fmt_val = v.stack_get(2);
    let fmt = fmt_val.as_bytes();
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0usize;
    let mut arg = 2usize;
    while i < fmt.len() {
        let c = fmt[i];
        if c != b'%' {
            out.push(c);
            i += 1;
            continue;
        }
        i += 1;
        if i >= fmt.len() {
            break;
        }
        if fmt[i] == b'%' {
            out.push(b'%');
            i += 1;
            continue;
        }
        // flags/width/precision
        let mut flags = String::new();
        while i < fmt.len() && matches!(fmt[i], b'-' | b'+' | b' ' | b'#' | b'0') {
            flags.push(fmt[i] as char);
            i += 1;
        }
        let mut width = String::new();
        while i < fmt.len() && fmt[i].is_ascii_digit() {
            width.push(fmt[i] as char);
            i += 1;
        }
        let mut prec = String::new();
        if i < fmt.len() && fmt[i] == b'.' {
            i += 1;
            while i < fmt.len() && fmt[i].is_ascii_digit() {
                prec.push(fmt[i] as char);
                i += 1;
            }
        }
        if i >= fmt.len() {
            break;
        }
        let conv = fmt[i];
        i += 1;
        if arg > nparams as usize {
            v.raise_error_str("not enough parameters for the given format string");
            return None;
        }
        // 参数 i 在栈槽 i+1（this 在 0）
        let aval = v.stack_get(arg as SQInteger + 1);
        arg += 1;
        let w: usize = width.parse().unwrap_or(0);
        let p: Option<usize> = if prec.is_empty() { None } else { Some(prec.parse().unwrap_or(0)) };
        let left = flags.contains('-');
        let zeropad = flags.contains('0') && !left;
        let piece: Vec<u8> = match conv {
            b's' => {
                let s = match v.to_string_value(&aval) {
                    Ok(s) => s,
                    Err(_) => return None,
                };
                let mut b = s.as_bytes().to_vec();
                if let Some(p) = p {
                    if b.len() > p {
                        b.truncate(p);
                    }
                }
                b
            }
            b'i' | b'd' => {
                let n = aval.as_integer();
                let mut s = n.abs().to_string();
                if let Some(p) = p {
                    while s.len() < p {
                        s.insert(0, '0');
                    }
                }
                if n < 0 {
                    s.insert(0, '-');
                } else if flags.contains('+') {
                    s.insert(0, '+');
                }
                s.into_bytes()
            }
            b'u' => {
                let n = aval.as_integer() as SQUnsignedInteger;
                n.to_string().into_bytes()
            }
            b'x' => format!("{:x}", aval.as_integer()).into_bytes(),
            b'X' => format!("{:X}", aval.as_integer()).into_bytes(),
            b'o' => format!("{:o}", aval.as_integer()).into_bytes(),
            b'c' => vec![aval.as_integer() as u8],
            b'f' => squirrel::cformat::format_f(aval.as_float() as f64, p.unwrap_or(6)).into_bytes(),
            b'g' | b'G' => {
                let mut s = squirrel::cformat::format_g_prec(aval.as_float() as f64, p.unwrap_or(6));
                if conv == b'G' {
                    s = s.to_uppercase();
                }
                s.into_bytes()
            }
            b'e' => squirrel::cformat::format_e(aval.as_float() as f64, p.unwrap_or(6)).into_bytes(),
            b'E' => {
                let s = squirrel::cformat::format_e(aval.as_float() as f64, p.unwrap_or(6));
                s.to_uppercase().into_bytes()
            }
            _ => {
                v.raise_error_str("invalid format");
                return None;
            }
        };
        if piece.len() < w {
            let pad = w - piece.len();
            if left {
                out.extend_from_slice(&piece);
                out.extend(std::iter::repeat(b' ').take(pad));
            } else if zeropad && matches!(conv, b'd' | b'i' | b'u' | b'x' | b'X' | b'o') && piece.first() != Some(&b'-') {
                out.extend(std::iter::repeat(b'0').take(pad));
                out.extend_from_slice(&piece);
            } else if zeropad && matches!(conv, b'd' | b'i' | b'u' | b'x' | b'X' | b'o') && piece.first() == Some(&b'-') {
                out.push(b'-');
                out.extend(std::iter::repeat(b'0').take(pad));
                out.extend_from_slice(&piece[1..]);
            } else {
                out.extend(std::iter::repeat(b' ').take(pad));
                out.extend_from_slice(&piece);
            }
        } else {
            out.extend_from_slice(&piece);
        }
    }
    if push_result {
        let s = v.ss.intern(&out);
        v.push(s);
    } else {
        // printf：走 printfunc
        if let Some(pf) = v.ss.printfunc.borrow().clone() {
            pf(v, &String::from_utf8_lossy(&out));
        }
    }
    Some(())
}

enum Trim {
    Left,
    Right,
    Both,
}

fn string_strip_impl(v: &mut VmData, t: Trim) -> SQInteger {
    let s = v.stack_get(2);
    let b = s.as_bytes();
    let is_ws = |c: &u8| matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C);
    let (start, end) = match t {
        Trim::Left => (b.iter().position(|c| !is_ws(c)).unwrap_or(b.len()), b.len()),
        Trim::Right => (0, b.iter().rposition(|c| !is_ws(c)).map(|x| x + 1).unwrap_or(0)),
        Trim::Both => (
            b.iter().position(|c| !is_ws(c)).unwrap_or(b.len()),
            b.iter().rposition(|c| !is_ws(c)).map(|x| x + 1).unwrap_or(0),
        ),
    };
    let s2 = v.ss.intern(&b[start.max(end.min(start))..end.max(start)]);
    v.push(s2);
    1
}

fn string_split(v: &mut VmData) -> SQInteger {
    let s = v.stack_get(1);
    let seps = v.stack_get(2);
    let mut skipempty = false;
    if v.top - v.stackbase > 2 {
        skipempty = !VmData::is_false(&v.stack_get(3));
    }
    let src = s.as_bytes();
    let seps = seps.as_bytes();
    let arr = squirrel::array::SQArray::create(&v.ss, 0);
    let mut start = 0usize;
    let mut i = 0usize;
    while i < src.len() {
        if seps.contains(&src[i]) {
            let piece = &src[start..i];
            if !piece.is_empty() || !skipempty {
                let s2 = v.ss.intern(piece);
                arr.borrow_mut().append(&s2);
            }
            start = i + 1;
        }
        i += 1;
    }
    if start <= src.len() {
        let piece = &src[start..];
        if !piece.is_empty() || !skipempty {
            let s2 = v.ss.intern(piece);
            arr.borrow_mut().append(&s2);
        }
    }
    v.push(Value::Array(arr));
    1
}

fn string_escape(v: &mut VmData) -> SQInteger {
    let s = v.stack_get(2);
    let src = s.as_bytes();
    let mut out = Vec::with_capacity(src.len());
    for &c in src {
        match c {
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'"' => out.extend_from_slice(b"\\\""),
            b'\'' => out.extend_from_slice(b"\\'"),
            0x07 => out.extend_from_slice(b"\\a"),
            0x08 => out.extend_from_slice(b"\\b"),
            0x0B => out.extend_from_slice(b"\\v"),
            0x0C => out.extend_from_slice(b"\\f"),
            c if c < 32 || c >= 127 => {
                out.extend_from_slice(format!("\\x{:02x}", c).as_bytes())
            }
            c => out.push(c),
        }
    }
    let s2 = v.ss.intern(&out);
    v.push(s2);
    1
}

fn string_startswith(v: &mut VmData) -> SQInteger {
    let s = v.stack_get(2);
    let p = v.stack_get(3);
    v.push(Value::Bool(s.as_bytes().starts_with(p.as_bytes())));
    1
}

fn string_endswith(v: &mut VmData) -> SQInteger {
    let s = v.stack_get(2);
    let p = v.stack_get(3);
    v.push(Value::Bool(s.as_bytes().ends_with(p.as_bytes())));
    1
}

// ===========================================================================
// blob（↔ sqstdblob.cpp）
// ===========================================================================

pub fn register_bloblib(vm: &mut VmData) {
    let ss = vm.ss.clone();
    let root = match &vm.roottable {
        Value::Table(t) => t.clone(),
        _ => return,
    };

    macro_rules! bfn {
        ($name:expr, $tm:expr, $f:expr) => {
            reg_global(&ss, &root, $name, Rc::new($f), 2, Some($tm));
        };
    }

    bfn!("castf2i", "f", |v: &mut VmData| {
        let f = v.stack_get(2).as_float();
        v.push(Value::Integer(squirrel::config::c_f32_to_i64(f)));
        1
    });
    bfn!("casti2f", "i", |v: &mut VmData| {
        let i = v.stack_get(2).as_integer();
        v.push(Value::Float(f32::from_bits(i as u32)));
        1
    });
    bfn!("swap2", "i", |v: &mut VmData| {
        let i = v.stack_get(2).as_integer();
        v.push(Value::Integer(((i & 0xFF) << 8) | ((i >> 8) & 0xFF)));
        1
    });
    bfn!("swap4", "i", |v: &mut VmData| {
        let i = v.stack_get(2).as_integer();
        v.push(Value::Integer(i.swap_bytes()));
        1
    });
    bfn!("swapfloat", "f", |v: &mut VmData| {
        let f = v.stack_get(2).as_float();
        v.push(Value::Float(f32::from_bits(f.to_bits().swap_bytes())));
        1
    });
}

// ===========================================================================
// aux（↔ sqstdaux.cpp：错误处理器与调用栈打印）
// ===========================================================================

/// ↔ sqstd_seterrorhandlers
pub fn set_error_handlers(vm: &squirrel::api::Vm) {
    // 编译错误钩子
    {
        let ss = vm.ss.clone();
        let handler: squirrel::state::CompilerErrorHandler = Rc::new(move |v, desc, source, line, column| {
            let _ = v;
            if let Some(ef) = ss.errorfunc.borrow().clone() {
                let msg = format!("{} line = ({}) column = ({}) : error {}\n", source, line, column, desc);
                ef(v, &msg);
            }
        });
        *vm.ss.compilererrorhandler.borrow_mut() = Some(handler);
    }
    // 运行时错误钩子（CALLSTACK/LOCALS 输出在 M11 完善格式）
    let mut v = vm.data_mut();
    let f: squirrel::closure::NativeFn = Rc::new(|v: &mut VmData| {
        let e = v.stack_get(2);
        let ss_ = v.ss.clone();
        let ef_ = ss_.errorfunc.borrow().clone();
        if let Some(ef) = ef_ {
            let msg = match v.to_string_value(&e) {
                Ok(s) => String::from_utf8_lossy(s.as_bytes()).into_owned(),
                Err(_) => "unknown".to_string(),
            };
            ef(v, &format!("\nAN ERROR HAS OCCURRED [{}]\n", msg));
            print_callstack(v);
        }
        0
    });
    let nc = squirrel::closure::SQNativeClosure::create(&vm.ss, f, 0);
    v.errorhandler = Value::NativeClosure(nc);
}

/// ↔ sqstd_printcallstack（格式逐字对齐 M11 处理）
fn print_callstack(v: &mut VmData) {
    let ss_ = v.ss.clone();
    let ef_ = ss_.errorfunc.borrow().clone();
    if let Some(ef) = ef_ {
        ef(v, "\nCALLSTACK\n");
        let mut level = 0;
        loop {
            let cssize = v.callsstack.len() as SQInteger;
            if cssize <= level {
                break;
            }
            let idx = (cssize - level - 1) as usize;
            let (fname, src, line) = match &v.callsstack[idx].closure {
                Value::Closure(c) => {
                    let proto = c.borrow().function.clone();
                    let line = proto.get_line(v.callsstack[idx].ip as SQInteger);
                    (
                        match &proto.name {
                            Value::String(s) => String::from_utf8_lossy(&s.val).into_owned(),
                            _ => "unknown".to_string(),
                        },
                        match &proto.sourcename {
                            Value::String(s) => String::from_utf8_lossy(&s.val).into_owned(),
                            _ => "unknown".to_string(),
                        },
                        line,
                    )
                }
                Value::NativeClosure(nc) => {
                    let n = nc.borrow().name.clone();
                    (
                        match &n {
                            Value::String(s) => String::from_utf8_lossy(&s.val).into_owned(),
                            _ => "unknown".to_string(),
                        },
                        "NATIVE".to_string(),
                        -1,
                    )
                }
                _ => continue,
            };
            ef(v, &format!("*FUNCTION [{}()] source line [{}]\n", fname, line));
            let _ = src;
            level += 1;
            if level > 10 {
                break;
            }
        }
    }
}

/// 注册全部标准库（sq.c 的调用顺序）
pub fn register_all(vm: &mut VmData) {
    register_bloblib(vm);
    register_iolib(vm);
    register_systemlib(vm);
    register_mathlib(vm);
    register_stringlib(vm);
}

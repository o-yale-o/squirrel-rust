//! Squirrel 标准库（↔ sqstdlib/*.cpp）：math / system / io / blob / string / aux。
//!
//! 原生函数直接操作 `&mut VmData`（与 baselib 相同的方式）。

pub mod regex;
pub mod stream;

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

    // ↔ _DECL_FUNC(name,nparams,tycheck)：掩码首位 '.' 是 `this` 占位，nparamscheck 含 `this`
    macro_rules! m1 {
        ($name:expr, $tm:expr, $f:expr) => {
            reg_global(&ss, &root, $name, Rc::new($f), 2, Some(concat!(".", $tm)));
        };
    }
    macro_rules! m2 {
        ($name:expr, $tm:expr, $f:expr) => {
            reg_global(&ss, &root, $name, Rc::new($f), 3, Some(concat!(".", $tm)));
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
    m2!("atan2", "nn", |v: &mut VmData| {
        let y = v.stack_get(2).as_float();
        let x = v.stack_get(3).as_float();
        v.push(Value::Float(y.atan2(x)));
        1
    });
    m2!("pow", "nn", |v: &mut VmData| {
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
    // ↔ _DECL_FUNC(rand,1,NULL)：无掩码，nparamscheck=1（只有 this）
    reg_global(&ss, &root, "rand", Rc::new(|v: &mut VmData| {
        let r = SQ_RNG.with(|r| r.borrow_mut().rand());
        v.push(Value::Integer(r));
        1
    }), 1, None);
    m1!("fabs", "n", |v: &mut VmData| {
        let x = v.stack_get(2).as_float();
        v.push(Value::Float(x.abs()));
        1
    });
    m1!("abs", "n", |v: &mut VmData| {
        // ↔ math_abs：sq_getinteger 先把数值取整，再 abs((int)n)（MSVC 32 位截断/回绕语义）
        let n = v.stack_get(2).as_integer();
        v.push(Value::Integer((n as i32).wrapping_abs() as SQInteger));
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

/// ↔ `clock()/CLOCKS_PER_SEC`：进程占用的 CPU 秒数（user + kernel）。
///
/// MSVC 的 `CLOCKS_PER_SEC` 是 1000，`clock()` 返回毫秒级的 CPU 时间；
/// 这里用 `GetProcessTimes` 拿同样的量（Windows）。其它平台退回墙上时间。
#[cfg(windows)]
fn process_cpu_seconds() -> f64 {
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    extern "system" {
        fn GetCurrentProcess() -> *mut core::ffi::c_void;
        fn GetProcessTimes(
            h: *mut core::ffi::c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }
    let ft = |f: FileTime| -> u64 { ((f.high as u64) << 32) | f.low as u64 };
    let (mut c, mut e, mut k, mut u) = (
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
    );
    unsafe {
        if GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u) != 0 {
            // FILETIME 单位是 100ns
            return (ft(k) + ft(u)) as f64 / 1.0e7;
        }
    }
    0.0
}

#[cfg(not(windows))]
fn process_cpu_seconds() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

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
        // ↔ CRT `system()`：子进程**继承**父进程的 stdin/stdout/stderr。
        // 原来用 `.output()` 把子进程输出捕获后丢掉 —— 实测 `system("echo SQTEST")`
        // 参考实现会把 SQTEST 打到脚本 stdout 上，我们什么都不打（P1-5 抓到的差异）。
        let out = std::process::Command::new("cmd")
            .args(["/C", &String::from_utf8_lossy(cmd.as_bytes())])
            .status();
        match out {
            Ok(o) => v.push(Value::Integer(o.code().unwrap_or(-1) as SQInteger)),
            Err(_) => v.push(Value::Integer(-1)),
        }
        1
    }), 2, Some(".s"));

    reg_global(&ss, &root, "clock", Rc::new(|v: &mut VmData| {
        // ↔ `clock()/CLOCKS_PER_SEC`：这是**进程 CPU 时间**，不是墙上时间。
        // 原来用 SystemTime 差值（墙上时间）——类型上测不出来，语义上是错的。
        v.push(Value::Float(process_cpu_seconds() as SQFloat));
        1
    }), 0, None); // ↔ _DECL_FUNC(clock,0,NULL)：不做参数个数检查

    reg_global(&ss, &root, "time", Rc::new(|v: &mut VmData| {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as SQInteger)
            .unwrap_or(0);
        v.push(Value::Integer(t));
        1
    }), 1, None);

    // ↔ systemlib_funcs 的顺序：getenv, system, clock, time, **date**, remove, rename
    // （根表迭代顺序可被 `foreach getroottable()` 观察到，必须逐位对齐）
    reg_global(&ss, &root, "date", Rc::new(date_fn), -1, Some(".nn"));

    reg_global(&ss, &root, "remove", Rc::new(|v: &mut VmData| {
        let p = v.stack_get(2);
        // ↔ `_system_remove`：失败**抛错** `remove() failed`，成功返回 0 个值
        // （脚本看到的是 null）。原来返回 -1/0，语义不同（P1-5 抓到）。
        match std::fs::remove_file(String::from_utf8_lossy(p.as_bytes()).as_ref()) {
            Ok(()) => 0,
            Err(_) => throw_error(v, "remove() failed"),
        }
    }), 2, Some(".s"));

    reg_global(&ss, &root, "rename", Rc::new(|v: &mut VmData| {
        let a = v.stack_get(2);
        let b = v.stack_get(3);
        // ↔ `_system_rename`：失败抛错 `rename() failed`，成功返回 0 个值
        match std::fs::rename(
            String::from_utf8_lossy(a.as_bytes()).as_ref(),
            String::from_utf8_lossy(b.as_bytes()).as_ref(),
        ) {
            Ok(()) => 0,
            Err(_) => throw_error(v, "rename() failed"),
        }
    }), 3, Some(".ss"));
}

/// ↔ sqstd_system_date：返回 {sec,min,hour,day,month,year,wday,yday}
fn date_fn(v: &mut VmData) -> SQInteger {
    let top = v.top - v.stackbase;
    let mut t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // ↔ _system_date：date() / date(t) / date(t, 'u'|'l')
    let mut format = b'l' as SQInteger;
    if top > 1 {
        t = v.stack_get(2).as_integer();
        if top > 2 {
            format = v.stack_get(3).as_integer();
        }
    }
    // ↔ MSVC CRT 的可表示范围：gmtime 自 −43200 起，localtime 不接受负值；
    // 上界为 UCRT 内部常量（约 3001-01-19，此处取实测可用上界）。超范围时 CRT 返回 NULL。
    const CRT_MAX_TIME: i64 = 32_536_800_000;
    let is_utc = format == b'u' as SQInteger;
    if t > CRT_MAX_TIME || (is_utc && t < -43200) || (!is_utc && t < 0) {
        return throw_error(v, "crt api failure");
    }
    let days = t.div_euclid(86400);
    let secs = t.rem_euclid(86400);
    let (y0, m1, d0) = civil_from_days(days);
    let (mut y, mut d, mut mon0) = (y0, d0, m1 - 1); // tm_mon 是 0 基
    let (mut hour, mut minute, mut sec) = (secs / 3600, (secs / 60) % 60, secs % 60);
    if !is_utc {
        // ↔ localtime（含夏令时）；非 Windows 平台退回 UTC
        if let Some((ly, lm0, ld, lh, lmi, ls)) = to_local_time(y, mon0, d, hour, minute, sec) {
            y = ly;
            mon0 = lm0;
            d = ld;
            hour = lh;
            minute = lmi;
            sec = ls;
        }
    }
    let wday = ((days_from_civil(y, mon0 + 1, d) + 4).rem_euclid(7)) as SQInteger; // 1970-01-01 是周四
    let yday = day_of_year(y, mon0 + 1, d);
    let t2 = SQ_TABLE_CREATE!(v);
    {
        let tt = match &t2 {
            Value::Table(tt) => tt.clone(),
            _ => unreachable!(),
        };
        let mut b = tt.borrow_mut();
        b.new_slot(&v.ss.intern(b"sec"), &Value::Integer(sec));
        b.new_slot(&v.ss.intern(b"min"), &Value::Integer(minute));
        b.new_slot(&v.ss.intern(b"hour"), &Value::Integer(hour));
        b.new_slot(&v.ss.intern(b"day"), &Value::Integer(d));
        b.new_slot(&v.ss.intern(b"month"), &Value::Integer(mon0));
        b.new_slot(&v.ss.intern(b"year"), &Value::Integer(y));
        b.new_slot(&v.ss.intern(b"wday"), &Value::Integer(wday));
        b.new_slot(&v.ss.intern(b"yday"), &Value::Integer(yday));
    }
    v.push(t2);
    1
}

/// ↔ civil_from_days 的逆运算（m 为 1 基，与 C++ days_from_civil 相同）
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// ↔ localtime：Windows 用 SystemTimeToTzSpecificLocalTime（自动处理夏令时）
#[cfg(windows)]
fn to_local_time(y: i64, m0: i64, d: i64, h: i64, mi: i64, s: i64) -> Option<(i64, i64, i64, i64, i64, i64)> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct WinSystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }
    extern "system" {
        fn SystemTimeToTzSpecificLocalTime(
            tz: *const core::ffi::c_void,
            utc: *const WinSystemTime,
            local: *mut WinSystemTime,
        ) -> i32;
    }
    let utc = WinSystemTime {
        year: y as u16,
        month: (m0 + 1) as u16,
        day_of_week: 0,
        day: d as u16,
        hour: h as u16,
        minute: mi as u16,
        second: s as u16,
        milliseconds: 0,
    };
    let mut local = utc;
    let ok = unsafe { SystemTimeToTzSpecificLocalTime(core::ptr::null(), &utc, &mut local) };
    if ok == 0 {
        return None;
    }
    Some((
        local.year as i64,
        local.month as i64 - 1,
        local.day as i64,
        local.hour as i64,
        local.minute as i64,
        local.second as i64,
    ))
}

#[cfg(not(windows))]
fn to_local_time(_y: i64, _m0: i64, _d: i64, _h: i64, _mi: i64, _s: i64) -> Option<(i64, i64, i64, i64, i64, i64)> {
    None
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
    reg_global(&ss, &root, "writeclosuretofile", Rc::new(io_writeclosuretofile), 3, Some(".sc"));

    // ↔ declare_stream("file", ...)：先注册 iolib 的全局函数，再插入 file 类
    // （stream/blob 已在 bloblib 里建好并注册，iolib 复用同一个 stream 基类）
    let (_stream2, file_cls, _blob2) = crate::stream::make_stream_classes(vm);
    crate::stream::register_class(vm, "file", &file_cls);

    // ↔ sqstd_createfile：顺序是 stdout, stdin, stderr
    for (name, fd) in [("stdout", 1u8), ("stdin", 0u8), ("stderr", 2u8)] {
        crate::stream::push_file_instance(vm, &file_cls, fd);
        let inst = v_take(vm);
        let key = ss.intern(name.as_bytes());
            root.borrow_mut().new_slot(&key, &inst);
    }
}

/// 取栈顶并弹出（注册辅助）
fn v_take(vm: &mut VmData) -> Value {
    vm.pop_get()
}

/// ↔ sqstd_loadfile：嗅探字节码 / UTF-8 BOM / UCS2 → 编译 → 把闭包压栈。
/// `printerror` 即 C++ 的 raiseerror：为真时调用 compilererrorhandler。
/// 失败时已设置 lasterror，调用方直接返回 SQ_ERROR。
pub fn sqstd_load_file(v: &mut VmData, filename: &str, printerror: bool) -> Result<(), ()> {
    let data = match std::fs::read(filename) {
        Ok(d) => d,
        Err(_) => {
            v.raise_error_str("cannot open the file");
            return Err(());
        }
    };
    // 前两字节按小端 u16 读（↔ sqstd_fread(&us,1,2,file)）
    let us: u16 = if data.len() >= 2 {
        u16::from_le_bytes([data[0], data[1]])
    } else {
        0 // 空文件
    };
    if us == 0xFAFA {
        // BYTECODE：回到偏移 0 再反序列化
        return match squirrel::serialization::read_closure(v, &data) {
            Ok(clo) => {
                v.push(clo);
                Ok(())
            }
            Err(e) => {
                v.raise_error_str(&e);
                Err(())
            }
        };
    }
    // 词法输入：UCS2 下喂完整码元（与 C++ 的 SQLEXREADFUNC 返回 SQInteger 一致）
    let units: Vec<SQInteger> = match us {
        0xFEFF => ucs2_units(&data[2..], false), // UTF-16LE
        0xFFFE => ucs2_units(&data[2..], true),  // UTF-16BE
        0xBBEF => {
            if data.len() < 3 {
                v.raise_error_str("io error");
                return Err(());
            }
            if data[2] != 0xBF {
                v.raise_error_str("Unrecognized encoding");
                return Err(());
            }
            data[3..].iter().map(|b| *b as SQInteger).collect() // UTF-8 BOM（非 UNICODE 构建按字节流）
        }
        _ => data.iter().map(|b| *b as SQInteger).collect(),
    };
    let mut pos = 0usize;
    let readf = Box::new(move || {
        if pos < units.len() {
            let c = units[pos];
            pos += 1;
            c
        } else {
            0
        }
    });
    let ss = v.ss.clone();
    match squirrel::compiler::compile(v, readf, filename.as_bytes(), printerror, ss.debuginfo.get()) {
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
            Ok(())
        }
        // 编译错误：lasterror 与 compilererrorhandler 已由 compiler 处理
        Err(()) => Err(()),
    }
}

/// ↔ _io_file_lexfeed_UCS2_LE/BE：每 2 字节一个码元；BE 需交换字节序
fn ucs2_units(tail: &[u8], swap: bool) -> Vec<SQInteger> {
    tail.chunks_exact(2)
        .map(|p| {
            let mut c = u16::from_le_bytes([p[0], p[1]]);
            if swap {
                c = c.swap_bytes();
            }
            c as SQInteger
        })
        .collect()
}

/// ↔ _g_io_loadfile：loadfile(path [, printerror])（printerror 默认 false）
fn io_loadfile(v: &mut VmData) -> SQInteger {
    let path = v.stack_get(2);
    let filename = String::from_utf8_lossy(path.as_bytes()).into_owned();
    let printerror = v.top - v.stackbase >= 3 && v.stack_get(3).as_bool();
    match sqstd_load_file(v, &filename, printerror) {
        Ok(()) => 1,
        Err(()) => SQ_ERROR,
    }
}

/// ↔ _g_io_dofile + sqstd_dofile：loadfile + 以调用者 this 为环境调用
fn io_dofile(v: &mut VmData) -> SQInteger {
    let printerror = v.top - v.stackbase >= 3 && v.stack_get(3).as_bool();
    let env = v.stack_get(1); // ↔ _g_io_dofile 的 sq_push(v,1)：环境 = 调用者 this
    let path = v.stack_get(2);
    let filename = String::from_utf8_lossy(path.as_bytes()).into_owned();
    if sqstd_load_file(v, &filename, printerror).is_err() {
        return SQ_ERROR;
    }
    let clo = v.pop_get();
    v.push(env);
    let clo2 = clo.clone();
    match v.call_value(&clo2, 1, v.top - 1, true) {
        Ok(r) => {
            v.push(r);
            1
        }
        Err(_) => SQ_ERROR,
    }
}

fn io_writeclosuretofile(v: &mut VmData) -> SQInteger {
    // ↔ _g_io_writeclosuretofile：栈 [this, filename, closure]
    let fname = v.get_up(-2);
    let clo = v.get_up(-1);
    let filename = String::from_utf8_lossy(fname.as_bytes()).into_owned();
    // ↔ sqstd_writeclosuretofile：**先 fopen("wb+")，再交给 sq_writeclosure**。
    // 顺序是实测出来的（模糊测试在私有工作目录里暴露的差异），而且很反直觉：
    //   - 第 3 参过不了 `.sc` 掩码时（比如传整数），掩码错误先报，**不会**建文件；
    //   - 过了掩码之后先开文件：路径不可写 → "cannot open the file"（哪怕第 3 参是
    //     native closure、后面还会类型不符）；
    //   - 路径可写则文件立刻建出来，之后无论序列化失败（自由变量）还是类型检查失败
    //     （native closure 不匹配 OT_CLOSURE），那个 **0 字节文件都会留在磁盘上**。
    let mut f = match std::fs::File::create(&filename) {
        Ok(f) => f,
        Err(_) => return throw_error(v, "cannot open the file"),
    };
    if let Value::Closure(c) = &clo {
        match squirrel::serialization::write_closure(v, c) {
            Ok(bytes) => {
                if std::io::Write::write_all(&mut f, &bytes).is_err() {
                    // 参考实现这一步**不查** file_write 的返回值，等于静默成功。
                    // 按本仓库的健壮性约定，这里报可捕获错误；该路径在差分里触发不到
                    // （要磁盘满之类的环境），属于有意保留的差异。
                    return throw_error(v, "cannot write the file");
                }
                drop(f);
                // C++ 返回 1 且栈顶仍是闭包：返回值就是被写出的闭包本身
                v.push(clo);
                1
            }
            // 错误已由 write_closure 设置；文件留着（与参考实现一致）
            Err(_) => SQ_ERROR,
        }
    } else {
        // ↔ sq_writeclosure 的 OT_CLOSURE 检查：native closure / 其它类型走到这里，
        // 文件此时已经建出来并会留下（与参考实现一致）
        drop(f);
        // ↔ sq_aux_gettypedarg：native closure 过得了 `.sc` 掩码、但过不了 OT_CLOSURE 检查，
        // 报的是 `wrong argument type, expected 'function' got '<PrintObjVal>'`
        // —— 后者对 native closure 就是类型名，于是出现那句
        // "expected 'function' got 'function'"
        let got = match &clo {
            Value::String(s) => String::from_utf8_lossy(&s.val).into_owned(),
            other => squirrel::object::get_type_name(other).unwrap_or("unknown").to_string(),
        };
        let got: String = got.chars().take(50).collect();
        throw_error(v, &format!("wrong argument type, expected 'function' got '{}'", got))
    }
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

    // ↔ sqstd_register_stringlib：regexp 类先建，其后才是 format/printf/...
    let rx_class = crate::regex::make_regexp_class(&ss);
    root.borrow_mut()
        .new_slot(&ss.intern(b"regexp"), &rx_class);

    reg_global(&ss, &root, "format", Rc::new(string_format), -2, Some(".s"));
    reg_global(&ss, &root, "printf", Rc::new(|v: &mut VmData| {
        // ↔ printf：format 到 scratchpad 后经 printfunc 输出
        match string_format_impl(v, false) {
            Some(_) => 0,
            None => SQ_ERROR,
        }
    }), -2, Some(".s"));
    reg_global(&ss, &root, "strip", Rc::new(|v| string_strip_impl(v, Trim::Both)), 2, Some(".s"));
    reg_global(&ss, &root, "lstrip", Rc::new(|v| string_strip_impl(v, Trim::Left)), 2, Some(".s"));
    reg_global(&ss, &root, "rstrip", Rc::new(|v| string_strip_impl(v, Trim::Right)), 2, Some(".s"));
    reg_global(&ss, &root, "split", Rc::new(string_split), -3, Some(".ssb"));
    reg_global(&ss, &root, "escape", Rc::new(string_escape), 2, Some(".s"));
    reg_global(&ss, &root, "startswith", Rc::new(string_startswith), 3, Some(".ss"));
    reg_global(&ss, &root, "endswith", Rc::new(string_endswith), 3, Some(".ss"));

}

/// ↔ sqstd_format：printf 风格（%s %i %d %c %u %o %x %X %f %g %G %e %E）
fn string_format(v: &mut VmData) -> SQInteger {
    match string_format_impl(v, true) {
        Some(_) => 1,
        None => SQ_ERROR,
    }
}

/// 格式说明符（↔ validate_format 的解析结果 + 转换字符）
#[derive(Default)]
struct FmtSpec {
    minus: bool,
    plus: bool,
    space: bool,
    hash: bool,
    zero: bool,
    width: usize,
    prec: Option<usize>,
    conv: u8,
}

/// ↔ sq_getinteger：数值（float 截断，越界给 INT64_MIN）或布尔，其余类型不接受
fn fmt_int(aval: &Value) -> Option<SQInteger> {
    match aval.type_() {
        OT_INTEGER | OT_FLOAT | OT_BOOL => Some(aval.as_integer()),
        _ => None,
    }
}

/// ↔ sq_getfloat：只接受数值（integer 可以，**bool 不行**）
fn fmt_float(aval: &Value) -> Option<f64> {
    match aval.type_() {
        OT_INTEGER | OT_FLOAT => Some(aval.as_float() as f64),
        _ => None,
    }
}

fn string_format_impl(v: &mut VmData, push_result: bool) -> Option<()> {
    // ↔ sqstd_format：this 在槽 1，格式串在槽 2，第一个可变参数在槽 3
    let nparams = v.top - v.stackbase - 1; // 参数个数（不含 this）
    let fmt_val = v.stack_get(2);
    let fmt = fmt_val.as_bytes().to_vec();
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
        if i < fmt.len() && fmt[i] == b'%' {
            out.push(b'%');
            i += 1;
            continue;
        }
        // ↔ sqstd_format：**先**查参数个数，所以 "abc%" 报的是 not enough parameters
        if arg > nparams as usize {
            v.raise_error_str("not enough parameters for the given format string");
            return None;
        }
        let start = i;
        let mut sp = FmtSpec::default();
        while i < fmt.len() && matches!(fmt[i], b'-' | b'+' | b' ' | b'#' | b'0') {
            match fmt[i] {
                b'-' => sp.minus = true,
                b'+' => sp.plus = true,
                b' ' => sp.space = true,
                b'#' => sp.hash = true,
                _ => sp.zero = true,
            }
            i += 1;
        }
        let mut wdigits = 0usize;
        while i < fmt.len() && fmt[i].is_ascii_digit() {
            sp.width = sp.width * 10 + (fmt[i] - b'0') as usize;
            i += 1;
            wdigits += 1;
            // ↔ MAX_WFORMAT_LEN=3：第 3 位数字就报错（"%" + 最多两位宽度）
            if wdigits >= 3 {
                v.raise_error_str("width format too long");
                return None;
            }
        }
        if i < fmt.len() && fmt[i] == b'.' {
            i += 1;
            let mut p = 0usize;
            let mut pdigits = 0usize;
            while i < fmt.len() && fmt[i].is_ascii_digit() {
                p = p * 10 + (fmt[i] - b'0') as usize;
                i += 1;
                pdigits += 1;
                if pdigits >= 3 {
                    v.raise_error_str("precision format too long");
                    return None;
                }
            }
            sp.prec = Some(p); // "%.d" 是精度 0，不是“没有精度”
        }
        // ↔ validate_format 的 "format too long"：整段最长 20 字符。参考实现在**恰好 20**
        // 时会越界写 fmt[20]（实测 STATUS_STACK_BUFFER_OVERRUN 崩溃），本实现从 20 起就报错。
        if i - start >= 20 {
            v.raise_error_str("format too long");
            return None;
        }
        if i >= fmt.len() {
            // ↔ validate_format 读到 '\0'：转换字符成了 '\0' → invalid format
            v.raise_error_str("invalid format");
            return None;
        }
        sp.conv = fmt[i];
        i += 1;

        let aval = v.stack_get(arg as SQInteger + 1);
        arg += 1;

        // 各转换字符产出 (符号, 前缀, 主体, 是否允许零填充)。
        // 主体是**裸字节**：%c/%s 可能不是合法 UTF-8，参考实现是非 UNICODE 构建，
        // 0xFF 这类字节会原样写出去。
        let (sign, prefix, body, zero_ok): (Vec<u8>, Vec<u8>, Vec<u8>, bool) = match sp.conv {
            b's' => {
                // ↔ sq_getstring：%s 只收真字符串（不做 ToString）
                if aval.type_() != OT_STRING {
                    v.raise_error_str("string expected for the specified format");
                    return None;
                }
                let mut b = aval.as_bytes().to_vec();
                // ↔ CRT 的 %s 走 C 字符串：第一个 NUL 之后不输出
                if let Some(z) = b.iter().position(|x| *x == 0) {
                    b.truncate(z);
                }
                if let Some(p) = sp.prec {
                    b.truncate(p);
                }
                (Vec::new(), Vec::new(), b, true)
            }
            b'c' => match fmt_int(&aval) {
                Some(n) => (Vec::new(), Vec::new(), vec![n as u8], true),
                None => {
                    v.raise_error_str("integer expected for the specified format");
                    return None;
                }
            },
            b'd' | b'i' | b'u' | b'o' | b'x' | b'X' => {
                let n = match fmt_int(&aval) {
                    Some(n) => n,
                    None => {
                        v.raise_error_str("integer expected for the specified format");
                        return None;
                    }
                };
                let u = n as u64;
                let signed = sp.conv == b'd' || sp.conv == b'i';
                let mut digits: Vec<u8> = match sp.conv {
                    b'd' | b'i' => n.unsigned_abs().to_string().into_bytes(),
                    b'u' => u.to_string().into_bytes(),
                    b'o' => format!("{:o}", u).into_bytes(),
                    b'x' => format!("{:x}", u).into_bytes(),
                    _ => format!("{:X}", u).into_bytes(),
                };
                // 精度 = 最少位数；精度 0 且值为 0 → 空串
                if let Some(p) = sp.prec {
                    if p == 0 && u == 0 {
                        digits.clear();
                    } else if digits.len() < p {
                        let mut z = vec![b'0'; p - digits.len()];
                        z.extend_from_slice(&digits);
                        digits = z;
                    }
                }
                let mut prefix: Vec<u8> = Vec::new();
                if sp.hash {
                    match sp.conv {
                        b'x' if u != 0 => prefix = b"0x".to_vec(),
                        b'X' if u != 0 => prefix = b"0X".to_vec(),
                        b'o' if digits.first() != Some(&b'0') => prefix = b"0".to_vec(),
                        _ => {}
                    }
                }
                let sign: Vec<u8> = if signed && n < 0 {
                    b"-".to_vec()
                } else if signed && sp.plus {
                    b"+".to_vec()
                } else if signed && sp.space {
                    b" ".to_vec()
                } else {
                    Vec::new()
                };
                // ↔ C：整型指定了精度时忽略 '0' 标志
                (sign, prefix, digits, sp.prec.is_none())
            }
            b'f' | b'e' | b'E' | b'g' | b'G' => {
                let val = match fmt_float(&aval) {
                    Some(f) => f,
                    None => {
                        v.raise_error_str("float expected for the specified format");
                        return None;
                    }
                };
                let prec = sp.prec.unwrap_or(6);
                let nan = val.is_nan();
                let inf = val.is_infinite();
                let neg = val.is_sign_negative();
                // 主体不含符号：符号位负责（负 NaN 也是 sign 位 + "nan(ind)"）
                let raw: String = if nan {
                    squirrel::cformat::nan_str(val)
                        .trim_start_matches('-')
                        .to_string()
                } else if inf {
                    "inf".to_string()
                } else {
                    let a = val.abs();
                    let mut t = match sp.conv {
                        b'f' => squirrel::cformat::format_f(a, prec),
                        b'e' | b'E' => squirrel::cformat::format_e_crt(a, prec),
                        _ => {
                            if sp.hash {
                                squirrel::cformat::format_g_alt(a, prec)
                            } else {
                                squirrel::cformat::format_g_prec(a, prec)
                            }
                        }
                    };
                    // ↔ '#'：一定带小数点（e 风格要插在指数之前）
                    if sp.hash && !t.contains('.') {
                        match t.find(['e', 'E']) {
                            Some(pos) => t.insert(pos, '.'),
                            None => t.push('.'),
                        }
                    }
                    if sp.conv == b'E' || sp.conv == b'G' {
                        t = t.to_uppercase();
                    }
                    t
                };
                // ↔ MSVC：%E/%G 连 inf/nan 一起大写（"INF"、"-NAN(IND)"）
                let raw = if sp.conv == b'E' || sp.conv == b'G' {
                    raw.to_uppercase()
                } else {
                    raw
                };
                let mut body = raw.into_bytes();
                // 符号：负 NaN 永远是 "-nan(ind)"，'+'/' ' 标志只作用于非负数
                let mut sign: Vec<u8> = if neg { b"-".to_vec() } else { Vec::new() };
                if sign.is_empty() {
                    if sp.plus {
                        sign = b"+".to_vec();
                    } else if sp.space {
                        sign = b" ".to_vec();
                    }
                }
                // ↔ MSVC 的怪癖：`%#.0f`/`%#.0e` 遇到 inf/nan 时把小数点插在**整段文本的
                // 第 2 个字符位置**（"i.nf"、"-." + "inf"、小写时 "n.an"、"-." + "nan(ind)"）；
                // 其它精度下 inf/nan 完全不插小数点（`%#f` → "inf"）。
                if sp.hash
                    && (nan || inf)
                    && sp.prec == Some(0)
                    && matches!(sp.conv, b'f' | b'e' | b'E')
                {
                    let pos = if sign.is_empty() { 1 } else { 0 };
                    if pos <= body.len() {
                        body.insert(pos, b'.');
                    }
                }
                // ↔ C：inf/nan 忽略 '0' 标志（`format("%010f", inf)` → 空格填充）
                (sign, Vec::new(), body, !nan && !inf)
            }
            _ => {
                v.raise_error_str("invalid format");
                return None;
            }
        };

        // 宽度与填充：'-' 优先于 '0'；'0' 被忽略时退化为空格
        let zeropad = sp.zero && !sp.minus && zero_ok;
        let total = sign.len() + prefix.len() + body.len();
        if total < sp.width {
            let pad = sp.width - total;
            if sp.minus {
                out.extend_from_slice(&sign);
                out.extend_from_slice(&prefix);
                out.extend_from_slice(&body);
                out.extend(std::iter::repeat(b' ').take(pad));
            } else if zeropad {
                out.extend_from_slice(&sign);
                out.extend_from_slice(&prefix);
                out.extend(std::iter::repeat(b'0').take(pad));
                out.extend_from_slice(&body);
            } else {
                out.extend(std::iter::repeat(b' ').take(pad));
                out.extend_from_slice(&sign);
                out.extend_from_slice(&prefix);
                out.extend_from_slice(&body);
            }
        } else {
            out.extend_from_slice(&sign);
            out.extend_from_slice(&prefix);
            out.extend_from_slice(&body);
        }
    }
    if push_result {
        let s = v.ss.intern(&out);
        v.push(s);
    } else {
        // printf：走 printfunc（↔ _printfunc(v, "%s", dest)，同样在 NUL 处截断）
        if let Some(pf) = v.ss.printfunc.borrow().clone() {
            pf(v, squirrel::baselib::c_string_prefix(&out));
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
        // ↔ _string_lstrip：`sq_pushstring(v, start, -1)` —— 负长度让 API 去 strlen(start)，
        // 于是结果在**第一个嵌入 NUL 处截断**。这不是 UB（串本身合法），是参考实现
        // 用 C 字符串处理 SQString 的确定行为，保真优先照做。
        Trim::Left => {
            let start = b.iter().position(|c| !is_ws(c)).unwrap_or(b.len());
            let stop = b[start..]
                .iter()
                .position(|&c| c == 0)
                .map(|x| start + x)
                .unwrap_or(b.len());
            (start, stop)
        }
        // ↔ _string_rstrip：`__strip_r` 按显式长度从尾部扫，不吃 NUL 截断
        Trim::Right => (0, b.iter().rposition(|c| !is_ws(c)).map(|x| x + 1).unwrap_or(0)),
        // ↔ _string_strip：`__strip_l` + `__strip_r(str, len)` 后按 `end - start` 推送；
        // `end < start` 只可能出现在"整串都是空白"（此时没有 NUL），退化为空串
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
    // ↔ _string_split：栈 [this, str, seps, skipempty?]
    let s = v.stack_get(2);
    let seps = v.stack_get(3);
    if seps.as_bytes().is_empty() {
        return throw_error(v, "empty separators string");
    }
    let mut skipempty = false;
    if v.top - v.stackbase > 3 {
        skipempty = !VmData::is_false(&v.stack_get(4));
    }
    let src = s.as_bytes();
    let seps = seps.as_bytes();
    let arr = squirrel::array::SQArray::create(&v.ss, 0);
    let mut start = 0usize;
    let mut end = 0usize;
    // ↔ `while(*end != '\0')`：参考实现按 C 字符串扫描，**遇到嵌入 NUL 就停**，
    // 尾段也用这个停下来的 end（于是 NUL 之后的内容整段丢掉）。确定的语义，照做。
    while end < src.len() && src[end] != 0 {
        let cur = src[end];
        if seps.contains(&cur) {
            if !skipempty || end != start {
                let s2 = v.ss.intern(&src[start..end]);
                arr.borrow_mut().append(&s2);
            }
            start = end + 1;
        }
        end += 1;
    }
    // ↔ if(end != start)：尾段为空时不追加（与 skipempty 无关）
    if end != start {
        let s2 = v.ss.intern(&src[start..end]);
        arr.borrow_mut().append(&s2);
    }
    v.push(Value::Array(arr));
    1
}

fn string_escape(v: &mut VmData) -> SQInteger {
    let s = v.stack_get(2);
    let src = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(src.len() * 4);
    for &c in src {
        // ↔ _string_escape：scisprint(c) || c == 0 走「具名转义」分支（控制字符因 isprint
        // 为假实际落到 \xNN），其余一律 sprintf("\\x%02x", c)
        if c == 0 {
            out.extend_from_slice(b"\\0");
        } else if (0x20..0x7f).contains(&c) {
            match c {
                b'\\' => out.extend_from_slice(b"\\\\"),
                b'"' => out.extend_from_slice(b"\\\""),
                b'\'' => out.extend_from_slice(b"\\'"),
                _ => out.push(c),
            }
        } else if c < 0x80 {
            out.extend_from_slice(format!("\\x{:02x}", c).as_bytes());
        } else {
            // SQChar 是有符号 char：%02x 收到负数 → 32 位补码的 8 位十六进制
            // （参考实现在小字符串上会越界写出不确定内容，这里取确定性的同一取值）
            out.extend_from_slice(format!("\\x{:08x}", (c as i8) as i32 as u32).as_bytes());
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

    // ↔ _DECL_GLOBALBLOB_FUNC(name,2,".n")：掩码首位 '.' 是 this 占位
    macro_rules! bfn {
        ($name:expr, $tm:expr, $f:expr) => {
            reg_global(&ss, &root, $name, Rc::new($f), 2, Some(concat!(".", $tm)));
        };
    }

    // ↔ init_streamclass：`stream` 基类先于 blob 的全局函数进 root
    // （file/blob 两个子类在各自的 register 里用缓存的类，不再重复构造）
    let (stream_cls, _file_cls, blob_cls) = crate::stream::make_stream_classes(vm);
    crate::stream::register_class(vm, "stream", &stream_cls);

    bfn!("casti2f", "n", |v: &mut VmData| {
        let i = v.stack_get(2).as_integer();
        v.push(Value::Float(f32::from_bits(i as u32)));
        1
    });
    bfn!("castf2i", "n", |v: &mut VmData| {
        // ↔ *(const SQInteger *)&f：参考实现按位重解释 f32 的 4 字节（高位读的是相邻栈内容，
        // 属 UB）；这里取确定性的低 32 位（即 f32 的位模式）
        let f = v.stack_get(2).as_float();
        v.push(Value::Integer(f.to_bits() as SQInteger));
        1
    });
    bfn!("swap2", "n", |v: &mut VmData| {
        let i = v.stack_get(2).as_integer();
        v.push(Value::Integer(((i & 0xFF) << 8) | ((i >> 8) & 0xFF)));
        1
    });
    bfn!("swap4", "n", |v: &mut VmData| {
        // ↔ unsigned int t4 = (unsigned int)i; __swap_dword(&t4)：只交换低 32 位
        let i = v.stack_get(2).as_integer();
        v.push(Value::Integer((i as u32).swap_bytes() as SQInteger));
        1
    });
    bfn!("swapfloat", "n", |v: &mut VmData| {
        let f = v.stack_get(2).as_float();
        v.push(Value::Float(f32::from_bits(f.to_bits().swap_bytes())));
        1
    });

    // ↔ declare_stream 末尾：类名插到 root（在 globals 之后）
    crate::stream::register_class(vm, "blob", &blob_cls);
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
                ef(v, msg.as_bytes());
            }
        });
        *vm.ss.compilererrorhandler.borrow_mut() = Some(handler);
    }
    // 运行时错误钩子 ↔ _sqstd_aux_printerror
    let mut v = vm.data_mut();
    let f: squirrel::closure::NativeFn = Rc::new(|v: &mut VmData| {
        let ss_ = v.ss.clone();
        let ef_ = ss_.errorfunc.borrow().clone();
        if let Some(ef) = ef_ {
            if v.top - v.stackbase >= 2 {
                // ↔ sq_getstring(v,2)：只有真字符串才内联，其余一律 [unknown]
                let e = v.stack_get(2);
                let msg = match &e {
                    // ↔ sq_getstring：字符串按裸字节内联（非 UNICODE 构建，不做 UTF-8 校验）
                    Value::String(s) => squirrel::baselib::c_string_prefix(&s.val).to_vec(),
                    _ => b"unknown".to_vec(),
                };
                let mut line = b"\nAN ERROR HAS OCCURRED [".to_vec();
                line.extend_from_slice(&msg);
                line.extend_from_slice(b"]\n");
                ef(v, &line);
            }
            print_callstack(v);
        }
        0
    });
    let nc = squirrel::closure::SQNativeClosure::create(&vm.ss, f, 0);
    v.errorhandler = Value::NativeClosure(nc);
}

/// 调用栈 dump 的**帧数上限**（P3 加固，参考实现没有这个概念）。
///
/// 为什么需要：本实现给递归加了上限（`MAX_CALL_FRAMES = 20 万`），未捕获的
/// `stack overflow` 会把**整条**调用栈逐帧打印出来 —— 实测脚本三行就能产出
/// **12.3MB stderr**。参考实现到不了那么深（先 OOM），所以这不是保真问题，
/// 而是"别让错误输出自己变成事故"。
///
/// 上限只影响超过 `MAX_DUMP_FRAMES` 的**病态深栈**；正常脚本
/// （`tests-corpus/t_errtail.nut` 那种 2~3 层）逐字节与参考实现一致。
const MAX_DUMP_FRAMES: SQInteger = 1000;

/// ↔ sqstd_printcallstack：CALLSTACK（level 从 1 起，跳过本函数自身）
/// + LOCALS（level 0..9 的 sq_getlocal 循环，按类型打印）
fn print_callstack(v: &mut VmData) {
    let ss_ = v.ss.clone();
    let ef_ = ss_.errorfunc.borrow().clone();
    if let Some(ef) = ef_ {
        ef(v, b"\nCALLSTACK\n");
        let mut level: SQInteger = 1; // 1 is to skip this function that is level 0
        let mut omitted: SQInteger = 0;
        loop {
            // ↔ sq_stackinfos(v, level, &si)
            let cssize = v.callsstack.len() as SQInteger;
            if !(level >= 0 && cssize > level) {
                break;
            }
            if level > MAX_DUMP_FRAMES {
                // 剩下的帧只报数量（见 MAX_DUMP_FRAMES 的注释）
                omitted = cssize - level;
                break;
            }
            let idx = (cssize - level - 1) as usize;
            let (fname, src, line) = match &v.callsstack[idx].closure {
                Value::Closure(c) => {
                    let proto = c.borrow().function.clone();
                    (
                        string_or(&proto.name, "unknown"),
                        string_or(&proto.sourcename, "unknown"),
                        proto.get_line(v.callsstack[idx].ip as SQInteger),
                    )
                }
                Value::NativeClosure(nc) => {
                    let n = nc.borrow().name.clone();
                    (string_or(&n, "unknown"), "NATIVE".to_string(), -1)
                }
                _ => ("unknown".to_string(), "unknown".to_string(), 0),
            };
            let line_txt = format!("*FUNCTION [{}()] {} line [{}]\n", fname, src, line);
            ef(v, line_txt.as_bytes());
            level += 1;
        }
        if omitted > 0 {
            let txt = format!("…省略 {} 层（本实现调用栈上限 20 万帧；参考实现到不了这么深）\n", omitted);
            ef(v, txt.as_bytes());
        }
        ef(v, b"\nLOCALS\n");
        for level in 0..10 {
            let mut seq: SQInteger = 0;
            while let Some(name) = squirrel::baselib::get_local_at(v, level, seq) {
                seq += 1;
                let line_txt = format_local_line(v, &name);
                ef(v, &line_txt);
                v.pop_get(); // ↔ sq_pop(v,1)
            }
        }
    }
}

/// ↔ sqstd_printcallstack 的 LOCALS 单项（类型化打印）
///
/// 返回裸字节：参考实现是非 UNICODE 构建，`pf("%s", ...)` 原样写出字符串内容，
/// 非 UTF-8 字节不能被 U+FFFD 替换掉。
fn format_local_line(v: &VmData, name: &str) -> Vec<u8> {
    let val = v.top_val();
    match &val {
        Value::Null => format!("[{}] NULL\n", name).into_bytes(),
        // ↔ pf("%d", SQInteger)：x64 varargs 下 %d 只取低 32 位
        Value::Integer(i) => format!("[{}] {}\n", name, (*i) as i32).into_bytes(),
        Value::Float(f) => format!(
            "[{}] {}\n",
            name,
            squirrel::cformat::format_g_prec(*f as f64, 14)
        )
        .into_bytes(),
        Value::Bool(b) => format!("[{}] {}\n", name, if *b { "true" } else { "false" }).into_bytes(),
        Value::String(s) => {
            let mut b = format!("[{}] \"", name).into_bytes();
            b.extend_from_slice(squirrel::baselib::c_string_prefix(&s.val));
            b.extend_from_slice(b"\"\n");
            b
        }
        Value::UserPointer(_) => format!("[{}] USERPOINTER\n", name).into_bytes(),
        Value::Table(_) => format!("[{}] TABLE\n", name).into_bytes(),
        Value::Array(_) => format!("[{}] ARRAY\n", name).into_bytes(),
        Value::Closure(_) => format!("[{}] CLOSURE\n", name).into_bytes(),
        Value::NativeClosure(_) => format!("[{}] NATIVECLOSURE\n", name).into_bytes(),
        Value::Generator(_) => format!("[{}] GENERATOR\n", name).into_bytes(),
        Value::Userdata(_) => format!("[{}] USERDATA\n", name).into_bytes(),
        Value::Thread(_) => format!("[{}] THREAD\n", name).into_bytes(),
        Value::Class(_) => format!("[{}] CLASS\n", name).into_bytes(),
        Value::Instance(_) => format!("[{}] INSTANCE\n", name).into_bytes(),
        Value::WeakRef(_) => format!("[{}] WEAKREF\n", name).into_bytes(),
        other => format!(
            "[{}] {}\n",
            name,
            squirrel::object::get_type_name(other).unwrap_or("unknown")
        )
        .into_bytes(),
    }
}

fn string_or(v: &Value, default: &str) -> String {
    match v {
        Value::String(s) => String::from_utf8_lossy(&s.val).into_owned(),
        _ => default.to_string(),
    }
}

/// ↔ sqstd_dofile：加载脚本文件并以 root table 为 this 执行（宿主嵌入用）
///
/// `printerror` 即 C++ 的 raiseerror：为真时编译错误走 compilererrorhandler。
pub fn run_file(vm: &squirrel::api::Vm, path: &str, printerror: bool) -> Result<Value, String> {
    let clo = {
        let mut v = vm.data_mut();
        match sqstd_load_file(&mut v, path, printerror) {
            Ok(()) => v.pop_get(),
            Err(()) => {
                let e = v.lasterror.clone();
                return Err(squirrel::api::value_to_error_string(&e));
            }
        }
    };
    let this = vm.data().roottable.clone();
    vm.call(&clo, this, &[], true)
        .map_err(|e| squirrel::api::value_to_error_string(&e))
}

/// 注册全部标准库（sq.c 的调用顺序）
pub fn register_all(vm: &mut VmData) {
    register_bloblib(vm);
    register_iolib(vm);
    register_systemlib(vm);
    register_mathlib(vm);
    register_stringlib(vm);
}

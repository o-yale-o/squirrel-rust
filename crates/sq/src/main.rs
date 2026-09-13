//! ↔ sq/sq.c —— 独立解释器（脚本执行 + REPL）。

use std::cell::RefCell;
use std::io::{Read, Write};
use std::rc::Rc;

use squirrel::config::*;
use squirrel::object::Value;

fn print_version_infos() {
    println!(
        "{} {} ({} bits)",
        SQUIRREL_VERSION,
        SQUIRREL_COPYRIGHT,
        std::mem::size_of::<SQInteger>() * 8
    );
}

fn print_usage() {
    eprint!(
        "usage: sq <options> <scriptpath [args]>.\n\
Available options are:\n   \
-c              compiles the file to bytecode(default output 'out.cnut')\n   \
-o              specifies output file for the -c option\n   \
-c              compiles only\n   \
-d              generates debug infos\n   \
-v              displays version infos\n   \
-h              prints help\n\
(下面两个是本实现**额外**提供的加固选项，参考实现没有)\n   \
--max-ops N     指令预算：超出报可捕获的 execution budget exceeded\n   \
--mem-limit N   进程内存上限（MB，Windows Job Object）：超出由系统终止进程\n"
    );
}

/// ↔ Interactive（REPL）
fn interactive(vm: &squirrel::api::Vm) {
    const MAXINPUT: usize = 1024;
    let mut buffer = [0u8; MAXINPUT];
    let mut blocks: SQInteger = 0;
    let mut string = false;
    let mut retval: SQInteger = 0;

    print_version_infos();

    // 注册 quit()（↔ sq.c：native 闭包 + 1 个 outer userpointer 指向 done 标志）
    let done = Rc::new(RefCell::new(false));
    {
        let v = vm.data_mut();
        let done2 = done.clone();
        let quit = move |_: &mut squirrel::vm::VmData| -> SQInteger {
            *done2.borrow_mut() = true;
            0
        };
        let nc = squirrel::closure::SQNativeClosure::create(&vm.ss, Rc::new(quit), 1);
        nc.borrow_mut().nparamscheck = 1;
        nc.borrow_mut().outervalues[0] = Value::UserPointer(0); // ↔ sq_pushuserpointer(&done)
        let key = vm.ss.intern(b"quit");
        if let Value::Table(root) = &v.roottable {
            root.borrow_mut().new_slot(&key, &Value::NativeClosure(nc));
        }
    }

    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    'outer: loop {
        let mut i = 0usize;
        print!("\nsq>");
        let _ = std::io::stdout().flush();
        loop {
            let mut cbuf = [0u8; 1];
            if reader.read_exact(&mut cbuf).is_err() {
                return;
            }
            let c = cbuf[0];
            if c == b'\n' {
                if i > 0 && buffer[i - 1] == b'\\' {
                    buffer[i - 1] = b'\n';
                } else if blocks == 0 {
                    break;
                }
                buffer[i] = b'\n';
                i += 1;
            } else if c == b'}' {
                blocks -= 1;
                buffer[i] = c;
                i += 1;
            } else if c == b'{' && !string {
                blocks += 1;
                buffer[i] = c;
                i += 1;
            } else if c == b'"' || c == b'\'' {
                string = !string;
                buffer[i] = c;
                i += 1;
            } else if i >= MAXINPUT - 1 {
                eprintln!("sq : input line too long");
                break;
            } else {
                buffer[i] = c;
                i += 1;
            }
            if i >= MAXINPUT {
                break;
            }
        }

        let mut line: Vec<u8> = buffer[..i].to_vec();
        if line.first() == Some(&b'=') {
            line = [b"return (", &line[1..], b")"].concat();
            retval = 1;
        }
        if !line.is_empty() {
            {
                let mut v = vm.data_mut();
                let oldtop = v.top;
                match squirrel::compiler::compile(&mut v, Box::new({
                    let mut pos = 0usize;
                    let src = line.clone();
                    move || {
                        if pos < src.len() {
                            let c = src[pos];
                            pos += 1;
                            c as SQInteger
                        } else {
                            0
                        }
                    }
                }), b"interactive console", true, false)
                {
                    Ok(proto) => {
                        let fp = match &proto {
                            Value::FuncProto(f) => f.clone(),
                            _ => unreachable!(),
                        };
                        let root_weak = match &v.roottable {
                            Value::Table(t) => Some(Value::Table(t.clone()).get_weakref_as_weakref()),
                            _ => None,
                        };
                        let clo = squirrel::closure::SQClosure::create(&vm.ss, fp, root_weak);
                        let clo_v = Value::Closure(clo);
                        v.push(clo_v.clone());
                        let rt = v.roottable.clone();
                        v.push(rt);
                        let base = v.top - 1;
                        let mut out = Value::Null;
                        let r = squirrel::vm::execute(
                            &mut v,
                            &clo_v,
                            1,
                            base,
                            &mut out,
                            true,
                            squirrel::vm::ExecutionType::Call,
                        );
                        v.top = oldtop; // 恢复干净栈（对应 sq_settop）
                        if r.is_ok() && retval != 0 {
                            retval = 0;
                            println!();
                            // ↔ print(结果)
                            let print_key = vm.ss.intern(b"print");
                            let pf = match &v.roottable {
                                Value::Table(rt) => rt.borrow().get(&print_key),
                                _ => None,
                            };
                            if let Some(pf) = pf {
                                v.push(pf);
                                let rt2 = v.roottable.clone();
                                v.push(rt2);
                                v.push(out);
                                let pfn = v.get_up(-3);
                                let base2 = v.top - 2;
                                let _ = squirrel::vm::execute(
                                    &mut v,
                                    &pfn,
                                    2,
                                    base2,
                                    &mut Value::Null,
                                    true,
                                    squirrel::vm::ExecutionType::Call,
                                );
                                v.top = oldtop;
                            }
                            println!();
                        }
                    }
                    Err(()) => {}
                }
                v.top = oldtop;
            }
            // quit 检查（↔ sq.c 的 while(!done)）
            if *done.borrow() {
                break 'outer;
            }
        }
    }
}

/// ↔ sqstd_loadfile（嗅探 .cnut / UTF-8 BOM / UCS2）→ 编译 → 返回栈顶闭包。
/// Err(Some(msg)) = 可打印的 lasterror（C++ sq_getstring 成功）；
/// Err(None) = lasterror 不是字符串（C++ 侧什么都打印不出来）。
fn load_file(vm: &squirrel::api::Vm, path: &str) -> Result<Value, Option<String>> {
    let mut v = vm.data_mut();
    let top = v.top;
    match squirrel_stdlib::sqstd_load_file(&mut v, path, true) {
        Ok(()) => Ok(v.get_at(top)),
        Err(()) => match &v.lasterror {
            Value::String(s) => Err(Some(String::from_utf8_lossy(&s.val).into_owned())),
            _ => Err(None),
        },
    }
}

/// 解释器线程的栈大小。
///
/// Rust 可执行文件的**主线程**栈在 Windows 上默认只有 1MB，而 MSVC 编译的
/// `sq_static.exe` 在 4000 层嵌套正则时还能跑（8000 层才 `0xC00000FD`）。
/// 为了不让"栈比 C++ 版小"变成行为差异，脚本执行放到这个 16MB 栈的线程上，
/// 同时把正则的递归预算放大到 8MB（默认值仍是保守的 384KB，见 regex.rs）。
const VM_THREAD_STACK: usize = 16 * 1024 * 1024;
const REGEX_STACK_BUDGET: usize = 8 * 1024 * 1024;

fn main() {
    // 真正干活的部分跑在大栈线程里，行为和 C++ 版对齐
    let child = std::thread::Builder::new()
        .stack_size(VM_THREAD_STACK)
        .spawn(real_main);
    match child {
        Ok(h) => {
            let _ = h.join();
        }
        Err(_) => real_main(),
    }
}

/// 给**本进程**设内存上限（P3 加固，参考实现没有）：Windows 用 Job Object，
/// 超限时由系统终止进程而不是把整台机器拖进 swap。
///
/// 失败只警告不退出 —— 这只是加固，不该让本来能跑的脚本跑不了。
#[cfg(windows)]
fn apply_memory_limit(mb: usize) {
    use std::ffi::c_void;
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct JobObjectBasicLimitInformation {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct IoCounters {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct JobObjectExtendedLimitInformation {
        basic: JobObjectBasicLimitInformation,
        io_info: IoCounters,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }
    const JOB_OBJECT_LIMIT_PROCESS_MEMORY: u32 = 0x0000_0100;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: i32 = 9;
    extern "system" {
        fn CreateJobObjectW(attrs: *mut c_void, name: *const u16) -> *mut c_void;
        fn SetInformationJobObject(
            job: *mut c_void,
            class: i32,
            info: *mut c_void,
            len: u32,
        ) -> i32;
        fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
        fn GetCurrentProcess() -> *mut c_void;
    }
    unsafe {
        let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
        if job.is_null() {
            eprintln!("warning: --mem-limit: CreateJobObject failed");
            return;
        }
        let mut info = JobObjectExtendedLimitInformation::default();
        info.basic.limit_flags = JOB_OBJECT_LIMIT_PROCESS_MEMORY;
        info.process_memory_limit = mb.saturating_mul(1024 * 1024);
        if SetInformationJobObject(
            job,
            JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
            &mut info as *mut _ as *mut c_void,
            std::mem::size_of::<JobObjectExtendedLimitInformation>() as u32,
        ) == 0
        {
            eprintln!("warning: --mem-limit: SetInformationJobObject failed");
            return;
        }
        if AssignProcessToJobObject(job, GetCurrentProcess()) == 0 {
            eprintln!("warning: --mem-limit: AssignProcessToJobObject failed");
        }
    }
}

#[cfg(not(windows))]
fn apply_memory_limit(_mb: usize) {
    eprintln!("warning: --mem-limit is only implemented on Windows");
}

fn real_main() {
    squirrel_stdlib::regex::set_stack_budget(REGEX_STACK_BUDGET);
    // 编译器递归上限也放到与参考实现同级：它能编的（实测 500 层括号）我们都能编，
    // 它崩的（800 层 STATUS_STACK_OVERFLOW）我们给可捕获的 nesting too deep
    squirrel::compiler::set_compiler_limits(800, 8 * 1024 * 1024);
    let args: Vec<String> = std::env::args().collect();
    let mut retval: SQInteger = 0;

    let vm = squirrel::api::Vm::new(1024);
    {
        let out = std::io::stdout();
        let err = std::io::stderr();
        vm.set_print_funcs(
            Some(Rc::new(move |_v, s: &[u8]| {
                let mut o = out.lock();
                let _ = o.write_all(s);
                let _ = o.flush();
            })),
            Some(Rc::new(move |_v, s: &[u8]| {
                let mut o = err.lock();
                let _ = o.write_all(s);
                let _ = o.flush();
            })),
        );
    }

    // ↔ sq.c：注册标准库 + aux 错误处理器
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);

    // ↔ getargs
    let mut compiles_only = false;
    let mut output: Option<String> = None;
    let mut arg = 1usize;
    let mut interactive_mode = true;

    while arg < args.len() {
        // ---- P3 加固：本实现**额外**提供的两个长选项（参考实现没有）----
        // --max-ops N   ：指令预算，超了报可捕获的 execution budget exceeded
        // --mem-limit N ：进程内存上限（MB，Windows Job Object），超了由系统杀进程
        if args[arg] == "--max-ops" || args[arg] == "--mem-limit" {
            let is_ops = args[arg] == "--max-ops";
            let val = args.get(arg + 1).cloned().unwrap_or_default();
            let n: i64 = match val.parse() {
                Ok(n) if n > 0 => n,
                _ => {
                    eprintln!("{} expects a positive integer, got {:?}", args[arg], val);
                    std::process::exit(1);
                }
            };
            if is_ops {
                vm.data_mut().op_budget = n;
            } else {
                apply_memory_limit(n as usize);
            }
            arg += 2;
            continue;
        }
        if args[arg].starts_with('-') {
            // ↔ argv[arg][1]：单独一个 "-" 时参考实现读到的是字符串结尾的 NUL，
            // 于是打出 "unknown prameter '-<NUL>'"（CLI 矩阵抓到的，原来给的是空格）
            let flag = args[arg].chars().nth(1).unwrap_or('\0');
            match flag {
                'd' => vm.ss.debuginfo.set(true),
                'c' => compiles_only = true,
                'o' => {
                    arg += 1;
                    if arg < args.len() {
                        output = Some(args[arg].clone());
                    }
                }
                'v' => {
                    print_version_infos();
                    std::process::exit(0);
                }
                'h' => {
                    print_version_infos();
                    print_usage();
                    std::process::exit(0);
                }
                _ => {
                    print_version_infos();
                    println!("unknown prameter '-{}'", flag);
                    print_usage();
                    // ↔ sq.c: `*retval = -1; return _ERROR;`（retval 是返回值 → 进程退出码 -1）
                    std::process::exit(-1);
                }
            }
            arg += 1;
        } else {
            break;
        }
    }

    if arg < args.len() {
        interactive_mode = false;
        let filename = args[arg].clone();
        arg += 1;
        match load_file(&vm, &filename) {
            Ok(_) => {
                if compiles_only {
                    // ↔ sqstd_loadfile + sqstd_writeclosuretofile
                    let outfile = output.clone().unwrap_or_else(|| "out.cnut".to_string());
                    let mut v = vm.data_mut();
                    let top = v.top;
                    let clo = v.get_at(top - 1);
                    if let Value::Closure(c) = &clo {
                        // ↔ sqstd_writeclosuretofile：先 fopen("wb+") 再写；任何失败都只把
                        // 错误放进 lasterror，由外层统一打 `Error [%s]`（**stdout**）并 retval = -2。
                        // 原来这里是 eprintln 到 stderr + exit(1)，与参考实现三处都不同
                        // （流、文本、退出码），CLI 矩阵抓到的。
                        let failed = match squirrel::serialization::write_closure(&mut v, c) {
                            Ok(bytes) => std::fs::File::create(&outfile)
                                .and_then(|mut f| std::io::Write::write_all(&mut f, &bytes))
                                .is_err(),
                            Err(_) => true,
                        };
                        if failed {
                            let err = v.lasterror.clone();
                            let msg = String::from_utf8_lossy(err.as_bytes()).into_owned();
                            let msg = if msg.is_empty() {
                                "cannot open the file".to_string()
                            } else {
                                msg
                            };
                            println!("Error [{}]", msg);
                            std::process::exit(-2);
                        }
                    }
                    std::process::exit(0);
                } else {
                    let mut callargs = 1;
                    {
                        let mut v = vm.data_mut();
                        let rt = v.roottable.clone();
                        v.push(rt);
                    }
                    for i in arg..args.len() {
                        let s = vm.ss.intern(args[i].as_bytes());
                        vm.data_mut().push(s);
                        callargs += 1;
                    }
                    match vm.call_top(callargs, true) {
                        Ok(r) => {
                            if let Value::Integer(n) = &r {
                                retval = *n;
                            }
                        }
                        Err(_) => {
                            // ↔ C++ getargs：运行失败仅返回 _ERROR，retval 不变（0）
                        }
                    }
                }
            }
            Err(e) => {
                // ↔ sq_getlasterror + scprintf("Error [%s]\n")，仅字符串可打印
                if let Some(msg) = e {
                    println!("Error [{}]", msg);
                }
                retval = -2;
            }
        }
    }

    if interactive_mode {
        interactive(&vm);
    }

    std::process::exit(retval as i32);
}

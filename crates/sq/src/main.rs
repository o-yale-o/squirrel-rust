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
-h              prints help\n"
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

    // 注册 quit()
    {
        let v = vm.data_mut();
        let done = Rc::new(RefCell::new(false));
        let done2 = done.clone();
        let quit = move |_: &mut squirrel::vm::VmData| -> SQInteger {
            *done2.borrow_mut() = true;
            0
        };
        let nc = squirrel::closure::SQNativeClosure::create(&vm.ss, Rc::new(quit), 1);
        nc.borrow_mut().nparamscheck = 1;
        nc.borrow_mut().outervalues[0] = Value::UserPointer(0); // 占位（done 标志经闭包捕获）
        let key = vm.ss.intern(b"quit");
        if let Value::Table(root) = &v.roottable {
            root.borrow_mut().new_slot(&key, &Value::NativeClosure(nc));
        }
        let _ = done;
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
            // quit 检查
            if let Value::Table(root) = &vm.data().roottable {
                if let Some(Value::NativeClosure(q)) = root.borrow().get(&vm.ss.intern(b"quit")) {
                    let quit_flag = q.borrow().outervalues[0].as_integer();
                    if quit_flag != 0 {
                        break 'outer;
                    }
                }
            }
        }
    }
}

/// ↔ sqstd_loadfile（暂不支持 .cnut/UCS2/UTF-8-BOM 嗅探：按普通字节流编译）
fn load_file(vm: &squirrel::api::Vm, path: &str) -> Result<Value, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("cannot open {}", e))?;
    let mut src = Vec::new();
    file.read_to_end(&mut src).map_err(|e| format!("cannot read {}", e))?;
    // ↔ 嗅探：0xFAFA → .cnut 字节码；否则按源码编译
    if src.len() >= 2 && src[0] == 0xFA && src[1] == 0xFA {
        let mut v = vm.data_mut();
        match squirrel::serialization::read_closure(&mut v, &src) {
            Ok(clo) => {
                v.push(clo.clone());
                Ok(clo)
            }
            Err(e) => Err(format!("{}", e)),
        }
    } else {
        vm.compile_buffer(&src, path.as_bytes(), true)
            .map_err(|_| "compile error".to_string())
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut retval: SQInteger = 0;

    let vm = squirrel::api::Vm::new(1024);
    {
        let out = std::io::stdout();
        let err = std::io::stderr();
        vm.set_print_funcs(
            Some(Rc::new(move |_v, s| {
                let mut o = out.lock();
                let _ = o.write_all(s.as_bytes());
                let _ = o.flush();
            })),
            Some(Rc::new(move |_v, s| {
                let mut o = err.lock();
                let _ = o.write_all(s.as_bytes());
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
        if args[arg].starts_with('-') {
            let flag = args[arg].chars().nth(1).unwrap_or(' ');
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
                    std::process::exit(1);
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
                        match squirrel::serialization::write_closure(&mut v, c) {
                            Ok(bytes) => {
                                if std::fs::write(&outfile, bytes).is_err() {
                                    eprintln!("cannot write {}", outfile);
                                    std::process::exit(1);
                                }
                            }
                            Err(e) => {
                                eprintln!("Error [{}]", e);
                                std::process::exit(1);
                            }
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
                println!("Error [{}]", e);
                retval = -2;
            }
        }
    }

    if interactive_mode {
        interactive(&vm);
    }

    std::process::exit(retval as i32);
}

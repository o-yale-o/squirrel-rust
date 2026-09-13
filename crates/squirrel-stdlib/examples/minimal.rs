//! 原版 `etc/minimal.c` 的 Rust 移植 —— 最小宿主嵌入示例。
//!
//! 对照关系：
//! ```text
//! sq_open(1024)                     -> Vm::new(1024)
//! sq_setprintfunc(printfunc, err)   -> vm.set_print_funcs(...)
//! sqstd_seterrorhandlers(v)         -> squirrel_stdlib::set_error_handlers(&vm)
//! sq_pushroottable + dofile         -> squirrel_stdlib::run_file(&vm, "test.nut", true)
//! sq_pushroottable + sq_get("foo")  -> vm.get_global("foo")
//! sq_call(v, 4, SQFalse, SQTrue)    -> vm.call(&foo, roottable, &[n, f, s], true)
//! ```
//!
//! 运行：`cargo run --example minimal`（工作目录需能读到 `examples/minimal_test.nut`）

use std::rc::Rc;

use squirrel::api::Vm;
use squirrel::object::Value;

/// ↔ minimal.c 的 `call_foo`：从 root table 取 `foo` 并以 3 个参数调用
fn call_foo(vm: &Vm, n: i64, f: f32, s: &str) {
    let foo = match vm.get_global("foo") {
        Some(v) => v,
        None => {
            eprintln!("foo() not found");
            return;
        }
    };
    let this = vm.data().roottable.clone();
    let args = vec![
        Value::Integer(n),
        Value::Float(f),
        {
            let v = vm.data();
            v.ss.intern(s.as_bytes())
        },
    ];
    // ↔ sq_call(v, 4, SQFalse, SQTrue)
    match vm.call(&foo, this, &args, true) {
        Ok(_) => {}
        Err(e) => eprintln!("call failed: {}", squirrel::api::value_to_error_string(&e)),
    }
}

fn main() {
    // ↔ sq_open(1024)
    let vm = Vm::new(1024);

    // ↔ sq_setprintfunc(v, printfunc, errorfunc)
    vm.set_print_funcs(
        Some(Rc::new(|_v, s| {
            use std::io::Write;
            let mut o = std::io::stdout();
            let _ = o.write_all(s.as_bytes());
            let _ = o.flush();
        })),
        Some(Rc::new(|_v, s| {
            use std::io::Write;
            let mut o = std::io::stderr();
            let _ = o.write_all(s.as_bytes());
            let _ = o.flush();
        })),
    );

    // ↔ sqstd_seterrorhandlers(v)：注册默认错误处理器（编译错误 + 运行时调用栈）
    // 说明：原版 minimal.c 里标准库注册是注释掉的；脚本只用 base 库的 print，
    // 所以这里也只装错误处理器，与之一致。需要 string/math/io 时调用
    // squirrel_stdlib::register_all(&mut vm.data_mut())。
    squirrel_stdlib::set_error_handlers(&vm);

    // ↔ sq_pushroottable + sqstd_dofile("test.nut", SQFalse, SQTrue)
    match squirrel_stdlib::run_file(&vm, "crates/squirrel-stdlib/examples/minimal_test.nut", true) {
        Ok(_) => call_foo(&vm, 1, 2.5, "teststring"),
        Err(e) => eprintln!("dofile failed: {}", e),
    }
}

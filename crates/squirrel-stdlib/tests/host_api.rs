//! 宿主 API 验收（计划书验收标准 #5）：用 Rust 宿主 API 复刻 `etc/minimal.c` 的嵌入流程。
//!
//! 覆盖：注册 native 函数、从宿主调用脚本函数（传参 + 取返回值）、读写全局、
//! 脚本回宿主的调用、错误路径（宿主拿到 Err 且错误处理器已打印调用栈）。

use std::cell::RefCell;
use std::rc::Rc;

use squirrel::api::Vm;
use squirrel::object::Value;
use squirrel::config::SQInteger;

/// 收集 print / error 输出的宿主
#[derive(Default)]
struct Sink {
    out: RefCell<String>,
    err: RefCell<String>,
}

fn make_vm() -> (Vm, Rc<Sink>) {
    let vm = Vm::new(1024);
    let sink = Rc::new(Sink::default());
    {
        let o = sink.clone();
        let e = sink.clone();
        vm.set_print_funcs(
            Some(Rc::new(move |_v, s| o.out.borrow_mut().push_str(s))),
            Some(Rc::new(move |_v, s| e.err.borrow_mut().push_str(s))),
        );
    }
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    (vm, sink)
}

fn intern(vm: &Vm, s: &str) -> Value {
    let v = vm.data();
    v.ss.intern(s.as_bytes())
}

/// ↔ minimal.c：注册原生函数 → 脚本调用它 → 宿主读回结果
#[test]
fn register_native_and_call_script_function() {
    let (vm, sink) = make_vm();

    // ↔ sq_newclosure + sq_setparamscheck(2, ".nn")
    vm.register_native(
        "host_add",
        Rc::new(|v: &mut squirrel::vm::VmData| -> SQInteger {
            let a = v.stack_get(2).as_integer();
            let b = v.stack_get(3).as_integer();
            v.push(Value::Integer(a + b));
            1
        }),
        3,
        Some(".nn"),
    );

    // 脚本调用宿主注册的函数（方向：脚本 → 宿主）
    vm.run_buffer(
        b"host_result <- host_add(20, 22); function echo(x) { return x * 2; }",
        b"host.nut",
    )
    .expect("script failed");
    assert_eq!(
        vm.get_global("host_result").map(|v| v.as_integer()),
        Some(42),
        "脚本应通过宿主原生函数得到 42"
    );

    // 宿主调用脚本函数并取返回值（方向：宿主 → 脚本，↔ minimal.c 的 call_foo）
    let echo = vm.get_global("echo").expect("echo not found");
    let this = vm.data().roottable.clone();
    let r = vm
        .call(&echo, this, &[Value::Integer(21)], true)
        .expect("call failed");
    assert_eq!(r.as_integer(), 42);

    // 浮点/字符串参数与返回值
    vm.run_buffer(
        b"function join(i, f, s) { return s + \":\" + i + \":\" + f; }",
        b"host.nut",
    )
    .unwrap();
    let join = vm.get_global("join").unwrap();
    let this = vm.data().roottable.clone();
    let r = vm
        .call(
            &join,
            this,
            &[Value::Integer(7), Value::Float(2.5), intern(&vm, "tag")],
            true,
        )
        .unwrap();
    assert_eq!(r.as_bytes(), b"tag:7:2.5");
    assert!(sink.err.borrow().is_empty(), "不应有错误输出");
}

/// ↔ minimal.c + test.nut：脚本里定义 foo()，宿主按名字取到并调用（打印到宿主 sink）
#[test]
fn dofile_then_call_foo_like_minimal_c() {
    let (vm, sink) = make_vm();
    // 与 examples/minimal_test.nut 相同的脚本内容（这里内联，避免依赖工作目录）
    vm.run_buffer(
        br#"function foo(i, f, s) { print("Called foo(), i=" + i + ", f=" + f + ", s='" + s + "'\n"); }"#,
        b"test.nut",
    )
    .expect("script failed");

    let foo = vm.get_global("foo").expect("foo not found");
    let this = vm.data().roottable.clone();
    vm.call(
        &foo,
        this,
        &[Value::Integer(1), Value::Float(2.5), intern(&vm, "teststring")],
        true,
    )
    .expect("call failed");

    assert_eq!(
        sink.out.borrow().as_str(),
        "Called foo(), i=1, f=2.5, s='teststring'\n"
    );
}

/// 宿主方向的错误路径：脚本抛错 → 宿主拿到 Err，且默认错误处理器已输出调用栈
#[test]
fn host_sees_errors_and_error_handler_prints() {
    let (vm, sink) = make_vm();
    vm.run_buffer(b"function boom() { throw \"host-visible\"; }", b"err.nut")
        .unwrap();
    let boom = vm.get_global("boom").unwrap();
    let this = vm.data().roottable.clone();
    let e = vm.call(&boom, this, &[], true).expect_err("should fail");
    assert_eq!(squirrel::api::value_to_error_string(&e), "host-visible");
    let err = sink.err.borrow().clone();
    assert!(
        err.contains("AN ERROR HAS OCCURRED [host-visible]") && err.contains("CALLSTACK"),
        "错误处理器应打印调用栈，实际：{}",
        err
    );

    // 编译错误也要能被宿主看到（raiseerror=true 时同时走 compilererrorhandler）
    let r = vm.compile_buffer(b"function (", b"bad.nut", true);
    assert!(r.is_err(), "坏脚本应编译失败");
    assert!(
        sink.err.borrow().contains("bad.nut line = (1)"),
        "编译错误应报到宿主 error 钩子：{}",
        sink.err.borrow()
    );
}

/// 全局符号读写、缺失符号返回 None、以及宿主调用类构造
#[test]
fn globals_and_class_construction_from_host() {
    let (vm, _sink) = make_vm();
    vm.set_global("host_const", Value::Integer(99));
    assert_eq!(
        vm.get_global("host_const").map(|v| v.as_integer()),
        Some(99)
    );
    assert!(vm.get_global("no_such_global").is_none());

    vm.run_buffer(
        b"class Point { x = 0; y = 0; constructor(a, b) { x = a; y = b; } function sum() { return x + y; } }",
        b"cls.nut",
    )
    .unwrap();
    let cls = vm.get_global("Point").unwrap();
    let this = vm.data().roottable.clone();
    let inst = vm
        .call(&cls, this, &[Value::Integer(3), Value::Integer(4)], true)
        .expect("constructor failed");
    assert!(matches!(inst, Value::Instance(_)), "类调用应返回实例");
    // 实例上调用方法，验证宿主拿到的确实是可用对象
    let sum = match &inst {
        Value::Instance(i) => i.borrow().get(&intern(&vm, "sum")),
        other => panic!("expected instance, got {:?}", other),
    }
    .expect("sum not found");
    let r = vm.call(&sum, inst.clone(), &[], true).unwrap();
    assert_eq!(r.as_integer(), 7);
}

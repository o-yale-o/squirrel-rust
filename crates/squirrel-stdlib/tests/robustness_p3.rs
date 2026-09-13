//! 健壮性加固（P3）回归。
//!
//! 这一档都是**参考实现没有**的加固（C++ 版要么直接 OOM、要么把错误输出自己变成事故），
//! 所以断言的是"本实现必须有的性质"，而不是与参考实现逐字对比。
//!
//! 已覆盖：
//! * **未捕获 stack overflow 的调用栈 dump 有帧数上限**：以前三行脚本就能产出
//!   12.3MB stderr（20 万帧全打印），现在 0.07MB —— 超过 1000 层只报"省略 N 层"。

use std::cell::RefCell;
use std::rc::Rc;

use squirrel::api::Vm;

fn run_capture(src: &str) -> (String, String) {
    let vm = Vm::new(1024);
    let out: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    let err: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let o = out.clone();
        let e = err.clone();
        vm.set_print_funcs(
            Some(Rc::new(move |_v, b: &[u8]| o.borrow_mut().extend_from_slice(b))),
            Some(Rc::new(move |_v, b: &[u8]| e.borrow_mut().extend_from_slice(b))),
        );
    }
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    let _ = vm.run_buffer(src.as_bytes(), b"robust.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    (o, e)
}

/// 深递归的未捕获错误：dump 有上限、且明说省略了多少层
#[test]
fn stack_overflow_dump_is_bounded() {
    let src = r#"
        function rec(n) {
            local pad1 = n, pad2 = n + 1, pad3 = n + 2;
            local x = rec(n + 1);
            return x;
        }
        rec(0);
    "#;
    let (_out, err) = run_capture(src);
    // 参考实现会一直涨到 OOM；本实现给可捕获错误，并且把 dump 限制在 1000 帧
    assert!(err.contains("stack overflow"), "应当报可捕获的 stack overflow：{}", &err[..err.len().min(200)]);
    let frames = err.matches("*FUNCTION").count();
    assert!(frames <= 1001, "dump 的帧数应当有上限，实际 {}", frames);
    assert!(
        err.contains("省略"),
        "超过上限时应当明确说明省略了多少层（否则看不出栈被截断）"
    );
    // 体量必须是有界的（以前 12MB+）
    assert!(err.len() < 512 * 1024, "stderr 体量应当有界，实际 {} 字节", err.len());
}

/// 浅栈的 dump 仍然逐帧完整（不因为加了上限就少打）
///
/// 注意 `outer` 里要写成 `local r = inner(1, 2); return r;`：
/// 直接 `return inner(1, 2);` 是**尾调用**，`outer` 的帧会被复用掉，
/// 调用栈里就不会有它（参考实现同样如此）。
#[test]
fn shallow_error_dump_is_complete() {
    let src = r#"
        function inner(a, b) { local c = a + b; return nope_global(c); }
        function outer() { local t = { x = 1 }; local r = inner(1, 2); return r; }
        outer();
    "#;
    let (_out, err) = run_capture(src);
    assert!(err.contains("AN ERROR HAS OCCURRED"), "err={}", err);
    assert!(err.contains("*FUNCTION [inner()]"), "err={}", err);
    assert!(err.contains("*FUNCTION [outer()]"), "err={}", err);
    assert!(err.contains("*FUNCTION [main()]"), "err={}", err);
    assert!(!err.contains("省略"), "浅栈不该出现省略行：{}", err);
    // LOCALS 段也要在
    assert!(err.contains("LOCALS"), "err={}", err);
}

/// 指令预算（`--max-ops`，P3 加固）：死循环脚本必须能被封顶
#[test]
fn op_budget_stops_runaway_scripts() {
    // 尾调用死循环：没有预算时永远不返回（参考实现就是这样挂住的）
    let src = "function loop(n) { return loop(n + 1); }\nloop(0);\n";
    let vm = Vm::new(1024);
    let err: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let e = err.clone();
        vm.set_print_funcs(
            None,
            Some(Rc::new(move |_v, b: &[u8]| e.borrow_mut().extend_from_slice(b))),
        );
    }
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    vm.data_mut().op_budget = 200_000; // 约 20 万条指令
    let r = vm.run_buffer(src.as_bytes(), b"loop.nut");
    assert!(r.is_err(), "预算用尽应当让脚本失败：{:?}", r);
    let msg = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(
        msg.contains("execution budget exceeded"),
        "错误消息应当说明预算用尽：{}",
        msg
    );
}

/// 编译器嵌套上限（P3-4）：深嵌套源码给**可捕获的编译错误**，不是把栈打爆
///
/// 参考实现实测：790 层括号能编、795 层 `STATUS_STACK_OVERFLOW` 崩溃。
/// 库里的默认上限保守（200 层，1MB 栈的调用方也安全），CLI 会调到 800。
#[test]
fn deep_nesting_is_a_catchable_compile_error() {
    // 默认上限内：能编
    let ok_src = format!("local a = {}1{};\n", "(".repeat(150), ")".repeat(150));
    let vm = Vm::new(1024);
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    assert!(
        vm.run_buffer(ok_src.as_bytes(), b"ok.nut").is_ok(),
        "150 层嵌套应当在默认上限内"
    );

    // 远超上限：可捕获的编译错误（不是 abort）
    let deep_src = format!("local a = {}1{};\n", "(".repeat(5000), ")".repeat(5000));
    let vm2 = Vm::new(1024);
    squirrel_stdlib::register_all(&mut vm2.data_mut());
    squirrel_stdlib::set_error_handlers(&vm2);
    let r = vm2.run_buffer(deep_src.as_bytes(), b"deep.nut");
    let e = r.err().unwrap_or_default();
    assert!(e.contains("nesting too deep"), "应当是 nesting too deep，实际 {:?}", e);

    // 放大上限之后（CLI 的做法）同样能编到参考实现的水平
    squirrel::compiler::set_compiler_limits(800, 8 * 1024 * 1024);
    let vm3 = Vm::new(1024);
    squirrel_stdlib::register_all(&mut vm3.data_mut());
    squirrel_stdlib::set_error_handlers(&vm3);
    let mid_src = format!("local a = {}1{};\n", "(".repeat(700), ")".repeat(700));
    assert!(
        vm3.run_buffer(mid_src.as_bytes(), b"mid.nut").is_ok(),
        "放大上限后 700 层应当能编（参考实测能编到 790）"
    );
    // 别把默认值留给别的测试
    squirrel::compiler::set_compiler_limits(
        squirrel::compiler::MAX_NESTING_DEPTH,
        squirrel::compiler::MAX_STACK_BUDGET,
    );
}

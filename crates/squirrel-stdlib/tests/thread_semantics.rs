//! 线程 / 生成器的语义回归（↔ sqbaselib.cpp 的 thread_call / thread_wakeup、
//! sqvm.cpp 的 EnterFrame/LeaveFrame/CloseOuters）。
//!
//! 这里固化的三条都是**实测与参考实现不一致**过的真实缺陷：
//!
//! 1. **跨 VM 的 open outer**：闭包捕获的局部变量（open outer）在 C++ 里是
//!    `SQOuter::_valptr` —— 直接指向**创建者 VM 的栈槽**。闭包若被另一个线程执行
//!    （`newthread(f)`），读到的仍是创建者那个槽。本实现曾经把 open outer 存成
//!    「宿主 VM + 槽位索引」，跨线程时去 `borrow()` 对方的 `RefCell`，而对方正在
//!    执行自己的代码（可变借用中）→ `RefCell already mutably borrowed` panic。
//!    现改为「栈槽与 outer 共享一个 `Rc<RefCell<Value>>` 单元」，任何 VM 都只碰单元。
//!
//! 2. **thread_call 的栈纪律**：↔ `sq_call` 的 `if(!v->_suspended) v->Pop(params)`
//!    加 `thread_call` 的 `sq_move(v,thread,-1); sq_pop(thread,1)`。正常返回要把本帧
//!    压入的 roottable/实参弹干净，否则线程栈每次调用都会长高一格，下一次
//!    `GetUp(-(nparams+1))` 读到的是 roottable（报 `attempt to call 'table'`）。
//!
//! 3. **失败路径同样要还原**：参考实现出错时 `sq_call` 也会 `Pop(params)`，
//!    所以「抛错的线程可以再 call 一次，并再次执行同一个闭包」。
//!
//! 期望值全部用参考实现（build-ref/bin/Release/sq_static.exe）实测确认过。

use std::cell::RefCell;
use std::rc::Rc;

use squirrel::api::Vm;

/// 跑脚本并收集 print 输出（错误走 stderr，一并返回）
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
    let rc = vm.run_buffer(src.as_bytes(), b"thread.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 跨 VM 的 outer：读、写、参数、外层的外层、帧已关闭后的快照
#[test]
fn open_outers_are_readable_and_writable_across_threads() {
    let src = r#"
        local v = "host";
        local t = newthread(function() { return v; });
        print(t.call() + "\n");

        local w = 0;
        local t2 = newthread(function() { w = 7; return 0; });
        t2.call();
        print(w + "\n");

        local h = function(p) { return newthread(function() { return p * 2; }); };
        print(h(21).call() + "\n");

        local a = "A";
        local h2 = function() { return function() { return a; }; };
        print(newthread(h2()).call() + "\n");

        local inner = function() { local q = "Q"; return function() { return q; }; };
        print(newthread(inner()).call() + "\n");

        local inner2 = newthread(function() {
            local z = 3;
            return newthread(function() { z = z + 4; return z; });
        });
        local t3 = inner2.call();
        print(t3.call() + "\n");

        local s = "init";
        local t4 = newthread(function() { suspend(1); return s; });
        t4.call();
        s = "changed";
        print(t4.wakeup() + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(out, "host\n7\n42\nA\nQ\n7\nchanged\n");
}

/// thread_call 的栈纪律：失败后再 call 仍应命中同一个闭包并再次抛错
#[test]
fn failed_thread_call_restores_thread_stack() {
    let src = r#"
        local t = newthread(function() { throw "boom"; });
        local out = "";
        for (local i = 0; i < 3; i++) {
            try { out += t.call(); } catch (e) { out += "E:" + e; }
            out += "/" + t.getstatus() + ";";
        }
        print(out + "\n");

        local u = newthread(function(a, b) { throw "a" + a + b; });
        local out2 = "";
        for (local i = 0; i < 2; i++) {
            try { out2 += u.call(1, 2); } catch (e) { out2 += "E:" + e; }
            out2 += ";";
        }
        print(out2 + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(out, "E:boom/idle;E:boom/idle;E:boom/idle;\nE:a12;E:a12;\n");
}

/// 正常返回（带实参 / 变长 / 多次）与挂起后的线程栈纪律
#[test]
fn thread_call_stack_matches_reference_for_args_and_suspend() {
    let src = r#"
        local t = newthread(function(a) { return "r" + a; });
        print(t.call(1) + "/" + t.call(2) + "/" + t.getstatus() + "\n");

        local va = newthread(function(...) { return vargv.len(); });
        print(va.call(1, 2, 3) + "/" + va.call() + "/" + va.call(9) + "\n");

        local s = newthread(function() { local v = suspend("first"); return "got:" + v; });
        local a = s.call();
        print(a + "/" + s.getstatus() + "\n");

        local e = newthread(function() { suspend(1); throw "e3"; });
        local out = "" + e.call();
        try { out += "/" + e.wakeup(); } catch (err) { out += "/E:" + err; }
        try { out += "/" + e.wakeup(); } catch (err) { out += "/E:" + err; }
        print(out + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(out, "r1/r2/idle\n3/0/1\nfirst/suspended\n1/E:e3/E:cannot wakeup a idle thread\n");
}

/// ↔ sq_wakeupvm：没有 wakeup 值时，suspend() 的返回值槽必须写 **null**
/// （曾经保留槽里的旧值，实测会拿到 `suspend` 那个 native 闭包）
#[test]
fn wakeup_without_value_delivers_null() {
    let src = r#"
        local t = newthread(function() { local v = suspend("first"); return typeof v + ":" + v; });
        print(t.call() + "\n");
        print(t.wakeup() + "/" + t.getstatus() + "\n");

        local u = newthread(function() { local v = suspend(1); return typeof v; });
        u.call();
        print(u.wakeup(5) + "\n");

        local c = newthread(function() { local a = suspend(1); local b = suspend(2); return "" + a + b; });
        local o = "" + c.call() + "/" + c.getstatus();
        o += "|" + c.wakeup("A") + "/" + c.getstatus();
        o += "|" + c.wakeup("B") + "/" + c.getstatus();
        print(o + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "first\nnull:null/idle\ninteger\n1/suspended|2/suspended|AB/idle\n"
    );
}

/// 生成器在线程里创建、跨 VM 读宿主局部；线程返回生成器后主线程继续 resume
#[test]
fn generators_across_threads_match_reference() {
    let src = r#"
        local v = 41;
        local t = newthread(function() {
            local g = (function() { yield v + 1; yield v + 2; })();
            return g;
        });
        local g = t.call();
        print(resume g + "/" + resume g + "/" + g.getstatus() + "\n");

        local n = 0;
        local t2 = newthread(function() {
            local g2 = (function() { yield 1; yield 2; })();
            n = resume g2;
            return resume g2;
        });
        print(t2.call() + "/" + n + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(out, "42/43/suspended\n2/1\n");
}

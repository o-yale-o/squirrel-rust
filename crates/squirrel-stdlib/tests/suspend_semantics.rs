//! `suspend` 穿透 native / metamethod 的错误路径回归（P1-8）。
//!
//! 语料侧是 `tests-corpus/t_suspmap.nut`（16 条 stdout + 47 行 stderr，两侧完全一致）
//! 与 `tests-corpus/t_suspend_main.nut`。本轮**没有发现差异**，但把几条容易踩的语义钉住了：
//!
//! * `suspend()` 只能在**线程顶层调用链**里用；穿过 native/元方法时报
//!   `cannot suspend through native calls/metamethods`（退出码/状态都对得上）；
//! * 在 **`_get` 元方法**里 suspend，参考实现先撞上"元方法里不能扩栈"的守卫，
//!   报的是 `stack overflow, cannot resize stack while in a metamethod`
//!   ——不是那句 canonical 的错误文本；
//! * 在**主脚本**里 `suspend`（或 resume 一个会 suspend 的生成器）会让主 VM 挂起、
//!   进程直接结束：两侧都是 rc=1、无输出（参考实现的 stdout 是块缓冲，之前的输出会丢；
//!   我们逐次 flush —— 所以这类用例必须单独一个文件，不能和其它用例混排）；
//! * `array.each` 在 3.2 里**不存在**（`the index 'each' does not exist`），
//!   要测 native 回调得用 `array.map`；
//! * 挂起失败的线程仍然是 `idle`，可以再次 `call()`。

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
    let rc = vm.run_buffer(src.as_bytes(), b"susp.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 穿过 native/元方法的 suspend 会被拒绝，且错误文本分两种
#[test]
fn suspend_through_native_and_metamethods_matches_reference() {
    let src = r#"
        function s(tag, f) { try { print(tag + "=" + f() + "\n"); } catch (e) { print(tag + "!" + e + "\n"); } }
        s("thread_ok", function() { local t = newthread(function() { suspend("x"); return "after"; }); return "" + t.call() + "/" + t.getstatus(); });
        s("call_meta", function() {
            local C = class { function _call(a) { suspend(1); return 1; } };
            local t = newthread(function() { local o = C(); return o(); });
            try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); }
        });
        s("native_cb", function() {
            local t = newthread(function() { local a = [1, 2]; a.map(function(x) { suspend(1); return x; }); return "no"; });
            try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); }
        });
        s("get_meta", function() {
            local C = class { function _get(k) { suspend(1); return 1; } };
            local t = newthread(function() { local o = C(); return o.anything; });
            try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); }
        });
        s("no_each", function() { local t = newthread(function() { local a = [1]; a.each(function(x) { }); return "no"; }); try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
        s("nested_fn_ok", function() { local t = newthread(function() { local inner = function() { return suspend("deep"); }; return "r:" + inner(); }); return "" + t.call() + "/" + t.getstatus(); });
        s("suspend_twice_seq", function() {
            local t = newthread(function() { local a = suspend(1); local b = suspend(2); return "" + a + b; });
            return "" + t.call() + "/" + t.wakeup("A") + "/" + t.wakeup("B") + "/" + t.getstatus();
        });
        s("status_after_fail", function() {
            local t = newthread(function() { local a = [1]; a.map(function(x) { suspend(1); return x; }); return "no"; });
            try { t.call(); } catch (e) { }
            local st = t.getstatus();
            try { return st + "/" + t.call(); } catch (e) { return st + "/E2:" + e; }
        });
        s("nested_threads", function() {
            local inner = newthread(function() { suspend("inner-suspend"); return "inner-done"; });
            local outer = newthread(function() { return "outer:" + inner.call(); });
            return "" + outer.call() + "/" + inner.getstatus() + "/" + outer.getstatus();
        });
        s("gen_in_thread", function() { local t = newthread(function() { local g = (function() { yield "y"; return "r"; })(); return "" + resume g + resume g; }); return t.call(); });
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "thread_ok=x/suspended\n\
         call_meta=E:cannot suspend through native calls/metamethods/idle\n\
         native_cb=E:cannot suspend through native calls/metamethods/idle\n\
         get_meta=E:stack overflow, cannot resize stack while in a metamethod/idle\n\
         no_each=E:the index 'each' does not exist/idle\n\
         nested_fn_ok=deep/suspended\n\
         suspend_twice_seq=1/2/AB/idle\n\
         status_after_fail=idle/E2:cannot suspend through native calls/metamethods\n\
         nested_threads=outer:inner-suspend/suspended/idle\n\
         gen_in_thread=yr\n"
    );
}

/// 主脚本里 `suspend` → 主 VM 挂起，**后面的语句不再执行**，`run_buffer` 正常返回
/// `suspend` 的参数（脚本"返回值"就是它，CLI 拿它当退出码 —— 与 sq.c 一致）。
#[test]
fn suspend_in_main_vm_stops_the_script() {
    let vm = Vm::new(1024);
    let out: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let o = out.clone();
        vm.set_print_funcs(
            Some(Rc::new(move |_v, b: &[u8]| o.borrow_mut().extend_from_slice(b))),
            None,
        );
    }
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    let r = vm.run_buffer(b"print(\"BEFORE\\n\");\nsuspend(1);\nprint(\"AFTER\\n\");", b"susp_main.nut");
    assert!(matches!(r, Ok(squirrel::object::Value::Integer(1))), "run_buffer: {:?}", r);
    assert_eq!(String::from_utf8_lossy(&out.borrow()), "BEFORE\n");
    assert!(vm.data().suspended, "主 VM 应当处于挂起态");
}

//! 多线程 × GC 交叉面回归（P1-9）。
//!
//! 语料侧是 `tests-corpus/t_gcx.nut`（18 条，两侧完全一致）。这一轮抓到**真 bug**：
//!
//! 1. **线程里调用 `collectgarbage()` 直接 panic**
//!    （`state.rs` 的 `RefCell already mutably borrowed`）：GC 去标记**宿 VM**，
//!    而宿 VM 正在执行 `thread_call`（它的 `RefCell` 被那层 `&mut VmData` 持有着）。
//! 2. 改成"用调用方根快照"之后又暴露第二个问题：**当前 VM 自己的根没进根集合**，
//!    线程栈上的对象被当成垃圾回收掉（`keep.v` → `the index 'v' does not exist`）。
//!
//! 参考实现的做法（`SQSharedState::RunMark`）是只从 `_root_vm` 出发，
//! 靠"线程对象会递归标记自己的栈"覆盖其它 VM；Rust 侧 borrow 不到正在执行的宿 VM，
//! 于是分成两步：**当前 VM 自己的根 + 宿 VM 的根快照**（`VmData::outer_roots`，
//! 由 `thread_call`/`thread_wakeup`/`thread_wakeupthrow` 在进入前放好、退出时清掉）。
//!
//! `run_mark` 里对 `Value::Thread` 一律用 `try_borrow`：真遇到正在执行的 VM 就跳过
//! （它的对象已由上面的两条路径覆盖），绝不 panic。

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
    let rc = vm.run_buffer(src.as_bytes(), b"gcx.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 线程里 `collectgarbage()`：保留线程自己栈上的对象、保留宿 VM 的对象、
/// 回收真正不可达的对象 —— 且**不能 panic**
#[test]
fn collectgarbage_from_thread_matches_reference() {
    let src = r#"
        function s(tag, f) { try { print(tag + "=" + f() + "\n"); } catch (e) { print(tag + "!" + e + "\n"); } }
        s("in_thread_keeps_own", function() {
            local t = newthread(function() {
                local keep = { v = 5 };
                local w = { q = 1 }.weakref();
                collectgarbage();
                return keep.v + "/" + ((w.ref() == null) ? "collected" : "alive");
            });
            return t.call();
        });
        s("in_thread_keeps_main", function() {
            local main_obj = { v = 3 };
            local w = main_obj.weakref();
            local t = newthread(function() { collectgarbage(); return "ran"; });
            return t.call() + "/" + ((w.ref() == null) ? "collected" : "alive");
        });
        s("nested_thread_keeps_main", function() {
            local main_obj = { v = 3 };
            local w = main_obj.weakref();
            local inner = newthread(function() { collectgarbage(); return "inner"; });
            local outer = newthread(function() { return inner.call(); });
            return outer.call() + "/" + ((w.ref() == null) ? "collected" : "alive");
        });
        s("suspended_thread_holds", function() {
            local t = newthread(function() {
                local obj = { v = 42 };
                local w = obj.weakref();
                suspend(1);
                return (w.ref() == null) ? "collected" : "alive";
            });
            t.call();
            collectgarbage();
            return t.wakeup() + "/" + t.getstatus();
        });
        s("suspended_gen_holds", function() {
            local g = (function() {
                local obj = { v = 11 };
                local w = obj.weakref();
                yield w;
                return (w.ref() == null) ? "collected" : "alive";
            })();
            resume g;
            collectgarbage();
            return resume g;
        });
        s("cross_thread_outer_held", function() {
            local shared = { v = 13 };
            local t = newthread(function() { local w = shared.weakref(); suspend(1); return (w.ref() == null) ? "collected" : "alive"; });
            t.call();
            collectgarbage();
            return t.wakeup();
        });
        s("many_threads_each_hold", function() {
            local threads = [];
            for (local i = 0; i < 5; i++) {
                local payload = { idx = i };
                local t = newthread(function() { local w = payload.weakref(); suspend(w); return (w.ref() == null) ? "collected" : "alive"; });
                t.call();
                threads.append(t);
            }
            collectgarbage();
            local out = "";
            foreach (t in threads) out += t.wakeup() + ",";
            return out;
        });
        s("thread_stack_values_usable", function() {
            local t = newthread(function() {
                local a = [1, 2, 3];
                local b = { arr = a };
                suspend(1);
                local sum = 0;
                foreach (v in b.arr) sum += v;
                return sum;
            });
            t.call();
            collectgarbage();
            return "" + t.wakeup() + "/" + t.getstatus();
        });
        s("thread_object_collected", function() {
            local w = null;
            { local t = newthread(function() { return 1; }); w = t.weakref(); }
            collectgarbage();
            return (w.ref() == null) ? "collected" : "alive";
        });
        s("suspended_thread_object_collected", function() {
            local w = null;
            { local t = newthread(function() { suspend(1); return 1; }); t.call(); w = t.weakref(); }
            collectgarbage();
            return (w.ref() == null) ? "collected" : "alive";
        });
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "in_thread_keeps_own=5/collected\n\
         in_thread_keeps_main=ran/alive\n\
         nested_thread_keeps_main=inner/alive\n\
         suspended_thread_holds=alive/idle\n\
         suspended_gen_holds=alive\n\
         cross_thread_outer_held=alive\n\
         many_threads_each_hold=alive,alive,alive,alive,alive,\n\
         thread_stack_values_usable=6/idle\n\
         thread_object_collected=collected\n\
         suspended_thread_object_collected=collected\n"
    );
}

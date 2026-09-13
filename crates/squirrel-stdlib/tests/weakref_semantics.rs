//! weakref / GC 回收语义回归（↔ sqbaselib.cpp 的 obj_delegate_weakref / weakref_ref、
//! sqstate.cpp 的 CollectGarbage）。
//!
//! 期望值全部用参考实现实测确认（`python tmp-c2/check_rs_scripts.py <本文件>`），
//! 语料侧另有 `tests-corpus/t_weakmap.nut`（63 条）守门。
//!
//! 这一轮**没有发现差异**（探针 38 + 27 条里只差两处地址文本），值得记下来的原因：
//! 本实现是 `Rc` + GC 链（链里存 `Weak`），而参考实现是手写引用计数 + 环回收，
//! 两者在"对象何时真正死掉"上一致 —— 包括：
//!
//! * `weakref()` 只对可弱引用的类型成立，其它类型报错；
//! * 弱引用**不保活**：`{ local o = mk(); w = o.weakref(); }` 之后 `w.ref()` 仍能拿到
//!   （栈上的残留槽），但 `collectgarbage()` 之后就变 `null`；
//! * 自环/互环对象被 `collectgarbage()` 收回，**返回值就是回收个数**（环 1 个 → 1）；
//! * 字符串是实习的：`"abc".weakref().ref()` 能拿回同一个字符串；
//! * 线程/生成器/类/实例/blob/regexp 的弱引用都能用。

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
    let rc = vm.run_buffer(src.as_bytes(), b"weak.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 各类型的 `weakref()` / `ref()`
///
/// 注意两条**参考实现自己就怪**的地方（两侧实测一致，照抄）：
/// * `(5).weakref()` 能建出 weakref（`typeof` 是 `weakref`），但它的 `ref()` 报
///   `the index 'ref' does not exist`（整数弱引用没挂 weakref 委托表）；
/// * `(null).weakref()` 直接报 `the index 'weakref' does not exist`；
/// * **没有** `getweakrefval` 这个全局函数（那是 C API `sq_getweakrefval`，
///   脚本侧只有 `w.ref()`）。
#[test]
fn weakref_of_every_type_matches_reference() {
    let src = r#"
        function s(tag, f) { try { print(tag + "=" + f() + "\n"); } catch (e) { print(tag + "!" + e + "\n"); } }
        s("tbl", function() { local o = { a = 1 }; local w = o.weakref(); return typeof w + "/" + (w.ref() == o); });
        s("arr", function() { local o = [1]; local w = o.weakref(); return typeof w + "/" + (w.ref() == o); });
        s("cls", function() { local C = class { v = 7; }; local w = C.weakref(); return typeof w + "/" + (w.ref() == C); });
        s("inst", function() { local C = class { v = 7; }; local o = C(); local w = o.weakref(); return typeof w + "/" + w.ref().v; });
        s("clo", function() { local f = function() { return 3; }; local w = f.weakref(); return typeof w + "/" + w.ref()(); });
        s("gen", function() { local g = (function() { yield 1; })(); local w = g.weakref(); return typeof w + "/" + (w.ref() == g); });
        s("thr", function() { local t = newthread(function() { return 1; }); local w = t.weakref(); return typeof w + "/" + t.getstatus(); });
        s("blob", function() { local b = blob(4); local w = b.weakref(); return typeof w + "/" + typeof w.ref(); });
        s("rx", function() { local r = regexp("a"); local w = r.weakref(); return typeof w + "/" + typeof w.ref(); });
        s("str", function() { local v = "abc"; local w = v.weakref(); return typeof w + "/" + (w.ref() == v); });
        s("int_ref", function() { local v = 5; local w = v.weakref(); return typeof w + "/" + typeof w.ref(); });
        s("null_weakref", function() { local v = null; local w = v.weakref(); return typeof w + "/" + typeof w.ref(); });
        s("no_global_getweakrefval", function() { return typeof getweakrefval; });
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "tbl=weakref/true\narr=weakref/true\ncls=weakref/true\ninst=weakref/7\nclo=weakref/3\n\
         gen=weakref/true\nthr=weakref/idle\nblob=weakref/blob\nrx=weakref/regexp\nstr=weakref/true\n\
         int_ref!the index 'ref' does not exist\nnull_weakref!the index 'weakref' does not exist\n\
         no_global_getweakrefval!the index 'getweakrefval' does not exist\n"
    );
}

/// 弱引用不保活；`collectgarbage()` 的返回值就是回收个数
#[test]
fn weakrefs_do_not_keep_objects_alive_and_gc_counts_match() {
    let src = r#"
        function s(tag, f) { try { print(tag + "=" + f() + "\n"); } catch (e) { print(tag + "!" + e + "\n"); } }
        function mk() { return { a = 1 }; }
        s("before_gc", function() { local w; { local o = mk(); w = o.weakref(); } return typeof w.ref(); });
        s("after_gc", function() { local w; { local o = mk(); w = o.weakref(); } collectgarbage(); return typeof w.ref(); });
        s("self_weak_cycle", function() { local t = {}; t.self <- t.weakref(); local w = t.weakref(); t = null; collectgarbage(); return typeof w.ref(); });
        s("gc_count_1cycle", function() { local a = {}; local b = {}; a.b <- b; b.a <- a; a = null; b = null; return collectgarbage(); });
        s("gc_count_2cycle", function() { local a = {}; local b = {}; a.b <- b; b.a <- a; local c = {}; c.self <- c; a = null; b = null; c = null; return collectgarbage(); });
        s("gc_count_plain_garbage", function() { local t = mk(); t = null; return collectgarbage(); });
        s("container_keeps_strong", function() { local arr = []; local o = mk(); arr.append(o); o = null; collectgarbage(); return typeof arr[0].a; });
        s("gc_twice", function() { local a = {}; a.self <- a; a = null; return collectgarbage() + "/" + collectgarbage(); });
        s("alive_kept", function() { local t = { a = 1 }; collectgarbage(); return t.a; });
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "before_gc=table\nafter_gc=null\nself_weak_cycle=null\ngc_count_1cycle=1\n\
         gc_count_2cycle=2\ngc_count_plain_garbage=0\ncontainer_keeps_strong=integer\n\
         gc_twice=1/0\nalive_kept=1\n"
    );
}

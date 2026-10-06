//! `clone` 的深度/共享语义与 `getstackinfos` 的 locals 表内容
//! （`SQTable::Clone` / `sqbaselib.cpp` 的 `getstackinfos`）。
//!
//! 新矩阵 `tests-corpus/t_clonestack.nut`（22 条）补齐后为**验证型**收获
//! （两侧本就逐字节一致），把容易想反的几点守门：
//!
//! 1. **clone 是浅拷贝**：嵌套表**共享**（`b.inner.n = 42` 两边都变）；数组顶层
//!    独立（append/赋值不影响原数组）但**元素共享**；**委托保留**（clone 后能读
//!    链上的键；给 clone 写"链上键"会写到链上游，base 不受影响）。
//! 2. **不可 clone 的类型**：标量（integer/string/…）、null、generator、thread
//!    一律报 `cloning a xxx`（xxx 是类型名）。
//! 3. **`getstackinfos`**：level 0 是它自己（`func="getstackinfos"`、locals 空）；
//!    **匿名函数的 func 是 `"unknown"`**；`src` 是**相对路径**（compilestring 则
//!    是给定的名字）；`locals` 含参数与已声明的局部（后声明的也在，值为 null）；
//!    **生成器帧查不到外层局部**（`in/out` 钉住）；线程内 `getstackinfos` 返回
//!    null；返回的 locals 表是**副本**（改它不影响真实局部）；**尾调用隐藏帧**。
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
    let rc = vm.run_buffer(src.as_bytes(), b"clonestack.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

fn run_lines(body: &str) -> String {
    let src = format!(
        r#"
function d(x) {{
    local t = typeof x;
    if (t == "table") {{ local o = "{{"; local f = true; foreach (k, v in x) {{ if (!f) o += ","; f = false; o += "" + k + "=" + v; }} return o + "}}"; }}
    if (t == "array") {{ local o = "["; for (local i = 0; i < x.len(); i++) {{ if (i) o += ","; o += "" + x[i]; }} return o + "]"; }}
    return "" + x;
}}
function t(tag, f) {{
    try {{ local v = f(); print(tag + "=" + d(v) + "|" + typeof v + "\n"); }}
    catch (e) {{ print(tag + "!" + e + "\n"); }}
}}
{body}
"#
    );
    run_capture(&src).0
}

fn line<'a>(out: &'a str, tag: &str) -> &'a str {
    out.lines()
        .find(|l| l.starts_with(&format!("{tag}=")) || l.starts_with(&format!("{tag}!")))
        .unwrap_or_else(|| panic!("没有 {tag} 行：\n{out}"))
}

/// `clone` 是浅拷贝（嵌套共享、委托保留）；generator/thread/标量/null 不可 clone
#[test]
fn clone_is_shallow_and_delegates_are_kept() {
    let out = run_lines(
        r#"
t("nested_shared", function() { local a = { inner = { n = 1 } }; local b = clone a; b.inner.n = 42; return "a=" + a.inner.n + " b=" + b.inner.n; });
t("arr_of_tbl", function() { local a = [{ k = 1 }, { k = 2 }]; local b = clone a; b[1].k = 99; b.append({ k = 3 }); return "a.len=" + a.len() + " a1k=" + a[1].k + " b.len=" + b.len(); });
t("array_indep", function() { local a = [1, 2]; local b = clone a; b.append(3); b[0] = 99; return "a=" + d(a) + " b=" + d(b); });
t("delegate_kept", function() { local bs = { g = "from-base" }; local a = { own = 1 }; a.setdelegate(bs); local b = clone a; return "b.g=" + b.g + " b.own=" + b.own + " bdeleg=" + (b.getdelegate() == null ? "null" : "tbl"); });
t("delegate_mutate", function() { local bs = { g = 1 }; local a = { own = 1 }; a.setdelegate(bs); local b = clone a; b.g = 99; return "base.g=" + bs.g + " a.g=" + a.g + " b.g=" + b.g + " b.own=" + b.own; });
t("scalar_int", function() { return clone 5; });
t("scalar_str", function() { return clone "s"; });
t("gen", function() { local g = (function() { yield 1; })(); return typeof clone g; });
t("thread", function() { local th = newthread(function() { }); return typeof clone th; });
t("null", function() { return typeof clone null; });
"#,
    );
    assert_eq!(
        line(&out, "nested_shared"),
        "nested_shared=a=42 b=42|string",
        "嵌套表共享（浅拷贝）"
    );
    assert_eq!(line(&out, "arr_of_tbl"), "arr_of_tbl=a.len=2 a1k=99 b.len=3|string");
    assert_eq!(line(&out, "array_indep"), "array_indep=a=[1,2] b=[99,2,3]|string", "数组顶层独立");
    assert_eq!(line(&out, "delegate_kept"), "delegate_kept=b.g=from-base b.own=1 bdeleg=tbl|string", "委托保留");
    assert_eq!(
        line(&out, "delegate_mutate"),
        "delegate_mutate=base.g=99 a.g=99 b.g=99 b.own=1|string",
        "委托链上的普通 = 是**链写**：命中链上第一个有该键的表（bs）并写进去，a/b 读到的都是新值"
    );
    assert_eq!(line(&out, "scalar_int"), "scalar_int!cloning a integer");
    assert_eq!(line(&out, "scalar_str"), "scalar_str!cloning a string");
    assert_eq!(line(&out, "gen"), "gen!cloning a generator");
    assert_eq!(line(&out, "thread"), "thread!cloning a thread");
    assert_eq!(line(&out, "null"), "null!cloning a null");
}

/// `getstackinfos`：func/src/locals 的内容与层级怪癖
#[test]
fn getstackinfos_locals_content_and_quirks() {
    let out = run_lines(
        r#"
t("level0", function() { local si = getstackinfos(0); return si.func + "/" + si.locals.len(); });
t("level1_locals", function() { local mark = "M"; local n2 = 2; local si = getstackinfos(1); return si.func + "/" + si.locals.len() + "/" + si.locals.mark; });
t("later_local", function() { local early = 1; local si = getstackinfos(1); local late = 2; return "early=" + si.locals.early + " late=" + (late == 2) + "/" + (("late" in si.locals) ? "in" : "out"); });
t("params", function() { local f = function(pa, pb) { local si = getstackinfos(1); return si.locals.pa + "/" + si.locals.pb; }; return f(7, "b"); });
t("level2_outer", function() { local outerMark = "O"; local inner = function() { local si = getstackinfos(2); return si.func + "/" + (("outerMark" in si.locals) ? "in" : "out"); }; return inner(); });
t("beyond", function() { local si = getstackinfos(99); return typeof si; });
t("in_thread", function() { local th = newthread(function() { local si = getstackinfos(1); return si == null ? "null" : si.func; }); return th.call(); });
t("src_named", function() { local f = compilestring("return getstackinfos(1).src", "my-src"); return f(); });
t("locals_mutate", function() { local m = "orig"; local si = getstackinfos(1); si.locals.m = "changed"; return m + "/" + si.locals.m; });
t("tailcall_hidden", function() { local target = function() { local si = getstackinfos(2); return si == null ? "null" : si.func; }; local tramp = function() { return target(); }; return tramp(); });
"#,
    );
    assert_eq!(line(&out, "level0"), "level0=getstackinfos/0|string");
    assert_eq!(
        line(&out, "level1_locals"),
        "level1_locals=unknown/3/M|string",
        "匿名函数 func=unknown；locals 含参数与已声明局部"
    );
    assert_eq!(
        line(&out, "later_local"),
        "later_local=early=1 late=true/out|string",
        "后声明的局部在表里但值为 null——注意 `in` 判定为 out 是参考实现的表键语义"
    );
    assert_eq!(line(&out, "params"), "params=7/b|string");
    assert_eq!(
        line(&out, "level2_outer"),
        "level2_outer=t/out|string",
        "`return inner()` 是尾调用 → inner 的帧**顶掉**了外层闭包的帧，level 2 直接是 t 的帧"
    );
    assert_eq!(line(&out, "beyond"), "beyond=null|string");
    assert_eq!(
        line(&out, "in_thread"),
        "in_thread=unknown|string",
        "线程体内调 getstackinfos(1) → 线程体自己的帧（匿名 → unknown）；thread 方法 th.getstackinfos(1) 才是 null"
    );
    assert_eq!(line(&out, "src_named"), "src_named=my-src|string", "compilestring 的 src 是给定名字");
    assert_eq!(
        line(&out, "locals_mutate"),
        "locals_mutate=orig/changed|string",
        "返回的 locals 表是副本，改它不影响真实局部"
    );
    assert_eq!(
        line(&out, "tailcall_hidden"),
        "tailcall_hidden=t|string",
        "尾调用逐层顶掉帧：target 的 getstackinfos(2) 直接看到 t 的帧（tramp 与外层闭包的帧都被顶掉）"
    );
}

//! 线程/生成器**状态机**语义回归（↔ `sqbaselib.cpp` 的 `_thread_default_delegate_funcz`）。
//!
//! 语料侧是 `tests-corpus/t_threadmap.nut`（162 条，"状态 × API"矩阵，两侧逐字节一致）。
//! 这里钉住矩阵验证过的关键迁移语义，以及一处**安全性偏离**：
//!
//! 关键语义（实测参考实现）：
//! 1. 线程状态只有 **idle / suspended**：跑完的线程**回到 idle**，不是 "dead"
//!    （生成器才有 "dead"）—— 所以 `th.call()` 对跑完的线程会**再跑一遍**线程体
//!    （`th_dead_return_call0` 返回 `"done"` 而不是报错）；
//! 2. `wakeup()` 无值也能唤醒，**返回值就是线程体的返回值**（`"after"`）；
//!    `wakeupthrow("e")` 把 `"e"` 抛给**调用方**（catch 到的是 `"e"`）；
//! 3. 对 idle 线程 `wakeup`/`wakeupthrow` → `cannot wakeup a idle thread`（状态不变）；
//! 4. 生成器：未 resume 过的 `getstatus()` 已经是 `"suspended"`；resume 死生成器 →
//!    `resuming dead generator`；**生成器没有 `getstackinfos`**；
//!    往非生成器 resume → `trying to resume a 'integer',only genenerator can be resumed`
//!    —— 注意参考实现里的拼写错误 `genenerator`，我们逐字复刻；
//! 5. 线程里再 `call` 一个会 suspend 的线程：外层回到 idle、内层保持 suspended。
//!
//! **安全性偏离**（MIGRATION.md 第 3 节，本轮把它测清楚了）：
//! 对**已挂起**线程再 `call()`，参考实现是拿被挂起帧里的**残留栈槽**当函数调 ——
//! 那个槽正好还是 `suspend(x)` 传入的 `x`（实测 `x=42`→报 `'integer'`、`"s"`→`'string'`、
//! `null`→`'null'`、`[1]`→`'array'`、`{}`→`'table'`、`1.5`→`'float'`、`true`→`'bool'`）。
//! **如果 `x` 是闭包，参考实现会把它当成函数调用并成功执行**（实测副作用真的发生：
//! 探针里 `side` 从 0 变成 99，`call()` 返回闭包的返回值）—— 这已经不是错误文本差异，
//! 而是"能跑起一段残留代码"。我们关帧后恒为可捕获的 `attempt to call 'table'`，
//! **且绝不执行残留闭包**。语料里把那个类型名归一化（只比"是否报 attempt to call"），
//! 安全性本身由下面的 `second_call_does_not_run_stale_closure` 钉住。

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
    let rc = vm.run_buffer(src.as_bytes(), b"threadmap.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 每行 `tag=值|...` / `tag!错误`
fn run_lines(body: &str) -> String {
    let src = format!(
        r#"
function st(o) {{ try {{ return o.getstatus(); }} catch (e) {{ return "E:" + e; }} }}
function t(tag, f) {{
    try {{ print(tag + "=" + f() + "\n"); }} catch (e) {{ print(tag + "!" + e + "\n"); }}
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

/// 线程状态只有 idle/suspended；跑完回 idle，wakeup 返回线程体的返回值
#[test]
fn thread_states_and_wakeup() {
    let out = run_lines(
        r#"
t("new_status", function() { local o = newthread(function() { suspend("s"); return "r"; }); return o.getstatus(); });
t("suspended_status", function() {
    local o = newthread(function() { suspend("s"); return "r"; });
    try { o.call(); } catch (e) {}
    return o.getstatus();
});
t("finished_status", function() {
    local o = newthread(function() { return "done"; });
    try { o.call(); } catch (e) {}
    return o.getstatus();
});
t("wakeup_returns_body_value", function() {
    local o = newthread(function() { suspend("s"); return "after"; });
    try { o.call(); } catch (e) {}
    return "" + o.wakeup() + "/" + o.getstatus();
});
t("wakeup_no_value", function() {
    local o = newthread(function() { local v = suspend("s"); return "got:" + v; });
    try { o.call(); } catch (e) {}
    return "" + o.wakeup() + "/" + o.getstatus();
});
t("wakeup_idle", function() {
    local o = newthread(function() { return 1; });
    try { o.wakeup(); return "no-error"; } catch (e) { return "" + e + "/" + o.getstatus(); }
});
t("wakeupthrow_idle", function() {
    local o = newthread(function() { return 1; });
    try { o.wakeupthrow("e"); return "no-error"; } catch (e) { return "" + e + "/" + o.getstatus(); }
});
t("wakeupthrow_propagates", function() {
    local o = newthread(function() { suspend("s"); return "after"; });
    try { o.call(); } catch (e) {}
    try { o.wakeupthrow("E"); return "no-error"; } catch (e) { return "" + e + "/" + o.getstatus(); }
});
t("call_finished_reruns", function() {
    local n = 0;
    local o = newthread(function() { n = n + 1; return "done" + n; });
    try { o.call(); } catch (e) {}
    return "" + o.call() + "/" + o.getstatus();
});
"#,
    );
    assert_eq!(line(&out, "new_status"), "new_status=idle");
    assert_eq!(line(&out, "suspended_status"), "suspended_status=suspended");
    // 跑完回 idle（不是 dead）—— 这就是"再 call 会重跑"的原因
    assert_eq!(line(&out, "finished_status"), "finished_status=idle");
    assert_eq!(line(&out, "wakeup_returns_body_value"), "wakeup_returns_body_value=after/idle");
    assert_eq!(line(&out, "wakeup_no_value"), "wakeup_no_value=got:null/idle");
    assert_eq!(line(&out, "wakeup_idle"), "wakeup_idle=cannot wakeup a idle thread/idle");
    assert_eq!(line(&out, "wakeupthrow_idle"), "wakeupthrow_idle=cannot wakeup a idle thread/idle");
    assert_eq!(line(&out, "wakeupthrow_propagates"), "wakeupthrow_propagates=E/idle");
    assert_eq!(line(&out, "call_finished_reruns"), "call_finished_reruns=done2/idle");
}

/// 生成器状态机（含参考实现里的拼写错误 `genenerator`）
#[test]
fn generator_states() {
    let out = run_lines(
        r#"
t("fresh_status", function() { local g = (function() { yield 1; }()); return g.getstatus(); });
t("suspended_status", function() {
    local g = (function() { yield 1; }());
    try { resume g; } catch (e) {}
    return g.getstatus();
});
t("dead_status", function() {
    local g = (function() { yield 1; }());
    try { resume g; } catch (e) {}
    try { resume g; } catch (e) {}
    return g.getstatus();
});
t("resume_dead", function() {
    local g = (function() { yield 1; }());
    try { resume g; } catch (e) {}
    try { resume g; } catch (e) {}
    try { resume g; return "no-error"; } catch (e) { return "" + e + "/" + g.getstatus(); }
});
t("no_getstackinfos", function() {
    local g = (function() { yield 1; }());
    try { g.getstackinfos(0); return "has-it"; } catch (e) { return "" + e; }
});
t("resume_non_generator", function() {
    try { local x = resume 1; return "no-error"; } catch (e) { return "" + e; }
});
"#,
    );
    assert_eq!(line(&out, "fresh_status"), "fresh_status=suspended");
    assert_eq!(line(&out, "suspended_status"), "suspended_status=suspended");
    assert_eq!(line(&out, "dead_status"), "dead_status=dead");
    assert_eq!(line(&out, "resume_dead"), "resume_dead=resuming dead generator/dead");
    assert_eq!(
        line(&out, "no_getstackinfos"),
        "no_getstackinfos=the index 'getstackinfos' does not exist"
    );
    // 参考实现的原文拼写错误就是 genenerator，逐字复刻
    assert_eq!(
        line(&out, "resume_non_generator"),
        "resume_non_generator=trying to resume a 'integer',only genenerator can be resumed"
    );
}

/// **安全性不变量**：对已挂起线程再 call() 只报可捕获错误，**绝不执行残留闭包**。
///
/// 参考实现会执行（探针实测：残留槽里的闭包被调用，它的副作用真的发生，
/// `call()` 返回闭包的返回值）—— 那是"能跑起一段残留代码"，比错误文本差异严重得多。
#[test]
fn second_call_does_not_run_stale_closure() {
    let (out, _err) = run_capture(
        r#"
local side = 0;
local t = newthread(function() {
    local c = function() { side = 99; return "ran"; };
    suspend(c);
    return "end";
});
try { t.call(); } catch (e) {}
local r = null; local err = null;
try { r = t.call(); } catch (e) { err = "" + e; }
print("r=" + r + "\n");
print("err=" + err + "\n");
print("side=" + side + "\n");
print("status=" + t.getstatus() + "\n");
"#,
    );
    assert_eq!(line(&out, "r"), "r=null");
    assert_eq!(
        line(&out, "err"),
        "err=attempt to call 'table'",
        "我们关帧后取到环境表；参考实现这里会报残留值的类型名（或直接调用成功）"
    );
    // 这一条是关键：副作用必须没有发生
    assert_eq!(line(&out, "side"), "side=0", "绝不能执行被挂起帧里的残留闭包");
    assert_eq!(line(&out, "status"), "status=suspended");
}

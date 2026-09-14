//! 生成器/线程 × 挂起-恢复 × 异常交错（`sqbaselib.cpp` 的生成器委托 +
//! `SQVM` 的线程挂起路径）。
//!
//! `t_suspmap`/`t_threadmap` 测的是状态机主干；这里钉的是**异常与挂起的交错**，
//! 新矩阵 `tests-corpus/t_gtmap.nut`（37 条）补齐后为**验证型**收获
//! （两侧本就逐字节一致），把容易想反的几点守门：
//!
//! 1. **生成器死态**：正常跑完 / 体内抛错都进入 dead，之后 `resume` 一律
//!    `resuming dead generator`；体内抛的错误传播到**恢复点**的 catch。
//! 2. **yield 与 try 的两种交错**：`yield` 在 try 里、恢复后体内 throw →
//!    走生成器**自己**的 catch；恢复点在 try 里、体内 throw → 走**调用方**
//!    的 catch。`yield` 是语句不是表达式（`local a = yield x` 编译错）。
//! 3. **线程状态串**是 `idle/suspended/…`——**没有 "dead"**，跑完回 idle；
//!    idle 的线程可以再 `call()` **从头重跑**；`wakeup`/`wakeupthrow` 打在
//!    idle 上报 `cannot wakeup a idle thread`。
//! 4. **`wakeupthrow` 注入的错误会被挂起点外层的 try 接住**
//!    （`caught:injected`），没有 try 则传给调用方。
//! 5. `wakeup` 不带参数 → `suspend` 表达式的值是 `null`；多带的参数被忽略；
//!    挂起线程的 `getstackinfos` 返回 null。
//!
//! **有意偏离**（参考实现会死，见 MIGRATION.md）：
//! `newthread(<原生闭包>)` 参考实现**创建时即挂死**（`tmp-c2/gt_isolate.py`
//! 实测 6 秒无输出）；本实现正常创建线程对象、调用报
//! `wrong number of parameters` —— 钉在最后一个测试里。

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
    let rc = vm.run_buffer(src.as_bytes(), b"gtmap.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

fn run_lines(body: &str) -> String {
    let src = format!(
        r#"
function t(tag, f) {{
    try {{ local v = f(); local o = typeof v; print(tag + "=" + (o == "string" ? v : "" + v) + "|" + o + "\n"); }}
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

/// 生成器：死态复活、异常与 yield 的两种交错、编译期形态
#[test]
fn generator_lifecycle_and_exception_interleaving() {
    let out = run_lines(
        r#"
t("life", function() { local it = (function() { yield 1; yield 2; return 9; })(); local a = it.getstatus(); resume it; local b = it.getstatus(); foreach (v in it) { } return a + "/" + b + "/" + it.getstatus(); });
t("dead", function() { local it = (function() { yield 1; })(); foreach (v in it) { } return resume it; });
t("dead_val", function() { local it = (function() { yield 1; return 42; })(); resume it; local r1 = resume it; local r2 = resume it; return r1 + "/" + r2; });
t("throw_inside", function() { local it = (function() { yield 1; throw "boom"; yield 2; })(); local s = ""; try { foreach (v in it) s += v; } catch (e) { s += "E:" + e; } return s + "/" + it.getstatus(); });
t("resume_after_throw", function() { local it = (function() { yield 1; throw "boom"; yield 2; })(); resume it; try { resume it; } catch (e) { } return resume it; });
t("yield_in_try", function() { local it = (function() { try { yield "in-try"; throw "inner"; } catch (e) { yield "caught:" + e; } })(); local a = resume it; local b = resume it; return a + "/" + b; });
t("throw_across", function() { local it = (function() { yield 1; throw "from-gen"; })(); local s = ""; resume it; try { resume it; } catch (e) { s = "C:" + e; } return s; });
t("catch_yields", function() { local it = (function() { try { throw "x"; } catch (e) { yield "c1"; yield "c2"; } })(); local a = resume it; local b = resume it; return a + "/" + b; });
t("yield_as_expr", function() { return compilestring("function f() { local a = yield 1; }", "y3") == null ? "compile-null" : "compiled"; });
t("yield_in_plain", function() { return compilestring("function f() { yield 1; }", "y6") == null ? "compile-null" : "compiled"; });
t("break_status", function() { local it = (function() { yield 1; yield 2; yield 3; })(); local s = ""; foreach (v in it) { s += v; if (v == 2) break; } return s + "/" + it.getstatus(); });
t("interleaved", function() { local a = (function() { yield "a1"; yield "a2"; })(); local b = (function() { yield "b1"; yield "b2"; })(); return "" + (resume a) + (resume b) + (resume a) + (resume b); });
"#,
    );
    assert_eq!(line(&out, "life"), "life=suspended/suspended/dead|string");
    assert_eq!(line(&out, "dead"), "dead!resuming dead generator");
    assert_eq!(line(&out, "dead_val"), "dead_val!resuming dead generator");
    assert_eq!(line(&out, "throw_inside"), "throw_inside=1E:boom/dead|string");
    assert_eq!(line(&out, "resume_after_throw"), "resume_after_throw!resuming dead generator");
    // yield 在 try 里 + 体内 throw → 生成器自己的 catch 接住
    assert_eq!(line(&out, "yield_in_try"), "yield_in_try=in-try/caught:inner|string");
    // 恢复点在 try 里 + 体内 throw → 调用方的 catch 接住
    assert_eq!(line(&out, "throw_across"), "throw_across=C:from-gen|string");
    assert_eq!(line(&out, "catch_yields"), "catch_yields=c1/c2|string");
    assert_eq!(line(&out, "yield_as_expr"), "yield_as_expr!expression expected", "yield 是语句不是表达式");
    assert_eq!(line(&out, "yield_in_plain"), "yield_in_plain=compiled|string", "含 yield 的函数自动成为生成器");
    assert_eq!(line(&out, "break_status"), "break_status=12/suspended|string");
    assert_eq!(line(&out, "interleaved"), "interleaved=a1b1a2b2|string");
}

/// 线程：状态迁移、wakeup/wakeupthrow、idle 重启
#[test]
fn thread_suspend_wakeup_and_injection() {
    let out = run_lines(
        r#"
t("basic", function() { local th = newthread(function() { local a = suspend("s1"); return "got:" + a; }); local r1 = th.call(); local r2 = th.wakeup(42); return r1 + "/" + r2 + "/" + th.getstatus(); });
t("life", function() { local th = newthread(function() { suspend(1); suspend(2); return 3; }); local a = th.getstatus(); th.call(); local b = th.getstatus(); th.wakeup(); local c = th.getstatus(); th.wakeup(); local d = th.getstatus(); return a + "/" + b + "/" + c + "/" + d; });
t("wakeup_dead", function() { local th = newthread(function() { return 1; }); th.call(); return th.wakeup(); });
t("call_restart", function() { local th = newthread(function() { return 1; }); local a = th.call(); local b = th.call(); return a + "/" + b + "/" + th.getstatus(); });
t("throw_in_body", function() { local th = newthread(function() { suspend("s"); throw "thread-boom"; }); local r1 = th.call(); local st = th.getstatus(); local s = r1 + "/" + st + "/"; try { th.wakeup(); } catch (e) { s += "E:" + e; } return s + "/" + th.getstatus(); });
t("wt_caught", function() { local th = newthread(function() { try { suspend("in-try"); return "no"; } catch (e) { return "caught:" + e; } }); local r1 = th.call(); local r2 = th.wakeupthrow("injected"); return r1 + "/" + r2; });
t("wt_notry", function() { local th = newthread(function() { suspend("plain"); return "no"; }); th.call(); try { th.wakeupthrow("inj"); } catch (e) { return "outer:" + e + "/" + th.getstatus(); } return "no-error"; });
t("wt_running", function() { local th = newthread(function() { return 1; }); return th.wakeupthrow("x"); });
t("wakeup_running", function() { local th = newthread(function() { return 1; }); return th.wakeup(); });
t("suspend_foreach", function() { local th = newthread(function() { foreach (v in [1, 2, 3]) { suspend(v); } return "done"; }); local a = th.call(); local b = th.wakeup(); local c = th.wakeup(); local d = th.wakeup(); return a + "/" + b + "/" + c + "/" + d; });
t("err_after_resume", function() { local th = newthread(function() { local x = suspend("s"); if (x == 2) throw "late"; return "ok" + x; }); local a = th.call(); local b = th.wakeup(1); local c; try { c = th.wakeup(2); } catch (e) { c = "E:" + e; } return a + "/" + b + "/" + c + "/" + th.getstatus(); });
t("noargs", function() { local th = newthread(function() { local a = suspend("s"); return "a=" + a; }); th.call(); return th.wakeup(); });
t("twoargs", function() { local th = newthread(function() { local a = suspend("s"); return "a=" + a; }); th.call(); return th.wakeup(1, 2); });
t("stkinfo", function() { local th = newthread(function() { suspend(1); }); th.call(); local si = th.getstackinfos(1); return typeof si + "/" + (si == null ? "null" : si.func); });
t("nonfunc", function() { return newthread(42); });
"#,
    );
    assert_eq!(line(&out, "basic"), "basic=s1/got:42/idle|string");
    assert_eq!(line(&out, "life"), "life=idle/suspended/suspended/idle|string");
    assert_eq!(line(&out, "wakeup_dead"), "wakeup_dead!cannot wakeup a idle thread");
    assert_eq!(line(&out, "call_restart"), "call_restart=1/1/idle|string", "idle 的线程可以再 call 从头重跑");
    assert_eq!(line(&out, "throw_in_body"), "throw_in_body=s/suspended/E:thread-boom/idle|string");
    assert_eq!(line(&out, "wt_caught"), "wt_caught=in-try/caught:injected|string", "注入的错误被挂起点外层的 try 接住");
    assert_eq!(line(&out, "wt_notry"), "wt_notry=outer:inj/idle|string");
    assert_eq!(line(&out, "wt_running"), "wt_running!cannot wakeup a idle thread");
    assert_eq!(line(&out, "wakeup_running"), "wakeup_running!cannot wakeup a idle thread");
    assert_eq!(line(&out, "suspend_foreach"), "suspend_foreach=1/2/3/done|string");
    assert_eq!(line(&out, "err_after_resume"), "err_after_resume=s/ok1/E:cannot wakeup a idle thread/idle|string");
    assert_eq!(line(&out, "noargs"), "noargs=a=null|string", "不带参数 → suspend 表达式的值是 null");
    assert_eq!(line(&out, "twoargs"), "twoargs=a=1|string", "多带的参数被忽略");
    assert_eq!(line(&out, "stkinfo"), "stkinfo=null/null|string");
    assert_eq!(
        line(&out, "nonfunc"),
        "nonfunc!parameter 1 has an invalid type 'integer' ; expected: 'function|function'"
    );
}

/// **有意偏离**：`newthread(<原生闭包>)` 参考实现创建时即挂死；本实现正常
#[test]
fn newthread_with_native_closure_is_safe_here() {
    let out = run_lines(
        r#"
t("create", function() { return typeof newthread(print); });
t("call_it", function() { local th = newthread(print); try { th.call(); } catch (e) { return "E:" + e; } return "no-error"; });
"#,
    );
    assert_eq!(line(&out, "create"), "create=thread|string");
    assert_eq!(
        line(&out, "call_it"),
        "call_it=E:wrong number of parameters|string"
    );
}

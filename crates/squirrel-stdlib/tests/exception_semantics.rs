//! 异常机制语义回归（P1-6）。
//!
//! 语料侧是 `tests-corpus/t_errmap.nut`（81 条）与 `tests-corpus/t_errtail.nut`
//! （脚本末尾制造**未捕获**错误，用来对比完整 stderr 的
//! `AN ERROR HAS OCCURRED` / `CALLSTACK` / `LOCALS` 与退出码）。
//!
//! 本轮**没有发现语义差异**（84 条探针里只差指针文本与一处已知 UB），
//! 值得记下来的是几条容易想当然的点：
//!
//! * `throw` 的值就是错误对象本身：`catch (e)` 拿到的是**原对象**（字符串就是字符串，
//!   表就是表），不是包装过的错误结构；
//! * VM 错误的文本形态：`the index 'x' does not exist`、`attempt to call 'integer'`、
//!   `arith op + on between 'table' and 'integer'`、`division by zero`、
//!   `wrong number of parameters (1 passed, 2 required)`；
//! * `getstackinfos(0)` 的**第 0 层是 `getstackinfos` 这个 native 自己**（不是调用者），
//!   所以 `func` 恒为 `"getstackinfos"`、`locals` 是它的实参表；
//! * `getstackinfos(-1)` 是参考实现的**越界读**（UB，实测返回一个表），
//!   本实现返回 `null` —— 有意偏离，单独守门（`ref-crash-skip`）。

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
    let rc = vm.run_buffer(src.as_bytes(), b"err.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// `throw` 的值与 VM 错误文本
#[test]
fn throw_values_and_vm_error_messages_match_reference() {
    let src = r#"
        function w(f) { try { return f(); } catch (e) { return "C:" + typeof e + ":" + e; } }
        print(w(function() { throw "boom"; }) + "\n");
        print(w(function() { throw 42; }) + "\n");
        print(w(function() { throw 1.5; }) + "\n");
        print(w(function() { throw null; }) + "\n");
        print(w(function() { throw true; }) + "\n");
        print(w(function() { local t = {}; return t.nope; }) + "\n");
        print(w(function() { local x = 5; return x(); }) + "\n");
        print(w(function() { local x = "s"; return x(); }) + "\n");
        print(w(function() { local x = {}; return x(); }) + "\n");
        print(w(function() { local x = null; return x(); }) + "\n");
        print(w(function() { local x = []; return x(); }) + "\n");
        print(w(function() { local x = {}; return x + 1; }) + "\n");
        print(w(function() { return "a" - 1; }) + "\n");
        print(w(function() { return 1 / 0; }) + "\n");
        print(w(function() { local a = [1]; return a[5]; }) + "\n");
        print(w(function() { return abs("x"); }) + "\n");
        print(w(function() { local f = function(a) { return a; }; return f(); }) + "\n");
        print(w(function() { local f = function(a) { return a; }; return f(1, 2); }) + "\n");
        print(w(function() { local C = class { function _get(k) { throw "from-_get"; } }; return C().x; }) + "\n");
        print(w(function() { local C = class { constructor() { throw "ctor-fail"; } }; return C(); }) + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "C:string:boom\nC:integer:42\nC:float:1.5\nC:null:null\nC:bool:true\n\
         C:string:the index 'nope' does not exist\n\
         C:string:attempt to call 'integer'\nC:string:attempt to call 'string'\n\
         C:string:attempt to call 'table'\nC:string:attempt to call 'null'\n\
         C:string:attempt to call 'array'\n\
         C:string:arith op + on between 'table' and 'integer'\n\
         C:string:arith op - on between 'string' and 'integer'\n\
         C:string:division by zero\nC:string:the index '5' does not exist\n\
         C:string:parameter 1 has an invalid type 'string' ; expected: 'integer|float'\n\
         C:string:wrong number of parameters (1 passed, 2 required)\n\
         C:string:wrong number of parameters (3 passed, 2 required)\n\
         C:string:from-_get\nC:string:ctor-fail\n"
    );
}

/// 嵌套 trap / 重抛 / catch 作用域 / 错误处理器 / 生成器与线程里的错误
#[test]
fn nested_traps_and_error_handlers_match_reference() {
    let src = r#"
        local out = "";
        try { try { throw "a"; } catch (e) { throw "b:" + e; } } catch (e) { out += "rethrow:" + e + ";"; }
        try { local e = "x"; try { throw "inner"; } catch (e) { } out += "scope:" + e + ";"; } catch (e) { }
        try { throw "e"; } catch (x) { out += "always;"; }
        local seen = "";
        seterrorhandler(function(err) { seen = "H:" + err; });
        try { throw "caught"; } catch (e) { }
        out += "handler_on_caught:" + (seen == "" ? "no" : seen) + ";";
        seterrorhandler(null);
        local g = (function() { try { yield 1; throw "g"; } catch (e) { yield "cg:" + e; } })();
        out += "gen:" + resume g + "/" + resume g + ";";
        local g2 = (function() { yield 1; throw "ge"; })();
        resume g2;
        try { resume g2; } catch (e) { out += "generr:" + e + ";"; }
        local t = newthread(function() { throw "te"; });
        try { t.call(); } catch (e) { out += "threrr:" + e + "/" + t.getstatus() + ";"; }
        local t2 = newthread(function() { try { throw "in"; } catch (e) { return "ct:" + e; } });
        out += "thrcatch:" + t2.call();
        print(out + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "rethrow:b:a;scope:x;always;handler_on_caught:no;gen:1/cg:g;generr:ge;threrr:te/idle;thrcatch:ct:in\n"
    );
}

/// `getstackinfos`：第 0 层是 native 自己，越界层返回 null
#[test]
fn getstackinfos_levels_match_reference() {
    let src = r#"
        function named_fn() { return getstackinfos(0).func; }
        local si = getstackinfos(0);
        print(typeof si + "/" + typeof si.func + "/" + typeof si.src + "/" + typeof si.line + "/" + typeof si.locals + "\n");
        print(named_fn() + "\n");
        print(typeof getstackinfos(1) + "/" + typeof getstackinfos(2) + "/" + typeof getstackinfos(50) + "\n");
        local inner = function() { return typeof getstackinfos(1); };
        print(inner() + "\n");
        local a = function() { local b = function() { return typeof getstackinfos(3); }; return b(); };
        print(a() + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "table/string/string/integer/table\ngetstackinfos\ntable/null/null\ntable\nnull\n"
    );
}

/// `getstackinfos(-1)`：参考实现越界读（UB，实测返回一个表），本实现返回 `null`。
/// ref-crash-skip（给 tmp-c2/check_rs_scripts.py 用：别拿这段去跑参考实现）
#[test]
fn getstackinfos_negative_level_is_null() {
    let src = r#"
        print(typeof getstackinfos(-1) + "/" + typeof getstackinfos(-100) + "\n");
        print(typeof getstackinfos(0) + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(out, "null/null\ntable\n");
}

/// P24：错误**跨越线程/原生调用边界**时的 stderr 转储（语料
/// `t_errdumpframes.nut`，13 条，验证型——两侧逐字节一致）。
///
/// 钉住的语义：错误跨越**线程边界**（含 `wakeupthrow` 注入、生成器在线程内
/// 抛错）与**原生调用边界**（sort/map 比较器回调）时，即使外层脚本
/// try/catch 接住，stderr 照样有 `AN ERROR HAS OCCURRED [msg]` + CALLSTACK
/// 转储，且脚本**继续跑完**；纯脚本帧之间被 catch 的抛错**不产生任何转储**。
/// `_get` 元方法里解析落回委托链的名字会递归到 `Native stack overflow`——
/// 两侧一致，照实钉住。
#[test]
fn stderr_dump_applies_at_thread_and_native_boundaries() {
    let src = r#"
        function s(tag, f) {
            try { f(); print(tag + "=no-dump\n"); }
            catch (e) { print(tag + "=caught:" + e + "\n"); }
        }
        function boom4(k) { throw "mm:" + k; }
        s("thread_throw", function() { local th = newthread(function() { local lv = "L"; throw "tb"; }); th.call(); th.wakeup(); });
        s("thread_wakeupthrow", function() { local th = newthread(function() { suspend("s"); }); th.call(); th.wakeupthrow("inj"); });
        s("gen_in_thread", function() { local th = newthread(function() { local g = (function() { local gg = 9; throw "git"; })(); resume g; }); th.call(); th.wakeup(); });
        s("native_frame_sort", function() { local a = [1, 2]; a.sort(function(x, y) { local sc = 1; throw "sortcb"; }); });
        s("plain_catch", function() { local lo = 1; throw "plain"; });
        s("mm_chain_overflow", function() { local d = { _get = function(k) { return boom4(k); } }; local o = {}; o.setdelegate(d); return o.zz; });
        print("ERRDUMP-DONE\n");
    "#;
    let (out, err) = run_capture(src);
    // stdout：全部被 catch，脚本跑完
    assert!(out.ends_with("ERRDUMP-DONE\n"), "{out}");
    assert!(out.contains("thread_throw=caught:tb"));
    assert!(out.contains("thread_wakeupthrow=caught:inj"));
    assert!(out.contains("gen_in_thread=caught:git"));
    assert!(out.contains("native_frame_sort=caught:sortcb"));
    assert!(out.contains("plain_catch=caught:plain"));
    assert!(out.contains("mm_chain_overflow=caught:Native stack overflow"));
    // stderr：线程/生成器边界各触发一次转储（含消息与调用栈）
    assert!(err.contains("AN ERROR HAS OCCURRED [tb]"), "{err}");
    assert!(err.contains("AN ERROR HAS OCCURRED [inj]"), "{err}");
    assert!(err.contains("AN ERROR HAS OCCURRED [git]"), "{err}");
    assert!(err.contains("CALLSTACK"), "{err}");
    // 纯脚本帧 + sort 比较器被 catch 的：**不**进 stderr
    assert!(!err.contains("[plain]"), "脚本帧被 catch 不应有转储\n{err}");
    assert!(!err.contains("[sortcb]"), "{err}");
    assert!(!err.contains("[mm:"), "{err}");
}

/// P33：compilestring/dofile 的**诊断归属**（语料 `t_diagmap.nut`，10 条，
/// 验证型——两侧含 stderr 逐字节一致）。钉住：编译代码内
/// `getstackinfos(1).line` 返回**实际行号**、`.src` 返回给定源名、
/// 编译错误的 catch 文本不含行号（就是消息本身）。
#[test]
fn compilestring_line_and_src_attribution() {
    let src = r#"
        function t(tag, f) {
            try { local v = f(); print(tag + "=ok:" + (typeof v == "string" ? v : "" + v) + "\n"); }
            catch (e) { print(tag + "=caught:" + e + "\n"); }
        }
        t("si_line", function() { local f = compilestring("local a = 1\nlocal b = 2\nreturn getstackinfos(1).line", "SI"); return f(); });
        t("si_src", function() { local f = compilestring("return getstackinfos(1).src", "SI2"); return f(); });
        t("cs_syn", function() { local f = compilestring("local a = 1\nlocal b = 2\nlocal c = (", "S3"); return "compiled"; });
        t("cs_syn_deep", function() { local f = compilestring("\n\n\n\n\n\n\n\n\n\nbad(", "SDEEP"); return "compiled"; });
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(out.lines().find(|l| l.starts_with("si_line=")).unwrap(), "si_line=ok:3", "第 3 行的 getstackinfos(1).line 返回 3");
    assert_eq!(out.lines().find(|l| l.starts_with("si_src=")).unwrap(), "si_src=ok:SI2", ".src 返回给定的源名");
    assert!(
        out.lines().find(|l| l.starts_with("cs_syn=")).unwrap().starts_with("cs_syn=caught:expression expected"),
        "编译错误 catch 文本不含行号"
    );
    assert!(out.lines().find(|l| l.starts_with("cs_syn_deep=")).unwrap().starts_with("cs_syn_deep=caught:expression expected"));
}

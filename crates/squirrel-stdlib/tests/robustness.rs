//! 健壮性回归：**任何**脚本 / 字节码 / 正则 / 原生调用输入都不得 panic。
//!
//! 约定（编码基本习惯）：在执行、解引用、索引之前先做非空/范围/类型校验；
//! 引擎内部不变式被破坏时，必须退化为可捕获的脚本错误或 Err，而不是 panic!/
//! unreachable!/越界索引。这个文件是那条约定的守卫：新增原生函数或语法特性后，
//! 若引入可被脚本触发的 panic，下面的用例会直接失败。

use std::cell::RefCell;
use std::rc::Rc;

use squirrel::api::Vm;

/// 跑一段源码（不带标准库）；允许编译/运行失败，但不允许 panic。
fn run(src: &[u8]) {
    let vm = Vm::new(1024);
    let _ = vm.run_buffer(src, b"robust.nut");
}

/// 跑一段用完整标准库初始化的源码（regexp/date/blob/file 等原生都在）。
fn run_script(src: &str) {
    let vm = Vm::new(1024);
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    let _ = vm.run_buffer(src.as_bytes(), b"robust.nut");
}

#[test]
fn degenerate_sources_do_not_panic() {
    let cases: &[&[u8]] = &[
        b"", b" ", b"\n", b"\0", b"\xff\xfe", b"{", b"}", b"(", b")", b"[", b"]", b"::", b"<-",
        b"=", b"...", b"@", b"$", b"\\", b"/*", b"//", b"\"", b"'", b"`", b"0x", b"1e", b"1.",
        b"function", b"function(", b"function f", b"class", b"class A", b"class A {",
        b"local", b"local x", b"local x =", b"if", b"if(", b"if()", b"while", b"while(",
        b"for", b"for(", b"foreach", b"foreach(", b"switch", b"case", b"enum", b"const",
        b"try", b"catch", b"throw", b"return", b"yield", b"resume", b"delete", b"newslot",
        b"typeof", b"clone", b"base", b"this", b"vargv", b"__LINE__", b"\"\\u\"", b"\"\\x\"",
        b"@\"", b"@\"unterminated", b"0b1010", b"1_000", b"#!", b"#include", b"break", b"continue",
        b"a.b.c.d.e", b"a[", b"a[[[[", b"function f() { break; }", b"function f() { continue; }",
        b"class A { function A() {} }", b"local class = 1", b"local base = 1", b"enum {}",
        b"const", b"const a", b"const a =", b"::(\"\\x00\\xff\")",
    ];
    for c in cases {
        run(c);
    }
}

#[test]
fn every_prefix_and_suffix_of_a_full_script_does_not_panic() {
    let full: &[u8] = br#"
class A {
    static s = 1;
    x = 2;
    constructor(a = 1, ...) { }
    function f(a, b = 2, ...) {
        local t = { k = [1, 2, { n = null }] };
        try { foreach (k, v in t) print(k); } catch (e) { throw e; }
        switch (a) { case 1: break; default: break; }
        return a + b;
    }
    function g() { yield 1; yield 2; }
}
const C = 1;
enum E { A, B }
local g = function () { yield this; };
local t = { ["k"] = 1, f = function () {} };
for (local i = 0; i < 3; i++) { if (i) continue; else break; }
local a = [1, 2, 3];
a.map(function (v) { return v * 2; });
local s = "x" + 1 + 1.5 + true + null;
local c = compilestring("return 1 + 1", "inner");
local ok = c() == 2;
print(ok ? "ok" : "bad");
"#;
    for i in 0..=full.len() {
        run(&full[..i]);
        run(&full[i..]);
    }
}

#[test]
fn adversarial_native_calls_do_not_panic() {
    run_script(
        r#"
        function t(f) { try { f(); } catch (e) {} }
        t(function() { blob(-1); });
        t(function() { blob(0x7FFFFFFF); });
        t(function() { blob(8).resize(-5); });
        t(function() { blob(8); local b = blob(8); b[99] = 1; });
        t(function() { local b = blob(8); return b[99]; });
        t(function() { local b = blob(8); b.swap2().swap4(); });
        t(function() { array(-1); });
        t(function() { array(1).resize(-1); });
        t(function() { array(1).insert(99, 1); });
        t(function() { array(1).remove(99); });
        t(function() { array(1).slice(-99, 99); });
        t(function() { array(1).sort(1); });
        t(function() { array(1).apply(null); });
        t(function() { array(1).reduce(null, 0); });
        t(function() { "abc".slice(-99, 99); });
        t(function() { "abc".slice(99, -99); });
        t(function() { "abc".find("", -99, 99); });
        t(function() { "abc"[-99]; });
        t(function() { "abc"[99]; });
        t(function() { "abc".tointeger(); });
        t(function() { "x".tochar(); });
        t(function() { format("%d"); });
        t(function() { format("%99999999d", 1); });
        t(function() { format("%", 1); });
        t(function() { format("%q", 1); });
        t(function() { format(null, 1); });
        // 整型最小值：`abs()` 会溢出（release 得到 "--9223372036854775808"，debug 直接 panic）
        t(function() { format("%d", -9223372036854775807 - 1); });
        t(function() { format("%x", -9223372036854775807 - 1); });
        t(function() { format("%u", -9223372036854775807 - 1); });
        t(function() { format("%+d", -9223372036854775807 - 1); });
        // 精度/宽度/标志的边界组合
        t(function() { format("%.99f", 1.5); });
        t(function() { format("%99.99f", 1.5); });
        t(function() { format("%0f", 0.0/0.0); });
        t(function() { format("%-+ #0.3f", 1.5); });
        t(function() { format("%#.0f", 1.0/0.0); });
        t(function() { format("%#.0f", 0.0/0.0); });
        t(function() { format("%c", 1.0e300); });
        t(function() { format("%d", 1.0e300); });
        t(function() { format("%d", 0.0/0.0); });
        t(function() { format("%.5x", -1); });
        t(function() { format("%#o", -1); });
        t(function() { format("%++++++++++++++++++d", 1); });
        t(function() { split("abc", ""); });
        t(function() { split(null, null); });
        t(function() { escape(null); });
        t(function() { getstackinfos(-1); });
        t(function() { getstackinfos(999); });
        t(function() { getstackinfos(1).locals.rawget("nope"); });
        t(function() { {}.rawget("nope"); });
        t(function() { {}.rawset("a", 1); (1).rawget("a"); });
        t(function() { [].rawget("nope"); });
        t(function() { (1).rawget("a"); });
        t(function() { date(-999999999999); });
        t(function() { date(99999999999999); });
        t(function() { date(0, 0); });
        t(function() { getenv(""); });
        t(function() { remove(""); });
        t(function() { rename("", ""); });
        t(function() { loadfile(""); });
        t(function() { dofile(""); });
        t(function() { loadfile("no_such_file.nut"); });
        t(function() { writeclosuretofile("", function() {}); });
        t(function() { writeclosuretofile("robust_tmp.cnut", 1); });
        t(function() { compilestring("", ""); });
        t(function() { compilestring(")))", ""); });
        t(function() { newthread(null); });
        t(function() { local th = newthread(function() {}); th.wakeup(); });
        t(function() { local th = newthread(function() {}); th.call(); th.call(); th.call(); });
        t(function() { local th = newthread(function() { suspend("s"); }); th.call(); th.call(); });
        t(function() { local th = newthread(function() { suspend("s"); }); th.call(); th.wakeupthrow("x"); });
        t(function() { local th = newthread(function() {}); th.getstackinfos(0); });
        t(function() { local th = newthread(function() {}); th.getstackinfos(99); });
        t(function() { collectgarbage(); resurrectunreachable(); });
        t(function() { local c = compilestring("return 1"); c.pcall(null); });
        t(function() { local c = function(a) {}; c.acall("notatable"); });
        t(function() { local c = function(a) {}; c.bindenv(1); });
        t(function() { local o = {}; o.setdelegate(o); });
        t(function() { local o = {}; local w = o.weakref(); w.ref().setdelegate(o); });
        t(function() { local w = ({}).weakref(); w.ref().weakref(); });
        t(function() { file("no_such_dir/x.txt", "r"); });
        t(function() { local b = blob(0); b.writen(1, 'z'); });
        t(function() { local b = blob(0); b.readn('z'); });
        t(function() { local s = stream(); s.seek(999999); s.readn('b'); });
        t(function() { local s = stream(); s.readblob(-1); });
        t(function() { local s = stream(); s.writeblob(1); });
        t(function() { local b = blob(4); local s = stream(); s.writeblob(b); s.readblob(999999); });
        t(function() { dummy(); });
        t(function() { assert(false, {}); });
        t(function() { error({}); });
        t(function() { local a = [1]; a[0].x; });
        t(function() { local n = null; n(); });
        t(function() { local n = null; n[0]; });
        t(function() { local n = null; n.x = 1; });
        t(function() { local n = null; foreach (v in n) {} });
        t(function() { local n = null; foreach (k, v in n) {} });
        t(function() { local s = "abc"; foreach (v in s) {} });
        t(function() { local i = 1; foreach (v in i) {} });
        t(function() { local o = {}; o.x.y = 1; });
        foreach (v in [blob(1), array(1), {}, [], "x", 1, 1.5, true, null, print, newthread(function(){})]) {
            t(function() use (v) { v.len(); });
            t(function() use (v) { "" + v; });
            t(function() use (v) { typeof v; });
            t(function() use (v) { clone v; });
            t(function() use (v) { v.weakref(); });
        }
        print("natives-alive\n");
        "#
    );
}

#[test]
fn malformed_bytecode_does_not_panic() {
    // 先拿到一份合法字节码，再做截断/逐字节篡改
    let mut legit: Vec<u8> = Vec::new();
    {
        let vm = Vm::new(1024);
        let mut v = vm.data_mut();
        let src = b"print(1 + 2); function f(a) { return a; }".to_vec();
        let mut pos = 0usize;
        if let Ok(proto) = squirrel::compiler::compile(
            &mut v,
            Box::new(move || {
                if pos < src.len() {
                    let c = src[pos];
                    pos += 1;
                    c as i64
                } else {
                    0
                }
            }),
            b"seed.nut",
            false,
            true,
        ) {
            if let squirrel::object::Value::FuncProto(fp) = proto {
                let clo = squirrel::closure::SQClosure::create(&v.ss, fp, None);
                if let Ok(bytes) = squirrel::serialization::write_closure(&mut v, &clo) {
                    legit = bytes;
                }
            }
        }
    }
    let mut seeds: Vec<Vec<u8>> = vec![
        vec![],
        vec![0xFA, 0xFA],
        b"\xFA\xFA\x52\x49\x51\x53".to_vec(),
        vec![0u8; 64],
        vec![0xFFu8; 64],
        b"\xFA\xFA\x52\x49\x51\x53\x01\x08\x04\x00".to_vec(),
    ];
    if !legit.is_empty() {
        for cut in 0..legit.len().min(64) {
            seeds.push(legit[..cut].to_vec());
        }
        for i in 0..legit.len().min(64) {
            let mut m = legit.clone();
            m[i] ^= 0xA5;
            seeds.push(m);
        }
        // 把 8 个 count 字段逐个改成 -1 / 极大正数：专打反序列化里的长度算术
        // （SQ_CLOSURESTREAM_PART = 0x50415254，小端字节为 54 52 41 50）
        let part = [0x54u8, 0x52, 0x41, 0x50];
        let mut marks = Vec::new();
        for i in 0..legit.len().saturating_sub(4) {
            if legit[i..i + 4] == part {
                marks.push(i + 4);
            }
        }
        if let Some(&p) = marks.get(1) {
            for k in 0..8 {
                let off = p + k * 8;
                if off + 8 > legit.len() {
                    break;
                }
                for (fill, label) in [(0xFFu8, 0u8), (0x7F, 1u8)] {
                    let mut m = legit.clone();
                    for b in 0..8 {
                        m[off + b] = fill;
                    }
                    let _ = label;
                    seeds.push(m);
                }
            }
        }
    }
    for s in &seeds {
        let vm = Vm::new(1024);
        let mut v = vm.data_mut();
        let _ = squirrel::serialization::read_closure(&mut v, s);
    }
    // 再走标准库 loadfile 的嗅探路径（与 CLI 相同）
    for s in &seeds {
        std::fs::write("robust_tmp.cnut", s).expect("write tmp");
        run_script("try { loadfile(\"robust_tmp.cnut\"); } catch (e) {}");
    }
    let _ = std::fs::remove_file("robust_tmp.cnut");
}

#[test]
fn malformed_regex_does_not_panic() {
    /// 把一个模式安全地嵌进 Squirrel 字符串字面量
    fn esc(p: &str) -> String {
        p.chars()
            .flat_map(|c| match c {
                '"' => vec!['\\', '"'],
                '\\' => vec!['\\', '\\'],
                '\n' => vec!['\\', 'n'],
                '\t' => vec!['\\', 't'],
                '\r' => vec!['\\', 'r'],
                c => vec![c],
            })
            .collect()
    }
    let patterns = [
        "", "a", "(", ")", "((", "))", "()", "(?:", "(?:)", "(?", "(?i)", "[", "[]", "[^", "[]]",
        "[a-", "[a-]", "[-a]", "[z-a]", "[\\", "\\", "\\d", "\\m", "\\m<", "\\m<>", "\\m<<", "\\x",
        "\\x4", "\\u", "*", "+", "?", "{", "}", "{}", "{1", "{1,", "{1,}", "{,1}",
        "{99999999999999}", "{0,}", "{2,1}", "a**", "a++", "a??", "a{1}{2}", "a*?b", "|", "||",
        "a|", "|a", "^", "$$", "^a$", "[[]", "(a", "a)", "\\b", "\\B", "\\Q", ".", ".*", ".+", ".?",
        "a{2,3}b", "[^]]", "a{0}", "a{0,0}", "()()()", "(|)", "((((((((((a))))))))))", "\\m<<>",
    ];
    let mut src = String::from("function t(f) { try { f(); } catch (e) {} }\n");
    for p in patterns {
        src.push_str(&format!("t(function() {{ regexp(\"{}\"); }});\n", esc(p)));
    }
    // 对若干模式喂各种（含 NUL／超长）输入串
    let inputs = [
        "", "a", "aaaa", "((((", "<a<b>c>", "\u{0}xy", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ];
    for s in inputs {
        for p in ["(a)?b|[c-f]{1,3}", "\\m<>(.*)", "a{2,}", ".*", "^$"] {
            src.push_str(&format!(
                "t(function() {{ regexp(\"{}\").match(\"{}\"); }});\n",
                esc(p),
                esc(s)
            ));
            src.push_str(&format!(
                "t(function() {{ regexp(\"{}\").capture(\"{}\"); }});\n",
                esc(p),
                esc(s)
            ));
            src.push_str(&format!(
                "t(function() {{ regexp(\"{}\").search(\"{}\", -5); }});\n",
                esc(p),
                esc(s)
            ));
        }
    }
    src.push_str("print(\"regex-alive\\n\");\n");
    run_script(&src);
}

#[test]
fn deep_values_are_released_without_stack_overflow() {
    // Rc 容器的默认 Drop 是递归的：十万层嵌套必须在迭代式释放下安全回收
    run_script(
        r#"
        local n = 100000;
        local a = [];
        for (local i = 0; i < n; i++) a = [a];
        a = null; collectgarbage();
        local t = {};
        for (local i = 0; i < n; i++) t = { inner = t };
        t = null; collectgarbage();
        class N { next = null; }
        local o = null;
        for (local i = 0; i < n; i++) { local x = N(); x.next = o; o = x; }
        o = null; collectgarbage();
        local f = null;
        for (local i = 0; i < n; i++) { local g = f; f = function() { return g; }; }
        f = null; collectgarbage();
        local gen = null;
        for (local i = 0; i < n; i++) { local h = gen; gen = function() { yield h; }; }
        gen = null; collectgarbage();
        local th = null;
        for (local i = 0; i < n; i++) { local u = th; th = newthread(function() { suspend(u); }); th.call(); }
        th = null; collectgarbage();
        local arr = [];
        for (local i = 0; i < n; i++) arr = [arr, [arr]];
        arr = null; collectgarbage();
        print("deep-values-released\n");
        "#
    );
}

/// 失控递归必须报**可捕获的** stack overflow，而不是把宿主内存吃光。
///
/// 参考实现的 `EnterFrame` 只做 `_stack.resize`，没有上限：非尾递归的无限递归会一直
/// 涨到进程被杀（实测一条 3 行脚本把宿主吃到 32GB）。本实现给 VM 栈封顶
/// （`MAX_STACK_SLOTS` / `MAX_CALL_FRAMES`），并给错误处理器留余量，
/// 保证「报得出、抓得住、内存有界」。
#[test]
fn runaway_recursion_is_catchable_and_bounded() {
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
    let _ = vm.run_buffer(
        b"function f(n) { local x = f(n + 1); return x; }\n\
          try { f(0); } catch (e) { print(\"caught:\" + e + \"\\n\"); }\n\
          print(\"after\\n\");",
        b"recursion.nut",
    );
    let s = String::from_utf8_lossy(&out.borrow()).into_owned();
    assert!(s.contains("caught:stack overflow"), "应报可捕获的 stack overflow，实际：{s:?}");
    assert!(s.contains("after"), "捕获后脚本应继续执行，实际：{s:?}");
}

/// 各条"能递归"的路径都要有界：互递归、`_call` 元方法、深栈上取调用栈信息。
#[test]
fn every_recursion_path_is_bounded() {
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
    let _ = vm.run_buffer(
        br#"
        function t(tag, f) {
            try { print(tag + "=" + f() + "\n"); }
            catch (e) { print(tag + "!" + e + "\n"); }
        }
        t("mutual", function() {
            local f, g;
            f = function(n) { local x = g(n + 1); return x; };
            g = function(n) { local x = f(n + 1); return x; };
            return f(0);
        });
        t("mt_call", function() {
            local tb = {};
            tb.setdelegate({ _call = function(...) { return tb(); } });
            return tb();
        });
        t("deep_stackinfo", function() {
            local f;
            f = function(n) {
                if (n <= 0) return typeof getstackinfos(3);
                local x = f(n - 1);
                return x;
            };
            return f(2000);
        });
        print("done\n");
        "#,
        b"paths.nut",
    );
    let s = String::from_utf8_lossy(&out.borrow()).into_owned();
    assert!(s.contains("mutual!"), "互递归必须报可捕获错误：{s:?}");
    assert!(s.contains("mt_call!"), "元方法递归必须报可捕获错误：{s:?}");
    assert!(s.contains("deep_stackinfo=table"), "深栈取信息应正常：{s:?}");
    assert!(s.contains("done"), "全部用例跑完：{s:?}");
}

#[test]
fn excessive_nesting_reports_an_error_instead_of_aborting() {
    // 超深嵌套必须变成可捕获的编译错误，而不是宿主栈溢出 abort
    run_script(
        r#"
        local r1 = false;
        try { compilestring("return " + "("*5000 + "1" + ")"*5000, "deep.nut"); }
        catch (e) { r1 = true; }
        print("parens=" + r1 + "\n");
        local blocks = "";
        for (local i = 0; i < 5000; i++) blocks += "{";
        for (local i = 0; i < 5000; i++) blocks += "}";
        local r2 = false;
        try { compilestring(blocks, "blocks.nut"); } catch (e) { r2 = true; }
        print("blocks=" + r2 + "\n");
        local conds = "return " + "1 ? " * 5000 + "1" + " : 0" * 5000 + ";";
        local r3 = false;
        try { compilestring(conds, "cond.nut"); } catch (e) { r3 = true; }
        print("conds=" + r3 + "\n");
        "#
    );
}

#[test]
fn deep_and_hostile_execution_does_not_panic() {
    run_script(
        r#"
        function t(f) { try { f(); } catch (e) {} }
        // 深递归：应由 VM 报栈溢出，而不是让宿主机栈溢出
        t(function() { local r; r = function(n) { return r(n + 1); }; r(0); });
        // 深括号表达式
        t(function() { compilestring("return " + "("*500 + "1" + ")"*500, "deep.nut"); });
        // 长委托链（含环检测）
        t(function() {
            local d = {};
            for (local i = 0; i < 300; i++) { local o = {}; o.setdelegate(d); d = o; }
            d.anything;
        });
        // 大量独立编译
        t(function() {
            for (local i = 0; i < 30; i++) compilestring("local a" + i + " = 1; return a" + i, "x.nut")();
        });
        // GC 环 + 弱引用
        t(function() {
            local a = {}; local b = { a = a }; a.b <- b;
            local w = a.weakref();
            a = null; b = null;
            collectgarbage();
            print(typeof w + " ");
        });
        // 生成器内部 resume 自身
        t(function() { local g; g = function() { yield resume g; }(); resume g; });
        // 线程互相唤醒
        t(function() {
            local a; local b;
            a = newthread(function() { wakeup b; });
            b = newthread(function() { wakeup a; });
            a.call();
        });
        // 元方法里再抛错 / 元方法返回自身
        t(function() {
            local o = {};
            o.setdelegate({ _get = function(k) { throw "mm"; }, _call = function() { return this.x; } });
            o.missing;
        });
        t(function() {
            local o = {};
            o.setdelegate({ _get = function(k) { return this; } });
            o.a.b.c.d;
        });
        print("hostile-alive\n");
        "#
    );
}

/// 容器**一次分配**的尺寸上限：超限必须是**可捕获错误**，不 panic、不挂死。
///
/// 参考实现在这些输入上自己就会死（实测，`tmp-c2/rz2_*.nut`）：
/// `array(9223372036854775807)` / `resize(0x7FFFFFFF)` → `0xC0000005` 崩溃，
/// `resize(4294967296)` → 分配几十 GB，Windows 错误报告把整台机器挂住；
/// `blob(4).readblob(-1)` → 负尺寸转无符号后巨型分配，同样挂死。
/// 本实现按健壮性承诺给可捕获的 `container too big`（`config.rs` 的
/// `MAX_CONTAINER_BYTES` = 2 GiB），负尺寸维持原有的负值错误/夹 0 语义。
/// 注意本测试必须**快速**结束（守卫在分配之前就拒绝），否则就是守卫失效。
#[test]
fn container_allocations_are_bounded_and_catchable() {
    let src = r#"
function t(tag, f) {
    try { local v = f(); print(tag + "=ok:" + (typeof v == "array" || typeof v == "instance" ? v.len() : "" + v) + "\n"); }
    catch (e) { print(tag + "!caught:" + e + "\n"); }
}
t("arr_max",     function() { return array(9223372036854775807); });
t("arr_2p31",    function() { return array(2147483647); });
t("arr_4g_fill", function() { return array(4294967296, 1); });
t("rs_max",      function() { local a = array(1); a.resize(9223372036854775807); return a; });
t("rs_2p31",     function() { local a = array(1); a.resize(2147483647); return a; });
t("rs_4g",       function() { local a = array(1); a.resize(4294967296); return a; });
t("blob_max",    function() { return blob(9223372036854775807); });
t("blob_4g",     function() { return blob(4294967296); });
t("brz_4g",      function() { local b = blob(4); b.resize(4294967296); return b; });
// negative sizes keep their existing semantics (not container too big)
t("arr_neg",     function() { return array(-1); });
t("rs_neg",      function() { local a = array(1); a.resize(-1); return a; });
t("blob_neg",    function() { return blob(-5); });
// legal sizes are unaffected
t("arr_100k",    function() { return array(100000, 7); });
t("rs_1k",       function() { local a = array(2, 1); a.resize(1000, 9); return a; });
t("blob_100k",   function() { return blob(100000); });
print("BOUND-DONE\n");
"#;
    let vm = Vm::new(1024);
    let out: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let o = out.clone();
        vm.set_print_funcs(Some(Rc::new(move |_v, b: &[u8]| {
            o.borrow_mut().extend_from_slice(b)
        })), None);
    }
    squirrel_stdlib::register_all(&mut vm.data_mut());
    let rc = vm.run_buffer(src.as_bytes(), b"bound.nut");
    assert!(rc.is_ok(), "脚本不该失败: {:?}", rc.err());
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    assert!(o.ends_with("BOUND-DONE\n"), "没跑到末尾：\n{o}");
    for tag in [
        "arr_max", "arr_2p31", "arr_4g_fill", "rs_max", "rs_2p31", "rs_4g",
        "blob_max", "blob_4g", "brz_4g",
    ] {
        let l = o
            .lines()
            .find(|l| l.starts_with(&format!("{tag}!")))
            .unwrap_or_else(|| panic!("{tag} 应该是可捕获错误：\n{o}"));
        assert!(l.contains("container too big"), "{tag} 错误类型不对：{l}");
    }
    // 负尺寸仍是原有语义
    assert!(o.lines().any(|l| l.starts_with("arr_neg!caught")), "array(-1) 应报错\n{o}");
    assert!(o.lines().any(|l| l.starts_with("rs_neg!caught:resizing to negative length")), "{o}");
    assert!(o.lines().any(|l| l.starts_with("blob_neg!caught:cannot create blob with negative size")), "{o}");
    // 合法尺寸照常工作
    for tag in ["arr_100k", "rs_1k", "blob_100k"] {
        assert!(
            o.lines().any(|l| l.starts_with(&format!("{tag}=ok:"))),
            "{tag} 应当成功：\n{o}"
        );
    }
}

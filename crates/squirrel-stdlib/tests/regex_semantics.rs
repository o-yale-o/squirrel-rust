//! 正则语义回归（↔ sqstdlib/sqstdrex.cpp 引擎）。
//!
//! 引擎是**逐行移植**，包括原版那些"不像正则"的怪癖——脚本能观察到它们，
//! 所以必须照搬（期望值全部用参考实现 `build-ref/bin/Release/sq_static.exe` 实测确认）：
//!
//! | 形态 | 参考实现 | 说明 |
//! |---|---|---|
//! | `<.*>` 搜 `"<a><b>"` | `{0,3}` | 贪婪量词每吃一个字符就试一次后继，一旦后继能匹配就返回 |
//! | `\bfoo\b` 搜 `"a foo b"` | 不匹配 | `\b` 是「空格↔非空格转移」，不是词边界 |
//! | `^b` 带 start=1 搜 `"ab"` | `{1,2}` | `^` 锚定的是本次 search 的 start 偏移 |
//! | `""` 上搜 `.*` | 不匹配 | `searchrange` 开头 `if(text_begin >= text_end) return false` |
//! | `a*?b` | 字面量 `?` | 引擎没有惰性量词，量词后的 `?` 当普通字符 |
//! | `[z-a]` / `[\d-z]` / `[a-` | `invalid range` | 范围检查用的是节点 type 与下一个字符的比较 |
//! | `a{1000000000}` | `overflow in numeric constant` | `parsenumber` 的位数溢出检查 |
//! | 部分匹配后失败 | 不回退到首个节点 | search 的 `node` 变量在 do-while 之外 |
//!
//! 另外固化两个**内存有界**的回归（参考实现靠引用计数即时回收，本实现是 `Rc` + GC 链，
//! 曾经把字符串实习表的键留成永久垃圾：`s += "x"` 两万次峰值 232MB）。

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
    let rc = vm.run_buffer(src.as_bytes(), b"rx.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 引擎怪癖：贪婪量词、`\b`、`^` 锚点、空主语、惰性量词写法
#[test]
fn engine_quirks_match_reference() {
    let src = r#"
        function d(x) {
            if (typeof x == "table") return "{" + x.begin + "," + x.end + "}";
            if (typeof x == "array") { local o = "["; for (local i = 0; i < x.len(); i++) { if (i) o += ","; o += "{" + x[i].begin + "," + x[i].end + "}"; } return o + "]"; }
            return "" + x;
        }
        print(d(regexp("<.*>").search("<a><b>")) + "\n");          // 贪婪其实是"第一个能停的位置"
        print(d(regexp("<.*?>").search("<a><b>")) + "\n");          // ? 是字面量 → 不匹配
        print(d(regexp("\\bfoo\\b").search("a foo b")) + "\n");     // 不是词边界
        print(d(regexp("\\b").search("a b")) + "\n");               // 只在转移处匹配（这里是位置 0）
        print(d(regexp("^b").search("ab", 1)) + "\n");              // ^ 锚定 start 偏移
        print(d(regexp("a$").search("aba")) + "\n");
        print(d(regexp(".*").search("")) + "\n");                   // 空主语：不匹配
        print(regexp(".*").match("") + "\n");                       // 但 match 是整串匹配
        print(d(regexp("a*?b").search("a?b")) + "\n");              // ? 是字面量
        print(d(regexp("(a)+").capture("aaa")) + "\n");             // 重复分组记第一次
        print(regexp("(a)(b)").subexpcount() + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "{0,3}\nnull\nnull\n{0,0}\n{1,2}\n{2,3}\nnull\ntrue\n{0,3}\n[{0,3},{0,1}]\n3\n"
    );
}

/// 编译期错误文本（含只有原版才会报的 `invalid range` 系列）
#[test]
fn compile_errors_match_reference() {
    let src = r#"
        function e(p) { try { regexp(p); return "ok"; } catch (err) { return "" + err; } }
        print(e("(") + "\n");
        print(e("[") + "\n");
        print(e("[]") + "\n");
        print(e("[z-a]") + "\n");
        print(e("[\\d-z]") + "\n");
        print(e("a{") + "\n");
        print(e("a{,") + "\n");
        print(e("a{2,1}") + "\n");
        print(e("a{1000000000}") + "\n");
        print(e("\\") + "\n");
        print(e("(?<a>") + "\n");
        print(e("(?a)") + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "letter expected\nletter expected\nempty class\ninvalid range\ninvalid range\n\
         number expected\nnumber expected\nok\noverflow in numeric constant\n\
         letter expected for argument of escape sequence\nexpected paren\nexpected paren\n"
    );
}

/// search 起点偏移、capture 形状、match 语义
#[test]
fn search_offsets_and_captures_match_reference() {
    let src = r#"
        function d(x) {
            if (typeof x == "table") return "{" + x.begin + "," + x.end + "}";
            if (typeof x == "array") { local o = "["; for (local i = 0; i < x.len(); i++) { if (i) o += ","; o += "{" + x[i].begin + "," + x[i].end + "}"; } return o + "]"; }
            return "" + x;
        }
        print(d(regexp("a").search("aab", 1)) + "/" + d(regexp("a").search("aab", 2)) + "\n");
        print(d(regexp("a").search("abc", 3)) + "/" + d(regexp("a").search("abc", 4)) + "\n");
        print(d(regexp("(a)(b)?").capture("ac")) + "\n");     // 未匹配组是 {0,0}
        print(d(regexp("(a)(b)?").capture("abc")) + "\n");
        print(regexp("abc").match("abc") + "/" + regexp("abc").match("xabc") + "\n");
        print(typeof regexp("a") + "/" + regexp("(a)").subexpcount() + "\n");
        try { regexp("a").constructor("b"); print("no\n"); } catch (e) { print("" + e + "\n"); }
        try { regexp("a").search(5); print("no\n"); } catch (e) { print("" + e + "\n"); }
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "{1,2}/null\nnull/null\n[{0,1},{0,1},{0,0}]\n[{0,2},{0,1},{1,2}]\ntrue/false\nregexp/2\n\
         invalid regexp object\nparameter 1 has an invalid type 'integer' ; expected: 'string'\n"
    );
}

/// 字符串 churn 不能让实习表留成永久垃圾（曾经：两万次拼接峰值 232MB）
#[test]
fn string_table_stays_bounded_under_churn() {
    let vm = Vm::new(1024);
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    let rc = vm.run_buffer(
        br#"
        local s = "";
        for (local i = 0; i < 20000; i++) s += "x";
        assert(s.len() == 20000);
        for (local i = 0; i < 20000; i++) { local t = "v" + i; }
        "#,
        b"churn.nut",
    );
    assert!(rc.is_ok(), "churn 脚本失败: {:?}", rc.err());
    let n = vm.data_mut().ss.stringtable.len();
    // 实习表按"插入字节数"清死条目：两万次长串拼接总插入 ~200MB，
    // 若不清就是 2 万个条目，这里留有界上限（远小于未清理时的规模）
    assert!(n < 8192, "字符串实习表条目数失控: {}", n);
    let chain = vm.data_mut().ss.gc_chain.borrow().len();
    assert!(chain < 65536 * 2, "GC 注册表条目数失控: {}", chain);
}

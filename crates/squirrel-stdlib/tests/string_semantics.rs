//! string 库全局函数语义回归（`sqstdlib/sqstdstring.cpp`）。
//!
//! 这一族在本轮之前**完全没有覆盖**，原因是个容易踩的坑：
//! `strip/lstrip/rstrip/split/escape/startswith/endswith/format/printf` 是
//! **string 库的全局函数**（第一个参数是串），**不是字符串的成员方法** ——
//! `"abc".startswith("a")` 在两个引擎里都报 `the index 'startswith' does not exist`。
//! 按委托表逐个成员生成的矩阵因此碰不到它们。
//!
//! 语料侧是 `tests-corpus/t_delmatrix.nut` 的 `m_s_*` 段（两侧逐字节一致）；
//! 这里固化其中**最容易回归**的语义，尤其是本轮修出来的两处真实差异：
//!
//! 1. **`split` 按 C 字符串扫描**：`while(*end != '\0')`，遇到嵌入 NUL 就停，
//!    尾段也用停下来的 `end` —— 于是 `split("a\x00,b", ",")` 是 `["a"]` 而不是
//!    `["a\x00", "b"]`；
//! 2. **`lstrip` 用 `sq_pushstring(v, start, -1)`**：负长度走 `strlen(start)`，
//!    结果在第一个嵌入 NUL 处截断 —— `lstrip(" a\x00b")` 是 `"a"` 而不是 `"a\x00b"`。
//!
//! 这两条都不是 UB（串本身合法），是参考实现用 C 字符串处理 `SQString` 的确定行为，
//! 按"保真优先"照做。注意 `rstrip`/`strip`/`startswith`/`escape` 走的是显式长度，
//! **不受** NUL 影响 —— 测试里一并钉住，免得以后"顺手统一"改错。
//!
//! 另注：`print` 在嵌入 NUL 处截断，两个引擎一致（所以语料里含 NUL 的用例
//! 用 `lens()` 打印各元素字节长度来观测）。

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
    let rc = vm.run_buffer(src.as_bytes(), b"strlib.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 把 `lens()` 风格的观测串起来跑：每行 `tag=值`
fn run_lines(body: &str) -> String {
    let src = format!(
        r#"
function lens(a) {{
    local s = "" + a.len() + ":";
    for (local i = 0; i < a.len(); i++) {{ if (i > 0) s += ","; s += ("" + a[i]).len(); }}
    return s;
}}
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

/// `split` 遇嵌入 NUL 停止（参考实现按 C 字符串扫描）
#[test]
fn split_stops_at_embedded_nul() {
    let out = run_lines(
        r#"
t("a", function() { return lens(split("a\x00,b", ",")); });
t("b", function() { return lens(split("a\x00,b", ",", true)); });
t("c", function() { return lens(split("a\x00", ",")); });
t("d", function() { return lens(split("\x00", ",")); });
t("e", function() { return lens(split("a\x00b", "\x00")); });
t("f", function() { return lens(split("a,b,c", ",")); });
t("g", function() { return lens(split(",a", ",")); });
t("h", function() { return lens(split("", ",")); });
"#,
    );
    assert_eq!(line(&out, "a"), "a=1:1", "NUL 之后整段丢掉");
    assert_eq!(line(&out, "b"), "b=1:1");
    assert_eq!(line(&out, "c"), "c=1:1");
    assert_eq!(line(&out, "d"), "d=0:");
    assert_eq!(line(&out, "e"), "e=1:1", "分隔串是 NUL 时同样先停在 NUL");
    assert_eq!(line(&out, "f"), "f=3:1,1,1", "无 NUL 时按长度正常切");
    assert_eq!(line(&out, "g"), "g=2:0,1", "首段为空仍入结果");
    assert_eq!(line(&out, "h"), "h=0:");
}

/// `lstrip` 结果在第一个嵌入 NUL 处截断（`sq_pushstring(..., -1)` → `strlen`）
#[test]
fn lstrip_truncates_at_embedded_nul() {
    let out = run_lines(
        r#"
t("ls", function() { return lstrip(" a\x00b").len(); });
t("ls_val", function() { return lstrip(" a\x00b"); });
t("ls_ws", function() { return lstrip("   ").len(); });
t("ls_plain", function() { return lstrip("  a  "); });
t("rs", function() { return rstrip("a\x00b ").len(); });
t("rs_nul_at1", function() { return rstrip("a\x00b ")[1]; });
t("st", function() { return strip("  a\x00b  ").len(); });
t("st_nul_at1", function() { return strip("  a\x00b  ")[1]; });
t("st_lead", function() { return strip(" \x00a").len(); });
t("st_trail", function() { return strip("a\x00  ").len(); });
t("st_all_ws", function() { return strip("   ").len(); });
"#,
    );
    assert_eq!(line(&out, "ls"), "ls=1", "lstrip 走 strlen，NUL 处截断");
    assert_eq!(line(&out, "ls_val"), "ls_val=a");
    assert_eq!(line(&out, "ls_ws"), "ls_ws=0");
    assert_eq!(line(&out, "ls_plain"), "ls_plain=a  ");
    assert_eq!(line(&out, "rs"), "rs=3", "rstrip 用显式长度，不吃 NUL 截断");
    assert_eq!(line(&out, "rs_nul_at1"), "rs_nul_at1=0", "NUL 仍在（截断的话这里是 'b'=98）");
    assert_eq!(line(&out, "st"), "st=3", "strip 用 end-start 显式长度：\"a\\0b\"");
    assert_eq!(line(&out, "st_nul_at1"), "st_nul_at1=0");
    assert_eq!(line(&out, "st_lead"), "st_lead=2");
    assert_eq!(line(&out, "st_trail"), "st_trail=2");
    assert_eq!(line(&out, "st_all_ws"), "st_all_ws=0");
}

/// `startswith`/`endswith`/`escape` 都按显式长度处理，不受 NUL 影响；
/// 且 `startswith/endswith` 是**全局函数**、严格 3 参数
#[test]
fn string_lib_globals_are_length_based_and_strict() {
    let out = run_lines(
        r#"
t("sw", function() { return "" + startswith("a\x00b", "a\x00"); });
t("sw2", function() { return "" + startswith("a\x00b", "a\x00b"); });
t("sw3", function() { return "" + startswith("a\x00b", "a\x00bc"); });
t("sw_empty", function() { return "" + startswith("hello", ""); });
t("sw_case", function() { return "" + startswith("Hello", "he"); });
t("ew", function() { return "" + endswith("a\x00b", "\x00b"); });
t("ew_empty", function() { return "" + endswith("", ""); });
t("esc_len", function() { return escape("a\x00b").len(); });
t("esc_nul", function() { return escape("a\x00b"); });
t("esc_tab", function() { return escape("a\tb"); });
t("esc_empty", function() { return escape("").len(); });
t("arity", function() { return "" + startswith("abc"); });
t("type", function() { return "" + startswith("abc", 1); });
t("method", function() { return "" + "abc".startswith("a"); });
"#,
    );
    assert_eq!(line(&out, "sw"), "sw=true", "startswith 用 memcmp + 显式长度");
    assert_eq!(line(&out, "sw2"), "sw2=true");
    assert_eq!(line(&out, "sw3"), "sw3=false", "模式比自身长 → false");
    assert_eq!(line(&out, "sw_empty"), "sw_empty=true", "空模式恒真");
    assert_eq!(line(&out, "sw_case"), "sw_case=false");
    assert_eq!(line(&out, "ew"), "ew=true");
    assert_eq!(line(&out, "ew_empty"), "ew_empty=true");
    assert_eq!(line(&out, "esc_len"), "esc_len=4", "NUL 转义成 \\0 两个字符");
    assert_eq!(line(&out, "esc_nul"), "esc_nul=a\\0b");
    assert_eq!(line(&out, "esc_tab"), "esc_tab=a\\x09b", "\\t 非 printable，走 \\xNN");
    assert_eq!(line(&out, "esc_empty"), "esc_empty=0");
    assert_eq!(line(&out, "arity"), "arity!wrong number of parameters");
    assert_eq!(
        line(&out, "type"),
        "type!parameter 2 has an invalid type 'integer' ; expected: 'string'"
    );
    // 它们不是字符串成员方法 —— 这条差异最容易被"顺手做成方法"改坏
    assert_eq!(line(&out, "method"), "method!the index 'startswith' does not exist");
}

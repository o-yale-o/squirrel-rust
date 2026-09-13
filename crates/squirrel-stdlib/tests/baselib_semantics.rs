//! base 库回归（`squirrel/sqbaselib.cpp`）。
//!
//! 前 5 个测试覆盖 `tools/global_coverage.py` 扫出来的、62 个已注册全局里
//! **仅有的**语料 0 覆盖项：`assert` / `error` / `array` / `type` / `dummy`。
//! 补齐后矩阵 `tests-corpus/t_baselibmap.nut`（90 行，stdout 与 stderr 都逐字节
//! 一致）与参考实现**没有差异**，属于验证型收获；测试把这些容易想当然的点钉住：
//!
//! 1. **`assert` 的判假是 `SQVM::IsFalse`：`null` / `false` / `0` / `0.0` 为假，
//!    而 `""` / `[]` / `{}` / `function(){}` / `blob(0)` 都是真。**
//!    （写之前我以为"只认 null/false"，实测 `assert(0)` 会失败 —— 所以这条尤其值得钉。）
//! 2. `assert(false, msg)` 先 `sq_tostring(msg)`：`123` → 错误文本 `123`，
//!    拿不到字符串才退回 `assertion failed`；`sq_tostring` 对 table/array 给的是
//!    `(table : 0x…)`，**不调 `_tostring`**。
//! 3. **`error(msg)` 不抛异常**：只调 errorfunc（打到 stderr），返回 `null`。
//! 4. `array(size[, fill])`：`tointeger(size)` 截断（`array(2.9)` 长度 2、
//!    `array(0.5)` 空），多余实参忽略；`array` 没有 `getclass`。
//! 5. `type` 就是 `typeof` 的别名；blob/regexp 都是 `instance`；
//!    **非引用类型的 `weakref()` 返回自身**（`(5).weakref()` 还是 integer）。
//! 6. `dummy(...)` 的 `nparams=0` 表示**不做参数个数检查**，任何参数都返回 `null`。
//!
//! 第 6 个测试（`string_to_number_follows_crt_str2num`）是**修差异**型：语料里
//! 的 `tointeger`/`tofloat` 用例全是参数掩码测试，"字符串解析"是结构性空白，
//! 于是 `str2num`（↔ `sqbaselib.cpp::str2num`）里的溢出饱和、`base > 13` 的
//! `'e'` 语义、NUL 截断、CRT `strtod` 的十六进制浮点/`inf`/`nan` 都没被测到 ——
//! 详见该测试的文档注释与 `MIGRATION.md`。

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
    let rc = vm.run_buffer(src.as_bytes(), b"baselibmap.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 每行 `tag=值|类型` 或 `tag!错误`（与语料 `t_baselibmap.nut` 同一口径）
fn run_lines(body: &str) -> String {
    let src = format!(
        r#"
function show(r) {{
    local t = typeof r;
    if (t == "array") {{
        local s = "[";
        for (local i = 0; i < r.len(); i++) {{ if (i > 0) s += ","; s += show(r[i]); }}
        return s + "]";
    }}
    if (t == "table") {{ local s = "{{"; foreach (k, v in r) s += "" + k + ":" + show(v) + ","; return s + "}}"; }}
    if (t == "string") return "\"" + r + "\"";
    return "" + r;
}}
function t(tag, f) {{
    try {{ print(tag + "=" + show(f()) + "|" + typeof f() + "\n"); }}
    catch (e) {{ print(tag + "!" + e + "\n"); }}
}}
class TypeProbeClass {{}}
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

/// `assert` 的假值集合：null / false / 0 / 0.0 失败，"" / [] / {} 通过
#[test]
fn assert_uses_squirrel_isfalse() {
    let out = run_lines(
        r#"
t("true", function() { assert(true); return "passed"; });
t("false", function() { assert(false); return "passed"; });
t("null", function() { assert(null); return "passed"; });
t("i0", function() { assert(0); return "passed"; });
t("i1", function() { assert(1); return "passed"; });
t("f0", function() { assert(0.0); return "passed"; });
t("f1", function() { assert(1.5); return "passed"; });
t("es", function() { assert(""); return "passed"; });
t("arr_e", function() { assert([]); return "passed"; });
t("tbl_e", function() { assert({}); return "passed"; });
t("fn", function() { assert(function() {}); return "passed"; });
t("blob", function() { assert(blob(0)); return "passed"; });
t("ret", function() { return assert(true); });
t("noargs", function() { assert(); return "passed"; });
t("manyargs", function() { assert(true, "a", "b", "c"); return "passed"; });
"#,
    );
    assert_eq!(line(&out, "true"), "true=\"passed\"|string");
    // 数值 0 是假 —— 这条最容易写反
    assert_eq!(line(&out, "false"), "false!assertion failed");
    assert_eq!(line(&out, "null"), "null!assertion failed");
    assert_eq!(line(&out, "i0"), "i0!assertion failed");
    assert_eq!(line(&out, "f0"), "f0!assertion failed");
    assert_eq!(line(&out, "i1"), "i1=\"passed\"|string");
    assert_eq!(line(&out, "f1"), "f1=\"passed\"|string");
    // 空串/空容器/函数/blob 都是真
    assert_eq!(line(&out, "es"), "es=\"passed\"|string");
    assert_eq!(line(&out, "arr_e"), "arr_e=\"passed\"|string");
    assert_eq!(line(&out, "tbl_e"), "tbl_e=\"passed\"|string");
    assert_eq!(line(&out, "fn"), "fn=\"passed\"|string");
    assert_eq!(line(&out, "blob"), "blob=\"passed\"|string");
    assert_eq!(line(&out, "ret"), "ret=null|null", "assert 通过时不返回值");
    assert_eq!(line(&out, "noargs"), "noargs!wrong number of parameters");
    assert_eq!(line(&out, "manyargs"), "manyargs=\"passed\"|string", "多给的实参被忽略");
}

/// 断言消息先 `sq_tostring`；拿不到字符串则退回默认文本
#[test]
fn assert_message_is_stringified() {
    let src = r#"
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); } catch (e) { print(tag + "!" + e + "\n"); }
}
t("s", function() { assert(false, "boom"); return "passed"; });
t("i", function() { assert(false, 123); return "passed"; });
t("f", function() { assert(false, 1.5); return "passed"; });
t("null", function() { assert(false, null); return "passed"; });
t("true", function() { assert(false, true); return "passed"; });
t("int_zero", function() { assert(0, 123); return "passed"; });
t("tostring_mt", function() { local o = { _tostring = function() { return "custom"; } }; assert(false, o); return "passed"; });
t("last_wins", function() { assert(false, "a", "b"); return "passed"; });
"#;
    let out = run_capture(src).0;
    assert_eq!(line(&out, "s"), "s!boom");
    assert_eq!(line(&out, "i"), "i!123", "数字被 tostring 成错误文本");
    assert_eq!(line(&out, "f"), "f!1.5");
    assert_eq!(line(&out, "null"), "null!null");
    assert_eq!(line(&out, "true"), "true!true");
    assert_eq!(line(&out, "int_zero"), "int_zero!123", "0 也是假");
    // sq_tostring 不调 _tostring，给的是容器描述（地址部分不比较）
    let l = line(&out, "tostring_mt");
    assert!(
        l.starts_with("tostring_mt!(table : 0x") && l.ends_with(')'),
        "sq_tostring 不该调 _tostring：{l}"
    );
    assert_eq!(line(&out, "last_wins"), "last_wins!a", "只用第 2 个参数当消息");
}

/// `error()` 只调 errorfunc，不抛异常，返回 null
#[test]
fn error_prints_and_returns_null() {
    let (out, err) = run_capture(
        r#"
function t(tag, f) {
    try { print(tag + "=" + ("" + f()) + "\n"); } catch (e) { print(tag + "!" + e + "\n"); }
}
t("s", function() { return error("boom"); });
t("i", function() { return error(123); });
t("null", function() { return error(null); });
t("after", function() { return "still running"; });
"#,
    );
    assert_eq!(line(&out, "s"), "s=null", "error 不抛异常，返回 null");
    assert_eq!(line(&out, "i"), "i=null");
    assert_eq!(line(&out, "null"), "null=null");
    assert_eq!(line(&out, "after"), "after=still running", "脚本继续执行");
    assert!(err.contains("boom"), "errorfunc 应该收到消息：{err:?}");
    assert!(err.contains("123"), "数字消息也要打出来：{err:?}");
}

/// `array(size[, fill])` 的截断、填充与错误路径
#[test]
fn array_constructor_semantics() {
    let out = run_lines(
        r#"
t("z0", function() { return array(0); });
t("z3", function() { return array(3); });
t("fill", function() { return array(3, 7); });
t("fill_s", function() { return array(2, "x"); });
t("frac", function() { return array(2.9); });
t("frac_small", function() { return array(0.5); });
t("extra", function() { return array(2, 5, 6); });
t("len", function() { return array(3, 7).len(); });
t("none", function() { return array(); });
t("str", function() { return array("x"); });
t("null", function() { return array(null); });
t("getclass", function() { return array(1).getclass(); });
"#,
    );
    assert_eq!(line(&out, "z0"), "z0=[]|array");
    assert_eq!(line(&out, "z3"), "z3=[null,null,null]|array");
    assert_eq!(line(&out, "fill"), "fill=[7,7,7]|array");
    assert_eq!(line(&out, "fill_s"), "fill_s=[\"x\",\"x\"]|array");
    assert_eq!(line(&out, "frac"), "frac=[null,null]|array", "2.9 截断成 2");
    assert_eq!(line(&out, "frac_small"), "frac_small=[]|array");
    assert_eq!(line(&out, "extra"), "extra=[5,5]|array", "多余的实参被忽略");
    assert_eq!(line(&out, "len"), "len=3|integer");
    assert_eq!(line(&out, "none"), "none!wrong number of parameters");
    assert_eq!(
        line(&out, "str"),
        "str!parameter 1 has an invalid type 'string' ; expected: 'integer|float'"
    );
    assert_eq!(
        line(&out, "null"),
        "null!parameter 1 has an invalid type 'null' ; expected: 'integer|float'"
    );
    assert_eq!(line(&out, "getclass"), "getclass!the index 'getclass' does not exist");
}

/// `type` 是 `typeof` 的别名；非引用类型的 `weakref()` 返回自身
#[test]
fn type_and_dummy() {
    let out = run_lines(
        r#"
t("null", function() { return type(null); });
t("int", function() { return type(1); });
t("float", function() { return type(1.5); });
t("bool", function() { return type(true); });
t("str", function() { return type("x"); });
t("arr", function() { return type([]); });
t("tbl", function() { return type({}); });
t("fn", function() { return type(function() {}); });
t("natfn", function() { return type(print); });
t("blob", function() { return type(blob(1)); });
t("cls", function() { return type(TypeProbeClass); });
t("inst", function() { return type(TypeProbeClass()); });
t("gen", function() { return type((function() { yield 1; }())); });
t("th", function() { return type(newthread(function() {})); });
t("re", function() { return type(regexp("a")); });
t("wref_int", function() { local i = 5; return type(i.weakref()) + "/" + i.weakref(); });
t("wref_real", function() { local tt = {}; return type(tt.weakref()); });
t("bare", function() { return type(); });
t("extra", function() { return type(1, 2); });
t("d_none", function() { return dummy(); });
t("d_many", function() { return dummy(1, 2, 3, "x", null); });
t("d_type", function() { return typeof dummy; });
"#,
    );
    assert_eq!(line(&out, "null"), "null=\"null\"|string");
    assert_eq!(line(&out, "int"), "int=\"integer\"|string");
    assert_eq!(line(&out, "float"), "float=\"float\"|string");
    assert_eq!(line(&out, "bool"), "bool=\"bool\"|string");
    assert_eq!(line(&out, "str"), "str=\"string\"|string");
    assert_eq!(line(&out, "arr"), "arr=\"array\"|string");
    assert_eq!(line(&out, "tbl"), "tbl=\"table\"|string");
    assert_eq!(line(&out, "fn"), "fn=\"function\"|string");
    assert_eq!(line(&out, "natfn"), "natfn=\"function\"|string");
    assert_eq!(line(&out, "blob"), "blob=\"instance\"|string", "blob 是 instance");
    assert_eq!(line(&out, "cls"), "cls=\"class\"|string");
    assert_eq!(line(&out, "inst"), "inst=\"instance\"|string");
    assert_eq!(line(&out, "gen"), "gen=\"generator\"|string");
    assert_eq!(line(&out, "th"), "th=\"thread\"|string");
    assert_eq!(line(&out, "re"), "re=\"instance\"|string", "regexp 也是 instance");
    assert_eq!(line(&out, "wref_int"), "wref_int=\"integer/5\"|string", "非引用类型返回自身");
    assert_eq!(line(&out, "wref_real"), "wref_real=\"weakref\"|string");
    assert_eq!(line(&out, "bare"), "bare!wrong number of parameters");
    assert_eq!(line(&out, "extra"), "extra!wrong number of parameters");
    assert_eq!(line(&out, "d_none"), "d_none=null|null");
    assert_eq!(line(&out, "d_many"), "d_many=null|null", "dummy 不检查参数个数");
    assert_eq!(line(&out, "d_type"), "d_type=\"function\"|string");
}

/// **字符串 → 数字**（`default_delegate_tointeger/-tofloat` ↔ `sqbaselib.cpp` 的 `str2num`）。
///
/// 语料 `t_delmatrix.nut` 里的 `tointeger`/`tofloat` 用例全是**参数掩码**测试，
/// 字符串**解析**本身是结构性空白；新矩阵 `tests-corpus/t_tonummap.nut`
/// （1152 个字符串 × 3 种观测 + base 矩阵，3859 行）把这四条语义钉住：
///
/// 1. **溢出按符号饱和**（MSVC `_strtoi64`），不是回绕：
///    `"9223372036854775808"`→INT64_MAX、`"-9223372036854775809"`→INT64_MIN。
///    注意饱和由**符号**决定 —— 负号 + 超出量级也返回 INT64_MIN，而不是"往回绕"。
/// 2. **`tofloat` 也走整数路径**：字符串里没有 `.`/`e` 就先当整数解析，
///    所以 `"99999999999999999999999999".tofloat()` 是 `9.2233720368547758e+18`
///    （饱和后的 INT64_MAX），**不是** 1e26。
/// 3. **扫描在第一个 NUL 处停止**（C 字符串）：`"12\x00.5"` 走整数路径。
/// 4. **`base` 的 `iseintbase = base > 13`**：base 16 时 `'e'` 是数字，
///    `"1e3".tointeger(16)` = 483；base 14 及以上同理，base 13 仍是科学计数法。
///    base 只吃 0 或 2..36（1/负数/>36 会让参考实现直接崩溃，本实现报可捕获错误）。
///
/// 解析器还复刻了 CRT `strtod` 的十进制/十六进制浮点/`inf`/`nan` 三条分支，
/// 连 NaN 的**位模式**都对齐（MSVC 给全 1 尾数 → 打印 `nan`/`-nan`，
/// 而 `0.0/0.0` 的 x86 indefinite 是 `-nan(ind)`）。
#[test]
fn string_to_number_follows_crt_str2num() {
    let src = format!(
        r#"
function obs(v) {{
    local ty = typeof v;
    if (ty == "float") return format("%.17g", v);
    return "" + v;
}}
function t(tag, f) {{
    try {{ local v = f(); print(tag + "=" + obs(v) + "|" + typeof v + "\n"); }}
    catch (e) {{ print(tag + "!" + e + "\n"); }}
}}
{}
"#,
        r#"
t("plain", function() { return "42".tointeger(); });
t("spaces", function() { return "  42  ".tointeger(); });
t("plus", function() { return "+42".tointeger(); });
t("trailing", function() { return "42abc".tointeger(); });
t("hex_like_base10", function() { return "0x10".tointeger(); });
t("hex_base16", function() { return "0x10".tointeger(16); });
t("ff_base16", function() { return "ff".tointeger(16); });
t("ff_base15", function() { return "ff".tointeger(15); });
t("ff_base14", function() { return "ff".tointeger(14); });
t("e3_base16", function() { return "1e3".tointeger(16); });
t("e3_base15", function() { return "1e3".tointeger(15); });
t("e3_base14", function() { return "1e3".tointeger(14); });
t("e3_base13", function() { return "1e3".tointeger(13); });
t("e3_base10", function() { return "1e3".tointeger(); });
t("e3_float", function() { return "1e3".tofloat(); });
t("dot_hex", function() { return "1.5".tointeger(16); });
t("base0_hex", function() { return "0x10".tointeger(0); });
t("base0_oct", function() { return "010".tointeger(0); });
t("base2", function() { return "101".tointeger(2); });
t("base2_bad", function() { return "2".tointeger(2); });
t("base36", function() { return "z".tointeger(36); });
t("base37_crash", function() { return "ff".tointeger(37); });
t("base1_crash", function() { return "ff".tointeger(1); });

t("ov_max", function() { return "9223372036854775807".tointeger(); });
t("ov_max1", function() { return "9223372036854775808".tointeger(); });
t("ov_min", function() { return "-9223372036854775808".tointeger(); });
t("ov_min1", function() { return "-9223372036854775809".tointeger(); });
t("ov_2p64", function() { return "18446744073709551616".tointeger(); });
t("ov_neg2p64", function() { return "-18446744073709551616".tointeger(); });
t("ov_long", function() { return "99999999999999999999999999".tointeger(); });
t("ov_long_float", function() { return "99999999999999999999999999".tofloat(); });
t("ov_hex16", function() { return "ffffffffffffffff".tointeger(16); });
t("ov_hex16_neg", function() { return "-ffffffffffffffff".tointeger(16); });

t("nul_int", function() { return "12\x00.5".tointeger(); });
t("nul_dot_only", function() { return ".5\x0012".tointeger(); });
t("nul_e", function() { return "12\x00e5".tointeger(); });

t("hexfloat", function() { return "0x1.8p3".tofloat(); });
t("hexfloat_noexp", function() { return "0x1.8".tofloat(); });
t("hexfloat_frac", function() { return "0x.8p3".tofloat(); });
t("hexfloat_partial_p", function() { return "0x1.8p+".tofloat(); });
t("hexfloat_dot_only", function() { return "0x.p1".tofloat(); });
t("hexfloat_big", function() { return "0x1.fffffffffffffp1023".tofloat(); });
t("hexfloat_tiny", function() { return "0x1p-1074".tofloat(); });

t("inf_dot", function() { return "inf.0".tofloat(); });
t("ninf_dot", function() { return "-inf.0".tofloat(); });
t("infinity_e", function() { return "infinity.e".tofloat(); });
t("nan_dot", function() { return "nan.0".tofloat(); });
t("nnan_dot", function() { return "-nan.0".tofloat(); });
t("inf_plain", function() { return "inf".tofloat(); });
t("inf_toint", function() { return "inf.0".tointeger(); });
t("nan_toint", function() { return "-nan.0".tointeger(); });
t("nan_bits", function() { local b = blob(4); b.writen("nan.0".tofloat(), 'f'); b.seek(0, 'b'); local s = ""; for (local i = 0; i < 4; i++) s += format("%02x", b.readn('b')); return s; });
t("nnan_bits", function() { local b = blob(4); b.writen("-nan.0".tofloat(), 'f'); b.seek(0, 'b'); local s = ""; for (local i = 0; i < 4; i++) s += format("%02x", b.readn('b')); return s; });
t("div00_bits", function() { local b = blob(4); b.writen(0.0 / 0.0, 'f'); b.seek(0, 'b'); local s = ""; for (local i = 0; i < 4; i++) s += format("%02x", b.readn('b')); return s; });
"#,
    );
    let out = run_capture(&src).0;
    macro_rules! chk {
        ($tag:expr, $want:expr) => {
            assert_eq!(line(&out, $tag), $want, "{}", $tag)
        };
        ($tag:expr, $want:expr, $msg:expr) => {
            assert_eq!(line(&out, $tag), $want, "{}: {}", $tag, $msg)
        };
    }
    chk!("plain", "plain=42|integer");
    chk!("spaces", "spaces=42|integer");
    chk!("plus", "plus=42|integer");
    chk!("trailing", "trailing=42|integer");
    chk!("hex_like_base10", "hex_like_base10=0|integer", "base 10 下 0x 只是前缀");
    chk!("hex_base16", "hex_base16=16|integer");
    chk!("ff_base16", "ff_base16=255|integer");
    chk!(
        "ff_base15",
        "ff_base15!cannot convert the string",
        "'f'=15 不是合法的 base15 数字 → 一个数字都没读到"
    );
    chk!("ff_base14", "ff_base14!cannot convert the string");
    chk!(
        "e3_base16",
        "e3_base16=483|integer",
        "base>13 → 'e' 当数字（iseintbase）"
    );
    chk!("e3_base15", "e3_base15=438|integer");
    chk!("e3_base14", "e3_base14=1|integer", "'e'=14 在 base14 里非法 → 停在 '1'");
    chk!("e3_base13", "e3_base13=1000|integer", "base=13 仍是科学计数法");
    chk!("e3_base10", "e3_base10=1000|integer");
    chk!("e3_float", "e3_float=1000|float");
    chk!("dot_hex", "dot_hex=1|integer", "'.' 一定走浮点路径");
    chk!("base0_hex", "base0_hex=16|integer");
    chk!("base0_oct", "base0_oct=8|integer");
    chk!("base2", "base2=5|integer");
    chk!("base2_bad", "base2_bad!cannot convert the string");
    chk!("base36", "base36=35|integer");
    chk!("base37_crash", "base37_crash!cannot convert the string", "参考实现在这里崩溃");
    chk!("base1_crash", "base1_crash!cannot convert the string");
    chk!("ov_max", "ov_max=9223372036854775807|integer");
    chk!("ov_max1", "ov_max1=9223372036854775807|integer", "饱和，不是回绕");
    chk!("ov_min", "ov_min=-9223372036854775808|integer");
    chk!("ov_min1", "ov_min1=-9223372036854775808|integer", "负号决定饱和方向");
    chk!("ov_2p64", "ov_2p64=9223372036854775807|integer");
    chk!("ov_neg2p64", "ov_neg2p64=-9223372036854775808|integer");
    chk!("ov_long", "ov_long=9223372036854775807|integer");
    chk!(
        "ov_long_float",
        "ov_long_float=9.2233720368547758e+18|float",
        "tofloat 也先走整数路径 → 饱和后再转 float"
    );
    chk!("ov_hex16", "ov_hex16=9223372036854775807|integer");
    chk!("ov_hex16_neg", "ov_hex16_neg=-9223372036854775808|integer");
    chk!("nul_int", "nul_int=12|integer");
    chk!("nul_dot_only", "nul_dot_only=0|integer");
    chk!("nul_e", "nul_e=12|integer");
    chk!("hexfloat", "hexfloat=12|float");
    chk!("hexfloat_noexp", "hexfloat_noexp=1.5|float");
    chk!("hexfloat_frac", "hexfloat_frac=4|float");
    chk!("hexfloat_partial_p", "hexfloat_partial_p=1.5|float");
    chk!("hexfloat_dot_only", "hexfloat_dot_only=0|float");
    chk!("hexfloat_big", "hexfloat_big=inf|float");
    chk!("hexfloat_tiny", "hexfloat_tiny=0|float", "2^-1074 转 f32 下溢成 0");
    chk!("inf_dot", "inf_dot=inf|float");
    chk!("ninf_dot", "ninf_dot=-inf|float");
    chk!("infinity_e", "infinity_e=inf|float");
    chk!("nan_dot", "nan_dot=nan|float");
    chk!("nnan_dot", "nnan_dot=-nan|float");
    chk!(
        "inf_plain",
        "inf_plain!cannot convert the string",
        "没有 '.'/'e' → 走整数路径，strtol 认不了 inf"
    );
    chk!("inf_toint", "inf_toint=-9223372036854775808|integer");
    chk!("nan_toint", "nan_toint=-9223372036854775808|integer");
    chk!("nan_bits", "nan_bits=ffffff7f|string", "MSVC strtod 的 NaN 是全 1 尾数");
    chk!("nnan_bits", "nnan_bits=ffffffff|string");
    chk!("div00_bits", "div00_bits=0000c0ff|string", "x86 indefinite：不是同一个 NaN");
}

//! base 库"零覆盖全局函数"回归（`squirrel/sqbaselib.cpp`）。
//!
//! 这 5 个是 `tools/global_coverage.py` 扫出来的、62 个已注册全局里**仅有的**
//! 语料 0 覆盖项：`assert` / `error` / `array` / `type` / `dummy`。本轮补齐后
//! 矩阵 `tests-corpus/t_baselibmap.nut`（90 行，stdout 与 stderr 都逐字节一致）
//! 与参考实现**没有差异**，属于验证型收获；测试把这些容易想当然的点钉住：
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

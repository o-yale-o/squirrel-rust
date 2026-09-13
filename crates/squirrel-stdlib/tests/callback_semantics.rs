//! 容器方法的**回调乱来**语义回归（`squirrel/sqbaselib.cpp` 的 `array_sort` /
//! `__map_array` / `array_filter` / `array_reduce` / `array_find`）。
//!
//! 这一族在语料里原本是 **0 覆盖**：`t_delmatrix.nut` 覆盖"方法 × 接收者 × 实参
//! 类型"，`t_meta3.nut` 覆盖"元方法抛异常"，但没有任何用例让**回调自己乱来**
//! （返回非整数 / 形参个数不对 / 回调里抛异常）。这类形态按现有生成器的结构
//! 碰不到 —— 它需要把"回调的**行为**"当输入维度。新矩阵
//! `tests-corpus/t_cbmap.nut`（111 条）补齐后抓到两处真实差异：
//!
//! 1. **比较器的返回值走 `sq_getinteger`**：`sq_isnumeric` 掩码里**包含 float**
//!    （值取 `tointeger(o)` = `(SQInteger)` 截断），bool 另有分支给 0/1。
//!    我们原来只收 `OT_INTEGER`，于是最常见的写法
//!    `a.sort(function(x, y) { return x - y; })` 在**浮点数组**上直接报
//!    `numeric value expected as return value of the compare function` ——
//!    参考实现能正常排序。这条是**脚本层面真会撞到**的差异。
//! 2. **`map`/`apply` 的回调只支持 1~3 个形参**：`__map_array` 每轮只压
//!    `2 + (nArgs>=3) + (nArgs>=4)` 个值，调用却是 `sq_call(v, nArgs, …)` →
//!    `GetUp(-(nArgs+1))`。闭包躺在 `T-pushed-1`，所以只有 `nArgs == pushed`
//!    时调用的才是闭包自己：
//!    * 0 形参（nArgs=1）→ 目标是刚压进去的 `src` → `attempt to call 'array'`
//!    * 4 形参及以上（nArgs>=5）→ 目标是闭包**下面**那一槽，在 `array_map` /
//!      `array_apply` 的帧里同样是 `this`（那个数组）→ 同样的错误
//!    空数组不进循环，因此**不报错**（矩阵里有 `m_empty_p0` 钉住）。
//!
//! 另外钉住的部分（本来就一致，属于覆盖型收获）：
//! - 回调实参语义：`this` = 数组本身，第 1 个实参是值、第 2 个是下标、第 3 个又
//!   是数组 —— 而且**实参个数取决于闭包声明的形参个数**（`nArgs = 形参个数`，
//!   含 `this`），不是固定的。
//! - `filter`/`reduce`/`find`/`table.map`/`table.filter`/`keys` 的形参个数检查
//!   是普通调用（`wrong number of parameters (3 passed, N required)`）。
//! - 回调抛异常时错误原样往外传播，`sort` 会停在原地（数组保持调用前的顺序）。
//!
//! **不测的**（参考实现自身的 UB，按约定不进语料/测试）：比较器返回 NaN 或 ±inf
//! （`(SQInteger)((SQFloat)o)` 是 UB；本机上恰好两侧都给 INT64_MIN →
//! `inconsistent compare function`，但那是巧合），以及排序过程中增删数组元素。

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
    let rc = vm.run_buffer(src.as_bytes(), b"cbmap.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 每行 `tag=值|类型` 或 `tag!错误`（与语料 `t_cbmap.nut` 同一口径）
fn run_lines(body: &str) -> String {
    let src = format!(
        r#"
function dump(a) {{
    local s = "[";
    for (local i = 0; i < a.len(); i++) {{
        if (i > 0) s += ",";
        local v = a[i];
        s += (typeof v == "string" ? v : "" + v);
    }}
    return s + "]";
}}
function t(tag, f) {{
    try {{ local v = f(); print(tag + "=" + (typeof v == "string" ? v : "" + v) + "|" + typeof v + "\n"); }}
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

/// 比较器返回值经 `sq_getinteger`：**float 截断、bool 给 0/1**，其余类型报错
#[test]
fn sort_comparator_return_value_is_coerced_like_sq_getinteger() {
    let out = run_lines(
        r#"
local A = [10, 20];
t("sub_int", function() { local a = [4, 2, 5, 1, 3]; a.sort(function(x, y) { return x - y; }); return dump(a); });
t("sub_float", function() { local a = [2.5, 1.5, 3.5]; a.sort(function(x, y) { return x - y; }); return dump(a); });
t("sub_float_rev", function() { local a = [2.5, 1.5, 3.5]; a.sort(function(x, y) { return y - x; }); return dump(a); });
t("half", function() { local a = [4, 2, 5, 1, 3]; a.sort(function(x, y) { return 0.5; }); return dump(a); });
t("neg_half", function() { local a = [4, 2, 5, 1, 3]; a.sort(function(x, y) { return -0.5; }); return dump(a); });
t("f_trunc", function() { local a = [4, 2, 5, 1, 3]; a.sort(function(x, y) { return 1.9; }); return dump(a); });
t("f_trunc_neg", function() { local a = [4, 2, 5, 1, 3]; a.sort(function(x, y) { return -1.9; }); return dump(a); });
t("bool_true", function() { local a = [4, 2, 5, 1, 3]; a.sort(function(x, y) { return true; }); return dump(a); });
t("bool_false", function() { local a = [4, 2, 5, 1, 3]; a.sort(function(x, y) { return false; }); return dump(a); });
t("bool_cmp", function() { local a = [2.5, 1.5, 3.5]; a.sort(function(x, y) { return x < y; }); return dump(a); });
t("null", function() { local a = [4, 2, 5, 1, 3]; local r; try { a.sort(function(x, y) { return null; }); r = "ok"; } catch (e) { r = "E:" + e; } return r + " " + dump(a); });
t("str", function() { local a = [4, 2, 5, 1, 3]; try { a.sort(function(x, y) { return "x"; }); } catch (e) { return "E:" + e; } return dump(a); });
t("arr", function() { local a = [4, 2, 5, 1, 3]; try { a.sort(function(x, y) { return []; }); } catch (e) { return "E:" + e; } return dump(a); });
t("throw", function() { local a = [4, 2, 5, 1, 3]; try { a.sort(function(x, y) { throw "boom"; }); } catch (e) { return "E:" + e + " " + dump(a); } return dump(a); });
t("consistency", function() { local a = [4, 2, 5, 1, 3]; try { a.sort(function(x, y) { return -1; }); } catch (e) { return "E:" + e + " " + dump(a); } return dump(a); });
t("arity2", function() { local a = [3, 1, 2]; a.sort(function(x, y) { return x <=> y; }); return dump(a); });
t("arity3", function() { local a = [3, 1, 2]; try { a.sort(function(x, y, z) { return 0; }); } catch (e) { return "E:" + e; } return dump(a); });
t("this_type", function() { local seen = "?"; local a = [2, 1]; a.sort(function(x, y) { seen = typeof this; return x <=> y; }); return seen; });
"#,
    );
    assert_eq!(line(&out, "sub_int"), "sub_int=[1,2,3,4,5]|string");
    assert_eq!(
        line(&out, "sub_float"),
        "sub_float=[1.5,2.5,3.5]|string",
        "浮点数组上 return x - y 返回的是 float，必须被接受"
    );
    assert_eq!(line(&out, "sub_float_rev"), "sub_float_rev=[3.5,2.5,1.5]|string");
    // 常量比较器：非传递 → 参考实现给出确定但"乱"的排列，我们逐字节复刻
    assert_eq!(line(&out, "half"), "half=[2,5,1,3,4]|string");
    assert_eq!(line(&out, "neg_half"), "neg_half=[2,5,1,3,4]|string");
    assert_eq!(line(&out, "f_trunc"), "f_trunc=[2,5,1,3,4]|string", "(SQInteger)1.9 == 1");
    assert_eq!(
        line(&out, "f_trunc_neg"),
        "f_trunc_neg!inconsistent compare function",
        "(SQInteger)-1.9 == -1 → 与恒 -1 同一条路径"
    );
    assert_eq!(line(&out, "bool_true"), "bool_true=[2,5,1,3,4]|string", "true → 1");
    assert_eq!(line(&out, "bool_false"), "bool_false=[2,5,1,3,4]|string", "false → 0");
    assert_eq!(line(&out, "bool_cmp"), "bool_cmp=[1.5,3.5,2.5]|string");
    // 其余类型：报错且数组保持原样
    assert_eq!(
        line(&out, "null"),
        "null=E:numeric value expected as return value of the compare function [4,2,5,1,3]|string"
    );
    assert_eq!(
        line(&out, "str"),
        "str=E:numeric value expected as return value of the compare function|string"
    );
    assert_eq!(
        line(&out, "arr"),
        "arr=E:numeric value expected as return value of the compare function|string"
    );
    assert_eq!(
        line(&out, "throw"),
        "throw=E:boom [4,2,5,1,3]|string",
        "回调抛异常时错误原样传播，数组保持原顺序"
    );
    assert_eq!(
        line(&out, "consistency"),
        "consistency=E:inconsistent compare function [5,3,4,2,1]|string"
    );
    assert_eq!(
        line(&out, "arity3"),
        "arity3=E:wrong number of parameters (3 passed, 4 required)|string",
        "比较器固定 3 个槽（this + 2）"
    );
    assert_eq!(line(&out, "this_type"), "this_type=table|string", "比较器的 this 是根表");
}

/// `map`/`apply` 的回调**形参个数**决定栈布局：只有 1~3 个形参能用
#[test]
fn map_apply_callback_arity_decides_call_target() {
    let out = run_lines(
        r#"
local A = [10, 20];
t("m_p0", function() { local r = A.map(function() { return "z"; }); return typeof r; });
t("m_p1", function() { local r = A.map(function(a) { return "z"; }); return typeof r + ":" + r.len(); });
t("m_p2", function() { local r = A.map(function(a, b) { return "z"; }); return typeof r + ":" + r.len(); });
t("m_p3", function() { local r = A.map(function(a, b, c) { return "z"; }); return typeof r + ":" + r.len(); });
t("m_p4", function() { local r = A.map(function(a, b, c, d) { return "z"; }); return typeof r; });
t("m_p5", function() { local r = A.map(function(a, b, c, d, e) { return "z"; }); return typeof r; });
t("m_empty_p0", function() { local e = []; return typeof e.map(function() { return 1; }); });
t("m_native_abs", function() { local a = [10, -20]; return dump(a.map(abs)); });
t("ap_p0", function() { local c = [1, 2]; c.apply(function() { return 9; }); return "ok"; });
t("ap_p1", function() { local c = [1, 2]; c.apply(function(v) { return v * 10; }); return dump(c); });
t("ap_p3", function() { local c = [1, 2]; c.apply(function(v, i, arr) { return i; }); return dump(c); });
t("ap_p4", function() { local c = [1, 2]; c.apply(function(a, b, c2, d) { return 9; }); return "ok"; });
t("ap_empty_p0", function() { local e = []; e.apply(function() { return 1; }); return "ok"; });
t("cb_int", function() { try { return "" + A.map(5); } catch (e) { return "E:" + e; } });
t("cb_null", function() { try { return "" + A.map(null); } catch (e) { return "E:" + e; } });
"#,
    );
    // 0 / 4 / 5 个形参：参考实现会去调栈上别的槽（都是那个数组）
    assert_eq!(line(&out, "m_p0"), "m_p0!attempt to call 'array'");
    assert_eq!(line(&out, "m_p4"), "m_p4!attempt to call 'array'");
    assert_eq!(line(&out, "m_p5"), "m_p5!attempt to call 'array'");
    assert_eq!(line(&out, "ap_p0"), "ap_p0!attempt to call 'array'");
    assert_eq!(line(&out, "ap_p4"), "ap_p4!attempt to call 'array'");
    // 空数组不进循环 → 不报错
    assert_eq!(line(&out, "m_empty_p0"), "m_empty_p0=array|string");
    assert_eq!(line(&out, "ap_empty_p0"), "ap_empty_p0=ok|string");
    // 1~3 个形参正常工作
    assert_eq!(line(&out, "m_p1"), "m_p1=array:2|string");
    assert_eq!(line(&out, "m_p2"), "m_p2=array:2|string");
    assert_eq!(line(&out, "m_p3"), "m_p3=array:2|string");
    assert_eq!(
        line(&out, "m_native_abs"),
        "m_native_abs=[10,20]|string",
        "原生闭包 nparamscheck=2 → nargs=2 == pushed，正常调用"
    );
    assert_eq!(line(&out, "ap_p1"), "ap_p1=[10,20]|string");
    assert_eq!(line(&out, "ap_p3"), "ap_p3=[0,1]|string");
    assert_eq!(
        line(&out, "cb_int"),
        "cb_int=E:parameter 1 has an invalid type 'integer' ; expected: 'function|function'|string"
    );
    assert_eq!(
        line(&out, "cb_null"),
        "cb_null=E:parameter 1 has an invalid type 'null' ; expected: 'function|function'|string"
    );
}

/// 回调**实参语义**：`this` = 数组；实参个数由闭包声明的形参个数决定
#[test]
fn container_callback_argument_shape() {
    let out = run_lines(
        r#"
local A = [10, 20];
local tb = { a = 1, b = 2 };
t("m1", function() { local r = A.map(function(a) { return "" + typeof a + ":" + a; }); return dump(r); });
t("m2", function() { local r = A.map(function(a, b) { return "" + typeof a + ":" + a + "|" + typeof b + ":" + b; }); return dump(r); });
t("m3", function() { local r = A.map(function(a, b, c) { return "" + typeof a + ":" + a + "|" + typeof b + ":" + b + "|" + typeof c + ":" + c.len(); }); return dump(r); });
t("this_is_arr", function() { local r = A.map(function(a) { return (this == A) + "/" + this.len(); }); return dump(r); });
t("filter2", function() { return dump(A.filter(function(a, b) { return b == 0; })); });
t("filter2_idx", function() { return dump(A.filter(function(i, v) { return i == 0; })); });
t("filter3", function() { return dump(A.filter(function(a, b, c) { return typeof c == "array"; })); });
t("find2", function() { return A.find(function(a, b) { return b == 1; }); });
t("reduce2", function() { return A.reduce(function(acc, v) { return "" + acc + "/" + v; }, "S"); });
t("reduce3", function() { return A.reduce(function(acc, v, i) { return "" + acc + "/" + v + "/" + typeof i + i; }, "S"); });
t("reduce_noinit", function() { return A.reduce(function(acc, v) { return "" + acc + "/" + v; }); });
t("reduce_empty", function() { local e = []; return "R:" + e.reduce(function(acc, v) { return 1; }); });
t("filter_p0", function() { local r = A.filter(function() { return true; }); return "n" + r.len(); });
t("find_p0", function() { return A.find(function() { return true; }); });
t("tb_map_p2", function() { local r = tb.map(function(k, v) { return 1; }); return typeof r; });
t("tb_map_p4", function() { local r = tb.map(function(k, v, x, y) { return 1; }); return typeof r; });
t("tb_keys_p2", function() { return tb.keys(function(k, v) { return true; }).len(); });
t("th_map", function() { local a = [1, 2, 3]; try { a.map(function(v) { throw "mb"; }); } catch (e) { return "E:" + e; } return "no"; });
t("th_filter", function() { local a = [1, 2, 3]; try { a.filter(function(i, v) { throw "fb"; }); } catch (e) { return "E:" + e; } return "no"; });
t("th_reduce", function() { local a = [1, 2, 3]; try { a.reduce(function(acc, v) { throw "rb"; }, 0); } catch (e) { return "E:" + e; } return "no"; });
t("th_tbkeys", function() { try { tb.keys(function(k) { throw "kb"; }); } catch (e) { return "E:" + e; } return "no"; });
"#,
    );
    assert_eq!(line(&out, "m1"), "m1=[integer:10,integer:20]|string");
    assert_eq!(line(&out, "m2"), "m2=[integer:10|integer:0,integer:20|integer:1]|string");
    assert_eq!(
        line(&out, "m3"),
        "m3=[integer:10|integer:0|array:2,integer:20|integer:1|array:2]|string",
        "第 3 个实参又是那个数组（取 len 观测，避免指针文本）"
    );
    assert_eq!(line(&out, "this_is_arr"), "this_is_arr=[true/2,true/2]|string");
    assert_eq!(
        line(&out, "filter2"),
        "filter2=[]|string",
        "filter 的实参是 (下标, 值)，与 map 的 (值, 下标) **相反** → b 是值而不是下标"
    );
    assert_eq!(line(&out, "filter2_idx"), "filter2_idx=[10]|string");
    assert_eq!(
        line(&out, "filter3"),
        "filter3!wrong number of parameters (3 passed, 4 required)",
        "filter 恒以 3 个槽调用（this + 下标 + 值），所以 3 形参的回调一定报错"
    );
    assert_eq!(
        line(&out, "find2"),
        "find2=null|null",
        "array.find 是 find(**值**)，根本不调回调 —— 找的是那个闭包本身"
    );
    assert_eq!(line(&out, "reduce2"), "reduce2=S/10/20|string");
    assert_eq!(
        line(&out, "reduce3"),
        "reduce3!wrong number of parameters (3 passed, 4 required)",
        "reduce 恒以 3 个槽调用（this + 累加值 + 当前值），回调最多 2 个形参"
    );
    assert_eq!(line(&out, "reduce_noinit"), "reduce_noinit=10/20|string");
    assert_eq!(line(&out, "reduce_empty"), "reduce_empty=R:null|string");
    assert_eq!(
        line(&out, "filter_p0"),
        "filter_p0!wrong number of parameters (3 passed, 1 required)",
        "filter 是固定 3 槽的普通调用，与 map 的栈布局不同"
    );
    assert_eq!(line(&out, "find_p0"), "find_p0=null|null");
    assert_eq!(
        line(&out, "tb_map_p2"),
        "tb_map_p2=array|string",
        "table.map 返回的是 **array**（↔ table_map 里 SQArray::Create）"
    );
    assert_eq!(
        line(&out, "tb_map_p4"),
        "tb_map_p4!wrong number of parameters (3 passed, 5 required)"
    );
    assert_eq!(
        line(&out, "tb_keys_p2"),
        "tb_keys_p2!wrong number of parameters",
        "table.keys 的回调只吃 1 个参数（键）；多声明就报错"
    );
    assert_eq!(line(&out, "th_map"), "th_map=E:mb|string");
    assert_eq!(line(&out, "th_filter"), "th_filter=E:fb|string");
    assert_eq!(line(&out, "th_reduce"), "th_reduce=E:rb|string");
    assert_eq!(
        line(&out, "th_tbkeys"),
        "th_tbkeys=E:wrong number of parameters|string",
        "table.keys 的回调参数个数永远对不上（TABLE_TO_ARRAY_FUNC 的固定槽数），\
所以连回调都进不去 —— 抛不抛异常都看不到"
    );
}

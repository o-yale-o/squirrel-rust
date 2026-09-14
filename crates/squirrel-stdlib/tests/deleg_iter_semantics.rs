//! 委托链与迭代期变更的语义回归（`squirrel/sqtable.cpp` / `SQVM` 的迭代路径）。
//!
//! 这两族是现有生成器结构性地碰不到的：`t_delmatrix.nut` 测"方法 × 实参类型"，
//! `t_meta3.nut` 测"单个元方法"——链的**动力学**（深度/循环/跨类型/raw*）与
//! 迭代的**动力学**（foreach 期间增/删/缩/清）都没有用例。新矩阵
//! `tests-corpus/t_delegiter.nut`（59 条）补齐后为**验证型**收获（两侧本就
//! 逐字节一致），测试把这些容易想当然的点钉住：
//!
//! 1. **循环委托在 `setdelegate` 赋值时就抛 `delegate cycle`**，不是等查找时
//!    才发现（`b.setdelegate(a)` 在 `a` 已挂 `b` 的当下直接抛）。a↔b / 自挂 /
//!    三角环全都构造不出来。
//! 2. `setdelegate` 只接受 `null|table`；实例和数组**没有**这个方法。
//! 3. `rawget/rawin/rawset` **不穿链**；`<-`（无 `_newslot` 元方法时）挂到
//!    **接收者自身**，不会挂到链上游。
//! 4. `foreach`/`len` 只看**自己的槽**，不看委托链。
//! 5. 迭代期变更（参考实现全程不崩，两侧逐字节一致，别"顺手修正"）：
//!    - 表：迭代中删当前键 → 计数照旧但剩 1；删别的键 → 其后的键被**跳过**；
//!      `clear()` 后迭代立即终止；边删边插有界。
//!    - 数组：`append` 会**延长**迭代；`remove`/缩短 `resize` 让迭代**提前结束**；
//!      变长 `resize` 按**旧长度**走完。

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
    let rc = vm.run_buffer(src.as_bytes(), b"delegiter.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

fn run_lines(body: &str) -> String {
    let src = format!(
        r#"
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

/// 循环委托：**赋值时**就报错，任何环都构造不出来
#[test]
fn delegation_cycles_are_detected_at_assignment() {
    let out = run_lines(
        r#"
t("pair", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a); return "made"; });
t("self", function() { local a = {}; a.setdelegate(a); return "made"; });
t("tri", function() { local a = {}; local b = {}; local c = {}; a.setdelegate(b); b.setdelegate(c); c.setdelegate(a); return "made"; });
t("deep_pair", function() { local a = {}; local b = {}; local c = {}; c.setdelegate(b); b.setdelegate(a); a.setdelegate(c); return "made"; });
t("valid_then_cyc", function() { local a = {}; local b = {}; a.setdelegate(b); return b.q; });
t("get_after_fail", function() { local a = {}; local b = {}; a.setdelegate(b); try { b.setdelegate(a); } catch (e) { } return a.q; });
t("detach_ok", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(null); b.setdelegate(a); return "made2"; });
t("t2c", function() { class K { y = 7 } local o = {}; o.setdelegate(K); return o.y; });
t("t2a", function() { local o = {}; o.setdelegate([1, 2]); return "ok"; });
"#,
    );
    // 各种形状的环都在第二个 setdelegate 上抛同一个错
    for tag in ["pair", "self", "tri", "deep_pair"] {
        assert_eq!(line(&out, tag), format!("{tag}!delegate cycle"), "环在赋值时被拒");
    }
    assert_eq!(line(&out, "valid_then_cyc"), "valid_then_cyc!the index 'q' does not exist");
    // 失败的挂链不留半成品：a 挂 b 仍然有效（b 的 q 不存在）
    assert_eq!(line(&out, "get_after_fail"), "get_after_fail!the index 'q' does not exist");
    // 先解开再反向挂，合法
    assert_eq!(line(&out, "detach_ok"), "detach_ok!delegate cycle", "null 解开后 b→a 仍是环（a 已挂 b）");
    assert_eq!(
        line(&out, "t2c"),
        "t2c!parameter 1 has an invalid type 'class' ; expected: 'null|table'"
    );
    assert_eq!(
        line(&out, "t2a"),
        "t2a!parameter 1 has an invalid type 'array' ; expected: 'null|table'"
    );
}

/// 链上查找 / raw* / newslot / foreach 的可见性
#[test]
fn delegation_chain_lookup_and_raw_visibility() {
    let out = run_lines(
        r#"
t("deep6", function() { local root = { x = "deep" }; local prev = root; for (local i = 0; i < 5; i++) { local n = {}; n.setdelegate(prev); prev = n; } return prev.x; });
t("shadow", function() { local a = { x = "a" }; local b = { x = "b" }; b.setdelegate(a); return b.x; });
t("raw_get", function() { local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.rawget("x"); });
t("raw_in", function() { local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.rawin("x"); });
t("raw_set", function() { local a = { x = 1 }; local b = {}; b.setdelegate(a); b.rawset("x", 9); return a.x + "/" + b.x; });
t("ns_chain", function() { local a = { }; local b = {}; b.setdelegate(a); b.q <- 5; return a.rawin("q") + "/" + b.rawin("q"); });
t("foreach_chain", function() { local a = { x = 1, y = 2 }; local b = { z = 3 }; b.setdelegate(a); local n = 0; foreach (k, v in b) n++; return n; });
t("len_chain", function() { local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.len(); });
t("del_chain", function() { local a = { q = 1 }; local b = {}; b.setdelegate(a); return delete b.q; });
t("mm_before_chain", function() { local bt = { x = "chain" }; local d = { _get = function(k) { return "mm"; } }; local o = {}; o.setdelegate(d); d.setdelegate(bt); return o.x; });
"#,
    );
    assert_eq!(line(&out, "deep6"), "deep6=deep|string");
    assert_eq!(line(&out, "shadow"), "shadow=b|string", "最近者优先");
    assert_eq!(line(&out, "raw_get"), "raw_get!the index doesn't exist", "rawget 不穿链");
    assert_eq!(line(&out, "raw_in"), "raw_in=false|bool");
    assert_eq!(line(&out, "raw_set"), "raw_set=1/9|string", "rawset 写自身");
    assert_eq!(line(&out, "ns_chain"), "ns_chain=false/true|string", "<- 挂接收者自身");
    assert_eq!(line(&out, "foreach_chain"), "foreach_chain=1|integer", "foreach 只看自己的槽");
    assert_eq!(line(&out, "len_chain"), "len_chain=0|integer");
    assert_eq!(
        line(&out, "del_chain"),
        "del_chain!the index 'q' does not exist",
        "delete 不穿链（只删自身的槽；穿链删除要靠 _delslot 元方法）"
    );
    assert_eq!(line(&out, "mm_before_chain"), "mm_before_chain=chain|string");
}

/// foreach 期间的容器变更（与参考实现逐字节一致，包括"怪"的结果）
#[test]
fn foreach_sees_mutation_like_the_reference() {
    let out = run_lines(
        r#"
t("tbl_del_cur", function() { local tb = { a = 1, b = 2, c = 3, d = 4 }; local n = 0; foreach (k, v in tb) { n++; delete tb[k]; } return n + "/" + tb.len(); });
t("tbl_del_other", function() { local tb = { a = 1, b = 2, c = 3, d = 4 }; local seen = ""; foreach (k, v in tb) { seen += k; if (k == "a") delete tb.d; } return seen + "/" + tb.len(); });
t("tbl_clear", function() { local tb = { a = 1, b = 2, c = 3 }; local seen = ""; foreach (k, v in tb) { seen += k; tb.clear(); } return seen + "/" + tb.len(); });
t("arr_append", function() { local a = [1, 2, 3]; local n = 0; foreach (v in a) { n++; if (n < 10) a.append(n); } return n + "/" + a.len(); });
t("arr_remove", function() { local a = [1, 2, 3, 4]; local n = 0; foreach (v in a) { n++; a.remove(0); } return n + "/" + a.len(); });
t("arr_shrink", function() { local a = [1, 2, 3, 4, 5]; local seen = ""; foreach (v in a) { seen += v; a.resize(a.len() - 1); } return seen + "/" + a.len(); });
t("arr_grow", function() { local a = [1, 2, 3]; local n = 0; foreach (v in a) { n++; a.resize(10); } return n + "/" + a.len(); });
t("nested", function() { local tb = { a = 1, b = 2 }; local n = 0; foreach (k1, v1 in tb) foreach (k2, v2 in tb) n++; return "" + n; });
t("break_", function() { local s = ""; foreach (v in [1, 2, 3]) { if (v == 2) break; s += v; } return s; });
t("throw_", function() { foreach (v in [1, 2, 3]) { if (v == 2) throw "mid"; } return 0; });
"#,
    );
    assert_eq!(line(&out, "tbl_del_cur"), "tbl_del_cur=3/1|string", "删当前键：计数照旧、剩 1");
    assert_eq!(line(&out, "tbl_del_other"), "tbl_del_other=acb/3|string", "删别的键会跳过其后的键");
    assert_eq!(line(&out, "tbl_clear"), "tbl_clear=a/0|string", "clear 后迭代立即终止");
    assert_eq!(line(&out, "arr_append"), "arr_append=12/12|string", "append 延长迭代");
    assert_eq!(line(&out, "arr_remove"), "arr_remove=2/2|string", "remove 让迭代提前结束");
    assert_eq!(line(&out, "arr_shrink"), "arr_shrink=123/2|string", "缩容让迭代提前结束");
    assert_eq!(line(&out, "arr_grow"), "arr_grow=10/10|string", "扩容按旧长度走");
    assert_eq!(line(&out, "nested"), "nested=4|string");
    assert_eq!(line(&out, "break_"), "break_=1|string");
    assert_eq!(line(&out, "throw_"), "throw_!mid");
}

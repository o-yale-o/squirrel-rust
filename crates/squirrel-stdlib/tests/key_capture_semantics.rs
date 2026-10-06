//! 表键身份与循环变量闭包捕获（`SQTable` 的哈希键 + 编译器的 local 槽分配）。
//!
//! `t_delmatrix` 测"方法 × 实参类型"、`t_ops` 测运算符——**表的键按什么身份
//! 区分**与**循环变量的闭包捕获**这两族没有用例。新矩阵
//! `tests-corpus/t_keymap.nut`（30 条）补齐后为**验证型**收获
//! （两侧本就逐字节一致），把最容易想反的几点守门：
//!
//! 1. **表键严格按类型区分**：`t[1]`、`t[1.0]`、`t["1"]`、`t[true]` 是**四个不同
//!    的槽**——值相等也不互通（`t[1.0]` 读不到 `t[1]` 写的值）；`-0.0` 与 `0`
//!    是两个键；**null 不能当键**（`null cannot be used as index`）。
//!    混合键型的遍历顺序是哈希序（`bool;string;float;integer;`）。
//! 2. **数组下标做 `tointeger` 转换**：`a[1.0]`→`a[1]`、`a[1.9]`→`a[1]`（截断）、
//!    `a[-0.0]`→`a[0]`；`a["1"]` 报错；`a[-1] = v` 报
//!    `the index '-1' does not exist`。
//! 3. **普通 `=` 不建新槽**（核心语义）：`x[1] = v` 对空表报
//!    `the index '1' does not exist`，建键要用 `<-`。
//! 4. **循环变量的闭包捕获**：`for`/`foreach` 的循环变量被闭包捕获时**共享同一个
//!    槽**——循环结束后所有闭包看到的都是最终值（`3,3,3` / `30,30,30`）；
//!    **循环体内 `local j` 才逐次独立**（`0,10,20`）；`while` + 体内 `local`
//!    同理；共享 `i` 的两个闭包一侧修改对另一侧可见（`2,102,102,202`）；
//!    捕获循环变量的生成器在循环结束后 resume 看到的是最终值。

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
    let rc = vm.run_buffer(src.as_bytes(), b"keymap.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

fn run_lines(body: &str) -> String {
    let src = format!(
        r#"
function d(x) {{
    local t = typeof x;
    if (t == "table") {{ local o = "{{"; local f = true; foreach (k, v in x) {{ if (!f) o += ","; f = false; o += "" + k + "=" + v; }} return o + "}}"; }}
    if (t == "array") {{ local o = "["; for (local i = 0; i < x.len(); i++) {{ if (i) o += ","; o += "" + x[i]; }} return o + "]"; }}
    return "" + x;
}}
function t(tag, f) {{
    try {{ local v = f(); print(tag + "=" + d(v) + "|" + typeof v + "\n"); }}
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

/// 表键**严格按类型**区分；null 不能当键；数组下标做 tointeger
#[test]
fn table_keys_are_strictly_typed() {
    let out = run_lines(
        r#"
t("int_read_float", function() { local x = {}; x[1] <- "i"; return x[1.0]; });
t("float_read_int", function() { local x = {}; x[1.0] <- "f"; return x[1]; });
t("str_vs_int", function() { local x = {}; x["1"] <- "s"; x[1] <- "i"; return d(x) + "/" + x.len(); });
t("neg_zero_vs_zero", function() { local x = {}; x[-0.0] <- "nz"; return x.len(); });
t("bool_vs_int", function() { local x = {}; x[true] <- "T"; return x.len(); });
t("null_key", function() { local x = {}; x[null] <- "N"; return x[null]; });
t("frac_key", function() { local x = {}; x[1.5] <- "f"; return x.len(); });
t("overwrite_eq", function() { local x = {}; x[1] <- "first"; x[1] = "second"; return d(x); });
t("a_float_idx", function() { local a = [10, 20, 30]; return a[1.0]; });
t("a_frac_idx", function() { local a = [10, 20, 30]; return a[1.9]; });
t("a_str_idx", function() { local a = [10, 20, 30]; return a["1"]; });
t("a_neg_zero_idx", function() { local a = [10, 20, 30]; return a[-0.0]; });
t("a_set_frac", function() { local a = [10, 20, 30]; a[1.9] = 99; return d(a); });
t("a_neg_set", function() { local a = [10, 20, 30]; a[-1] = 99; return d(a); });
t("plain_eq", function() { local x = {}; x[1] = "i"; return d(x); });
"#,
    );
    assert_eq!(line(&out, "int_read_float"), "int_read_float!the index '1' does not exist", "float 键 1.0 ≠ int 键 1");
    assert_eq!(line(&out, "float_read_int"), "float_read_int!the index '1' does not exist");
    assert_eq!(line(&out, "str_vs_int"), "str_vs_int={1=s,1=i}/2|string", "两个槽都显示为 1");
    assert_eq!(line(&out, "neg_zero_vs_zero"), "neg_zero_vs_zero=1|integer", "-0.0 与 0 是两个键");
    assert_eq!(line(&out, "bool_vs_int"), "bool_vs_int=1|integer", "true ≠ 1");
    assert_eq!(line(&out, "null_key"), "null_key!null cannot be used as index");
    assert_eq!(line(&out, "frac_key"), "frac_key=1|integer", "1.5 与 1 无关");
    assert_eq!(line(&out, "overwrite_eq"), "overwrite_eq={1=second}|string", "已存在的键用 = 覆盖");
    // 数组：下标做 tointeger 转换
    assert_eq!(line(&out, "a_float_idx"), "a_float_idx=20|integer");
    assert_eq!(line(&out, "a_frac_idx"), "a_frac_idx=20|integer", "1.9 截断成 1");
    assert_eq!(line(&out, "a_neg_zero_idx"), "a_neg_zero_idx=10|integer", "-0.0 截断成 0");
    assert_eq!(
        line(&out, "a_str_idx"),
        "a_str_idx!the index '1' does not exist",
        "字符串下标不做转换"
    );
    assert_eq!(line(&out, "a_set_frac"), "a_set_frac=[10,99,30]|string");
    assert_eq!(line(&out, "a_neg_set"), "a_neg_set!the index '-1' does not exist");
    assert_eq!(
        line(&out, "plain_eq"),
        "plain_eq!the index '1' does not exist",
        "普通 = 不建新槽（要用 <-）"
    );
}

/// 循环变量的闭包捕获：for/foreach **共享**，体内 `local` **逐次独立**
#[test]
fn loop_variable_capture_shares_or_isolates() {
    let out = run_lines(
        r#"
t("cap_for", function() { local fs = []; for (local i = 0; i < 3; i++) fs.append(function() { return i; }); local s = ""; foreach (f in fs) s += f() + ","; return s; });
t("cap_foreach", function() { local fs = []; foreach (v in [10, 20, 30]) fs.append(function() { return v; }); local s = ""; foreach (f in fs) s += f() + ","; return s; });
t("cap_while", function() { local fs = []; local i = 0; while (i < 3) { local j = i; fs.append(function() { return j; }); i++; } local s = ""; foreach (f in fs) s += f() + ","; return s; });
t("cap_for_local", function() { local fs = []; for (local i = 0; i < 3; i++) { local j = i * 10; fs.append(function() { return j; }); } local s = ""; foreach (f in fs) s += f() + ","; return s; });
t("cap_gen", function() { local fs = []; for (local i = 0; i < 3; i++) fs.append((function() { yield i; })()); local s = ""; foreach (g in fs) { resume g; s += resume g; s += ","; } return s; });
t("cap_two_shared", function() { local fs = []; for (local i = 0; i < 2; i++) { fs.append(function() { return i; }); fs.append(function() { i = i + 100; return i; }); } local s = ""; foreach (f in fs) s += f() + ","; return s; });
t("cap_delegate", function() { local fs = []; local tbl = { n = 0 }; for (local i = 0; i < 2; i++) { tbl.n = i; fs.append(function() { return tbl.n; }); } local s = ""; foreach (f in fs) s += f() + ","; return s; });
"#,
    );
    assert_eq!(line(&out, "cap_for"), "cap_for=3,3,3,|string", "for 的 i 共享同一槽");
    assert_eq!(line(&out, "cap_foreach"), "cap_foreach=30,30,30,|string", "foreach 的 v 同样共享");
    assert_eq!(line(&out, "cap_while"), "cap_while=0,1,2,|string", "体内 local 逐次独立");
    assert_eq!(line(&out, "cap_for_local"), "cap_for_local=0,10,20,|string");
    assert_eq!(line(&out, "cap_gen"), "cap_gen=null,null,null,|string", "循环结束后 resume 看到最终 i");
    assert_eq!(line(&out, "cap_two_shared"), "cap_two_shared=2,102,102,202,|string", "共享 i 的修改互相可见");
    assert_eq!(line(&out, "cap_delegate"), "cap_delegate=1,1,|string", "成员不快照，读的是最终值");
}

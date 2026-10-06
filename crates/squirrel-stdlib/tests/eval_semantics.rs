//! 编译期折叠 / switch 值形态 / 短路求值顺序 / 链式赋值（`sqcompiler.cpp` + `sqvm.cpp`）。
//!
//! `t_ops`（4533 条）测的是运行时运算符 × 类型、`t_lexmatrix` 的 switch 只有
//! 主干 6 条；这里钉的是**编译/求值的动力学**。新矩阵
//! `tests-corpus/t_evalmap.nut`（50 条）为**验证型 + 1 个真修复**：
//!
//! **P23 修复的真 bug（DMOVE 的执行顺序）**：窥孔把连续两条 MOVE 合并成一条
//! `DMOVE(d0←s0, d1←s1)`。参考实现是严格顺序执行
//! （`STK(arg0)=STK(arg1); STK(arg2)=STK(arg3);`），而链式赋值 `x = y = z`
//! 正是 `DMOVE(y←z, x←y)`——arg3 与 arg1 是同一个寄存器，第二读必须看到
//! 第一写。我们原来把两个源先快照再写（并行语义），于是 `x` 拿到旧 `y`：
//! `local x=0; local y=0; local z=3; x = y = z;` 参考给 `x=3`，我们给 `x=0`。
//! 所有"外层赋值、内层 RHS 是变量/调用"的链式赋值都中招；RHS 是字面量时
//! 碰巧没事（常量直接 Load 进目标）。
//!
//! 其余钉住的（两侧本就一致）：
//! 1. **switch**：case 标签可以是任意表达式（含变量）；重复 case 先到先得；
//!    浮点 case 与整数交叉匹配；**case 写在 default 之后是语法错**；
//!    switch 里的 break 归 switch 不归外层循环。
//! 2. **折叠**：`7 % 0` → **7**（% 编译期折叠！）、`1 / 0` → 运行时
//!    `division by zero`（整数除法不折叠）、`1.0/0.0`→inf、`1.0%0.0`→nan、
//!    INT64 字面量折叠回绕、`1<<64`→1、`1<<-1`→INT64_MIN。
//! 3. **`&&`/`||` 返回操作数值本身**且短路：`true && 2`→2、`"" && 5`→5
//!    （`""` 真值）、`false && side()` 不调用 side、
//!    `1 && 2 || 3 && 4` → 2（右侧被短路）。
//! 4. **赋值作表达式**：`(a = 5)` 值为 5、`if ((a = 7) == 7)`、实参里赋值、
//!    `x ? a = 1 : b = 2` 只赋命中的分支。

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
    let rc = vm.run_buffer(src.as_bytes(), b"evalmap.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

fn run_lines(body: &str) -> String {
    let src = format!(
        r#"
function side(x) {{ print("S:" + x + "\n"); return x; }}
function t(tag, f) {{
    try {{ local v = f(); print(tag + "=" + (typeof v == "array" ? "arr" : (typeof v == "string" ? v : "" + v)) + "|" + typeof v + "\n"); }}
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

/// 链式赋值与赋值作表达式（DMOVE 顺序修复的守门）
#[test]
fn chained_assignment_yields_the_new_value() {
    let out = run_lines(
        r#"
t("chain3", function() { local x = 0; local y = 0; local z = 3; x = y = z; return x + "/" + y + "/" + z; });
t("chain4", function() { local x = 0; local y = 0; local z = 0; local w = 7; x = y = z = w; return x + "/" + y + "/" + z; });
t("paren_var", function() { local a = 0; local b = 0; local c = 3; a = (b = c); return a + "/" + b; });
t("paren_lit", function() { local p = 0; local q = (p = 5); return p + "/" + q; });
t("str_chain", function() { local s = 0; local t2 = 0; local u = "uv"; s = t2 = u; return s + "/" + t2; });
t("call_rhs", function() { local g = 0; local h = 0; local f2 = function() { return 3; }; g = h = f2(); return g + "/" + h; });
t("obj_chain", function() { local o = { m = 0, n = 0 }; o.m = o.n = 5; return o.m + "/" + o.n; });
t("asg_value", function() { local a = 1; local b = (a = 5); return a + "/" + b; });
t("asg_in_cond", function() { local a = 0, r = ""; if ((a = 7) == 7) r = "yes"; return r + "/" + a; });
t("asg_in_call", function() { local a = 0; local f = function(x) { return x * 2; }; local r = f(a = 4); return r + "/" + a; });
"#,
    );
    // 修复前：chain3 给 0/3/3、chain4 给 0/0/7（外层赋值拿到旧值）
    assert_eq!(line(&out, "chain3"), "chain3=3/3/3|string");
    assert_eq!(line(&out, "chain4"), "chain4=7/7/7|string");
    assert_eq!(line(&out, "paren_var"), "paren_var=3/3|string");
    assert_eq!(line(&out, "paren_lit"), "paren_lit=5/5|string");
    assert_eq!(line(&out, "str_chain"), "str_chain=uv/uv|string");
    assert_eq!(line(&out, "call_rhs"), "call_rhs=3/3|string");
    assert_eq!(line(&out, "obj_chain"), "obj_chain=5/5|string");
    assert_eq!(line(&out, "asg_value"), "asg_value=5/5|string");
    assert_eq!(line(&out, "asg_in_cond"), "asg_in_cond=yes/7|string");
    assert_eq!(line(&out, "asg_in_call"), "asg_in_call=8/4|string");
}

/// `&&`/`||` 返回操作数值本身 + 短路副作用顺序
#[test]
fn logical_operators_return_operands_and_short_circuit() {
    let out = run_lines(
        r#"
t("and_vals", function() { return (1 && 2) + "/" + (0 && 2) + "/" + (null && 2) + "/" + ("" && 2); });
t("or_vals", function() { return (1 || 2) + "/" + (0 || 2) + "/" + (null || "x") + "/" + ("" || 2); });
t("and_short", function() { local s = ""; local r = false && side(1); return s + "/" + r; });
t("or_short", function() { local s = ""; local r = true || side(1); return s + "/" + r; });
t("and_calls", function() { local s = ""; local r = side(true) && side(2); return s + "/" + r; });
t("or_calls", function() { local s = ""; local r = side(false) || side(3); return s + "/" + r; });
t("chain", function() { local s = ""; local r = side(1) && side(2) || side(3) && side(4); return s + "/" + r; });
t("mixed", function() { return ("" && 5) + "/" + typeof ([] || "arr"); });
t("not_not", function() { return !!5 + "/" + !!0 + "/" + !!"x"; });
"#,
    );
    assert_eq!(line(&out, "and_vals"), "and_vals=2/0/null/2|string", "返回的是操作数值本身：`\"\" && 2` → 2");
    assert_eq!(line(&out, "or_vals"), "or_vals=1/2/x/|string", "`\"\"` 真值 → `||` 返回左操作数（空串）");
    assert_eq!(line(&out, "and_short"), "and_short=/false|string", "false 短路：side 不被调用");
    assert_eq!(line(&out, "or_short"), "or_short=/true|string");
    assert!(
        out.contains("S:true\nS:2\n"),
        "&& 左真值 → 两个 side 都被调用\n{out}"
    );
    assert_eq!(line(&out, "and_calls"), "and_calls=/2|string");
    assert!(
        out.contains("S:false\nS:3\n"),
        "|| 左假值 → 两个 side 都被调用\n{out}"
    );
    assert_eq!(line(&out, "or_calls"), "or_calls=/3|string");
    // `1 && 2 || 3 && 4`：左侧算完（S:1、S:2），右侧 `3 && 4` 整体被短路
    assert_eq!(
        out.matches("S:1\n").count(),
        1,
        "S:1 只出现一次\n{out}"
    );
    assert_eq!(out.matches("S:2\n").count(), 2, "S:2 出现两次（and_calls 与 chain）\n{out}");
    assert_eq!(out.matches("S:3\n").count(), 1, "S:3 只来自 or_calls——chain 的右半被短路\n{out}");
    assert!(!out.contains("S:4"), "side(4) 绝不应被调用\n{out}");
    assert_eq!(line(&out, "chain"), "chain=/2|string");
    assert_eq!(line(&out, "mixed"), "mixed=5/array|string");
    assert_eq!(line(&out, "not_not"), "not_not=true/false/true|string");
}

/// switch 值形态与编译期折叠边界
#[test]
fn switch_shapes_and_constant_folding_boundaries() {
    let out = run_lines(
        r#"
t("sw_dup", function() { local x = 2; switch (x) { case 2: return "first"; case 2: return "second"; } return "none"; });
t("sw_min", function() { local x = -9223372036854775808; switch (x) { case -9223372036854775808: return "min"; } return "none"; });
t("sw_float_eq_int", function() { local x = 2.0; switch (x) { case 2: return "i2"; } return "none"; });
t("sw_default_first", function() { return compilestring("local x = 9; switch (x) { default: return 42; }", "sdf") == null ? "compile-null" : "compiled"; });
t("sw_break_in_loop", function() { local out = ""; for (local i = 0; i < 3; i++) { switch (i) { case 1: break; default: out += i; } out += "|"; } return out; });
t("sw_case_expr", function() { local x = 5, y = 5; switch (x) { case y: return "var-case"; } return "none"; });
t("cf_div0", function() { return 1 / 0; });
t("cf_mod0", function() { return 7 % 0; });
t("cf_div0f", function() { return 1.0 / 0.0; });
t("cf_mod0f", function() { return 1.0 % 0.0; });
t("cf_ovf_add", function() { return 9223372036854775807 + 1; });
t("cf_ovf_mul", function() { return 3037000500 * 3037000500; });
t("cf_shift64", function() { return 1 << 64; });
t("cf_shift_neg", function() { return 1 << -1; });
t("cf_float_add", function() { return 0.1 + 0.2; });
t("cf_str_concat", function() { return "a" + "b"; });
"#,
    );
    assert_eq!(line(&out, "sw_dup"), "sw_dup=first|string", "重复 case 先到先得");
    assert_eq!(line(&out, "sw_min"), "sw_min=min|string");
    assert_eq!(line(&out, "sw_float_eq_int"), "sw_float_eq_int=i2|string", "浮点与整数 case 交叉匹配");
    assert_eq!(
        line(&out, "sw_default_first"),
        "sw_default_first=compiled|string",
        "case 在 default 之后才是语法错；default 在前合法"
    );
    assert_eq!(line(&out, "sw_break_in_loop"), "sw_break_in_loop=0||2||string", "break 归 switch 不归循环");
    assert_eq!(line(&out, "sw_case_expr"), "sw_case_expr=var-case|string", "case 标签可以是变量表达式");
    assert_eq!(line(&out, "cf_div0"), "cf_div0!division by zero", "整数除法**不**折叠");
    assert_eq!(line(&out, "cf_mod0"), "cf_mod0=7|integer", "% 竟然被编译期折叠（a % 0 → a）");
    assert_eq!(line(&out, "cf_div0f"), "cf_div0f=inf|float");
    assert_eq!(line(&out, "cf_mod0f"), "cf_mod0f=nan|float");
    assert_eq!(line(&out, "cf_ovf_add"), "cf_ovf_add=-9223372036854775808|integer");
    assert_eq!(line(&out, "cf_ovf_mul"), "cf_ovf_mul=-9223372036709301616|integer");
    assert_eq!(line(&out, "cf_shift64"), "cf_shift64=1|integer");
    assert_eq!(line(&out, "cf_shift_neg"), "cf_shift_neg=-9223372036854775808|integer");
    assert_eq!(line(&out, "cf_float_add"), "cf_float_add=0.3|float");
    assert_eq!(line(&out, "cf_str_concat"), "cf_str_concat=ab|string");
}

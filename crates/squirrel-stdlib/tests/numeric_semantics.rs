//! 数值 / 运算符边界语义回归（↔ sqvm.cpp 的 ARITH_OP / BW_OP / CMP_OP / ObjCmp）。
//!
//! 期望值全部用参考实现实测确认（`python tmp-c2/check_rs_scripts.py <本文件>`）。
//! 这里挑的是**容易踩、且踩过**的点：
//!
//! * **移位计数掩码**：x86 的硬件移位只用计数低 6 位，`1 << 2147483647` == `1 << 63`
//!   == `INT64_MIN`。本实现原来对越界计数返回 0 —— 矩阵一次抓出 18 条差异。
//! * **`INT64_MIN / -1`**：x86 `idiv` 商溢出会触发硬件异常，参考实现直接
//!   `STATUS_INTEGER_OVERFLOW(0xC0000095)` 崩溃（用户实际看到过崩溃框）。
//!   本实现给回绕结果，不崩 —— 这条**不能**做成差分用例（参考实现活不下来）。
//! * **整数溢出回绕**：`INT64_MAX + 1`、`INT64_MIN - 1`、`INT64_MAX * 2` 都是回绕。
//! * **除零**：整数 `1/0` 报 `division by zero`；浮点 `1.0/0.0` = inf、`0.0/0.0` = nan。
//! * **int/float 混算**提升为浮点；`<=>` 对数字给 -1/0/1。
//! * **字面量**：`9223372036854775808`（超出 int64）解析成浮点。

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
    let rc = vm.run_buffer(src.as_bytes(), b"num.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 移位：计数只在低 6 位有效（x86 硬件语义，参考实现相同）
#[test]
fn shift_counts_are_masked_like_x86() {
    let src = r#"
        print((1 << 63) + "\n");
        print((1 << 64) + "\n");          // == 1 << 0
        print((1 << 65) + "\n");          // == 1 << 1
        print((1 << 2147483647) + "\n");  // == 1 << 63
        print((1 << -1) + "\n");          // == 1 << 63
        print((1 << -64) + "\n");         // == 1 << 0
        print((-1 >> 1) + "\n");          // 算术右移
        print((-1 >>> 1) + "\n");         // 逻辑右移
        print((9223372036854775807 >> 62) + "\n");
        print((1 << 7) + "/" + (256 >> 4) + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "-9223372036854775808\n1\n2\n-9223372036854775808\n-9223372036854775808\n\
         1\n-1\n9223372036854775807\n1\n128/16\n"
    );
}

/// 整数溢出回绕 / 除零 / 取模符号
///
/// 注意 `%` 的两个怪行为（**参考实现就是这样，两侧实测一致**）：
/// * `1 % 0` 被编译器**常量折叠**成 `1`；
/// * 运行时的 `a % b`（b == 0）**不报错**，函数返回 `null`（错误对象是 null）。
#[test]
fn integer_overflow_and_division_match_reference() {
    let src = r#"
        function t(f) { try { return "" + f(); } catch (e) { return "E:" + e; } }
        print(t(function() { return 9223372036854775807 + 1; }) + "\n");
        print(t(function() { return (-9223372036854775807 - 1) - 1; }) + "\n");
        print(t(function() { return 9223372036854775807 * 2; }) + "\n");
        print(t(function() { return 1 / 0; }) + "\n");
        print(t(function() { local a = 1; local b = 0; return a / b; }) + "\n");
        print(t(function() { return 1 % 0; }) + "\n");
        print(t(function() { local a = 5; local b = 0; return a % b; }) + "\n");
        print(t(function() { local a = -1; local b = 0; return a % b; }) + "\n");
        print(t(function() { return -7 / 2; }) + "\n");
        print(t(function() { return -7 % 2; }) + "\n");
        print(t(function() { return 7 % -2; }) + "\n");
        print(t(function() { return 7 / 2; }) + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "-9223372036854775808\n9223372036854775807\n-2\nE:division by zero\n\
         E:division by zero\n1\nnull\nnull\n-3\n-1\n1\n3\n"
    );
}

/// `INT64_MIN / -1`：x86 `idiv` 商溢出会触发硬件异常，**参考实现直接崩溃**
/// （`STATUS_INTEGER_OVERFLOW` 0xC0000095，用户实际见过这个崩溃框）。
/// 本实现给回绕结果 —— 这条不能进差分语料（参考实现活不下来），所以单独守门。
/// ref-crash-skip（给 tmp-c2/check_rs_scripts.py 用：别拿这段去跑参考实现）
#[test]
fn min_div_minus_one_does_not_crash() {
    let src = r#"
        function t(f) { try { return "" + f(); } catch (e) { return "E:" + e; } }
        print(t(function() { return (-9223372036854775807 - 1) / -1; }) + "\n");
        print(t(function() { return (-9223372036854775807 - 1) % -1; }) + "\n");
        print(t(function() { return -(-9223372036854775807 - 1); }) + "\n");
        print(t(function() { return abs(-9223372036854775807 - 1); }) + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(out, "-9223372036854775808\n0\n-9223372036854775808\n0\n");
}

/// 浮点特殊值、int/float 混算、比较、转换
#[test]
fn float_specials_and_mixed_arithmetic_match_reference() {
    let src = r#"
        function t(f) { try { return "" + f(); } catch (e) { return "E:" + e; } }
        print(t(function() { return 1.0 / 0.0; }) + "\n");
        print(t(function() { return -1.0 / 0.0; }) + "\n");
        print(t(function() { return 0.0 / 0.0; }) + "\n");
        print(t(function() { return 1.0 % 0.0; }) + "\n");
        print(t(function() { return 2 + 1.5; }) + "\n");
        print(t(function() { return 2 * 0.5; }) + "\n");
        print(t(function() { return (1.0 / 0.0) > 1.0e308; }) + "\n");
        print(t(function() { return (0.0 / 0.0) == (0.0 / 0.0); }) + "\n");
        print(t(function() { return (0.0 / 0.0) != (0.0 / 0.0); }) + "\n");
        print(t(function() { return (1.0 / 0.0).tointeger(); }) + "\n");
        print(t(function() { return (0.0 / 0.0).tointeger(); }) + "\n");
        print(t(function() { return (1.0e100).tointeger(); }) + "\n");
        print(t(function() { return 9223372036854775808; }) + "/" + typeof 9223372036854775808 + "\n");
        print(t(function() { return -9223372036854775808; }) + "/" + typeof -9223372036854775808 + "\n");
        print(t(function() { return (1 == 1.0) + "/" + (1 <=> 2.0) + "/" + (2.0 <=> 1); }) + "\n");
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "inf\n-inf\n-nan(ind)\nnan\n3.5\n1\nfalse\nfalse\ntrue\n\
         -9223372036854775808\n-9223372036854775808\n-9223372036854775808\n\
         -9223372036854775808/integer\n-9223372036854775808/integer\ntrue/-1/1\n"
    );
}

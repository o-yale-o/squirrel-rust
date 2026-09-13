//! math 库定义域/边界语义回归（`sqstdlib/sqstdmath.cpp`）。
//!
//! 这一轮是**验证型**：新矩阵 `tests-corpus/t_mathmap.nut` 377 行与参考实现
//! **逐位一致**，没有需要修的差异。测试的用处是把它钉住 —— 下面每一条换成
//! "看起来更自然"的写法都会分叉：
//!
//! 1. **`SQFloat` 是 f32，但计算走 double**：参考实现是 `(SQFloat)sqrt((double)f)`
//!    （`SINGLE_ARG_FUNC` 把 float 提升为 double 调 CRT，再降回 f32）。
//!    改成 `f32::sin` 之类的纯 f32 路径，多项式不同就会在**尾位**分叉 ——
//!    默认打印格式（约 6 位有效数字）看不出来，所以这里比 17 位。
//! 2. **`abs` 是 `abs((int)n)`**：内部先截断成 **32 位** 再取绝对值。
//!    `abs(2147483648)`→`-2147483648`、`abs(-2147483649)`→`2147483647`、
//!    `abs(INT64_MAX)`→`1`、`abs(INT64_MIN)`→`0`、`abs(5000000000)`→`705032704`。
//!    写成 `i64::abs` 全都会不一样。
//! 3. **`abs` 只吃整数**（`sq_getinteger`，浮点先截断；浮点版是 `fabs`），
//!    `floor`/`ceil` 返回的是 **float** 而不是 int。
//! 4. **定义域边界不报错**：`sqrt(-1)`→NaN、`log(0)`→`-inf`、`log(-1)`→NaN、
//!    `asin(2)`→NaN、`exp(f32::MAX)`→inf、`pow(0,-1)`→inf、`pow(-1,0.5)`→NaN。
//!    NaN 的文本是 MSVC 的 `-nan(ind)`，我们逐字复刻。
//! 5. `PI` 是 `(float)M_PI`，**`RAND_MAX` 是 32767**（MSVC 的 CRT 就是这么定义的，
//!    不是 2147483647）；`rand()` 的**值**跨实现不可比（CRT 序列另有
//!    `t_math.nut` 钉住），这里只钉类型与 `srand` 的返回值 `null`。
//!
//! 观测口径：整数按十进制原样打印。**不能**给整数套 `format("%.17g", ...)` ——
//! 那会把整数转成 f32（`SQFloat`），`abs(2147483647)` 会被打成 `2147483648`，
//! 整数用例就白测了（这一点在写矩阵时踩过）。

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
    let rc = vm.run_buffer(src.as_bytes(), b"mathmap.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 每行 `tag=值|类型` 或 `tag!错误`（与语料 `t_mathmap.nut` 同一口径）
fn run_lines(body: &str) -> String {
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

/// 定义域外不报错，结果按 IEEE（NaN 的文本是 MSVC 的 `-nan(ind)`）
#[test]
fn math_domain_edges_return_ieee_values() {
    let out = run_lines(
        r#"
t("sqrt_n1", function() { return sqrt(-1.0); });
t("log_p0", function() { return log(0.0); });
t("log_n1", function() { return log(-1.0); });
t("asin_p2", function() { return asin(2.0); });
t("acos_n2", function() { return acos(-2.0); });
t("exp_fmax", function() { return exp(3.4028235e38); });
t("pow_0_n1", function() { return pow(0.0, -1.0); });
t("pow_n1_half", function() { return pow(-1.0, 0.5); });
t("atan2_00", function() { return atan2(0.0, 0.0); });
t("sqrt_nan", function() { return sqrt(0.0 / 0.0); });
t("floor_inf", function() { return floor(1.0 / 0.0); });
t("fabs_n0", function() { return fabs(-0.0); });
"#,
    );
    assert_eq!(line(&out, "sqrt_n1"), "sqrt_n1=-nan(ind)|float");
    assert_eq!(line(&out, "log_p0"), "log_p0=-inf|float");
    assert_eq!(line(&out, "log_n1"), "log_n1=-nan(ind)|float");
    assert_eq!(line(&out, "asin_p2"), "asin_p2=-nan(ind)|float");
    assert_eq!(line(&out, "acos_n2"), "acos_n2=-nan(ind)|float");
    assert_eq!(line(&out, "exp_fmax"), "exp_fmax=inf|float");
    assert_eq!(line(&out, "pow_0_n1"), "pow_0_n1=inf|float");
    assert_eq!(line(&out, "pow_n1_half"), "pow_n1_half=-nan(ind)|float");
    assert_eq!(line(&out, "atan2_00"), "atan2_00=0|float");
    assert_eq!(line(&out, "sqrt_nan"), "sqrt_nan=-nan(ind)|float");
    assert_eq!(line(&out, "floor_inf"), "floor_inf=inf|float");
    assert_eq!(line(&out, "fabs_n0"), "fabs_n0=0|float");
}

/// 中间 double、结果 f32：17 位有效数字下与参考逐位一致
#[test]
fn math_results_are_bit_exact_f32() {
    let out = run_lines(
        r#"
t("sqrt2", function() { return sqrt(2.0); });
t("sin1", function() { return sin(1.0); });
t("sin_big", function() { return sin(1e30); });
t("cos1", function() { return cos(1.0); });
t("tan1", function() { return tan(1.0); });
t("atan1", function() { return atan(1.0); });
t("atan2_11", function() { return atan2(1.0, 1.0); });
t("atan2_1n1", function() { return atan2(1.0, -1.0); });
t("exp1", function() { return exp(1.0); });
t("log_pi", function() { return log(3.1415927); });
t("log10_1000", function() { return log10(1000.0); });
t("pow_2_10", function() { return pow(2.0, 10.0); });
t("pow_half_half", function() { return pow(0.5, 0.5); });
t("pi", function() { return PI; });
t("near1", function() { return sqrt(0.99999994); });
"#,
    );
    // 只改一处（例如 f64→f32 计算路径）这些尾位就会变
    assert_eq!(line(&out, "sqrt2"), "sqrt2=1.4142135381698608|float");
    assert_eq!(line(&out, "sin1"), "sin1=0.84147095680236816|float");
    assert_eq!(line(&out, "sin_big"), "sin_big=-0.79116344451904297|float");
    assert_eq!(line(&out, "cos1"), "cos1=0.54030227661132812|float");
    assert_eq!(line(&out, "tan1"), "tan1=1.5574077367782593|float");
    assert_eq!(line(&out, "atan1"), "atan1=0.78539818525314331|float");
    assert_eq!(line(&out, "atan2_11"), "atan2_11=0.78539818525314331|float");
    assert_eq!(line(&out, "atan2_1n1"), "atan2_1n1=2.3561944961547852|float");
    assert_eq!(line(&out, "exp1"), "exp1=2.7182817459106445|float");
    assert_eq!(line(&out, "log_pi"), "log_pi=1.1447299718856812|float");
    assert_eq!(line(&out, "log10_1000"), "log10_1000=3|float");
    assert_eq!(line(&out, "pow_2_10"), "pow_2_10=1024|float");
    assert_eq!(line(&out, "pow_half_half"), "pow_half_half=0.70710676908493042|float");
    assert_eq!(line(&out, "pi"), "pi=3.1415927410125732|float");
    assert_eq!(line(&out, "near1"), "near1=0.99999994039535522|float");
}

/// `abs` 是 `abs((int)n)`：32 位截断；且只吃整数
#[test]
fn abs_truncates_to_32_bits() {
    let out = run_lines(
        r#"
t("p7", function() { return abs(7); });
t("n7", function() { return abs(-7); });
t("f79", function() { return abs(-7.9); });
t("f32k", function() { return abs(-32000.75); });
t("i32max", function() { return abs(2147483647); });
t("i32min", function() { return abs(-2147483648); });
t("over_i32max", function() { return abs(2147483648); });
t("over_i32min", function() { return abs(-2147483649); });
t("i64max", function() { return abs(9223372036854775807); });
t("i64min", function() { return abs(-9223372036854775808); });
t("big3e9", function() { return abs(-3000000000); });
t("big5e9", function() { return abs(5000000000); });
t("t_f", function() { return typeof abs(-7.9); });
t("t_fabs", function() { return fabs(-32000.75); });
t("floor_t", function() { return typeof floor(2.5); });
t("ceil_t", function() { return typeof ceil(2.5); });
t("floor_v", function() { return floor(-2.5); });
t("str", function() { return abs("x"); });
t("none", function() { return abs(); });
"#,
    );
    assert_eq!(line(&out, "p7"), "p7=7|integer");
    assert_eq!(line(&out, "n7"), "n7=7|integer");
    assert_eq!(line(&out, "f79"), "f79=7|integer", "浮点被截断成整数再 abs");
    assert_eq!(line(&out, "f32k"), "f32k=32000|integer");
    assert_eq!(line(&out, "i32max"), "i32max=2147483647|integer");
    assert_eq!(line(&out, "i32min"), "i32min=-2147483648|integer", "abs(INT32_MIN) 仍是它自己");
    assert_eq!(line(&out, "over_i32max"), "over_i32max=-2147483648|integer", "先截 32 位再取绝对值");
    assert_eq!(line(&out, "over_i32min"), "over_i32min=2147483647|integer");
    assert_eq!(line(&out, "i64max"), "i64max=1|integer", "(int)INT64_MAX == -1");
    assert_eq!(line(&out, "i64min"), "i64min=0|integer");
    assert_eq!(line(&out, "big3e9"), "big3e9=1294967296|integer");
    assert_eq!(line(&out, "big5e9"), "big5e9=705032704|integer");
    assert_eq!(line(&out, "t_f"), "t_f=integer|string");
    assert_eq!(line(&out, "t_fabs"), "t_fabs=32000.75|float", "fabs 才是浮点版");
    assert_eq!(line(&out, "floor_t"), "floor_t=float|string", "floor/ceil 返回 float");
    assert_eq!(line(&out, "ceil_t"), "ceil_t=float|string");
    assert_eq!(line(&out, "floor_v"), "floor_v=-3|float");
    assert_eq!(
        line(&out, "str"),
        "str!parameter 1 has an invalid type 'string' ; expected: 'integer|float'"
    );
    assert_eq!(line(&out, "none"), "none!wrong number of parameters");
}

/// `PI` / `RAND_MAX` 的精确值，以及整数参数经 `.n` 掩码被接受
#[test]
fn math_constants_and_integer_args() {
    let out = run_lines(
        r#"
t("pi", function() { return PI; });
t("pi_t", function() { return typeof PI; });
t("randmax", function() { return RAND_MAX; });
t("srand_ret", function() { return srand(1); });
t("rand_t", function() { srand(7); return typeof rand(); });
t("sqrt_int", function() { return sqrt(9); });
t("sin_int", function() { return sin(1); });
t("floor_int", function() { return floor(5); });
t("sqrt_none", function() { return sqrt(); });
t("sqrt_two", function() { return sqrt(1.0, 2.0); });
t("sqrt_str", function() { return sqrt("x"); });
t("pow_one", function() { return pow(1.0); });
t("pow_str", function() { return pow(1.0, "x"); });
"#,
    );
    assert_eq!(line(&out, "pi"), "pi=3.1415927410125732|float", "(float)M_PI");
    assert_eq!(line(&out, "pi_t"), "pi_t=float|string");
    assert_eq!(line(&out, "randmax"), "randmax=32767|integer", "MSVC 的 RAND_MAX 是 32767");
    assert_eq!(line(&out, "srand_ret"), "srand_ret=null|null");
    assert_eq!(line(&out, "rand_t"), "rand_t=integer|string");
    assert_eq!(line(&out, "sqrt_int"), "sqrt_int=3|float", "整数经 .n 掩码被接受");
    assert_eq!(line(&out, "sin_int"), "sin_int=0.84147095680236816|float");
    assert_eq!(line(&out, "floor_int"), "floor_int=5|float");
    assert_eq!(line(&out, "sqrt_none"), "sqrt_none!wrong number of parameters");
    assert_eq!(line(&out, "sqrt_two"), "sqrt_two!wrong number of parameters");
    assert_eq!(
        line(&out, "sqrt_str"),
        "sqrt_str!parameter 1 has an invalid type 'string' ; expected: 'integer|float'"
    );
    assert_eq!(line(&out, "pow_one"), "pow_one!wrong number of parameters");
    assert_eq!(
        line(&out, "pow_str"),
        "pow_str!parameter 2 has an invalid type 'string' ; expected: 'integer|float'"
    );
}

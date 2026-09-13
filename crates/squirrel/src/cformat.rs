//! CRT 兼容的 printf 浮点格式化（%g/%f/%e）。
//!
//! MSVC 的 printf("%g", x) 与 Rust 的 `format!("{}", x)` 语义不同：
//! %g 取 6 位有效数字、按指数切换定点/科学计数、去掉尾随零、指数至少两位。
//! print/tostring 的输出必须逐字节一致（差分测试），故手工实现。

/// ↔ printf("%g", v)（默认精度 6）
pub fn format_g(value: f64) -> String {
    format_g_prec(value, 6)
}

/// ↔ printf("%.<prec>g", v)（prec=0 按 1 处理，同 C 标准）
pub fn format_g_prec(value: f64, prec: usize) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let p = if prec == 0 { 1 } else { prec };

    // 先按 p 位有效数字做科学计数舍入，取指数 X
    let formatted_e = format!("{:.*e}", p - 1, value);
    let (mant, exp) = formatted_e.split_once('e').unwrap();
    let x: i32 = exp.parse().unwrap();

    if x < -4 || x >= p as i32 {
        // e 风格：精度 p-1，去尾随零；MSVC 指数至少 2 位（如 1e+010）
        let mut m = mant.to_string();
        if m.contains('.') {
            while m.ends_with('0') {
                m.pop();
            }
            if m.ends_with('.') {
                m.pop();
            }
        }
        if m.is_empty() || m == "-" {
            m.push('0');
        }
        format!("{}e{}{:02}", m, if x < 0 { "-" } else { "+" }, x.abs())
    } else {
        // f 风格：精度 p-1-X，去尾随零
        let decimals = (p as i32 - 1 - x).max(0) as usize;
        let s = format!("{:.*}", decimals, value);
        let mut r = s;
        if r.contains('.') {
            while r.ends_with('0') {
                r.pop();
            }
            if r.ends_with('.') {
                r.pop();
            }
        }
        r
    }
}

/// ↔ printf("%.<prec>e", v)（MSVC 指数至少 2 位，**不**去尾随零）
///
/// CRT 的 %e 与 %g 不同：%e 保留全部有效位（`format("%e", 1234.5)` → `1.234500e+03`）。
pub fn format_e_crt(value: f64, prec: usize) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    let s = format!("{:.*e}", prec, value);
    let (m, e) = s.split_once('e').unwrap();
    let x: i32 = e.parse().unwrap();
    format!("{}e{}{:02}", m, if x < 0 { "-" } else { "+" }, x.abs())
}

/// ↔ printf("%.<prec>f", v)
pub fn format_f(value: f64, prec: usize) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    format!("{:.*}", prec, value)
}

/// ↔ printf("%.<prec>e", v)（MSVC 指数至少 2 位）
pub fn format_e(value: f64, prec: usize) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    let s = format!("{:.*e}", prec, value);
    let (m, e) = s.split_once('e').unwrap();
    let x: i32 = e.parse().unwrap();
    let mut mant = m.to_string();
    if mant.contains('.') {
        while mant.ends_with('0') {
            mant.pop();
        }
        if mant.ends_with('.') {
            mant.pop();
        }
    }
    if mant.is_empty() || mant == "-" {
        mant.push('0');
    }
    format!("{}e{}{:02}", mant, if x < 0 { "-" } else { "+" }, x.abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g_format_matches_msvc() {
        // 与 MSVC CRT printf("%g") 的已知输出对比
        assert_eq!(format_g(0.1), "0.1");
        assert_eq!(format_g(1.0 / 3.0), "0.333333");
        assert_eq!(format_g(2.0), "2");
        assert_eq!(format_g(-2.5), "-2.5");
        assert_eq!(format_g(100000.0), "100000");
        assert_eq!(format_g(1000000.0), "1e+06");
        assert_eq!(format_g(1234567.0), "1.23457e+06");
        assert_eq!(format_g(0.0001), "0.0001");
        assert_eq!(format_g(0.00001), "1e-05");
        assert_eq!(format_g(1.5e300), "1.5e+300");
        assert_eq!(format_g(0.0), "0");
    }

    #[test]
    fn g14_format() {
        // PrintObjVal 用 %.14g
        assert_eq!(format_g_prec(1.0f32 as f64 / 3.0, 14), "0.33333333333333");
    }

    #[test]
    fn f_format() {
        assert_eq!(format_f(1.5, 2), "1.50");
        assert_eq!(format_f(0.125, 3), "0.125");
    }
}

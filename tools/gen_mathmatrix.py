# -*- coding: utf-8 -*-
"""生成 math 库**定义域/边界**矩阵语料 tests-corpus/t_mathmap.nut。

为什么还要一份（`t_math.nut` 已经有基础形态）：
1. `t_math.nut` 打印的是默认浮点格式（约 6 位有效数字），**尾位差异看不见**；
   这里统一用 `format("%.17g", x)` 暴露 f32 的精确值 —— `SQFloat` 是 f32，
   但参考实现是 `(SQFloat)sqrt((double)f)`：**中间用 double、结果降回 f32**。
   我们若直接用 f32 版本（`f32::sin` 等），多项式不同就会在尾位上分叉。
2. 定义域边界完全没覆盖：`sqrt(-1)`、`log(0)`、`log(-1)`、`asin(2)`、`exp(1000)`、
   `pow(-1,0.5)`、`pow(0,-1)`、`pow(1e30,2)`、`atan2(0,0)`、±inf/NaN 进每个函数。
3. `abs` 是 `abs((int)n)` —— **32 位截断**（`t_math.nut` 只钉了 `abs(-3000000000)`）：
   这里补 INT32 边界、INT64 边界、以及 `abs` 只吃整数（`sq_getinteger`）而
   `fabs` 吃浮点的区别。
4. NaN 的**符号**：参考实现走 `(SQFloat)pow((double), (double))`，`pow(±NaN, 奇整数)`
   会清掉底数 NaN 的符号返回**正** NaN，其余情况保留输入符号 —— 纯 f32 `powf`
   在这一格分叉（fuzzer seed=2554 抓到）。`sgn_pow_*` / `sgn_atan2_*` 把两个方向
   与指数边界（f32 最大奇整数 2^24-1）都钉住。

安全约束（参考实现自己的 UB，一律不进语料）：
- `abs(NaN)` / `abs(±inf)`：`sq_getinteger` 内部是 `(SQInteger)f`，NaN/inf 转整数是 UB；
- `rand()` 的**值**不可跨实现比较（`t_math.nut` 已单独钉住 CRT 序列）。

用法：
    python tools/gen_mathmatrix.py
"""
import os

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_mathmap.nut")

UNARY = ["sqrt", "fabs", "sin", "cos", "tan", "asin", "acos", "atan",
         "log", "log10", "exp", "floor", "ceil"]

# (标签, 表达式) —— 表达式直接写进脚本，特殊值用表达式构造
ARGS = [
    ("p0", "0.0"),
    ("n0", "-0.0"),
    ("p1", "1.0"),
    ("n1", "-1.0"),
    ("half", "0.5"),
    ("nhalf", "-0.5"),
    ("p2", "2.0"),
    ("n2", "-2.0"),
    ("p3", "3.0"),
    ("big", "1e30"),
    ("nbig", "-1e30"),
    ("tiny", "1e-30"),
    ("ntiny", "-1e-30"),
    ("fmax", "3.4028235e38"),
    ("nfmax", "-3.4028235e38"),
    ("fmin", "1.1754944e-38"),
    ("inf", "(1.0 / 0.0)"),
    ("ninf", "(-1.0 / 0.0)"),
    ("nan", "(0.0 / 0.0)"),
    ("i9", "9"),
    ("ni9", "-9"),
    ("near1", "0.99999994"),
    ("e", "2.7182817"),
    ("pi", "3.1415927"),
]

BINARY = ["pow", "atan2"]
BARGS = [
    ("p0", "0.0"), ("n0", "-0.0"), ("p1", "1.0"), ("n1", "-1.0"),
    ("half", "0.5"), ("nhalf", "-0.5"), ("p2", "2.0"), ("n2", "-2.0"),
    ("p10", "10.0"), ("n10", "-10.0"),
    ("inf", "(1.0 / 0.0)"), ("ninf", "(-1.0 / 0.0)"), ("nan", "(0.0 / 0.0)"),
    ("big", "1e30"), ("i2", "2"), ("i3", "3"),
]

# abs：只吃整数（sq_getinteger），且内部 abs((int)n) 是 32 位截断
ABS = [
    ("p0", "0"), ("p7", "7"), ("n7", "-7"),
    ("f79", "-7.9"), ("f19", "1.9"),
    ("i32max", "2147483647"), ("i32min", "-2147483648"),
    ("over_i32max", "2147483648"), ("over_i32min", "-2147483649"),
    ("i64max", "9223372036854775807"),
    ("i64min", "-9223372036854775808"),
    ("big3e9", "-3000000000"), ("big5e9", "5000000000"),
    ("f32k", "-32000.75"),
]

lines = [
    "// 自动生成：math 库定义域/边界矩阵（tools/gen_mathmatrix.py）",
    "// obs()：整数按十进制原样打印（**不能**过 %.17g —— 那会把整数转成 f32，",
    "//        例如 abs(2147483647) 会被打成 2147483648，整数用例就白测了）；",
    "//        浮点用 %.17g 暴露 f32 的精确值（默认格式只有约 6 位有效数字，看不出尾位差）。",
    "// 注意 SQFloat = f32，而参考实现是 (SQFloat)sqrt((double)f)：中间 double、结果降 f32",
    "function obs(v) {",
    "    local ty = typeof v;",
    "    if (ty == \"float\") return format(\"%.17g\", v);",
    "    return \"\" + v;",
    "}",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=\" + obs(v) + \"|\" + typeof v + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "",
]

n_unary = 0
for fn in UNARY:
    for aname, aexpr in ARGS:
        lines.append('t("u_%s_%s", function() { return %s(%s); });' % (fn, aname, fn, aexpr))
        n_unary += 1

n_bin = 0
for fn in BINARY:
    for aname, aexpr in BARGS:
        for bname, bexpr in BARGS:
            if fn == "atan2" and (aname, bname) not in (
                ("p0", "p0"), ("p1", "p1"), ("p1", "n1"), ("n1", "n1"),
                ("p0", "p1"), ("p1", "p0"), ("inf", "inf"), ("ninf", "p1"),
                ("nan", "p1"), ("big", "big"), ("i2", "i3"), ("half", "nhalf"),
            ):
                continue
            if fn == "pow" and (aname, bname) not in (
                ("p0", "p0"), ("p0", "n1"), ("p0", "p1"), ("p1", "p0"),
                ("p2", "p10"), ("p2", "n10"), ("n2", "p2"), ("n2", "p3"),
                ("n1", "half"), ("n1", "nhalf"), ("p1", "inf"), ("p1", "ninf"),
                ("half", "half"), ("half", "nhalf"), ("big", "p2"), ("i2", "i3"),
                ("p10", "n10"), ("nan", "p0"), ("inf", "n1"), ("ninf", "p2"),
            ):
                continue
            lines.append('t("b_%s_%s_%s", function() { return %s(%s, %s); });'
                         % (fn, aname, bname, fn, aexpr, bexpr))
            n_bin += 1

for aname, aexpr in ABS:
    lines.append('t("abs_%s", function() { return abs(%s); });' % (aname, aexpr))
lines.append('t("abs_str", function() { return abs("x"); });')
lines.append('t("abs_none", function() { return abs(); });')
lines.append('t("abs_two", function() { return abs(1, 2); });')
lines.append('t("abs_type", function() { return typeof abs(-7.9); });')
lines.append('t("fabs_of_abs", function() { return fabs(-32000.75); });')

# ---------------------------------------------------------------------------
# NaN 符号矩阵：`pow` 的"指数是奇整数就清掉底数 NaN 的符号"规则
#
# 为什么单列一节：`t_mathmap.nut` 原先只有 `pow(nan, 0)` 一个 NaN 底数用例，
# 而**奇整数指数**这一格恰恰是 f32 `powf` 与参考实现 `(SQFloat)pow((double),
# (double))` 分叉的地方（fuzzer seed=2554 抓到的）：x86 上 `0.0/0.0` 是负 NaN，
# 参考实现在奇整数指数下返回**正** NaN，其余情况保留输入 NaN 的符号。
# 这里把两个方向（负 NaN / 正 NaN 底数）和指数边界都钉住。
lines.append('// NaN 符号矩阵：pow 的奇整数指数规则 + atan2 的 NaN/inf 传播')
lines.append('local NEG_NAN = (0.0 / 0.0);')
lines.append('local POS_NAN = -(0.0 / 0.0);')

lines.append('t("sgn_nan_neg_text", function() { return NEG_NAN; });')
lines.append('t("sgn_nan_pos_text", function() { return POS_NAN; });')
lines.append('t("sgn_nan_div", function() { return 1.0 / NEG_NAN; });')
n_sgn = 3
# 注意：**不能用 `1.0 / NAN < 0` 判断 NaN 的符号** —— 与 NaN 的比较恒为 false，
# 那样写出来的用例两侧都会"一致"，但什么都没测到（本节初稿踩过这个坑，
# `sgn_nan_kind` 两种 NaN 都报 `pos`）。符号只能从文本渲染上看出来。

# f32 能表示的最大奇整数是 2^24-1；超过 2^24 的浮点整数一律是偶数（16777217 会被舍入到 2^24）
NAN_EXPS = [
    ("p0", "0.0"), ("n0", "-0.0"),
    ("p1", "1.0"), ("n1", "-1.0"), ("p3", "3.0"), ("n3", "-3.0"),
    ("p2", "2.0"), ("n2", "-2.0"), ("p10", "10.0"), ("n10", "-10.0"),
    ("half", "0.5"), ("nhalf", "-0.5"), ("one5", "1.5"),
    ("odd24", "16777215.0"), ("pow24", "16777216.0"), ("over24", "16777217.0"),
    ("e17", "1e17"), ("big", "1e30"), ("nbig", "-1e30"),
    ("inf", "(1.0 / 0.0)"), ("ninf", "(-1.0 / 0.0)"),
    ("nan", "(0.0 / 0.0)"),
    ("i2", "2"), ("i3", "3"),
]
for base_name, base_expr in (("n", "NEG_NAN"), ("p", "POS_NAN")):
    for ename, eexpr in NAN_EXPS:
        lines.append('t("sgn_pow_%s_%s", function() { return pow(%s, %s); });'
                     % (base_name, ename, base_expr, eexpr))
        n_sgn += 1

# 指数是 NaN、底数是 ±2：符号同样要跟参考一致（负底数 + NaN 指数是"定义域错"路径）
for bname, bexpr in (("p2", "2.0"), ("n2", "-2.0"), ("p0", "0.0"), ("n0", "-0.0")):
    lines.append('t("sgn_pow_%s_nan", function() { return pow(%s, NEG_NAN); });' % (bname, bexpr))
    lines.append('t("sgn_pow_%s_posnan", function() { return pow(%s, POS_NAN); });' % (bname, bexpr))
    n_sgn += 2

# atan2 的 NaN/inf/±0 传播（参考实现走 double，我们用 f32::atan2）—— 12×12 全交叉
ATAN2_VALS = [
    ("p0", "0.0"), ("n0", "-0.0"), ("p1", "1.0"), ("n1", "-1.0"),
    ("pinf", "(1.0 / 0.0)"), ("ninf", "(-1.0 / 0.0)"),
    ("pnan", "POS_NAN"), ("nnan", "NEG_NAN"),
    ("big", "1e30"), ("nbig", "-1e30"), ("tiny", "1e-30"), ("ntiny", "-1e-30"),
]
for aname, aexpr in ATAN2_VALS:
    for bname, bexpr in ATAN2_VALS:
        lines.append('t("sgn_atan2_%s_%s", function() { return atan2(%s, %s); });'
                     % (aname, bname, aexpr, bexpr))
        n_sgn += 1

lines.append('t("pi_exact", function() { return PI; });')
lines.append('t("pi_type", function() { return typeof PI; });')
lines.append('t("randmax_exact", function() { return RAND_MAX; });')
lines.append('t("srand_ret", function() { return srand(1); });')
lines.append('t("srand_big", function() { return srand(4294967296); });')
lines.append('t("rand_type", function() { srand(7); return typeof rand(); });')
lines.append('t("sqrt_none", function() { return sqrt(); });')
lines.append('t("sqrt_two", function() { return sqrt(1.0, 2.0); });')
lines.append('t("sqrt_str", function() { return sqrt("x"); });')
lines.append('t("pow_one", function() { return pow(1.0); });')
lines.append('t("pow_str", function() { return pow(1.0, "x"); });')
# 整数参数走 .n 掩码 → sq_getfloat 接受整数
lines.append('t("sqrt_int", function() { return sqrt(9); });')
lines.append('t("sin_int", function() { return sin(1); });')
lines.append('t("floor_int", function() { return floor(5); });')

lines.append('print("MATHMAP-DONE\\n");')

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")

print("已写出 %s：一元 %d 条 + 二元 %d 条 + NaN 符号 %d 条 + 其它" % (OUT, n_unary, n_bin, n_sgn))

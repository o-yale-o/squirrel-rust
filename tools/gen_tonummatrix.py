# -*- coding: utf-8 -*-
"""生成"字符串 → 数字"矩阵语料 tests-corpus/t_tonummap.nut。

为什么单开一份：`t_delmatrix.nut` 里的 `tointeger`/`tofloat` 用例全是**参数掩码**
测试（例如 `("").tointeger("s")`），**没有一个**在测字符串**解析**本身。于是
`default_delegate_tointeger` / `str2num`（↔ `sqbaselib.cpp` 的 `str2num`）这条
路径在语料里是结构性空白 —— 本轮由它抓到 4 处溢出差异（见 MIGRATION.md）。

覆盖的语义（都对着 `sqbaselib.cpp` 的 `str2num` 写）：
1. **先扫 '.'/'e'/'E' 决定走浮点还是整数**：`iseintbase = base > 13`，所以
   base 16 时 `'e'` 是数字而不是科学计数法（`"1e3".tointeger(16)` = 483）。
   扫描在**第一个 NUL 处停止**（`str2num` 用的是 C 字符串）。
2. **整数路径 = MSVC `_strtoi64`**：溢出**按符号饱和**（`"9223372036854775808"`
   → INT64_MAX、`"-9223372036854775809"` → INT64_MIN），不是回绕；
   带 `0x` 前缀只在 base 0/16 下有意义；遇非法字符**停止**（前缀语义）。
3. **浮点路径 = MSVC `strtod`**：十进制、十六进制浮点（`0x1.8p3`）、
   `inf`/`nan` 各种拼写（大小写不敏感）都在其列。
4. `base` 参数的合法范围是 0 或 2..36 —— **base 1/负数/>36 会让参考实现崩溃**
   （CRT invalid parameter handler，实测 exit=0xC0000409），属参考实现自身的 UB，
   不进语料（本实现在这些值上给"cannot convert the string"）。

用法：
    python tools/gen_tonummatrix.py
    python tools/gen_tonummatrix.py --list 12 15    # 查下标 → 字符串（差分失败时用）
"""
import os
import random
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_tonummap.nut")

# ---------------------------------------------------------------------------
# 1) 手工清单：每一类语义的边界
CURATED = [
    # 空/空白
    "", " ", "   ", "\t", "\n", "\t\n ", "\v12", "\f12", "\r\n12", " \t\n\v\f\r12",
    "12 ", " 12 ", "\t42\n", "  +  ",
    # 符号
    "12", "+12", "-12", "+ 12", "- 12", "--1", "++1", "1-2", "1+2", "+", "-",
    # 零与进制前缀（base 10 时都是"前缀语义"：解析到非法字符就停）
    "0", "-0", "+0", "007", "010", "0x10", "0X1f", "0xg", "0b101", "0o17",
    "0777", "0x", "0xzz", "0x7fffffffffffffff", "0xffffffffffffffff", "-0x10",
    # 科学计数法
    "1e3", "1E3", "1e+3", "1e-3", "1e", "1e+", "1e-", "e3", "E3", "e", "1.e",
    "1.2e5x", "1.2E5", ".e1", "1.2e", "1.2e+", "1.2e-", "0x1.8e3",
    # 小数点
    "1.9", "-1.9", "+1.9", ".5", "5.", "-.5", ".", "..", ".e", "1.2.3", "1..2",
    ".5.", "-.", "+.",
    # 尾随垃圾
    "12abc", "abc", "abc12", "12 34", "1_000", "12\u00a0x",
    # INT64 边界与溢出（整数路径按符号饱和）
    "9223372036854775806", "9223372036854775807", "9223372036854775808",
    "9223372036854775809", "18446744073709551615", "18446744073709551616",
    "99999999999999999999999999",
    "-9223372036854775806", "-9223372036854775807", "-9223372036854775808",
    "-9223372036854775809", "-18446744073709551615", "-18446744073709551616",
    "-99999999999999999999999999",
    "11111111111111111111111111111111111111111111111111",
    "11111111111111111111111111111111111111111111111111.5",
    # 浮点溢出/下溢（浮点路径不受 int64 限制）
    "1e999", "1e-999", "1e40", "3.4028235e38", "3.5e38", "1e-50", "1e-46",
    "0.1", "1.7976931348623157e308", "1e309",
    # inf / nan（**只有**含 '.' 或 'e' 才走浮点路径，"inf"/"infinity" 本身走整数路径）
    "inf", "INF", "Inf", "infinity", "Infinity", "nan", "NaN", "-nan",
    "inf.0", "nan.0", "-inf.0", "INF.0", "NaN.0", "inf.e", "infe", "inf5e",
    "nan5e", "nan.e", "-inf.e", "+inf.0",
    # 十六进制浮点（C99：走 strtod）
    "0x1.8p3", "0x1p3", "0X1.8P3", "0x.8p3", "0x1.8", "0x1.p3", "0x1.8p",
    "0x0.1p4", "0x1.8p+3", "0x1.8p-3", "0xff.8p1", "0x.8", "0x.",
    # NUL 截断：'.'/'e' 的扫描本身也在 NUL 处停
    ".5\x0012", "12\x00.5", "12\x00e5", "1\x00.5", "\x00.5", ".\x005",
    "e\x005", "5e\x003", "12\x00abc", "\x0021", "\x00", "34\x0056",
    "\xff12", " 12\x00",
]

# ---------------------------------------------------------------------------
# 2) 随机清单：固定种子，字母表覆盖所有会分叉的分支
ALPHABET = list("0123456789.eE+-xXpPiInNfFaAbBzZ \t") + ["\x00", "0x", "inf", "nan"]
rng = random.Random(20260101)
RANDOM = []
for _ in range(700):
    n = rng.randint(1, 10)
    s = "".join(rng.choice(ALPHABET) for _ in range(n))
    RANDOM.append(s)
# 定向随机：0x 前缀 + 十六进制浮点形状
for _ in range(150):
    intpart = "".join(rng.choice("0123456789abcdefABCDEF") for _ in range(rng.randint(0, 4)))
    frac = "".join(rng.choice("0123456789abcdefABCDEF") for _ in range(rng.randint(0, 4)))
    exp = rng.choice(["", "p", "P", "p+", "p-", "e", "E"]) + rng.choice(["", "0", "3", "-3", "+12"])
    RANDOM.append(rng.choice(["0x", "0X", "-0x", "+0x"]) + intpart + "." + frac + exp)
# 定向随机：长数字串（整数路径饱和 / 浮点路径精度）
for _ in range(150):
    n = rng.randint(15, 24)
    body = "".join(rng.choice("0123456789") for _ in range(n))
    RANDOM.append(rng.choice(["", "-", "+"]) + body + rng.choice(["", ".5", ".0", "e2", ".5e-3"]))

# base 参数：0 与 2..36 合法；1/负数/>36 是参考实现 UB，**不进语料**
BASES = [0, 2, 3, 8, 10, 13, 14, 16, 17, 35, 36]
BASE_STRS = ["ff", "FF", "0xff", "0Xff", "10", "010", "1e3", "1E3", "1.5",
             "56f0791e", "z", "Z", "101", "2", "9", "8", "12abc", "", " ",
             "-ff", "+ff", "0x", "0777", "777777777777777777777777",
             "ffffffffffffffff", "-ffffffffffffffff", "8000000000000000",
             "10000000000000000", "g", "zzzzzzzzzzzzzzzzzzzzzz", "1.5e3",
             "0x1.8p3", "inf.0", "12\u00a0x"]

FMTS = [
    ("i", 's.tointeger()'),
    ("f", 's.tofloat()'),
    ("t", 'typeof s.tointeger() + "/" + typeof s.tofloat()'),
]

if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(CURATED + RANDOM)) + 1):
        print("%4d  %r" % (k, (CURATED + RANDOM)[k]))
    sys.exit(0)

lines = [
    "// 自动生成：字符串 → 数字矩阵（tools/gen_tonummatrix.py）",
    "// 观测口径：整数原样打印、浮点用 %.17g（默认格式只有约 6 位有效数字）。",
    "// 注意 tag 用**下标**而不是字符串本身 —— 字符串里什么字符都有。",
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
    "local NUL = \"\\x00\";",
    "local CASE = [",
]
n_case = 0


def emit(s):
    """把 Python 字符串转成 Squirrel 字面量（只用 \\xNN 转义，避免任何原生字符）"""
    out = []
    for ch in s:
        out.append("\\x%02x" % ord(ch))
    return '"' + "".join(out) + '"'


for s in CURATED + RANDOM:
    lines.append("    %s," % emit(s))
    n_case += 1
lines.append("];")
lines.append("")

# 1) 无 base 的解析
for name, expr in FMTS:
    lines.append("for (local i = 0; i < CASE.len(); i++) {")
    lines.append("    local s = CASE[i];")
    lines.append('    t("%s" + i, function() { return %s; });' % (name, expr))
    lines.append("}")
lines.append("")

# 2) 带 base 的解析
lines.append("local BASES = [%s];" % ", ".join(str(b) for b in BASES))
lines.append("local BSTR = [")
for s in BASE_STRS:
    lines.append("    %s," % emit(s))
lines.append("];")
lines.append("for (local i = 0; i < BSTR.len(); i++) {")
lines.append("    for (local j = 0; j < BASES.len(); j++) {")
lines.append("        local s = BSTR[i]; local b = BASES[j];")
lines.append('        t("B" + i + "_" + b, function() { return s.tointeger(b); });')
lines.append("    }")
lines.append("}")
lines.append("")

# 3) base 参数本身的类型/个数语义
lines += [
    't("base_float16", function() { return "ff".tointeger(16.0); });',
    't("base_float16_9", function() { return "ff".tointeger(16.9); });',
    't("base_null", function() { return "ff".tointeger(null); });',
    't("base_str", function() { return "ff".tointeger("16"); });',
    't("base_bool", function() { return "ff".tointeger(true); });',
    't("base_extra", function() { return "ff".tointeger(16, 99); });',
    't("base_none_hex", function() { return "0xff".tointeger(); });',
    't("num_basearg", function() { return (42).tointeger(16); });',
    't("num_toint", function() { return (42).tointeger(); });',
    't("num_toint_f32", function() { return (42.9).tointeger(); });',
    't("num_toint_neg", function() { return (-42.9).tointeger(); });',
    't("num_toint_inf", function() { return (1.0 / 0.0).tointeger(); });',
    't("num_toint_nan", function() { return (0.0 / 0.0).tointeger(); });',
    't("num_toint_min", function() { return (-9223372036854775808).tointeger(); });',
    't("num_tofloat_basearg", function() { return (42.9).tofloat(16); });',
    't("str_tofloat_basearg", function() { return "1.5".tofloat(16); });',
    't("bool_basearg", function() { return true.tointeger(16); });',
    't("bool_toint", function() { return true.tointeger(); });',
    't("bool_toint_false", function() { return false.tointeger(); });',
    't("null_toint", function() { return null.tointeger(); });',
    't("tbl_toint", function() { return ({}).tointeger(); });',
    't("arr_toint", function() { return ([1]).tointeger(); });',
    't("clo_toint", function() { return (function() {}).tointeger(); });',
    't("blob_toint", function() { return (blob(2)).tointeger(); });',
    't("null_tofloat", function() { return null.tofloat(); });',
    't("tbl_tofloat", function() { return ({}).tofloat(); });',
    't("num_tofloat", function() { return (42).tofloat(); });',
    't("num_tofloat_neg", function() { return (-42.9).tofloat(); });',
]

lines.append('print("TONUMMAP-DONE\\n");')

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")

print("已写出 %s：%d 个字符串 × 3 种观测 + base 矩阵" % (OUT, n_case))

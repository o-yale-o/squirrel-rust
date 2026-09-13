# -*- coding: utf-8 -*-
"""生成"数值字面量"矩阵语料 tests-corpus/t_litmap.nut。

为什么单开一份：`t_lexmatrix.nut` 覆盖的是字符串转义 / Unicode / 原样串 /
优先级 —— **数值字面量的形态一个都没有**（`grep num|hex|oct|float` 只命中
`prec_typeof_eq` 之类的偶然匹配），`errmsg_matrix.py` 的 60 条只对拍**编译错误
文本**、不看值。而词法器里的数字路径与 `"…".tointeger()` 是**两条完全不同的
代码路径**（`sqlexer.cpp::ReadNumber`）：

1. **十六进制字面量**走 `LexHexadecimal`：累加进 `SQUnsignedInteger`，**回绕**，
   且超过 `MAX_HEX_DIGITS = sizeof(SQInteger)*2 = 16` 位直接报
   `too many digits for an Hex number`。所以 `0xffffffffffffffff` 是 **-1**、
   `0x10000000000000000` 是**编译错误**。
2. **十进制整数字面量**走 `LexInteger`：同样**回绕**
   （`99999999999999999999999999` → `-2537764290115403777`）。
   注意这与 `"99999999999999999999999999".tointeger()` 的**饱和**结果
   （`INT64_MAX`）**不一样** —— 同一个数字，写成字面量和写成字符串结果不同，
   这是最容易踩的一格。
3. **八进制**走 `LexOctal`（`0777`=511、`08` 报 `invalid octal number`），
   没有 0b/0o 前缀（`0b101` 是 `0` 后面跟标识符 `b101`）。
4. **浮点/科学计数法**走 `(SQFloat)scstrtod`：`1e999`→inf、`1e-999`→0、
   `1.2.3` 是 `1.2` 后面跟 `.3`（报 `end of statement expected`）、
   `1e` 报 `exponent expected`。

安全约束：
- **不能用 1 参的 `compilestring(src)`**：参考实现编译失败时该 native 返回 0 个值，
  而调用方期望 1 个 —— 实测会让整个脚本**静默停住**（这是参考实现自身的 UB）。
  必须用 2 参形式（带 name → raiseerror=1），错误才是可 catch 的。
- 字面量都塞进 `compilestring("return <lit>;", "litmap")`，逐个 try/catch，
  这样一条编译错误不会带走整个文件。

用法：
    python tools/gen_litmatrix.py
    python tools/gen_litmatrix.py --list 12 15   # 查下标 → 字面量（差分失败时用）
"""
import os
import random
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_litmap.nut")

# ---------------------------------------------------------------------------
# 1) 手工清单：每一类形态的边界
CURATED = [
    # 十进制整数
    "0", "1", "00", "000", "0000000000000000000000001", "7", "10", "42",
    "2147483647", "2147483648", "4294967295", "4294967296", "4294967297",
    "9223372036854775807", "9223372036854775808", "9223372036854775809",
    "18446744073709551615", "18446744073709551616", "18446744073709551617",
    "99999999999999999999999999", "0000000000000000000000000000000000000009",
    # 八进制
    "01", "07", "010", "0777", "0777777777777777777777", "08", "09", "078",
    "0", "00", "00000000000000000000000008",
    # 十六进制
    "0x0", "0xf", "0xF", "0xff", "0xFF", "0xabcdef", "0xABCDEF", "0x7fffffff",
    "0x80000000", "0xffffffff", "0x100000000", "0x7fffffffffffffff",
    "0x8000000000000000", "0xffffffffffffffff", "0x10000000000000000",
    "0x0000000000000001", "0x00000000000000001", "0x000000000000000001",
    "0xabcdef0123456789", "0xABCDEF0123456789", "0xdeadbeefcafebabe",
    # 浮点
    "0.0", "-0.0", "1.0", ".5", "5.", "5.0", "0.5", "00.5", ".0", "0.",
    "3.14159265358979", "2.718281828459045",
    "3.4028235e38", "3.5e38", "1e40", "1e308", "1e309", "1e999",
    "1e-308", "1e-320", "1e-999", "1e-46", "1e-45", "1.1754944e-38",
    # 科学计数法
    "1e3", "1E3", "1e+3", "1e-3", "1.5e3", "1.5E+3", "1.5e-3", "0e0", "0e999",
    "5.e3", ".5e3", "1.e3", "9e18", "9e19", "9.2233720368547758e18",
    # 负号是**一元运算符**，不是字面量的一部分
    "-1", "-0", "-0x10", "-1.5", "-1e3", "-9223372036854775808",
    "-9223372036854775809", "-18446744073709551615", "-99999999999999999999",
    # 残缺/非法形态（错误文本也要逐字节一致）
    "1e", "1e+", "1e-", "1.2e", "1.2e+", ".e1", "e1", "1.2.3", "1..2", ".5.",
    "0x", "0xg", "0x1g", "0b101", "0o17", "1_000", "1_0", "0x1p3", "1p3",
    "00.5", "0.5e", "1.0e", "0x1.8p3", "1__0", "0xL", ".e",
]

# ---------------------------------------------------------------------------
# 2) 随机清单：固定种子
rng = random.Random(20260202)
RANDOM = []


def add(s):
    RANDOM.append(s)


for _ in range(160):                       # 随机十进制整数（含溢出）
    n = rng.randint(1, 26)
    add("".join(rng.choice("0123456789") for _ in range(n)))
for _ in range(160):                       # 随机十六进制（含 >16 位的编译错误）
    n = rng.randint(1, 20)
    add("0x" + "".join(rng.choice("0123456789abcdefABCDEF") for _ in range(n)))
for _ in range(80):                        # 随机八进制
    n = rng.randint(1, 24)
    add("0" + "".join(rng.choice("01234567") for _ in range(n)))
for _ in range(60):                        # 随机八进制里混进 8/9
    n = rng.randint(1, 6)
    add("0" + "".join(rng.choice("0123456789") for _ in range(n)))
for _ in range(160):                       # 随机浮点 / 科学计数法
    ip = "".join(rng.choice("0123456789") for _ in range(rng.randint(0, 18)))
    fp = "".join(rng.choice("0123456789") for _ in range(rng.randint(0, 18)))
    if not ip and not fp:
        ip = "1"
    ex = ""
    if rng.random() < 0.7:
        ex = rng.choice("eE") + rng.choice(["", "+", "-"]) + \
            "".join(rng.choice("0123456789") for _ in range(rng.randint(0, 4)))
    add("%s.%s%s" % (ip, fp, ex))
for _ in range(60):                        # 残缺指数（编译错误）
    body = "".join(rng.choice("0123456789") for _ in range(rng.randint(1, 4)))
    add(body + rng.choice("eE") + rng.choice(["", "+", "-"]) + ".")
for _ in range(60):                        # 前导零 + 小数点
    add("0" * rng.randint(1, 6) + "." + "".join(rng.choice("0123456789") for _ in range(rng.randint(0, 6))))

ALL = CURATED + RANDOM

if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k]))
    sys.exit(0)

lines = [
    "// 自动生成：数值字面量矩阵（tools/gen_litmatrix.py）",
    "//",
    "// 字面量都塞进 compilestring(\"return <lit>;\", \"litmap\") 里逐个 try/catch：",
    "// 一条编译错误不会带走整个文件，而且错误文本也一起对拍。",
    "// **不能用 1 参的 compilestring** —— 参考实现在编译失败时该 native 返回 0 个值",
    "// 而调用方期望 1 个，实测整个脚本会静默停住（参考实现自身的 UB）。",
    "function obs(v) {",
    "    local t = typeof v;",
    "    if (t == \"float\") return format(\"%.17g\", v);",
    "    return \"\" + v;",
    "}",
    "function lit(tag, src) {",
    "    try {",
    "        local f = compilestring(\"return \" + src + \";\", \"litmap\");",
    "        local v = f();",
    "        print(tag + \"=\" + obs(v) + \"|\" + typeof v + \"\\n\");",
    "    }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "",
    "local LITS = [",
]
for s in ALL:
    lines.append('    "%s",' % s)
lines[-1] = lines[-1][:-1]  # Squirrel 的数组字面量**不支持尾随逗号**
lines += [
    "];",
    "for (local i = 0; i < LITS.len(); i++) {",
    "    lit(\"L\" + i, LITS[i]);",
    "}",
    "",
    "// 直接写在脚本里的（真的走一遍编译期常量折叠），都是能编译的形态",
    "local DIRECT = [",
    '    ["d_dec", 42],',
    '    ["d_hex", 0xdeadbeef],',
    '    ["d_oct", 0777],',
    '    ["d_float", 1.5],',
    '    ["d_sci", 1.5e3],',
    '    ["d_big_dec", 18446744073709551616],',
    '    ["d_big_hex", 0xffffffffffffffff],',
    '    ["d_neg_hex", -0x10],',
    '    ["d_inf", 1e999],',
    '    ["d_zero", 0],',
    '    ["d_neg_zero", -0.0],',
    '    ["d_dot5", 0.5],',
    '    ["d_int_max", 9223372036854775807]',
    "];",
    "for (local i = 0; i < DIRECT.len(); i++) {",
    "    print(DIRECT[i][0] + \"=\" + obs(DIRECT[i][1]) + \"|\" + typeof DIRECT[i][1] + \"\\n\");",
    "}",
    "// 字面量出现在表达式/容器键/条件里",
    'print("e_or=" + (0xffffffff | 0) + "\\n");',
    'print("e_shift=" + (0x1 << 40) + "\\n");',
    'print("e_add=" + (99999999999999999999 + 0) + "\\n");',
    'print("e_key=" + ({ a = 1, b = 2 }.len()) + "\\n");',
    'print("e_cmp=" + (1e999 > 0) + "/" + (-1e999 < 0) + "\\n");',
    'print("e_div=" + (1 / 3) + "\\n");',
    'print("e_fdiv=" + (1.0 / 3) + "\\n");',
    'print("e_neg=" + (- 5) + "/" + (- -5) + "\\n");',
    'print("LITMAP-DONE\\n");',
]

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")

print("已写出 %s：%d 条字面量 + %d 条直接/表达式形态" % (OUT, len(ALL), 13 + 8))

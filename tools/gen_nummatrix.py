# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_nummatrix.nut：数值 / 运算符边界组合矩阵。

设计原则（沿用前几轮）：
- 每个用例独立 try/catch，只输出一行 `tag=值` 或 `tag!错误`；语料必须整跑完。
- 只纳入参考实现**确定**的形态。可能 UB 的（移位计数 ≥64 / 负数、浮点转整数的溢出）
  先单独探针确认，再决定是否纳入。
- 产品式组合：值 × 运算符、值 × 比较、值 × 转换、边界值 × 自增/复合赋值。

用法：python tools/gen_nummatrix.py  → 写 tests-corpus/t_nummatrix.nut
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "tests-corpus", "t_nummatrix.nut")

L = []
A = L.append

# 整数边界（含 64 位极值；MIN 用表达式写，避免字面量解析本身的歧义）
INTS = [
    ("z", "0"), ("one", "1"), ("mone", "-1"), ("two", "2"), ("seven", "7"),
    ("neg7", "-7"), ("b31", "2147483647"), ("nb31", "-2147483648"),
    ("b32", "4294967296"), ("max", "9223372036854775807"),
    ("maxm1", "9223372036854775806"), ("min", "(-9223372036854775807 - 1)"),
    ("minp1", "(-9223372036854775807)"),
]
FLOATS = [
    ("fz", "0.0"), ("fnz", "-0.0"), ("fone", "1.0"), ("fmone", "-1.0"),
    ("fhalf", "0.5"), ("fnhalf", "-0.5"), ("f15", "1.5"), ("f25", "2.5"),
    ("fpi", "3.14159"), ("f1e15", "1.0e15"), ("f1e16", "1.0e16"),
    ("f1e100", "1.0e100"), ("fn1e100", "-1.0e100"), ("fmax", "1.0e308"),
    ("f2p63", "9223372036854775808.0"),
]
OTHERS = [
    ("tru", "true"), ("fals", "false"), ("nul", "null"),
    ("snum", "\"42\""), ("sflt", "\"1.5\""), ("sbad", "\"abc\""), ("sempty", "\"\""),
    ("tbl", "{}"), ("arr", "[1]"), ("fn", "function(){}"),
]

# 探针里先确认过：C++ 的 << >> >>> 在计数越界时行为不可靠，所以移位只取 0..63 与 63 边界。
SHIFT_OK = ["s0", "s1", "s7", "s31", "s62", "s63"]

A("// 自动生成：python tools/gen_nummatrix.py —— 请勿手改")
A("// 数值 / 运算符边界组合矩阵")
A("")
A("function s(tag, f) {")
A("    try { print(tag + \"=\" + f() + \"\\n\"); }")
A("    catch (e) { print(tag + \"!\" + e + \"\\n\"); }")
A("}")
A("// 浮点用 %.17g 之外的形式会丢信息，这里统一用 Squirrel 自己的转换（两侧一致才有意义）")
A("function d(x) {")
A("    local t = typeof x;")
A("    if (t == \"table\" && \"begin\" in x) return \"{\" + x.begin + \",\" + x.end + \"}\";")
A("    if (t == \"array\") return \"[\" + x.len() + \"]\";")
A("    if (t == \"table\") return \"{tbl}\";")
A("    if (t == \"function\") return \"{fn}\";")
A("    return \"\" + x;")
A("}")
A("")

# ---------------------------------------------------------------- 1. 整数二元运算
A("// ---- 1. 整数二元运算（含 64 位边界）")
# 排除：INT64_MIN / -1 与 INT64_MIN % -1 —— x86 的 idiv 会因商溢出触发硬件异常，
# 参考实现直接 STATUS_INTEGER_OVERFLOW(0xC0000095) 崩溃（截图见 STATUS 第四节）。
# 本实现给回绕结果（INT64_MIN / 0），确定性且不崩。
TRAP_PAIRS = {("min", "mone")}
for op in ["+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>", ">>>"]:
    for an, a in INTS:
        for bn, b in INTS:
            if op in ("<<", ">>", ">>>") and bn not in ("z", "one", "two", "seven", "b31"):
                continue
            if op in ("/", "%") and (an, bn) in TRAP_PAIRS:
                continue
            A('s("i_%s_%s_%s", function() { local a = %s; local b = %s; return d(a %s b); });'
              % (op.replace("<", "l").replace(">", "g"), an, bn, a, b, op))
A("// INT64_MIN / -1 与 % -1 不进语料：参考实现直接崩（STATUS 第四节），")
A("// 本实现的确定性行为（回绕）由 crates/squirrel-stdlib/tests/numeric_semantics.rs 守门")
A("")

# ---------------------------------------------------------------- 2. 浮点二元运算
A("// ---- 2. 浮点二元运算")
for op in ["+", "-", "*", "/", "%"]:
    for an, a in FLOATS:
        for bn, b in FLOATS:
            A('s("f_%s_%s_%s", function() { local a = %s; local b = %s; return d(a %s b); });'
              % (op.replace("%", "m"), an, bn, a, b, op))
A("")

# ---------------------------------------------------------------- 3. 混合与特殊情况
A("// ---- 3. int/float 混合、除零、特殊值")
SPECIALS = [
    ("inf", "1.0/0.0"), ("ninf", "-1.0/0.0"), ("nan", "0.0/0.0"),
    ("nan2", "(0.0/0.0)"), ("infm", "1.0e308 * 10.0"),
]
for an, a in INTS[:6] + [("max", "9223372036854775807")]:
    for bn, b in [("fhalf", "0.5"), ("f15", "1.5"), ("fone", "1.0"), ("fbig", "1.0e100")]:
        for op in ["+", "-", "*", "/"]:
            A('s("mix_%s_%s_%s", function() { return d(%s %s %s); });' % (op.replace("/", "d"), an, bn, a, op, b))
for sn, s in SPECIALS:
    A('s("sp_%s_tostr", function() { return d(%s); });' % (sn, s))
    A('s("sp_%s_cmp0", function() { return d(%s == 0.0) + "/" + d(%s < 0.0) + "/" + d(%s > 0.0); });' % (sn, s, s, s))
    A('s("sp_%s_toint", function() { return d((%s).tointeger()); });' % (sn, s))
    A('s("sp_%s_int", function() { try { return d(1 / 0); } catch (e) { return "E:" + e; } });' % sn)
    A('s("sp_%s_imod", function() { try { return d(1 %% 0); } catch (e) { return "E:" + e; } });' % sn)
    A('s("sp_%s_fdiv", function() { return d(1.0 / 0.0); });' % sn)
    A('s("sp_%s_fmod", function() { return d(1.0 %% 0.0); });' % sn)
A("")

# ---------------------------------------------------------------- 4. 一元与比较
A("// ---- 4. 一元运算与比较")
for vn, v in INTS + FLOATS + OTHERS:
    A('s("u_neg_%s", function() { return d(-(%s)); });' % (vn, v))
    A('s("u_not_%s", function() { return d(!(%s)); });' % (vn, v))
    A('s("u_bnot_%s", function() { local x = %s; try { return d(~x); } catch (e) { return "E:" + e; } });' % (vn, v))
    A('s("u_typeof_%s", function() { return typeof(%s); });' % (vn, v))
    A('s("u_tostr_%s", function() { return d((%s).tostring()); });' % (vn, v))
    A('s("u_toint_%s", function() { try { return d((%s).tointeger()); } catch (e) { return "E:" + e; } });' % (vn, v))
    A('s("u_tofloat_%s", function() { try { return d((%s).tofloat()); } catch (e) { return "E:" + e; } });' % (vn, v))
A("")
for an, a in INTS + FLOATS:
    for bn, b in INTS + FLOATS:
        A('s("cmp_%s_%s", function() { local x = %s; local y = %s; return d(x == y) + d(x < y) + d(x > y) + d(x <= y) + d(x >= y) + d(x <=> y); });'
          % (an, bn, a, b))
A("")

# ---------------------------------------------------------------- 5. 自增/复合赋值/就地运算
A("// ---- 5. 自增、复合赋值、就地运算（边界溢出）")
for vn, v in [("max", "9223372036854775807"), ("min", "(-9223372036854775807 - 1)"),
              ("z", "0"), ("one", "1"), ("fmax", "1.0e308"), ("fnz", "-0.0")]:
    A('s("inc_%s", function() { local x = %s; x++; return d(x); });' % (vn, v))
    A('s("dec_%s", function() { local x = %s; x--; return d(x); });' % (vn, v))
    for op in ["+=", "-=", "*=", "/=", "%="]:
        A('s("ca_%s_%s", function() { local x = %s; try { x %s 2; } catch (e) { return "E:" + e; } return d(x); });'
          % (vn, op.replace("=", ""), v, op))
    for op in ["+=", "-="]:
        A('s("castr_%s_%s", function() { local x = %s; try { x %s 1; } catch (e) { return "E:" + e; } return d(x); });'
          % (vn, op.replace("=", ""), v, op))
A("// 表/数组槽位上的就地运算")
A('s("slot_inc", function() { local t = { v = 9223372036854775807 }; t.v++; return d(t.v); });')
A('s("slot_dec", function() { local a = [0]; a[0]--; return d(a[0]); });')
A('s("slot_ca", function() { local t = { v = 1 }; t.v += 1.5; return d(t.v); });')
A('s("global_ca", function() { gnum <- 5; gnum += 2.5; return d(gnum); });')
A("")

# ---------------------------------------------------------------- 6. 移位细节
A("// ---- 6. 移位：计数在 0..63 内，以及**越界计数**")
A("// ↔ x86 硬件移位把计数掩到 6 位：`1 << 64` == `1 << 0`、`1 << -1` == `1 << 63`。")
A("// 参考实现（MSVC x86-64）实测就是这个语义，本实现用 wrapping_shl/shr 对齐。")
for vn, v in [("one", "1"), ("neg1", "-1"), ("max", "9223372036854775807"),
              ("min", "(-9223372036854775807 - 1)"), ("b31", "2147483647"),
              ("z", "0"), ("neg2", "-2")]:
    for sn, sc in [("s0", "0"), ("s1", "1"), ("s7", "7"), ("s31", "31"), ("s62", "62"), ("s63", "63"),
                   ("s64", "64"), ("s65", "65"), ("s127", "127"), ("s128", "128"),
                   ("sm1", "-1"), ("sm64", "-64"), ("sm65", "-65"),
                   ("shuge", "2147483647"), ("smax", "9223372036854775807")]:
        for op in ["<<", ">>", ">>>"]:
            A('s("sh_%s_%s_%s", function() { return d(%s %s %s); });'
              % (vn, sn, op.replace("<", "l").replace(">", "g"), v, op, sc))
A("// 浮点参与位运算/移位")
for vn, v in [("fone", "1.0"), ("fhalf", "0.5")]:
    for op in ["&", "|", "^", "<<", ">>", ">>>"]:
        A('s("fb_%s_%s", function() { local x = %s; try { return d(x %s 2); } catch (e) { return "E:" + e; } });'
          % (vn, op.replace("<", "l").replace(">", "g"), v, op))
A("")

# ---------------------------------------------------------------- 7. 非数值类型参与运算
A("// ---- 7. 非数值类型参与运算（错误消息也要一致）")
for vn, v in OTHERS:
    for op in ["+", "-", "*", "/", "%", "&", "<<", "<", "<=>"]:
        A('s("o_%s_%s", function() { local x = %s; local y = 1; try { return d(x %s y); } catch (e) { return "E:" + e; } });'
          % (vn, op.replace("<", "l").replace(">", "g").replace("%", "m"), v, op))
    A('s("o_%s_rsub", function() { local x = %s; try { return d(1 - x); } catch (e) { return "E:" + e; } });' % (vn, v))
    A('s("o_%s_concat", function() { local x = %s; try { return d("v" + x); } catch (e) { return "E:" + e; } });' % (vn, v))
    A('s("o_%s_in", function() { local x = %s; try { return d("a" in x); } catch (e) { return "E:" + e; } });' % (vn, v))
A("")

# ---------------------------------------------------------------- 8. 数学函数边界
A("// ---- 8. math 边界（与 C 运行库的对应）")
for fname, call in [
    ("abs", "abs(x)"), ("floor", "floor(x)"), ("ceil", "ceil(x)"), ("round", "round(x)"),
    ("sqrt", "sqrt(x)"), ("exp", "exp(x)"), ("log", "log(x)"), ("log10", "log10(x)"),
    ("sin", "sin(x)"), ("cos", "cos(x)"), ("tan", "tan(x)"), ("asin", "asin(x)"),
    ("acos", "acos(x)"), ("atan", "atan(x)"), ("fabs", "fabs(x)"),
    ("pow", "pow(x, 2.0)"), ("fmod", "fmod(x, 3.0)"), ("srand", "srand(1)"),
]:
    for vn, v in [("z", "0"), ("mz", "-0.0"), ("one", "1"), ("mone", "-1"), ("fhalf", "0.5"),
                  ("fbig", "1.0e100"), ("nan", "0.0/0.0"), ("inf", "1.0/0.0")]:
        A('s("math_%s_%s", function() { local x = %s; try { return d(%s); } catch (e) { return "E:" + e; } });'
          % (fname, vn, v, call))
A("")

# ---------------------------------------------------------------- 9. 数值字面量解析
A("// ---- 9. 数值字面量的解析（词法面，P1-3 会继续扩）")
LITS = [
    ("dec0", "0"), ("dec00", "00"), ("hex", "0x1F"), ("hexcap", "0xFF"), ("hex0", "0x0"),
    ("neghex", "-0x10"), ("fdot", "1."), ("fexp", "1e3"), ("fexpc", "1E3"),
    ("fexpneg", "1e-3"), ("fexpplus", "1e+3"), ("f_exp_f", "1.5e3"), ("flong", "1.0000000000000002"),
    ("i_max", "9223372036854775807"), ("i_over", "9223372036854775808"),
    ("i_min_lit", "-9223372036854775808"), ("f_under", "0.000000000000000000001"),
    # 注：`.5` 与 `1_000` 不是合法字面量（两侧都应报语法错），归到 P1-3 词法探针里单独测
]
for n, lit in LITS:
    A('s("lit_%s", function() { local v = %s; return typeof v + ":" + d(v); });' % (n, lit))
A("")

A('print("NUMMATRIX-DONE\\n");')
A("")

open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(L))
print("wrote %s (%d 行, %d 用例)" % (OUT, len(L), len([x for x in L if x.startswith("s(")])))

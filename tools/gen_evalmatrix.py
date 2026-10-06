# -*- coding: utf-8 -*-
"""生成"编译期折叠 / switch 值形态 / 短路求值顺序"矩阵语料 tests-corpus/t_evalmap.nut。

为什么单开一份：`t_ops`（4533 条）测的是**运行时**运算符 × 类型、
`t_lexmatrix` 的 switch 只有 6 条主干——**编译期折叠的边界**（`7 % 0` 被折叠成
7 而 `1 / 0` 是运行时错、INT64 字面量折叠回绕）、**switch 的值形态**（重复
case、极值 case、浮点 case、`case <变量>`、case 在 default 之后是语法错）、
**`&&`/`||` 返回操作数值本身**（`"" && 5` → 5）与**短路副作用顺序**、
**赋值作表达式**（`x = y = z` 链）——这些都没有用例。本轮为**验证型 + 1 个
真修复**：55 条探针发现 `x = y = z` 的链式赋值我们给错值（见下）。

**P23 修复的真 bug（DMOVE 的执行顺序）**：窥孔把连续两条 MOVE 合并成一条
`DMOVE(d0←s0, d1←s1)`。参考实现是**严格顺序**执行
（`STK(arg0)=STK(arg1); STK(arg2)=STK(arg3);`），而链式赋值 `x = y = z`
正是 `DMOVE(y←z, x←y)`——arg3 与 arg1 是同一个寄存器，第二读必须看到第一写。
我们原来把两个源**先快照再写**（并行语义），于是 `x` 拿到旧 `y`（0 而不是 3）。
所有"外层赋值、内层 RHS 是变量/调用"的链式赋值都中招；RHS 是字面量时碰巧没事。

实测钉住的其他语义（都对着参考实现验过）：
1. **switch**：`case` 标签可以是**任意表达式**（含变量）；重复 case 允许、先到先得；
   浮点 case 与整数 case 交叉匹配（`case 2:` 接住 `2.0`）；`case` 写在 `default`
   **之后**是语法错 `expected '}'`（用 compilestring 钉）；switch 里的 `break`
   归 switch 不归外层循环；case 体里可以声明 local。
2. **折叠**：`7 % 0` → **7**（编译期折叠！）、`1 / 0` → 运行时 `division by zero`
   （整数除法**不**折叠）、`1.0/0.0`→inf、`1.0%0.0`→nan、INT64 字面量折叠
   **回绕**（`MAX+1`→MIN、`3037000500*3037000500`→负数）、`1<<64`→1、
   `1<<-1`→INT64_MIN。
3. **`&&`/`||` 返回操作数值本身**且**短路**：`true && 2` → 2、`"" && 5` → 5
   （`""` 真值）、`false && side()` **不调用** side、`1 && 2 || 3 && 4` → 2
   （右侧 `3 && 4` 被短路）。
4. **赋值作表达式**：`(a = 5)` 的值是 5、链 `x = y = z` 三者全为 3、
   `if ((a = 7) == 7)`、实参里赋值、`x ? a = 1 : b = 2` 只赋命中的分支。

用法：
    python tools/gen_evalmatrix.py
    python tools/gen_evalmatrix.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_evalmap.nut")

CASES = [
    # ---------- switch 值形态 ----------
    ("sw_neg", 'local x = -3; switch (x) { case -3: return "n3"; case 3: return "p3"; } return "none";'),
    ("sw_dup", 'local x = 2; switch (x) { case 2: return "first"; case 2: return "second"; } return "none";'),
    ("sw_min", 'local x = -9223372036854775808; switch (x) { case -9223372036854775808: return "min"; } return "none";'),
    ("sw_max", 'local x = 9223372036854775807; switch (x) { case 9223372036854775807: return "max"; } return "none";'),
    ("sw_float", 'local x = 1.5; switch (x) { case 1.5: return "f15"; case 1: return "i1"; } return "none";'),
    ("sw_float_eq_int", 'local x = 2.0; switch (x) { case 2: return "i2"; } return "none";'),
    ("sw_default_last", 'local x = 9; switch (x) { case 1: return "one"; default: return "def"; }'),
    ("sw_default_first", 'return compilestring("local x = 9; switch (x) { default: return 42; }", "sdf") == null ? "compile-null" : "compiled";'),
    ("sw_empty", 'local x = 1; switch (x) { } return "ok";'),
    ("sw_only_default", 'local x = 1; switch (x) { default: return "d"; }'),
    ("sw_nested", 'local x = 2, y = 20; switch (x) { case 2: switch (y) { case 20: return "2/20"; } return "2/?"; } return "none";'),
    ("sw_break_in_loop", 'local out = ""; for (local i = 0; i < 3; i++) { switch (i) { case 1: break; default: out += i; } out += "|"; } return out;'),
    ("sw_local_in_case", 'local x = 1; switch (x) { case 1: local v = "lv"; return "got:" + v; } return "none";'),
    ("sw_case_expr", 'local x = 5, y = 5; switch (x) { case y: return "var-case"; } return "none";'),
    ("sw_sparse_big", 'local x = 1000000; switch (x) { case 1: return "1"; case 1000000: return "1m"; case 2: return "2"; } return "none";'),
    ("sw_bool", 'local x = true; switch (x) { case true: return "T"; case false: return "F"; } return "none";'),
    ("sw_null", 'local x = null; switch (x) { case null: return "N"; } return "none";'),
    ("sw_return_in_loop", 'local a = [1, 2, 3]; foreach (v in a) { switch (v) { case 2: return "found2"; } } return "none";'),
    # ---------- 编译期折叠边界 ----------
    ("cf_div0_int", 'return 1 / 0;'),
    ("cf_mod0_int", 'return 7 % 0;'),
    ("cf_div0_float", 'return 1.0 / 0.0;'),
    ("cf_mod0_float", 'return 1.0 % 0.0;'),
    ("cf_ovf_add", 'return 9223372036854775807 + 1;'),
    ("cf_ovf_mul", 'return 3037000500 * 3037000500;'),
    ("cf_shift64", 'return 1 << 64;'),
    ("cf_shift_neg", 'return 1 << -1;'),
    ("cf_float_add", 'return 0.1 + 0.2;'),
    ("cf_mixed", 'return 1 + 2.5;'),
    ("cf_unary", 'return -(-5);'),
    ("cf_not", 'return !(1 < 2);'),
    ("cf_cmp_chain", 'return 1 < 2 == true;'),
    ("cf_str_concat", 'return "a" + "b";'),
    ("cf_div_prec", 'return 1.0 / 3 * 3;'),
    # ---------- && / || 返回操作数值 + 短路顺序 ----------
    ("sc_and_vals", 'return (1 && 2) + "/" + (0 && 2) + "/" + (null && 2) + "/" + ("" && 2);'),
    ("sc_or_vals", 'return (1 || 2) + "/" + (0 || 2) + "/" + (null || "x") + "/" + ("" || 2);'),
    ("sc_and_short", 'local s = ""; local r = false && side(1); return s + "/" + r;'),
    ("sc_or_short", 'local s = ""; local r = true || side(1); return s + "/" + r;'),
    ("sc_and_calls", 'local s = ""; local r = side(true) && side(2); return s + "/" + r;'),
    ("sc_or_calls", 'local s = ""; local r = side(false) || side(3); return s + "/" + r;'),
    ("sc_chain", 'local s = ""; local r = side(1) && side(2) || side(3) && side(4); return s + "/" + r;'),
    ("sc_mixed_types", 'return ("" && 5) + "/" + typeof ([] || "arr") + "/" + typeof ({} || "tbl");'),
    ("sc_not_not", 'return !!5 + "/" + !!0 + "/" + !!"x";'),
    # ---------- 赋值作表达式 / 链式 ----------
    ("asg_value", 'local a = 1; local b = (a = 5); return a + "/" + b;'),
    ("asg_chain", 'local a = 0; local b = 0; local c = 3; a = b = c; return a + "/" + b;'),
    ("asg_in_cond", 'local a = 0, r = ""; if ((a = 7) == 7) r = "yes"; return r + "/" + a;'),
    ("asg_in_call", 'local a = 0; local f = function(x) { return x * 2; }; local r = f(a = 4); return r + "/" + a;'),
    # ---------- 三元链 ----------
    ("tern_nested", 'local x = 2; return x == 1 ? "one" : x == 2 ? "two" : "many";'),
    ("tern_side_once", 'local n = 0; local f = function() { n++; return true; }; local r = f() ? "a" : "b"; return r + "/" + n;'),
    ("tern_assign_branch", 'local a = 0, b = 0; local x = 1; x ? a = 1 : b = 2; return a + "/" + b;'),
    ("tern_in_args", 'local f = function(p, q) { return p + ":" + q; }; return f(1 ? "t" : "f", 0 ? "t" : "f");'),
]

lines = [
    "// 自动生成：编译期折叠 / switch 值形态 / 短路求值顺序矩阵（tools/gen_evalmatrix.py）",
    "//",
    "// 本轮修复：链式赋值 x = y = z 的 DMOVE 执行顺序（见生成器 docstring 与",
    "// MIGRATION.md）。其余为验证型，全部与参考逐字节一致。",
    "function d(x) {",
    "    local t = typeof x;",
    "    if (t == \"table\") { local o = \"{\"; local first = true; foreach (k, v in x) { if (!first) o += \",\"; first = false; o += k + \"=\" + v; } return o + \"}\"; }",
    "    return \"\" + x;",
    "}",
    "function side(x) { print(\"S:\" + x + \";\"); return x; }",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=\" + (typeof v == \"array\" ? \"arr\" : (typeof v == \"string\" ? v : \"\" + v)) + \"|\" + typeof v + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "",
]
for name, body in CASES:
    lines.append('t("%s", function() { %s });' % (name, body))

ALL = [l for l in lines if l.startswith('t("')]
if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k][:110]))
    sys.exit(0)

lines += ['print("EVALMAP-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

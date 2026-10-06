# -*- coding: utf-8 -*-
"""生成"表键身份 + 循环变量闭包捕获"矩阵语料 tests-corpus/t_keymap.nut。

为什么单开一份：`t_delmatrix` 测"方法 × 实参类型"、`t_ops` 测运算符——
**表的键按什么身份区分**（int `1` / float `1.0` / 字符串 `"1"` / bool 是不是
同一个槽？`-0.0`？null 能当键吗？）与**循环变量的闭包捕获**（`for`/`foreach`
的循环变量被闭包捕获时共享还是逐次独立？）这两族没有用例。
本轮为**验证型**：31 条探针两侧逐字节一致。

实测钉住的语义（都对着参考实现验过）：
1. **表键严格按类型区分**：`t[1]`、`t[1.0]`、`t["1"]`、`t[true]` 是**四个不同的
   槽**——即使值相等也不互通（`t[1.0]` 读不到 `t[1]` 写的值，报
   `the index '1' does not exist`）；`-0.0` 与 `0` 也是两个键；**null 不能当键**
   （报 `null cannot be used as index`）。混合键型的遍历顺序是哈希序
   （`bool;string;float;integer;`）。
2. **数组下标做 `tointeger` 转换**：`a[1.0]`→`a[1]`、`a[1.9]`→`a[1]`（截断）、
   `a[-0.0]`→`a[0]`；但 `a["1"]` 报错；`a[-1] = v` 报 `index -1 does not exist`。
3. **普通 `=` 不建新槽**：`x[1] = v` 对空表报 `the index '1' does not exist`
   （建键要用 `<-`；已存在键的覆盖才用 `=`）。
4. **循环变量的闭包捕获**：`for` 与 `foreach` 的循环变量被闭包捕获时**共享同一个
   槽**——循环结束后所有闭包看到的都是最终值（`3,3,3` / `30,30,30`）；
   **循环体内 `local j` 才是逐次独立**（`0,10,20`）；`while` + 体 内 `local` 同理；
   捕获循环变量的生成器在循环结束后 resume，看到的是最终值；两个闭包共享
   `i` 时一侧的修改对另一侧可见（`2,102,102,202`）。

安全约束：无。

用法：
    python tools/gen_keymatrix.py
    python tools/gen_keymatrix.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_keymap.nut")

CASES = [
    # --- 表键身份（<- 建键）---
    ("k_int_float", 'local x = {}; x[1] <- "i"; return x[1.0];'),
    ("k_float_int", 'local x = {}; x[1.0] <- "f"; return x[1];'),
    ("k_str_vs_int", 'local x = {}; x["1"] <- "s"; x[1] <- "i"; return d(x) + "/" + ("1" in x) + "/" + (1 in x);'),
    ("k_neg_zero", 'local x = {}; x[-0.0] <- "nz"; return x[0.0] + "/" + d(x);'),
    ("k_zero_float", 'local x = {}; x[0.0] <- "zf"; return x.len();'),
    ("k_bool", 'local x = {}; x[true] <- "T"; return x[true] + "/" + x.len();'),
    ("k_null", 'local x = {}; x[null] <- "N"; return x[null];'),
    ("k_bool_vs_int", 'local x = {}; x[true] <- "T"; return x[1] == null;'),
    ("k_float_frac", 'local x = {}; x[1.5] <- "f"; return (1 in x) + "/" + x[1.5] + "/" + x.len();'),
    ("k_big_float", 'local x = {}; x[1e15] <- "big"; return x[1e15] + "/" + x.len();'),
    ("k_delete_float", 'local x = {}; x[1.0] <- "f"; return "" + (delete x[1.0]) + "/" + x.len();'),
    ("k_overwrite_eq", 'local x = {}; x[1] <- "first"; x[1] = "second"; return d(x);'),
    # --- 普通 = 不建新槽（核心语义）---
    ("plain_eq_int", 'local x = {}; x[1] = "i"; return d(x);'),
    ("plain_eq_str", 'local x = {}; x.s = "v"; return d(x);'),
    # --- 数组下标做 tointeger ---
    ("a_float_idx", 'local a = [10, 20, 30]; return a[1.0];'),
    ("a_float_frac_idx", 'local a = [10, 20, 30]; return a[1.9];'),
    ("a_str_idx", 'local a = [10, 20, 30]; return a["1"];'),
    ("a_neg_zero_idx", 'local a = [10, 20, 30]; return a[-0.0];'),
    ("a_set_float", 'local a = [10, 20, 30]; a[1.0] = 99; return d(a);'),
    ("a_set_frac", 'local a = [10, 20, 30]; a[1.9] = 99; return d(a);'),
    ("a_neg_idx_set", 'local a = [10, 20, 30]; a[-1] = 99; return d(a);'),
    # --- 混合键型遍历（哈希序）---
    ("iter_mixed", 'local x = {}; x[1] <- "i"; x["s"] <- "s"; x[2.5] <- "f"; x[true] <- "b"; local o = ""; foreach (k, v in x) o += typeof k + ";"; return o;'),
    ("iter_keys_types", 'local x = {}; x[1] <- "i"; x["s"] <- "s"; local ks = x.keys(); local o = ""; foreach (k in ks) o += typeof k + ";"; return o;'),
    # --- 循环变量的闭包捕获 ---
    ("cap_for", 'local fs = []; for (local i = 0; i < 3; i++) fs.append(function() { return i; }); local s = ""; foreach (f in fs) s += f() + ","; return s;'),
    ("cap_foreach", 'local fs = []; foreach (v in [10, 20, 30]) fs.append(function() { return v; }); local s = ""; foreach (f in fs) s += f() + ","; return s;'),
    ("cap_while", 'local fs = []; local i = 0; while (i < 3) { local j = i; fs.append(function() { return j; }); i++; } local s = ""; foreach (f in fs) s += f() + ","; return s;'),
    ("cap_for_local_in_body", 'local fs = []; for (local i = 0; i < 3; i++) { local j = i * 10; fs.append(function() { return j; }); } local s = ""; foreach (f in fs) s += f() + ","; return s;'),
    ("cap_generator", 'local fs = []; for (local i = 0; i < 3; i++) fs.append((function() { yield i; })()); local s = ""; foreach (g in fs) { resume g; s += resume g; s += ","; } return s;'),
    ("cap_two_shared", 'local fs = []; for (local i = 0; i < 2; i++) { fs.append(function() { return i; }); fs.append(function() { i = i + 100; return i; }); } local s = ""; foreach (f in fs) s += f() + ","; return s;'),
    ("cap_delegate_fn", 'local fs = []; local tbl = { n = 0 }; for (local i = 0; i < 2; i++) { tbl.n = i; fs.append(function() { return tbl.n; }); } local s = ""; foreach (f in fs) s += f() + ","; return s;'),
]

lines = [
    "// 自动生成：表键身份 + 循环变量闭包捕获矩阵（tools/gen_keymatrix.py）",
    "//",
    "// **表键严格按类型区分**：int/float/string/bool 是不同的槽（1.0 读不到 1 写的值），",
    "// null 不能当键；数组下标做 tointeger 转换（a[1.9] → a[1]）；普通 `=` 不建新槽。",
    "// **循环变量的闭包捕获**：for/foreach 的循环变量共享同一槽（闭包看到最终值），",
    "// 循环体内 `local` 才逐次独立。两侧逐字节一致。",
    "function d(x) {",
    "    local t = typeof x;",
    "    if (t == \"table\") { local o = \"{\"; local f = true; foreach (k, v in x) { if (!f) o += \",\"; f = false; o += \"\" + k + \"=\" + v; } return o + \"}\"; }",
    "    if (t == \"array\") { local o = \"[\"; for (local i = 0; i < x.len(); i++) { if (i) o += \",\"; o += \"\" + x[i]; } return o + \"]\"; }",
    "    return \"\" + x;",
    "}",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=\" + d(v) + \"|\" + typeof v + \"\\n\"); }",
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

lines += ['print("KEYMAP-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

# -*- coding: utf-8 -*-
"""生成"容器回调乱来"矩阵语料 tests-corpus/t_cbmap.nut。

为什么单开一份：`t_delmatrix.nut` 覆盖的是**方法 × 接收者 × 实参类型**，
`t_meta3.nut` 覆盖的是**元方法**抛异常 —— 但"**回调自己乱来**"这一族
（比较器返回非整数 / 回调形参个数不对 / 回调里抛异常 / 回调里改容器）在语料里
是 0 覆盖（`grep` 确认：抛异常的回调用例 0 条）。这类形态**按现有生成器的结构
碰不到**，因为它需要"回调的**行为**"当输入维度，而不是"实参的类型"。

本轮抓到两处真实差异（都在 `sqbaselib.cpp`）：
1. **比较器返回值走 `sq_getinteger`**：`sq_isnumeric` 掩码里**包含 float**
   （值取 `tointeger(o)` = `(SQInteger)` 截断），bool 另有分支给 0/1。
   我们原来只收 `OT_INTEGER`，于是 `a.sort(function(x, y) { return x - y; })`
   在浮点数组上直接报 `numeric value expected…` —— 而这是**最常见的写法**。
2. **`map`/`apply` 的回调只支持 1~3 个形参**：`__map_array` 每轮只压
   `2 + (nArgs>=3) + (nArgs>=4)` 个值，调用却是 `sq_call(v, nArgs, …)` →
   `GetUp(-(nArgs+1))`。只有 `nArgs == pushed` 时调用的才是闭包本身：
   0 参回调（nArgs=1）会去调刚压进去的 `src`，4 参及以上（nArgs>=5）会去调闭包
   **下面**那一槽（`array_map`/`array_apply` 帧里同样是 `this` 那个数组）——
   两种情况参考实现都报 `attempt to call 'array'`。空数组不进循环 → 不报错。

安全约束（参考实现自身的 UB，一律不进语料）：
- 比较器返回 **NaN / ±inf**：`sq_getinteger` 内部是 `(SQInteger)((SQFloat)o)`，
  NaN/inf 转整数是 UB（本机上是 INT64_MIN，实测两侧**碰巧**一致，但仍是 UB）。
- `sort` 的比较器在排序过程中**增删数组元素**：`_sort_compare` 有
  `array resized during sort operation` 检查，但检查发生在 `sq_call` 之后、
  指针已可能失效；变异测试另外处理，不放进语料。

用法：
    python tools/gen_cbmatrix.py
    python tools/gen_cbmatrix.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_cbmap.nut")

# ---------------------------------------------------------------------------
# 1) sort 比较器：返回值类型矩阵（值都要能安全转 int）
SORT_RET = [
    ("int0", "return 0;"),
    ("int1", "return 1;"),
    ("intn1", "return -1;"),
    ("int2", "return 2;"),
    ("intn2", "return -2;"),
    ("float0", "return 0.0;"),
    ("f_half", "return 0.5;"),
    ("f_1_5", "return 1.5;"),
    ("f_n_half", "return -0.5;"),
    ("f_2_9", "return 2.9;"),
    ("f_n_1_9", "return -1.9;"),
    ("f_from_sub", "return a - b;"),
    ("f_big", "return 1e15;"),
    ("bool_true", "return true;"),
    ("bool_false", "return false;"),
    ("null", "return null;"),
    ("str", 'return "x";'),
    ("arr", "return [];"),
    ("tbl", "return {};"),
    ("func", "return function() {};"),
    ("throw", 'throw "sort-boom";'),
]

# 2) 回调形参个数：map/apply（只支持 1~3）与 filter/reduce/find/table.*
ARITIES = [
    ("p0", ""),
    ("p1", "a"),
    ("p2", "a, b"),
    ("p3", "a, b, c"),
    ("p4", "a, b, c, d"),
    ("p5", "a, b, c, d, e"),
]

# 3) 回调抛异常/乱返回
THROW_CBS = [
    ("map", "a.map(function(v) { throw \"map-boom\"; })"),
    ("filter", "a.filter(function(v) { throw \"f\"; })"),
    ("reduce", "a.reduce(function(acc, v) { throw \"r\"; }, 0)"),
    ("apply", "a.apply(function(v) { throw \"ap\"; })"),
    ("find", "a.find(function(v) { throw \"fi\"; })"),
    ("sort", "a.sort(function(x, y) { throw \"s\"; })"),
    ("tmap", "tb.map(function(k, v) { throw \"tm\"; })"),
    ("tfilter", "tb.filter(function(k, v) { throw \"tf\"; })"),
    ("tkeys", "tb.keys(function(k) { throw \"tk\"; })"),
]

lines = [
    "// 自动生成：容器回调矩阵（tools/gen_cbmatrix.py）",
    "//",
    "// 观测口径：**一律用 dump() 打印内容**，绝不直接 print 容器 ——",
    "// `(array : 0x…)` 里的指针值两侧本来就不同（diff_test 会归一化，但",
    "// 一次性脚本里踩过一次假绿，索性从源头避免）。",
    "function dump(a) {",
    "    local s = \"[\";",
    "    for (local i = 0; i < a.len(); i++) {",
    "        if (i > 0) s += \",\";",
    "        local v = a[i];",
    "        s += (typeof v == \"string\" ? v : \"\" + v);",
    "    }",
    "    return s + \"]\";",
    "}",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=\" + (typeof v == \"string\" ? v : \"\" + v) + \"|\" + typeof v + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "",
    "local A = [10, 20];",
    "local tb = { a = 1, b = 2 };",
    "",
    "// --- 1) sort 比较器返回值：类型与截断 ---",
]
n_sort = 0
for name, body in SORT_RET:
    lines.append('t("sr_%s", function() {' % name)
    lines.append("    local arr = [4, 2, 5, 1, 3];")
    lines.append("    try { arr.sort(function(a, b) { %s }); }" % body)
    lines.append('    catch (e) { return "E:" + e + " after " + dump(arr); }')
    lines.append("    return dump(arr);")
    lines.append("});")
    n_sort += 1
# 浮点数组（真实写法：x - y）
lines += [
    't("sr_float_array_sub", function() { local a = [2.5, 1.5, 3.5]; a.sort(function(x, y) { return x - y; }); return dump(a); });',
    't("sr_float_array_rev", function() { local a = [2.5, 1.5, 3.5]; a.sort(function(x, y) { return y - x; }); return dump(a); });',
    't("sr_float_bool_cmp", function() { local a = [2.5, 1.5, 3.5]; a.sort(function(x, y) { return x < y; }); return dump(a); });',
    't("sr_string_array", function() { local a = ["b", "a", "c"]; a.sort(function(x, y) { return x <=> y; }); return dump(a); });',
    't("sr_mixed_sizes", function() { local a = [5, 1, 4, 2, 8, 3, 9, 7, 6, 10, 0]; a.sort(function(x, y) { return x - y; }); return dump(a); });',
    't("sr_empty", function() { local a = []; a.sort(function(x, y) { throw "never"; }); return dump(a); });',
    't("sr_one", function() { local a = [7]; a.sort(function(x, y) { throw "never"; }); return dump(a); });',
    't("sr_two", function() { local a = [2, 1]; a.sort(function(x, y) { return x - y; }); return dump(a); });',
    't("sr_stability", function() { local a = [[1, "a"], [0, "b"], [1, "c"], [0, "d"]]; a.sort(function(x, y) { return x[0] - y[0]; }); local s = ""; foreach (e in a) s += e[1]; return s; });',
    't("sr_arity0", function() { local a = [3, 1, 2]; a.sort(function() { return 0; }); return dump(a); });',
    't("sr_arity1", function() { local a = [3, 1, 2]; a.sort(function(x) { return 0; }); return dump(a); });',
    't("sr_arity3", function() { local a = [3, 1, 2]; a.sort(function(x, y, z) { return 0; }); return dump(a); });',
    't("sr_native", function() { local a = [3, 1, 2]; a.sort(abs); return dump(a); });',
    't("sr_null", function() { local a = [3, 1, 2]; a.sort(null); return dump(a); });',
    't("sr_args", function() { local seen = ""; local a = [2, 1]; a.sort(function(x, y) { seen += typeof x + "/" + typeof y + ";"; return x <=> y; }); return seen; });',
    't("sr_this", function() { local a = [2, 1]; local seen = "?"; a.sort(function(x, y) { seen = typeof this; return x <=> y; }); return seen; });',
]
n_sort += 17

# 2) 回调形参个数
lines.append("")
lines.append("// --- 2) 回调形参个数（map/apply 参考实现只支持 1~3 个）---")
for meth, tmpl, label in [
    ("map", 'local r = A.map(function(%s) { return "z"; }); return typeof r + ":" + r.len();', "m"),
    ("apply", 'local c = [1, 2]; c.apply(function(%s) { return 9; }); return dump(c);', "ap"),
    ("filter", 'local r = A.filter(function(%s) { return true; }); return "n" + r.len();', "ft"),
    ("reduce", 'return A.reduce(function(%s) { return 1; }, 0);', "rd"),
    ("find", 'return A.find(function(%s) { return true; });', "fd"),
]:
    for pname, params in ARITIES:
        lines.append('t("%s_%s", function() { %s });' % (label, pname, tmpl % params))
lines.append('t("m_empty_p0", function() { local e = []; return typeof e.map(function() { return 1; }); });')
lines.append('t("ap_empty_p0", function() { local e = []; e.apply(function() { return 1; }); return "ok"; });')
lines.append('t("m_native", function() { local a = [1]; return typeof a.map(print); });')
lines.append('t("tb_map_p0", function() { return typeof tb.map(function() { return 1; }); });')
lines.append('t("tb_map_p1", function() { return typeof tb.map(function(k) { return 1; }); });')
lines.append('t("tb_map_p2", function() { return typeof tb.map(function(k, v) { return 1; }); });')
lines.append('t("tb_map_p4", function() { return typeof tb.map(function(k, v, x, y) { return 1; }); });')
lines.append('t("tb_filter_p0", function() { return tb.filter(function() { return true; }).len(); });')
lines.append('t("tb_filter_p2", function() { return tb.filter(function(k, v) { return true; }).len(); });')
lines.append('t("tb_keys_p0", function() { return tb.keys(function() { return true; }).len(); });')
lines.append('t("tb_keys_p2", function() { return tb.keys(function(k, v) { return true; }).len(); });')
lines.append('t("tb_len_cb", function() { return typeof tb.len(); });')

# 3) 回调实参语义（this / 值 / 下标 / 数组）
lines += [
    "",
    "// --- 3) 回调实参语义：this = 数组本身，第 1 个实参是值、第 2 个是下标、第 3 个又是数组 ---",
    't("as_m1", function() { local r = A.map(function(a) { return "" + typeof a + ":" + a; }); return dump(r); });',
    't("as_m2", function() { local r = A.map(function(a, b) { return "" + typeof a + ":" + a + "|" + typeof b + ":" + b; }); return dump(r); });',
    't("as_m3", function() { local r = A.map(function(a, b, c) { return "" + typeof a + ":" + a + "|" + typeof b + ":" + b + "|" + typeof c + ":" + c.len(); }); return dump(r); });',
    't("as_this_is_arr", function() { local r = A.map(function(a) { return (this == A) + "/" + this.len(); }); return dump(r); });',
    't("as_filter2", function() { local r = A.filter(function(a, b) { return b == 0; }); return dump(r); });',
    't("as_filter3", function() { local r = A.filter(function(a, b, c) { return typeof c == "array"; }); return dump(r); });',
    't("as_find2", function() { return A.find(function(a, b) { return b == 1; }); });',
    't("as_reduce2", function() { return A.reduce(function(acc, v) { return "" + acc + "/" + v; }, "S"); });',
    't("as_reduce3", function() { return A.reduce(function(acc, v, i) { return "" + acc + "/" + v + "/" + typeof i + i; }, "S"); });',
    't("as_reduce_noinit", function() { return A.reduce(function(acc, v) { return "" + acc + "/" + v; }); });',
    't("as_reduce_one", function() { local a = [5]; return a.reduce(function(acc, v) { return "" + acc + "/" + v; }); });',
    't("as_reduce_empty", function() { local a = []; return "R:" + a.reduce(function(acc, v) { return 1; }); });',
    't("as_reduce_null_init", function() { return "R:" + A.reduce(function(acc, v) { return acc; }, null); });',
    't("as_apply1", function() { local c = [1, 2]; c.apply(function(v) { return v * 10; }); return dump(c); });',
    't("as_apply_idx", function() { local c = [1, 2]; c.apply(function(v, i) { return "" + i; }); return dump(c); });',
    't("as_tb_map", function() { local r = tb.map(function(k, v) { return "" + k + "=" + v; }); return typeof r + ":" + r.len(); });',
    't("as_tb_keys", function() { return tb.keys(function(k) { return true; }).len(); });',
]

# 4) 回调抛异常 / 非函数回调
lines.append("")
lines.append("// --- 4) 回调抛异常 / 回调不是函数 ---")
for name, expr in THROW_CBS:
    lines.append('t("th_%s", function() { local a = [1, 2, 3]; try { %s } catch (e) { return "E:" + e; } return "no-error"; });' % (name, expr.replace("a.", "a.").replace("tb.", "tb.")))
for name, expr in [
    ("int", "A.map(5)"),
    ("null", "A.map(null)"),
    ("str", "A.filter(\"x\")"),
    ("arr", "A.reduce([], 0)"),
    ("int_sort", "[2, 1].sort(5)"),
    ("arr_sort", "[2, 1].sort([])"),
]:
    lines.append('t("nf_%s", function() { try { return "" + (%s); } catch (e) { return "E:" + e; } });' % (name, expr))

lines.append('print("CBMAP-DONE\\n");')

ALL = [l for l in lines if l.startswith('t("')]
if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k]))
    sys.exit(0)

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")

print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

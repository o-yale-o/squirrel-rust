# -*- coding: utf-8 -*-
"""生成"clone 深度/共享 + getstackinfos locals 内容"矩阵语料 tests-corpus/t_clonestack.nut。

为什么单开一份：`t_sershape`/`t_attrmap` 测的是序列化与属性的往返；**clone 的
深度与共享**（嵌套表共享？委托保留？哪些类型根本不可 clone？）与
**`getstackinfos` 的 locals 表内容**（func 对匿名函数给 "unknown"、src 是相对
路径、locals 里有没有"后声明的局部"、生成器帧、尾调用隐藏帧）都没有用例。
本轮为**验证型**：23 条探针两侧逐字节一致。

实测钉住的语义（都对着参考实现验过）：
1. **clone 是浅拷贝**：嵌套表**共享**（`b.inner.n = 42` 两边都变）；数组顶层
   独立（append/赋值不影响原数组）但**元素共享**；**委托保留**（clone 后能读到
   链上的键）；给 clone 改"链上键"会写到链上游（base 不变）。
2. **不可 clone 的类型**：`clone 5` / `clone null` → `cloning a integer` /
   `cloning a null`；generator / thread → `cloning a generator` /
   `cloning a thread`。
3. **`getstackinfos`**：level 0 是它自己（func="getstackinfos"、locals.len=0）；
   **匿名函数的 func 是 `"unknown"`**；`src` 是**相对路径**；`locals` 含参数与
   **当时已声明**的局部（后声明的也在，值为 null）；**生成器帧查不到外层局部**；
   线程内 `getstackinfos` 返回 null；`compilestring(name)` 的 src 是那个名字；
   返回的 locals 表是**副本**（改它不影响真实局部）；**尾调用会隐藏帧**。

安全约束（**不进语料**，两侧同样静默退出但会截断脚本）：
- `suspend` 在**生成器体内**：与主线程 `suspend()` 一样**静默退出**（无错误文本、
  try/catch 接不住），实测见 tmp-c2/gsusp.nut——挂在生成器用例之后的用例全部
  丢失。记入 MIGRATION.md 的 suspend 条目。

用法：
    python tools/gen_clonestack.py
    python tools/gen_clonestack.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_clonestack.nut")

CASES = [
    # ---------- clone 的深度与共享 ----------
    ("cl_nested_shared", 'local a = { inner = { n = 1 } }; local b = clone a; b.inner.n = 42; return "a=" + a.inner.n + " b=" + b.inner.n;'),
    ("cl_arr_of_tbl", 'local a = [{ k = 1 }, { k = 2 }]; local b = clone a; b[1].k = 99; b.append({ k = 3 }); return "a.len=" + a.len() + " a1k=" + a[1].k + " b.len=" + b.len();'),
    ("cl_array_indep", 'local a = [1, 2]; local b = clone a; b.append(3); b[0] = 99; return "a=" + d(a) + " b=" + d(b);'),
    ("cl_delegate_kept", 'local bs = { g = "from-base" }; local a = { own = 1 }; a.setdelegate(bs); local b = clone a; return "b.g=" + b.g + " b.own=" + b.own + " bdeleg=" + (b.getdelegate() == null ? "null" : "tbl");'),
    ("cl_delegate_mutate", 'local bs = { g = 1 }; local a = { own = 1 }; a.setdelegate(bs); local b = clone a; b.g = 99; return "base.g=" + bs.g + " a.g=" + a.g + " b.g=" + b.g + " b.own=" + b.own;'),
    # ---------- 不可 clone 的类型 ----------
    ("cl_scalar_int", 'return clone 5;'),
    ("cl_scalar_str", 'return clone "s";'),
    ("cl_gen", 'local g = (function() { yield 1; })(); return typeof clone g;'),
    ("cl_thread", 'local th = newthread(function() { }); return typeof clone th;'),
    ("cl_null", 'return typeof clone null;'),
    # ---------- getstackinfos：locals 内容与层级 ----------
    ("gs_level0", 'local si = getstackinfos(0); return si.func + "/" + si.locals.len();'),
    ("gs_level1_locals", 'local mark = "M"; local n2 = 2; local si = getstackinfos(1); return si.func + "/" + si.src + "/" + si.locals.len() + "/" + si.locals.mark;'),
    ("gs_level1_later_local", 'local early = 1; local si = getstackinfos(1); local late = 2; return "early=" + si.locals.early + " late=" + (late == 2) + "/" + (("late" in si.locals) ? "in" : "out");'),
    ("gs_params_in_locals", 'local f = function(pa, pb) { local si = getstackinfos(1); return si.locals.pa + "/" + si.locals.pb; }; return f(7, "b");'),
    ("gs_level2_outer", 'local outerMark = "O"; local inner = function() { local si = getstackinfos(2); return si.func + "/" + (("outerMark" in si.locals) ? "in" : "out"); }; return inner();'),
    ("gs_beyond_depth", 'local si = getstackinfos(99); return typeof si;'),
    ("gs_main_frame", 'local topMark = "T"; local f = function() { local si = getstackinfos(2); return si.func + "/" + (("topMark" in si.locals) ? "in" : "out"); }; return f();'),
    ("gs_in_thread", 'local th = newthread(function() { local si = getstackinfos(1); return si == null ? "null" : si.func; }); return th.call();'),
    ("gs_src_named", 'local f = compilestring("return getstackinfos(1).src", "my-src"); return f();'),
    ("gs_locals_mutate", 'local m = "orig"; local si = getstackinfos(1); si.locals.m = "changed"; return m + "/" + si.locals.m;'),
    ("gs_tailcall_hidden", 'local target = function() { local si = getstackinfos(2); return si == null ? "null" : si.func; }; local tramp = function() { return target(); }; return tramp();'),
]

lines = [
    "// 自动生成：clone 深度/共享 + getstackinfos locals 内容矩阵（tools/gen_clonestack.py）",
    "//",
    "// **clone 是浅拷贝**（嵌套表共享、委托保留）、generator/thread/标量/null 都",
    "// 不可 clone（cloning a xxx）；**getstackinfos**：匿名函数 func=\"unknown\"、",
    "// src 是相对路径、locals 含参数与已声明局部、生成器帧查不到外层局部、",
    "// 线程内返回 null、返回的 locals 表是副本、尾调用隐藏帧。",
    "// 两侧逐字节一致。",
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

lines += ['print("CLONESTACK-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

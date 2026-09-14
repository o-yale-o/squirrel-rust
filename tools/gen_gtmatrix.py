# -*- coding: utf-8 -*-
"""生成"生成器/线程 × 挂起-恢复 × 异常交错"矩阵语料 tests-corpus/t_gtmap.nut。

为什么单开一份：`t_suspmap`（16 条）测 suspend 穿透、`t_threadmap`（162 条）测
线程状态机**主干**——但"**异常与挂起交错**"这一族（yield 在 try 里、恢复点在
try 里、生成器抛错后的死态复活、wakeupthrow 注入被生成器自己的 catch 接住、
错误后线程重启…）没有用例。本轮为**验证型**：探针 34 + 7 条两侧逐字节一致。

实测钉住的语义（都对着参考实现验过）：
1. **生成器**：死后 `resume` 报 `resuming dead generator`（含正常跑完/抛错后
   两种死法）；生成器体内抛错 → 错误传到**恢复点**的 catch，状态变 dead；
   yield 在 try 里、恢复后体内 throw → 走生成器**自己**的 catch；
   `yield` 是**语句不是表达式**（`local a = yield x` 是编译错误）；
   含 yield 的普通函数/顶层**能编译**（自动成为生成器）。
2. **线程**：状态串是 `idle/suspended/…`（没有 "dead"，跑完回 **idle**）；
   `wakeup`/`wakeupthrow` 打在 idle 上报 `cannot wakeup a idle thread`；
   **idle 的线程可以再 `call()`（从头重跑）**；`wakeupthrow` 注入的错误会被
   挂起点外层的 try 接住（`caught:injected`），没有 try 则传给调用方；
   `wakeup` 不带参数 → suspend 表达式的值是 null；多带的参数被忽略；
   `getstackinfos` 对挂起线程返回 null。
3. **交叉**：线程体内开生成器、生成器体内开线程，挂起/恢复各自独立。

安全约束（**会中止/挂死参考实现的输入，不进语料**，实测见 tmp-c2/gt_isolate.py）：
- `newthread(<原生闭包>)`：参考实现**创建时即崩溃**（实测 WER 弹窗、无输出）；
  本实现正常创建线程对象、调用时报 `wrong number of parameters` —— 有意偏离。
- 主线程里 `suspend()`：两侧**同样静默退出**（stdout 截止、无错误文本、
  try/catch 接不住）—— 一致但会中止脚本，故不进语料（无 DONE 标记会被
  health 步骤判 SUSPECT），记入 MIGRATION.md。

用法：
    python tools/gen_gtmatrix.py
    python tools/gen_gtmatrix.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_gtmap.nut")

CASES = [
    # ---------- 生成器 ----------
    ("g_basic", 'local g = function() { yield 1; yield 2; }; local it = g(); local s = ""; foreach (v in it) s += v; return s;'),
    ("g_status_life", 'local it = (function() { yield 1; yield 2; return 9; })(); local a = it.getstatus(); resume it; local b = it.getstatus(); foreach (v in it) { } local c = it.getstatus(); return a + "/" + b + "/" + c;'),
    ("g_resume_dead", 'local it = (function() { yield 1; })(); foreach (v in it) { } return resume it;'),
    ("g_resume_dead_val", 'local it = (function() { yield 1; return 42; })(); resume it; local r1 = resume it; local r2 = resume it; return r1 + "/" + r2;'),
    ("g_throw_inside", 'local it = (function() { yield 1; throw "boom"; yield 2; })(); local s = ""; try { foreach (v in it) s += v; } catch (e) { s += "E:" + e; } return s + "/" + it.getstatus();'),
    ("g_resume_after_throw", 'local it = (function() { yield 1; throw "boom"; yield 2; })(); resume it; try { resume it; } catch (e) { } return resume it;'),
    ("g_yield_in_try", 'local it = (function() { try { yield "in-try"; throw "inner"; } catch (e) { yield "caught:" + e; } })(); local a = resume it; local b = resume it; return a + "/" + b;'),
    ("g_throw_across_yield", 'local it = (function() { yield 1; throw "from-gen"; })(); local s = ""; resume it; try { resume it; } catch (e) { s = "C:" + e; } return s;'),
    ("g_catch_yields", 'local it = (function() { try { throw "x"; } catch (e) { yield "c1"; yield "c2"; } })(); local a = resume it; local b = resume it; return a + "/" + b;'),
    ("g_yield_as_expr", 'return compilestring("function f() { local a = yield 1; }", "y3") == null ? "compile-null" : "compiled";'),
    ("g_resume_arg_syntax", 'local a = compilestring("local g = (function() { yield 1; })(); resume g(1);", "y4") == null ? "null" : "ok"; local b = compilestring("local g = (function() { yield 1; })(); resume g;", "y5") == null ? "null" : "ok"; return a + "/" + b;'),
    ("g_yield_no_func", 'return compilestring("function f() { yield 1; }", "y6") == null ? "compile-null" : "compiled";'),
    ("g_yield_toplevel", 'return compilestring("yield 1;", "y7") == null ? "compile-null" : "compiled";'),
    ("g_nested", 'local inner = function() { yield 10; yield 20; }; local outer = function() { foreach (v in inner()) yield v + 1; }; local s = ""; foreach (v in outer()) s += v + ","; return s;'),
    ("g_foreach_break", 'local it = (function() { yield 1; yield 2; yield 3; })(); local s = ""; foreach (v in it) { s += v; if (v == 2) break; } return s + "/" + it.getstatus();'),
    ("g_interleaved", 'local a = (function() { yield "a1"; yield "a2"; })(); local b = (function() { yield "b1"; yield "b2"; })(); return "" + (resume a) + (resume b) + (resume a) + (resume b);'),
    # ---------- 线程 ----------
    ("th_basic", 'local th = newthread(function() { local a = suspend("s1"); return "got:" + a; }); local r1 = th.call(); local r2 = th.wakeup(42); return r1 + "/" + r2 + "/" + th.getstatus();'),
    ("th_status_life", 'local th = newthread(function() { suspend(1); suspend(2); return 3; }); local a = th.getstatus(); th.call(); local b = th.getstatus(); th.wakeup(); local c = th.getstatus(); th.wakeup(); local d = th.getstatus(); return a + "/" + b + "/" + c + "/" + d;'),
    ("th_suspend_returns", 'local th = newthread(function() { suspend("a"); suspend("b"); return "r"; }); try { return th.call() + "/" + th.wakeup() + "/" + th.wakeup() + "/" + th.wakeup(); } catch (e) { return "E:" + e; }'),
    ("th_throw_in_body", 'local th = newthread(function() { suspend("s"); throw "thread-boom"; }); local r1 = th.call(); local st = th.getstatus(); local s = r1 + "/" + st + "/"; try { th.wakeup(); } catch (e) { s += "E:" + e; } return s + "/" + th.getstatus();'),
    ("th_throw_first_call", 'local th = newthread(function() { throw "immediate"; }); try { th.call(); } catch (e) { return "E:" + e + "/" + th.getstatus(); } return "no-error";'),
    ("th_wakeup_dead", 'local th = newthread(function() { return 1; }); th.call(); return th.wakeup();'),
    ("th_call_dead_restart", 'local th = newthread(function() { return 1; }); local a = th.call(); local b = th.call(); return a + "/" + b + "/" + th.getstatus();'),
    ("th_wakeupthrow_suspended", 'local th = newthread(function() { try { suspend("in-try"); return "no"; } catch (e) { return "caught:" + e; } }); local r1 = th.call(); local r2 = th.wakeupthrow("injected"); return r1 + "/" + r2;'),
    ("th_wakeupthrow_notry", 'local th = newthread(function() { suspend("plain"); return "no"; }); th.call(); try { th.wakeupthrow("inj"); } catch (e) { return "outer:" + e + "/" + th.getstatus(); } return "no-error";'),
    ("th_wakeupthrow_running", 'local th = newthread(function() { return 1; }); return th.wakeupthrow("x");'),
    ("th_wakeup_running", 'local th = newthread(function() { return 1; }); return th.wakeup();'),
    ("th_suspend_in_foreach", 'local th = newthread(function() { foreach (v in [1, 2, 3]) { suspend(v); } return "done"; }); local a = th.call(); local b = th.wakeup(); local c = th.wakeup(); local d = th.wakeup(); return a + "/" + b + "/" + c + "/" + d;'),
    ("th_suspend_in_try", 'local th = newthread(function() { try { suspend("s"); } catch (e) { return "caught"; } return "after"; }); local a = th.call(); local b = th.wakeup(); return a + "/" + b;'),
    ("th_err_after_resume", 'local th = newthread(function() { local x = suspend("s"); if (x == 2) throw "late"; return "ok" + x; }); local a = th.call(); local b = th.wakeup(1); local c; try { c = th.wakeup(2); } catch (e) { c = "E:" + e; } return a + "/" + b + "/" + c + "/" + th.getstatus();'),
    ("th_nested_threads", 'local inner = newthread(function() { suspend("i1"); return "i2"; }); local outer = newthread(function() { local a = inner.call(); local b = inner.wakeup(); suspend(a + b); return "od"; }); local a = outer.call(); local b = outer.wakeup(); return a + "/" + b;'),
    ("th_newthread_nonfunc", 'return newthread(42);'),
    ("th_gen_in_thread", 'local th = newthread(function() { local g = (function() { yield "gy"; })(); local v = resume g; suspend(v + "/" + g.getstatus()); return "end"; }); local a = th.call(); local b = th.wakeup(); return a + "/" + b;'),
    ("th_thread_in_gen", 'local gen = function() { local th = newthread(function() { suspend("ts"); return "tr"; }); local a = th.call(); yield a; local b = th.wakeup(); yield b; }; local g = gen(); local a = resume g; local b = resume g; return a + "/" + b;'),
    ("th_getstackinfos", 'local th = newthread(function() { suspend(1); }); th.call(); local si = th.getstackinfos(1); return typeof si + "/" + (si == null ? "null" : si.func);'),
    ("th_wakeup_noargs", 'local th = newthread(function() { local a = suspend("s"); return "a=" + a; }); th.call(); return th.wakeup();'),
    ("th_wakeup_two_args", 'local th = newthread(function() { local a = suspend("s"); return "a=" + a; }); th.call(); return th.wakeup(1, 2);'),
]

lines = [
    "// 自动生成：生成器/线程 × 挂起-恢复 × 异常交错矩阵（tools/gen_gtmatrix.py）",
    "//",
    "// **不含**两个会中止/挂死参考实现的形态（见生成器 docstring）：",
    "//   newthread(<原生闭包>) —— 参考实现创建时即崩溃（WER），本实现正常（有意偏离）；",
    "//   主线程 suspend() —— 两侧同样静默退出（一致但中止脚本，无 DONE 标记）。",
    "function t(tag, f) {",
    "    try { local v = f(); local o = typeof v; print(tag + \"=\" + (o == \"string\" ? v : (o == \"thread\" || o == \"generator\" ? o : \"\" + v)) + \"|\" + o + \"\\n\"); }",
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

lines += ['print("GTMAP-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

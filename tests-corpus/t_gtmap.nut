// 自动生成：生成器/线程 × 挂起-恢复 × 异常交错矩阵（tools/gen_gtmatrix.py）
//
// **不含**两个会中止/挂死参考实现的形态（见生成器 docstring）：
//   newthread(<原生闭包>) —— 参考实现创建时即崩溃（WER），本实现正常（有意偏离）；
//   主线程 suspend() —— 两侧同样静默退出（一致但中止脚本，无 DONE 标记）。
function t(tag, f) {
    try { local v = f(); local o = typeof v; print(tag + "=" + (o == "string" ? v : (o == "thread" || o == "generator" ? o : "" + v)) + "|" + o + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

t("g_basic", function() { local g = function() { yield 1; yield 2; }; local it = g(); local s = ""; foreach (v in it) s += v; return s; });
t("g_status_life", function() { local it = (function() { yield 1; yield 2; return 9; })(); local a = it.getstatus(); resume it; local b = it.getstatus(); foreach (v in it) { } local c = it.getstatus(); return a + "/" + b + "/" + c; });
t("g_resume_dead", function() { local it = (function() { yield 1; })(); foreach (v in it) { } return resume it; });
t("g_resume_dead_val", function() { local it = (function() { yield 1; return 42; })(); resume it; local r1 = resume it; local r2 = resume it; return r1 + "/" + r2; });
t("g_throw_inside", function() { local it = (function() { yield 1; throw "boom"; yield 2; })(); local s = ""; try { foreach (v in it) s += v; } catch (e) { s += "E:" + e; } return s + "/" + it.getstatus(); });
t("g_resume_after_throw", function() { local it = (function() { yield 1; throw "boom"; yield 2; })(); resume it; try { resume it; } catch (e) { } return resume it; });
t("g_yield_in_try", function() { local it = (function() { try { yield "in-try"; throw "inner"; } catch (e) { yield "caught:" + e; } })(); local a = resume it; local b = resume it; return a + "/" + b; });
t("g_throw_across_yield", function() { local it = (function() { yield 1; throw "from-gen"; })(); local s = ""; resume it; try { resume it; } catch (e) { s = "C:" + e; } return s; });
t("g_catch_yields", function() { local it = (function() { try { throw "x"; } catch (e) { yield "c1"; yield "c2"; } })(); local a = resume it; local b = resume it; return a + "/" + b; });
t("g_yield_as_expr", function() { return compilestring("function f() { local a = yield 1; }", "y3") == null ? "compile-null" : "compiled"; });
t("g_resume_arg_syntax", function() { local a = compilestring("local g = (function() { yield 1; })(); resume g(1);", "y4") == null ? "null" : "ok"; local b = compilestring("local g = (function() { yield 1; })(); resume g;", "y5") == null ? "null" : "ok"; return a + "/" + b; });
t("g_yield_no_func", function() { return compilestring("function f() { yield 1; }", "y6") == null ? "compile-null" : "compiled"; });
t("g_yield_toplevel", function() { return compilestring("yield 1;", "y7") == null ? "compile-null" : "compiled"; });
t("g_nested", function() { local inner = function() { yield 10; yield 20; }; local outer = function() { foreach (v in inner()) yield v + 1; }; local s = ""; foreach (v in outer()) s += v + ","; return s; });
t("g_foreach_break", function() { local it = (function() { yield 1; yield 2; yield 3; })(); local s = ""; foreach (v in it) { s += v; if (v == 2) break; } return s + "/" + it.getstatus(); });
t("g_interleaved", function() { local a = (function() { yield "a1"; yield "a2"; })(); local b = (function() { yield "b1"; yield "b2"; })(); return "" + (resume a) + (resume b) + (resume a) + (resume b); });
t("th_basic", function() { local th = newthread(function() { local a = suspend("s1"); return "got:" + a; }); local r1 = th.call(); local r2 = th.wakeup(42); return r1 + "/" + r2 + "/" + th.getstatus(); });
t("th_status_life", function() { local th = newthread(function() { suspend(1); suspend(2); return 3; }); local a = th.getstatus(); th.call(); local b = th.getstatus(); th.wakeup(); local c = th.getstatus(); th.wakeup(); local d = th.getstatus(); return a + "/" + b + "/" + c + "/" + d; });
t("th_suspend_returns", function() { local th = newthread(function() { suspend("a"); suspend("b"); return "r"; }); try { return th.call() + "/" + th.wakeup() + "/" + th.wakeup() + "/" + th.wakeup(); } catch (e) { return "E:" + e; } });
t("th_throw_in_body", function() { local th = newthread(function() { suspend("s"); throw "thread-boom"; }); local r1 = th.call(); local st = th.getstatus(); local s = r1 + "/" + st + "/"; try { th.wakeup(); } catch (e) { s += "E:" + e; } return s + "/" + th.getstatus(); });
t("th_throw_first_call", function() { local th = newthread(function() { throw "immediate"; }); try { th.call(); } catch (e) { return "E:" + e + "/" + th.getstatus(); } return "no-error"; });
t("th_wakeup_dead", function() { local th = newthread(function() { return 1; }); th.call(); return th.wakeup(); });
t("th_call_dead_restart", function() { local th = newthread(function() { return 1; }); local a = th.call(); local b = th.call(); return a + "/" + b + "/" + th.getstatus(); });
t("th_wakeupthrow_suspended", function() { local th = newthread(function() { try { suspend("in-try"); return "no"; } catch (e) { return "caught:" + e; } }); local r1 = th.call(); local r2 = th.wakeupthrow("injected"); return r1 + "/" + r2; });
t("th_wakeupthrow_notry", function() { local th = newthread(function() { suspend("plain"); return "no"; }); th.call(); try { th.wakeupthrow("inj"); } catch (e) { return "outer:" + e + "/" + th.getstatus(); } return "no-error"; });
t("th_wakeupthrow_running", function() { local th = newthread(function() { return 1; }); return th.wakeupthrow("x"); });
t("th_wakeup_running", function() { local th = newthread(function() { return 1; }); return th.wakeup(); });
t("th_suspend_in_foreach", function() { local th = newthread(function() { foreach (v in [1, 2, 3]) { suspend(v); } return "done"; }); local a = th.call(); local b = th.wakeup(); local c = th.wakeup(); local d = th.wakeup(); return a + "/" + b + "/" + c + "/" + d; });
t("th_suspend_in_try", function() { local th = newthread(function() { try { suspend("s"); } catch (e) { return "caught"; } return "after"; }); local a = th.call(); local b = th.wakeup(); return a + "/" + b; });
t("th_err_after_resume", function() { local th = newthread(function() { local x = suspend("s"); if (x == 2) throw "late"; return "ok" + x; }); local a = th.call(); local b = th.wakeup(1); local c; try { c = th.wakeup(2); } catch (e) { c = "E:" + e; } return a + "/" + b + "/" + c + "/" + th.getstatus(); });
t("th_nested_threads", function() { local inner = newthread(function() { suspend("i1"); return "i2"; }); local outer = newthread(function() { local a = inner.call(); local b = inner.wakeup(); suspend(a + b); return "od"; }); local a = outer.call(); local b = outer.wakeup(); return a + "/" + b; });
t("th_newthread_nonfunc", function() { return newthread(42); });
t("th_gen_in_thread", function() { local th = newthread(function() { local g = (function() { yield "gy"; })(); local v = resume g; suspend(v + "/" + g.getstatus()); return "end"; }); local a = th.call(); local b = th.wakeup(); return a + "/" + b; });
t("th_thread_in_gen", function() { local gen = function() { local th = newthread(function() { suspend("ts"); return "tr"; }); local a = th.call(); yield a; local b = th.wakeup(); yield b; }; local g = gen(); local a = resume g; local b = resume g; return a + "/" + b; });
t("th_getstackinfos", function() { local th = newthread(function() { suspend(1); }); th.call(); local si = th.getstackinfos(1); return typeof si + "/" + (si == null ? "null" : si.func); });
t("th_wakeup_noargs", function() { local th = newthread(function() { local a = suspend("s"); return "a=" + a; }); th.call(); return th.wakeup(); });
t("th_wakeup_two_args", function() { local th = newthread(function() { local a = suspend("s"); return "a=" + a; }); th.call(); return th.wakeup(1, 2); });
print("GTMAP-DONE\n");

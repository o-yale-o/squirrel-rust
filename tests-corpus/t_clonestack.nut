// 自动生成：clone 深度/共享 + getstackinfos locals 内容矩阵（tools/gen_clonestack.py）
//
// **clone 是浅拷贝**（嵌套表共享、委托保留）、generator/thread/标量/null 都
// 不可 clone（cloning a xxx）；**getstackinfos**：匿名函数 func="unknown"、
// src 是相对路径、locals 含参数与已声明局部、生成器帧查不到外层局部、
// 线程内返回 null、返回的 locals 表是副本、尾调用隐藏帧。
// 两侧逐字节一致。
function d(x) {
    local t = typeof x;
    if (t == "table") { local o = "{"; local f = true; foreach (k, v in x) { if (!f) o += ","; f = false; o += "" + k + "=" + v; } return o + "}"; }
    if (t == "array") { local o = "["; for (local i = 0; i < x.len(); i++) { if (i) o += ","; o += "" + x[i]; } return o + "]"; }
    return "" + x;
}
function t(tag, f) {
    try { local v = f(); print(tag + "=" + d(v) + "|" + typeof v + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

t("cl_nested_shared", function() { local a = { inner = { n = 1 } }; local b = clone a; b.inner.n = 42; return "a=" + a.inner.n + " b=" + b.inner.n; });
t("cl_arr_of_tbl", function() { local a = [{ k = 1 }, { k = 2 }]; local b = clone a; b[1].k = 99; b.append({ k = 3 }); return "a.len=" + a.len() + " a1k=" + a[1].k + " b.len=" + b.len(); });
t("cl_array_indep", function() { local a = [1, 2]; local b = clone a; b.append(3); b[0] = 99; return "a=" + d(a) + " b=" + d(b); });
t("cl_delegate_kept", function() { local bs = { g = "from-base" }; local a = { own = 1 }; a.setdelegate(bs); local b = clone a; return "b.g=" + b.g + " b.own=" + b.own + " bdeleg=" + (b.getdelegate() == null ? "null" : "tbl"); });
t("cl_delegate_mutate", function() { local bs = { g = 1 }; local a = { own = 1 }; a.setdelegate(bs); local b = clone a; b.g = 99; return "base.g=" + bs.g + " a.g=" + a.g + " b.g=" + b.g + " b.own=" + b.own; });
t("cl_scalar_int", function() { return clone 5; });
t("cl_scalar_str", function() { return clone "s"; });
t("cl_gen", function() { local g = (function() { yield 1; })(); return typeof clone g; });
t("cl_thread", function() { local th = newthread(function() { }); return typeof clone th; });
t("cl_null", function() { return typeof clone null; });
t("gs_level0", function() { local si = getstackinfos(0); return si.func + "/" + si.locals.len(); });
t("gs_level1_locals", function() { local mark = "M"; local n2 = 2; local si = getstackinfos(1); return si.func + "/" + si.src + "/" + si.locals.len() + "/" + si.locals.mark; });
t("gs_level1_later_local", function() { local early = 1; local si = getstackinfos(1); local late = 2; return "early=" + si.locals.early + " late=" + (late == 2) + "/" + (("late" in si.locals) ? "in" : "out"); });
t("gs_params_in_locals", function() { local f = function(pa, pb) { local si = getstackinfos(1); return si.locals.pa + "/" + si.locals.pb; }; return f(7, "b"); });
t("gs_level2_outer", function() { local outerMark = "O"; local inner = function() { local si = getstackinfos(2); return si.func + "/" + (("outerMark" in si.locals) ? "in" : "out"); }; return inner(); });
t("gs_beyond_depth", function() { local si = getstackinfos(99); return typeof si; });
t("gs_main_frame", function() { local topMark = "T"; local f = function() { local si = getstackinfos(2); return si.func + "/" + (("topMark" in si.locals) ? "in" : "out"); }; return f(); });
t("gs_in_thread", function() { local th = newthread(function() { local si = getstackinfos(1); return si == null ? "null" : si.func; }); return th.call(); });
t("gs_src_named", function() { local f = compilestring("return getstackinfos(1).src", "my-src"); return f(); });
t("gs_locals_mutate", function() { local m = "orig"; local si = getstackinfos(1); si.locals.m = "changed"; return m + "/" + si.locals.m; });
t("gs_tailcall_hidden", function() { local target = function() { local si = getstackinfos(2); return si == null ? "null" : si.func; }; local tramp = function() { return target(); }; return tramp(); });
print("CLONESTACK-DONE\n");

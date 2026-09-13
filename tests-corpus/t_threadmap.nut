
// 自动生成：线程/生成器 状态 × API 矩阵（tools/gen_threadmatrix.py）
// 观测口径：前置状态 | 返回值 | 错误 | 后置状态
function show(r, d = 0) {
    if (d > 2) return "...";
    local t = typeof r;
    if (t == "array") { local s = "["; for (local i = 0; i < r.len(); i++) { if (i > 0) s += ","; s += show(r[i], d + 1); } return s + "]"; }
    if (t == "table") { local s = "{"; foreach (k, v in r) s += show(k, d + 1) + ":" + show(v, d + 1) + ","; return s + "}"; }
    if (t == "string") return "\"" + r + "\"";
    if (t == "float") return format("%.17g", r);
    return "" + r;
}
function st(o) { try { return o.getstatus(); } catch (e) { return "E:" + e; } }
// 已知偏离（MIGRATION.md 第 3 节）：对**已挂起**线程再 call() 时报的类型名，
// 参考实现读的是**残留栈槽** —— 实测那个槽正好还是 suspend 传入的值：
//   42→'integer'、"s"→'string'、null→'null'、[1]→'array'、{}→'table'、1.5→'float'、true→'bool'
// 我们关帧后恒为 'table'（可捕获错误）。更糟的是：残留值若是闭包，参考实现会把它
// **当成函数调用并成功**（实测 err=null），我们只报错。
// 所以这里把类型名归一化，语料只比较"是否报了 attempt to call"，不钉那个不稳定的名字。
function nerr(e) {
    if (e == null) return null;
    local s = "" + e;
    if (s.find("attempt to call '") == 0) return "attempt to call <type>";
    return s;
}
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!outer:" + e + "\n"); }
}
// 跑一次调用并汇报：前置状态 | 返回值 | 错误 | 后置状态
function go(o, f) {
    local before = st(o);
    local r = null; local err = null;
    try { r = f(o); } catch (e) { err = nerr("" + e); }
    return "before=" + before + "|r=" + show(r) + "|err=" + show(err) + "|after=" + st(o);
}

// ---- 线程状态构造器（setup 里的失败一律吞掉，保证状态一定到达）
function th_idle() { return newthread(function() { suspend("s1"); return "after"; }); }
function th_suspended() {
    local o = newthread(function() { suspend("s1"); return "after"; });
    try { o.call(); } catch (e) {}
    return o;
}
function th_suspended_val() {
    local o = newthread(function() { local v = suspend("s1"); return "got:" + v; });
    try { o.call(); } catch (e) {}
    return o;
}
function th_dead_return() {
    local o = newthread(function() { return "done"; });
    try { local x = o.call(); } catch (e) {}
    return o;
}
function th_dead_throw() {
    local o = newthread(function() { throw "boom"; });
    try { local x = o.call(); } catch (e) {}
    return o;
}
function th_suspended_nested() {
    local o = newthread(function() { return th_inner(); });
    try { o.call(); } catch (e) {}
    return o;
}
function th_inner() { suspend("nested"); return "inner"; }
function th_suspended_twice() {
    local o = newthread(function() { suspend("a"); suspend("b"); return "twice"; });
    try { o.call(); } catch (e) {}
    try { o.wakeup(); } catch (e) {}
    return o;
}
function th_dead_after_wakeup() {
    local o = newthread(function() { suspend("a"); return "end"; });
    try { o.call(); } catch (e) {}
    try { o.wakeup(); } catch (e) {}
    return o;
}

// ---- 生成器状态构造器
function gen_new() { return (function() { yield 1; yield 2; }()); }
function gen_suspended() {
    local g = (function() { yield 1; yield 2; }());
    try { resume g; } catch (e) {}
    return g;
}
function gen_dead() {
    local g = (function() { yield 1; }());
    try { resume g; } catch (e) {}
    try { resume g; } catch (e) {}
    return g;
}
function gen_never_started_dead() { return (function() { return 1; }()); }
function gst(o) { try { return o.getstatus(); } catch (e) { return "E:" + e; } }
function ggo(o, f) {
    local before = gst(o);
    local r = null; local err = null;
    try { r = f(o); } catch (e) { err = "" + e; }
    return "before=" + before + "|r=" + show(r) + "|err=" + show(err) + "|after=" + gst(o);
}

t("th_idle_call0", function() { local o = th_idle(); return go(o, function(o) { return o.call(); }); });
t("th_idle_call_args", function() { local o = th_idle(); return go(o, function(o) { return o.call(1, 2); }); });
t("th_idle_call_thrice", function() { local o = th_idle(); return go(o, function(o) { return o.call(1, 2, 3); }); });
t("th_idle_wakeup0", function() { local o = th_idle(); return go(o, function(o) { return o.wakeup(); }); });
t("th_idle_wakeup_val", function() { local o = th_idle(); return go(o, function(o) { return o.wakeup(9); }); });
t("th_idle_wakeup_null", function() { local o = th_idle(); return go(o, function(o) { return o.wakeup(null); }); });
t("th_idle_wakeup_str", function() { local o = th_idle(); return go(o, function(o) { return o.wakeup("w"); }); });
t("th_idle_wakeupthrow_val", function() { local o = th_idle(); return go(o, function(o) { return o.wakeupthrow("e"); }); });
t("th_idle_wakeupthrow_bool", function() { local o = th_idle(); return go(o, function(o) { return o.wakeupthrow("e", true); }); });
t("th_idle_wakeupthrow_3", function() { local o = th_idle(); return go(o, function(o) { return o.wakeupthrow("e", true, 1); }); });
t("th_idle_getstatus", function() { local o = th_idle(); return go(o, function(o) { return o.getstatus(); }); });
t("th_idle_stackinfos0", function() { local o = th_idle(); return go(o, function(o) { return o.getstackinfos(0); }); });
t("th_idle_stackinfos_big", function() { local o = th_idle(); return go(o, function(o) { return o.getstackinfos(99); }); });
t("th_idle_weakref", function() { local o = th_idle(); return go(o, function(o) { return o.weakref(); }); });
t("th_idle_tostring", function() { local o = th_idle(); return go(o, function(o) { return o.tostring(); }); });
t("th_idle_typeof", function() { local o = th_idle(); return go(o, function(o) { return typeof o; }); });
t("th_suspended_call0", function() { local o = th_suspended(); return go(o, function(o) { return o.call(); }); });
t("th_suspended_call_args", function() { local o = th_suspended(); return go(o, function(o) { return o.call(1, 2); }); });
t("th_suspended_call_thrice", function() { local o = th_suspended(); return go(o, function(o) { return o.call(1, 2, 3); }); });
t("th_suspended_wakeup0", function() { local o = th_suspended(); return go(o, function(o) { return o.wakeup(); }); });
t("th_suspended_wakeup_val", function() { local o = th_suspended(); return go(o, function(o) { return o.wakeup(9); }); });
t("th_suspended_wakeup_null", function() { local o = th_suspended(); return go(o, function(o) { return o.wakeup(null); }); });
t("th_suspended_wakeup_str", function() { local o = th_suspended(); return go(o, function(o) { return o.wakeup("w"); }); });
t("th_suspended_wakeupthrow_val", function() { local o = th_suspended(); return go(o, function(o) { return o.wakeupthrow("e"); }); });
t("th_suspended_wakeupthrow_bool", function() { local o = th_suspended(); return go(o, function(o) { return o.wakeupthrow("e", true); }); });
t("th_suspended_wakeupthrow_3", function() { local o = th_suspended(); return go(o, function(o) { return o.wakeupthrow("e", true, 1); }); });
t("th_suspended_getstatus", function() { local o = th_suspended(); return go(o, function(o) { return o.getstatus(); }); });
t("th_suspended_stackinfos0", function() { local o = th_suspended(); return go(o, function(o) { return o.getstackinfos(0); }); });
t("th_suspended_stackinfos_big", function() { local o = th_suspended(); return go(o, function(o) { return o.getstackinfos(99); }); });
t("th_suspended_weakref", function() { local o = th_suspended(); return go(o, function(o) { return o.weakref(); }); });
t("th_suspended_tostring", function() { local o = th_suspended(); return go(o, function(o) { return o.tostring(); }); });
t("th_suspended_typeof", function() { local o = th_suspended(); return go(o, function(o) { return typeof o; }); });
t("th_suspended_val_call0", function() { local o = th_suspended_val(); return go(o, function(o) { return o.call(); }); });
t("th_suspended_val_call_args", function() { local o = th_suspended_val(); return go(o, function(o) { return o.call(1, 2); }); });
t("th_suspended_val_call_thrice", function() { local o = th_suspended_val(); return go(o, function(o) { return o.call(1, 2, 3); }); });
t("th_suspended_val_wakeup0", function() { local o = th_suspended_val(); return go(o, function(o) { return o.wakeup(); }); });
t("th_suspended_val_wakeup_val", function() { local o = th_suspended_val(); return go(o, function(o) { return o.wakeup(9); }); });
t("th_suspended_val_wakeup_null", function() { local o = th_suspended_val(); return go(o, function(o) { return o.wakeup(null); }); });
t("th_suspended_val_wakeup_str", function() { local o = th_suspended_val(); return go(o, function(o) { return o.wakeup("w"); }); });
t("th_suspended_val_wakeupthrow_val", function() { local o = th_suspended_val(); return go(o, function(o) { return o.wakeupthrow("e"); }); });
t("th_suspended_val_wakeupthrow_bool", function() { local o = th_suspended_val(); return go(o, function(o) { return o.wakeupthrow("e", true); }); });
t("th_suspended_val_wakeupthrow_3", function() { local o = th_suspended_val(); return go(o, function(o) { return o.wakeupthrow("e", true, 1); }); });
t("th_suspended_val_getstatus", function() { local o = th_suspended_val(); return go(o, function(o) { return o.getstatus(); }); });
t("th_suspended_val_stackinfos0", function() { local o = th_suspended_val(); return go(o, function(o) { return o.getstackinfos(0); }); });
t("th_suspended_val_stackinfos_big", function() { local o = th_suspended_val(); return go(o, function(o) { return o.getstackinfos(99); }); });
t("th_suspended_val_weakref", function() { local o = th_suspended_val(); return go(o, function(o) { return o.weakref(); }); });
t("th_suspended_val_tostring", function() { local o = th_suspended_val(); return go(o, function(o) { return o.tostring(); }); });
t("th_suspended_val_typeof", function() { local o = th_suspended_val(); return go(o, function(o) { return typeof o; }); });
t("th_dead_return_call0", function() { local o = th_dead_return(); return go(o, function(o) { return o.call(); }); });
t("th_dead_return_call_args", function() { local o = th_dead_return(); return go(o, function(o) { return o.call(1, 2); }); });
t("th_dead_return_call_thrice", function() { local o = th_dead_return(); return go(o, function(o) { return o.call(1, 2, 3); }); });
t("th_dead_return_wakeup0", function() { local o = th_dead_return(); return go(o, function(o) { return o.wakeup(); }); });
t("th_dead_return_wakeup_val", function() { local o = th_dead_return(); return go(o, function(o) { return o.wakeup(9); }); });
t("th_dead_return_wakeup_null", function() { local o = th_dead_return(); return go(o, function(o) { return o.wakeup(null); }); });
t("th_dead_return_wakeup_str", function() { local o = th_dead_return(); return go(o, function(o) { return o.wakeup("w"); }); });
t("th_dead_return_wakeupthrow_val", function() { local o = th_dead_return(); return go(o, function(o) { return o.wakeupthrow("e"); }); });
t("th_dead_return_wakeupthrow_bool", function() { local o = th_dead_return(); return go(o, function(o) { return o.wakeupthrow("e", true); }); });
t("th_dead_return_wakeupthrow_3", function() { local o = th_dead_return(); return go(o, function(o) { return o.wakeupthrow("e", true, 1); }); });
t("th_dead_return_getstatus", function() { local o = th_dead_return(); return go(o, function(o) { return o.getstatus(); }); });
t("th_dead_return_stackinfos0", function() { local o = th_dead_return(); return go(o, function(o) { return o.getstackinfos(0); }); });
t("th_dead_return_stackinfos_big", function() { local o = th_dead_return(); return go(o, function(o) { return o.getstackinfos(99); }); });
t("th_dead_return_weakref", function() { local o = th_dead_return(); return go(o, function(o) { return o.weakref(); }); });
t("th_dead_return_tostring", function() { local o = th_dead_return(); return go(o, function(o) { return o.tostring(); }); });
t("th_dead_return_typeof", function() { local o = th_dead_return(); return go(o, function(o) { return typeof o; }); });
t("th_dead_throw_call0", function() { local o = th_dead_throw(); return go(o, function(o) { return o.call(); }); });
t("th_dead_throw_call_args", function() { local o = th_dead_throw(); return go(o, function(o) { return o.call(1, 2); }); });
t("th_dead_throw_call_thrice", function() { local o = th_dead_throw(); return go(o, function(o) { return o.call(1, 2, 3); }); });
t("th_dead_throw_wakeup0", function() { local o = th_dead_throw(); return go(o, function(o) { return o.wakeup(); }); });
t("th_dead_throw_wakeup_val", function() { local o = th_dead_throw(); return go(o, function(o) { return o.wakeup(9); }); });
t("th_dead_throw_wakeup_null", function() { local o = th_dead_throw(); return go(o, function(o) { return o.wakeup(null); }); });
t("th_dead_throw_wakeup_str", function() { local o = th_dead_throw(); return go(o, function(o) { return o.wakeup("w"); }); });
t("th_dead_throw_wakeupthrow_val", function() { local o = th_dead_throw(); return go(o, function(o) { return o.wakeupthrow("e"); }); });
t("th_dead_throw_wakeupthrow_bool", function() { local o = th_dead_throw(); return go(o, function(o) { return o.wakeupthrow("e", true); }); });
t("th_dead_throw_wakeupthrow_3", function() { local o = th_dead_throw(); return go(o, function(o) { return o.wakeupthrow("e", true, 1); }); });
t("th_dead_throw_getstatus", function() { local o = th_dead_throw(); return go(o, function(o) { return o.getstatus(); }); });
t("th_dead_throw_stackinfos0", function() { local o = th_dead_throw(); return go(o, function(o) { return o.getstackinfos(0); }); });
t("th_dead_throw_stackinfos_big", function() { local o = th_dead_throw(); return go(o, function(o) { return o.getstackinfos(99); }); });
t("th_dead_throw_weakref", function() { local o = th_dead_throw(); return go(o, function(o) { return o.weakref(); }); });
t("th_dead_throw_tostring", function() { local o = th_dead_throw(); return go(o, function(o) { return o.tostring(); }); });
t("th_dead_throw_typeof", function() { local o = th_dead_throw(); return go(o, function(o) { return typeof o; }); });
t("th_suspended_nested_call0", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.call(); }); });
t("th_suspended_nested_call_args", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.call(1, 2); }); });
t("th_suspended_nested_call_thrice", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.call(1, 2, 3); }); });
t("th_suspended_nested_wakeup0", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.wakeup(); }); });
t("th_suspended_nested_wakeup_val", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.wakeup(9); }); });
t("th_suspended_nested_wakeup_null", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.wakeup(null); }); });
t("th_suspended_nested_wakeup_str", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.wakeup("w"); }); });
t("th_suspended_nested_wakeupthrow_val", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.wakeupthrow("e"); }); });
t("th_suspended_nested_wakeupthrow_bool", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.wakeupthrow("e", true); }); });
t("th_suspended_nested_wakeupthrow_3", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.wakeupthrow("e", true, 1); }); });
t("th_suspended_nested_getstatus", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.getstatus(); }); });
t("th_suspended_nested_stackinfos0", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.getstackinfos(0); }); });
t("th_suspended_nested_stackinfos_big", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.getstackinfos(99); }); });
t("th_suspended_nested_weakref", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.weakref(); }); });
t("th_suspended_nested_tostring", function() { local o = th_suspended_nested(); return go(o, function(o) { return o.tostring(); }); });
t("th_suspended_nested_typeof", function() { local o = th_suspended_nested(); return go(o, function(o) { return typeof o; }); });
t("th_suspended_twice_call0", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.call(); }); });
t("th_suspended_twice_call_args", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.call(1, 2); }); });
t("th_suspended_twice_call_thrice", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.call(1, 2, 3); }); });
t("th_suspended_twice_wakeup0", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.wakeup(); }); });
t("th_suspended_twice_wakeup_val", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.wakeup(9); }); });
t("th_suspended_twice_wakeup_null", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.wakeup(null); }); });
t("th_suspended_twice_wakeup_str", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.wakeup("w"); }); });
t("th_suspended_twice_wakeupthrow_val", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.wakeupthrow("e"); }); });
t("th_suspended_twice_wakeupthrow_bool", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.wakeupthrow("e", true); }); });
t("th_suspended_twice_wakeupthrow_3", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.wakeupthrow("e", true, 1); }); });
t("th_suspended_twice_getstatus", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.getstatus(); }); });
t("th_suspended_twice_stackinfos0", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.getstackinfos(0); }); });
t("th_suspended_twice_stackinfos_big", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.getstackinfos(99); }); });
t("th_suspended_twice_weakref", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.weakref(); }); });
t("th_suspended_twice_tostring", function() { local o = th_suspended_twice(); return go(o, function(o) { return o.tostring(); }); });
t("th_suspended_twice_typeof", function() { local o = th_suspended_twice(); return go(o, function(o) { return typeof o; }); });
t("th_dead_after_wakeup_call0", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.call(); }); });
t("th_dead_after_wakeup_call_args", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.call(1, 2); }); });
t("th_dead_after_wakeup_call_thrice", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.call(1, 2, 3); }); });
t("th_dead_after_wakeup_wakeup0", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.wakeup(); }); });
t("th_dead_after_wakeup_wakeup_val", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.wakeup(9); }); });
t("th_dead_after_wakeup_wakeup_null", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.wakeup(null); }); });
t("th_dead_after_wakeup_wakeup_str", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.wakeup("w"); }); });
t("th_dead_after_wakeup_wakeupthrow_val", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.wakeupthrow("e"); }); });
t("th_dead_after_wakeup_wakeupthrow_bool", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.wakeupthrow("e", true); }); });
t("th_dead_after_wakeup_wakeupthrow_3", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.wakeupthrow("e", true, 1); }); });
t("th_dead_after_wakeup_getstatus", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.getstatus(); }); });
t("th_dead_after_wakeup_stackinfos0", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.getstackinfos(0); }); });
t("th_dead_after_wakeup_stackinfos_big", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.getstackinfos(99); }); });
t("th_dead_after_wakeup_weakref", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.weakref(); }); });
t("th_dead_after_wakeup_tostring", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return o.tostring(); }); });
t("th_dead_after_wakeup_typeof", function() { local o = th_dead_after_wakeup(); return go(o, function(o) { return typeof o; }); });
t("gn_new_resume", function() { local o = gen_new(); return ggo(o, function(o) { return resume o; }); });
t("gn_new_getstatus", function() { local o = gen_new(); return ggo(o, function(o) { return o.getstatus(); }); });
t("gn_new_weakref", function() { local o = gen_new(); return ggo(o, function(o) { return o.weakref(); }); });
t("gn_new_tostring", function() { local o = gen_new(); return ggo(o, function(o) { return o.tostring(); }); });
t("gn_new_typeof", function() { local o = gen_new(); return ggo(o, function(o) { return typeof o; }); });
t("gn_new_stackinfos0", function() { local o = gen_new(); return ggo(o, function(o) { return o.getstackinfos(0); }); });
t("gn_suspended_resume", function() { local o = gen_suspended(); return ggo(o, function(o) { return resume o; }); });
t("gn_suspended_getstatus", function() { local o = gen_suspended(); return ggo(o, function(o) { return o.getstatus(); }); });
t("gn_suspended_weakref", function() { local o = gen_suspended(); return ggo(o, function(o) { return o.weakref(); }); });
t("gn_suspended_tostring", function() { local o = gen_suspended(); return ggo(o, function(o) { return o.tostring(); }); });
t("gn_suspended_typeof", function() { local o = gen_suspended(); return ggo(o, function(o) { return typeof o; }); });
t("gn_suspended_stackinfos0", function() { local o = gen_suspended(); return ggo(o, function(o) { return o.getstackinfos(0); }); });
t("gn_dead_resume", function() { local o = gen_dead(); return ggo(o, function(o) { return resume o; }); });
t("gn_dead_getstatus", function() { local o = gen_dead(); return ggo(o, function(o) { return o.getstatus(); }); });
t("gn_dead_weakref", function() { local o = gen_dead(); return ggo(o, function(o) { return o.weakref(); }); });
t("gn_dead_tostring", function() { local o = gen_dead(); return ggo(o, function(o) { return o.tostring(); }); });
t("gn_dead_typeof", function() { local o = gen_dead(); return ggo(o, function(o) { return typeof o; }); });
t("gn_dead_stackinfos0", function() { local o = gen_dead(); return ggo(o, function(o) { return o.getstackinfos(0); }); });
t("gn_dead_nostart_resume", function() { local o = gen_never_started_dead(); return ggo(o, function(o) { return resume o; }); });
t("gn_dead_nostart_getstatus", function() { local o = gen_never_started_dead(); return ggo(o, function(o) { return o.getstatus(); }); });
t("gn_dead_nostart_weakref", function() { local o = gen_never_started_dead(); return ggo(o, function(o) { return o.weakref(); }); });
t("gn_dead_nostart_tostring", function() { local o = gen_never_started_dead(); return ggo(o, function(o) { return o.tostring(); }); });
t("gn_dead_nostart_typeof", function() { local o = gen_never_started_dead(); return ggo(o, function(o) { return typeof o; }); });
t("gn_dead_nostart_stackinfos0", function() { local o = gen_never_started_dead(); return ggo(o, function(o) { return o.getstackinfos(0); }); });
t("seq_wakeup_twice", function() { local o = th_suspended(); local a = null; local b = null; try { a = o.wakeup(1); } catch (e) { a = "E:" + e; } try { b = o.wakeup(2); } catch (e) { b = "E:" + e; } return show(a) + " / " + show(b) + " / " + st(o); });
t("seq_call_after_dead", function() { local o = th_dead_return(); local r = null; try { r = o.call(); } catch (e) { r = "E:" + e; } return show(r) + " / " + st(o); });
t("seq_wakeup_then_call", function() { local o = th_suspended(); try { o.wakeup(); } catch (e) {} local r = null; try { r = o.call(); } catch (e) { r = "E:" + e; } return show(r) + " / " + st(o); });
t("seq_call_twice", function() { local o = th_suspended_val(); local a = null; local b = null; try { a = o.call(); } catch (e) { a = nerr("" + e); } local mid = st(o); try { b = o.call(); } catch (e) { b = nerr("" + e); } return show(a) + " / mid=" + mid + " / " + show(b) + " / " + st(o); });
t("seq_wakeupthrow_then_status", function() { local o = th_suspended(); local r = null; try { r = o.wakeupthrow("x"); } catch (e) { r = "E:" + e; } return show(r) + " / " + st(o); });
t("seq_suspend_value_seen", function() { local o = th_suspended_val(); local r = null; try { r = o.wakeup(42); } catch (e) { r = "E:" + e; } return show(r) + " / " + st(o); });
t("seq_gen_resume_twice", function() { local g = gen_new(); local a = null; local b = null; try { a = resume g; } catch (e) { a = "E:" + e; } local mid = gst(g); try { b = resume g; } catch (e) { b = "E:" + e; } return show(a) + " / mid=" + mid + " / " + show(b) + " / " + gst(g); });
t("seq_gen_resume_dead", function() { local g = gen_dead(); local r = null; try { r = resume g; } catch (e) { r = "E:" + e; } return show(r) + " / " + gst(g); });
t("seq_wakeup_stackinfos", function() { local o = th_suspended(); local si = null; try { si = o.getstackinfos(0); } catch (e) { si = "E:" + e; } local r = null; try { r = o.wakeup(); } catch (e) { r = "E:" + e; } return show(si) + " / " + show(r) + " / " + st(o); });
t("seq_thread_in_thread", function() { local inner = newthread(function() { suspend("i"); return "iv"; }); local outer = newthread(function() { return inner.call(); }); local r = null; try { r = outer.call(); } catch (e) { r = "E:" + e; } return show(r) + " / " + st(outer) + " / " + st(inner); });
print("THREADMAP-DONE\n");

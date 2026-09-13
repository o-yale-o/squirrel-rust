// 自动生成：python tools/gen_genmatrix.py —— 请勿手改
// 生成器 / 线程语义组合矩阵

function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

// 三 yield + return 的生成器工厂
function mk3() { return function() { yield 1; yield 2; yield 3; return 99; } }
function mk0() { return function() { return 7; } }
function yieldv(v) { local h = function() { yield v; }; return h(); }

// ---- 1. resume 次数 × getstatus（生成器状态机）
function resume_seq(n) {
    local g = mk3()();
    local out = "";
    for (local i = 0; i < n; i++) {
        if (i > 0) out += ",";
        try { out += resume g; } catch (e) { out += "E:" + e; }
        out += "/" + g.getstatus();
    }
    return out;
}
s("resume1", function() { return resume_seq(1); });
s("resume2", function() { return resume_seq(2); });
s("resume3", function() { return resume_seq(3); });
s("resume4", function() { return resume_seq(4); });
s("resume5", function() { return resume_seq(5); });

// 返回值的生成器（无 yield）：resume 一次即死
s("resume_ret_only", function() { local g = mk0()(); return resume g + "/" + g.getstatus(); });
s("resume_ret_twice", function() {
    local g = mk0()(); resume g;
    try { return "" + resume g; } catch (e) { return "E:" + e; }
});

// ---- 2. resume 作用于非生成器：类型 × 形态
s("resume_non_null", function() {
    local v = null;
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_int", function() {
    local v = 5;
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_float", function() {
    local v = 1.5;
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_str", function() {
    local v = "a";
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_bool", function() {
    local v = true;
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_table", function() {
    local v = {};
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_array", function() {
    local v = [1];
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_func", function() {
    local v = function(){};
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_class", function() {
    local v = class {};
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_inst", function() {
    local C = class {};
    local v = C();
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_thread", function() {
    local v = newthread(function(){});
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_weakref", function() {
    local t = {};
    local v = t.weakref();
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});
s("resume_non_gen_itself", function() {
    local v = mk0()();
    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }
});

// ---- 3. yield 值形态 → resume 返回值
s("yield_int", function() { local g = yieldv(1); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });
s("yield_float", function() { local g = yieldv(2.5); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });
s("yield_str", function() { local g = yieldv("s"); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });
s("yield_bool", function() { local g = yieldv(false); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });
s("yield_null", function() { local g = yieldv(null); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });
s("yield_table", function() { local g = yieldv({a=1}); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });
s("yield_array", function() { local g = yieldv([1,2]); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });
s("yield_func", function() { local g = yieldv(function(){}); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });
s("yield_class", function() { local g = yieldv(class {}); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });
s("yield_nogen", function() { local g = yieldv(mk3()()); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });
s("yield_thread", function() { local g = yieldv(newthread(function(){})); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });

// ---- 4. suspend 值形态 → wakeup 返回值（线程侧同构）
s("suspend_noval", function() {
    local t = newthread(function() { local v = suspend(); return "w:" + typeof v; });
    local r = t.wakeup();
    return typeof r + "/" + t.getstatus();
});
s("suspend_int", function() {
    local t = newthread(function() { local v = suspend(1); return "w:" + typeof v; });
    local r = t.wakeup();
    return typeof r + "/" + t.getstatus();
});
s("suspend_float", function() {
    local t = newthread(function() { local v = suspend(2.5); return "w:" + typeof v; });
    local r = t.wakeup();
    return typeof r + "/" + t.getstatus();
});
s("suspend_str", function() {
    local t = newthread(function() { local v = suspend("s"); return "w:" + typeof v; });
    local r = t.wakeup();
    return typeof r + "/" + t.getstatus();
});
s("suspend_null", function() {
    local t = newthread(function() { local v = suspend(null); return "w:" + typeof v; });
    local r = t.wakeup();
    return typeof r + "/" + t.getstatus();
});
s("suspend_table", function() {
    local t = newthread(function() { local v = suspend({a=1}); return "w:" + typeof v; });
    local r = t.wakeup();
    return typeof r + "/" + t.getstatus();
});
s("suspend_array", function() {
    local t = newthread(function() { local v = suspend([1,2]); return "w:" + typeof v; });
    local r = t.wakeup();
    return typeof r + "/" + t.getstatus();
});
s("suspend_func", function() {
    local t = newthread(function() { local v = suspend(function(){}); return "w:" + typeof v; });
    local r = t.wakeup();
    return typeof r + "/" + t.getstatus();
});

// ---- 5. 线程状态 × 操作（fresh / suspended / finished / error-finished）
function mk_thread(kind) {
    if (kind == "plain") return newthread(function() { return 1; });
    if (kind == "susp") return newthread(function() { local v = suspend("s1"); return "got:" + v; });
    if (kind == "throw") return newthread(function() { throw "boom"; });
    return newthread(function() { suspend(1); throw "late"; });
}
function state_prep(kind) {
    local t = mk_thread(kind);
    if (kind == "done") { try { t.wakeup(); } catch (e) {} }
    if (kind == "err") { try { t.wakeup(); } catch (e) {} }
    return t;
}
s("th_plain_getstatus", function() { local t = state_prep("plain"); try { return "ok:" + t.getstatus(); } catch (e) { return "E:" + e; } });
s("th_plain_call", function() { local t = state_prep("plain"); try { return "ok:" + t.call(); } catch (e) { return "E:" + e; } });
s("th_plain_wakeup", function() { local t = state_prep("plain"); try { return "ok:" + t.wakeup(); } catch (e) { return "E:" + e; } });
s("th_plain_wakeup_val", function() { local t = state_prep("plain"); try { return "ok:" + t.wakeup("x"); } catch (e) { return "E:" + e; } });
s("th_plain_getstackinfos0", function() { local t = state_prep("plain"); try { return "ok:" + typeof t.getstackinfos(0); } catch (e) { return "E:" + e; } });
s("th_susp_getstatus", function() { local t = state_prep("susp"); try { return "ok:" + t.getstatus(); } catch (e) { return "E:" + e; } });
s("th_susp_call", function() { local t = state_prep("susp"); try { return "ok:" + t.call(); } catch (e) { return "E:" + e; } });
s("th_susp_wakeup", function() { local t = state_prep("susp"); try { return "ok:" + t.wakeup(); } catch (e) { return "E:" + e; } });
s("th_susp_wakeup_val", function() { local t = state_prep("susp"); try { return "ok:" + t.wakeup("x"); } catch (e) { return "E:" + e; } });
s("th_susp_getstackinfos0", function() { local t = state_prep("susp"); try { return "ok:" + typeof t.getstackinfos(0); } catch (e) { return "E:" + e; } });
s("th_done_getstatus", function() { local t = state_prep("done"); try { return "ok:" + t.getstatus(); } catch (e) { return "E:" + e; } });
s("th_done_call", function() { local t = state_prep("done"); try { return "ok:" + t.call(); } catch (e) { return "E:" + e; } });
s("th_done_wakeup", function() { local t = state_prep("done"); try { return "ok:" + t.wakeup(); } catch (e) { return "E:" + e; } });
s("th_done_wakeup_val", function() { local t = state_prep("done"); try { return "ok:" + t.wakeup("x"); } catch (e) { return "E:" + e; } });
s("th_done_getstackinfos0", function() { local t = state_prep("done"); try { return "ok:" + typeof t.getstackinfos(0); } catch (e) { return "E:" + e; } });
s("th_err_getstatus", function() { local t = state_prep("err"); try { return "ok:" + t.getstatus(); } catch (e) { return "E:" + e; } });
s("th_err_call", function() { local t = state_prep("err"); try { return "ok:" + t.call(); } catch (e) { return "E:" + e; } });
s("th_err_wakeup", function() { local t = state_prep("err"); try { return "ok:" + t.wakeup(); } catch (e) { return "E:" + e; } });
s("th_err_wakeup_val", function() { local t = state_prep("err"); try { return "ok:" + t.wakeup("x"); } catch (e) { return "E:" + e; } });
s("th_err_getstackinfos0", function() { local t = state_prep("err"); try { return "ok:" + typeof t.getstackinfos(0); } catch (e) { return "E:" + e; } });

s("th_suspended_getstatus", function() { local t = newthread(function() { local v = suspend(1); return "g:" + v; }); t.call(); try { return "ok:" + t.getstatus() + "/" + t.getstatus(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("th_suspended_wakeup", function() { local t = newthread(function() { local v = suspend(1); return "g:" + v; }); t.call(); try { return "ok:" + t.wakeup() + "/" + t.getstatus(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("th_suspended_wakeup_val", function() { local t = newthread(function() { local v = suspend(1); return "g:" + v; }); t.call(); try { return "ok:" + t.wakeup("x") + "/" + t.getstatus(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("th_suspended_getstackinfos0", function() { local t = newthread(function() { local v = suspend(1); return "g:" + v; }); t.call(); try { return "ok:" + typeof t.getstackinfos(0) + "/" + t.getstatus(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });

s("susp_wakeup_plain", function() {
    local t = newthread(function() { local v = suspend("first"); return "got:" + v; });
    local a = t.call();
    return "" + a + "/" + t.wakeup() + "/" + t.getstatus();
});
s("susp_wakeup_val", function() {
    local t = newthread(function() { local v = suspend("first"); return "got:" + v; });
    t.call();
    return "" + t.wakeup("x") + "/" + t.getstatus();
});
s("susp_wakeup_null", function() {
    local t = newthread(function() { local v = suspend("first"); return "got:" + v; });
    t.call();
    return "" + t.wakeup(null) + "/" + t.getstatus();
});
s("susp_wakeup_table", function() {
    local t = newthread(function() { local v = suspend("first"); return typeof v; });
    t.call();
    return "" + t.wakeup({a=1}) + "/" + t.getstatus();
});
s("susp_suspend_noarg", function() {
    local t = newthread(function() { local v = suspend(); return typeof v; });
    t.call();
    return "" + t.wakeup(5);
});
s("susp_chain_wakeup", function() {
    local t = newthread(function() { local a = suspend(1); local b = suspend(2); return "" + a + b; });
    local out = "" + t.call() + "/" + t.getstatus();
    out += "|" + t.wakeup("A") + "/" + t.getstatus();
    out += "|" + t.wakeup("B") + "/" + t.getstatus();
    return out;
});
s("susp_from_other_thread", function() {
    local t = newthread(function() { local v = suspend(1); return v; });
    t.call();
    local u = newthread(function() { return t.wakeup("w"); });
    return u.call();
});
s("susp_status_names", function() {
    local t = newthread(function() { local v = suspend(1); return "g:" + v; });
    local out = t.getstatus();
    t.call();
    out += "/" + t.getstatus();
    t.wakeup(1);
    out += "/" + t.getstatus();
    return out;
});

// ---- 6. 线程与生成器的参数、返回值边界
function va(...) { return vargv.len(); }
s("th_noargs", function() { return newthread(function() { return 1; }).call(1, 2, 3); });
s("th_missing_args", function() { local t = newthread(function(a, b) { return typeof a + "/" + typeof b; }); return t.call(1); });
s("th_varargs", function() { return newthread(va).call(1, 2, 3, 4); });
s("th_varargs_zero", function() { return newthread(va).call(); });
s("gen_noargs", function() { local h = function() { return 5; }; return resume h(1, 2); });
s("gen_missing_args", function() { local h = function(a, b) { yield typeof a + "/" + typeof b; }; local g = h(1); return resume g; });
s("gen_varargs", function() { local h = function(...) { yield vargv.len(); }; return resume h(1, 2, 3); });
s("gen_default", function() { local h = function(a, b = 5) { yield a + b; }; return resume h(1); });
s("gen_default_both", function() { local h = function(a, b = 5) { yield a + b; }; return resume h(1, 2); });
s("th_ret_gen", function() { local t = newthread(function() { return mk3()(); }); local g = t.call(); return typeof g + "/" + resume g + "/" + g.getstatus(); });

// ---- 7. 错误路径：生成器/线程内的异常如何传播
s("gen_throw_first", function() { local g = (function() { throw "e1"; yield 1; })(); try { return "" + resume g; } catch (e) { return "E:" + e + "/" + g.getstatus(); } });
s("gen_throw_mid", function() { local g = (function() { yield 1; throw "e2"; })(); resume g; try { return "" + resume g; } catch (e) { return "E:" + e + "/" + g.getstatus(); } });
s("gen_throw_after_runtime_err", function() { local g = (function() { yield 1; return "x".nosuch(); })(); resume g; try { return "" + resume g; } catch (e) { return "E:" + e + "/" + g.getstatus(); } });
s("th_throw_call", function() { local t = newthread(function() { throw "boom"; }); try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("th_throw_wakeup", function() { local t = newthread(function() { suspend(1); throw "boom2"; }); t.wakeup(); try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("th_throw_second_call", function() { local t = newthread(function() { throw "boom3"; }); try { t.call(); } catch (e) {} try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("gen_throw_inside_thread", function() { local t = newthread(function() { local g = (function() { yield 1; throw "in"; })(); resume g; resume g; return "no"; }); try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("gen_yield_in_thread", function() { local t = newthread(function() { local g = (function() { yield "y"; return "r"; })(); return "a" + resume g; }); return t.call(); });

// ---- 8. 跨 VM 的 outer 读写（闭包捕获的局部变量被别的线程读/改）
s("xvm_read", function() {
    local v = "host";
    local t = newthread(function() { return v; });
    return t.call();
});
s("xvm_write", function() {
    local v = 0;
    local t = newthread(function() { v = 7; return 0; });
    t.call();
    return v;
});
s("xvm_param", function() {
    local h = function(p) { return newthread(function() { return p * 2; }); };
    return h(21).call();
});
s("xvm_nested_read", function() {
    local a = "A";
    local h = function() { return function() { return a; }; };
    local t = newthread(h());
    return t.call();
});
s("xvm_after_frame", function() {
    local inner = function() { local q = "Q"; return function() { return q; }; };
    local f = inner();
    local t = newthread(f);
    return t.call();
});
s("xvm_gen_in_thread", function() {
    local v = 5;
    local t = newthread(function() { v = v + 1; return v; });
    t.call();
    return v;
});
s("xvm_both_ways", function() {
    local v = 1;
    local t = newthread(function() { v = v * 10; return v; });
    local r = t.call();
    v = v + 1;
    return r + "/" + v;
});
s("xvm_thread_closure_into_thread", function() {
    local t1 = newthread(function() { local z = 3; return newthread(function() { z = z + 4; return z; }); });
    local t2 = t1.call();
    return t2.call() + "/" + t2.getstatus();
});
s("xvm_suspend_keeps_outer", function() {
    local v = "init";
    local t = newthread(function() { suspend(1); return v; });
    t.wakeup();
    v = "changed";
    return t.call();
});

// ---- 9. 生成器/线程作为值：存表、存数组、类成员、再嵌套
s("gen_in_table", function() { local t = { g = mk3()() }; return resume t.g + "/" + resume t.g; });
s("gen_in_array", function() { local a = [mk3()()]; return resume a[0] + "/" + resume a[0]; });
s("gen_in_class", function() { local C = class { g = null; constructor() { g = mk3()(); } }; local c = C(); return resume c.g + "/" + resume c.g; });
s("gen_reentrant", function() { local inner = mk3()(); local outer = (function() { yield resume inner; yield resume inner; })(); return resume outer + "/" + resume outer + "/" + inner.getstatus(); });
s("gen_yield_gen", function() { local inner = mk3()(); local outer = yieldv(inner); local got = resume outer; return typeof got + "/" + resume got; });
s("th_in_table", function() { local t = { th = newthread(function() { return 3; }) }; return t.th.call(); });
s("th_in_array", function() { local a = [newthread(function() { return 4; })]; return a[0].call(); });
s("th_newthread_inside", function() { local t = newthread(function() { local u = newthread(function() { return "u"; }); return "t:" + u.call(); }); return t.call(); });
s("gen_foreach", function() { local out = ""; foreach (v in mk3()()) out += v + ","; return out; });
s("gen_foreach_break", function() { local out = ""; foreach (v in mk3()()) { out += v; if (v == 2) break; } return out; });
s("gen_foreach_partial", function() { local g = mk3()(); local out = ""; foreach (v in g) { out += v; if (v == 1) break; } return out + "/" + g.getstatus(); });
s("gen_while_resume", function() { local g = mk3()(); local out = ""; local n = 0; while (g.getstatus() != "dead" && n < 6) { out += resume g; n++; } return out + "/" + n; });
s("gen_locals_kept", function() { local g = (function() { local a = 1; yield a; a = a + 1; yield a; return a; })(); return resume g + "/" + resume g + "/" + resume g; });
s("gen_this", function() { local C = class { v = 5; function f() { yield this.v; } }; local c = C(); return resume c.f(); });
s("gen_callee", function() { local h = function() { yield typeof callee; }; return resume h(); });
s("th_this", function() { local C = class { v = 6; function f() { return this.v; } }; local c = C(); return newthread(c.f).call(); });
s("th_typeof", function() { local t = newthread(function() { return 1; }); return typeof t + "/" + t.getstatus(); });
s("th_getstatus_after_suspend", function() { local t = newthread(function() { suspend("x"); suspend("y"); return 0; }); t.wakeup(); return t.getstatus() + "/" + t.wakeup() + "/" + t.getstatus(); });
s("shared_table_across", function() { local tbl = {}; local t = newthread(function() { tbl.k <- 1; return 0; }); t.call(); return "k" in tbl; });
s("th_many", function() { local sum = 0; for (local i = 0; i < 5; i++) { local t = newthread(function() { return 1; }); sum += t.call(); } return sum; });
s("gen_many", function() { local sum = 0; for (local i = 0; i < 5; i++) { local g = (function() { yield 2; })(); sum += resume g; } return sum; });

// ---- 10. 线程栈状态：失败/正常结束后再次 call、wakeup 的栈纪律
// （↔ sq_call 的 `if(!_suspended) Pop(params)` 与 thread_call 的 move+pop：
//   正常返回要把 roottable/实参弹干净，挂起则保留 suspend 现场）
s("stack_call_err_twice", function() {
    local t = newthread(function() { throw "e1"; });
    local out = "";
    for (local i = 0; i < 3; i++) {
        try { out += t.call(); } catch (e) { out += "E:" + e; }
        out += "/" + t.getstatus() + ";";
    }
    return out;
});
s("stack_call_err_args_twice", function() {
    local t = newthread(function(a, b) { throw "a" + a + b; });
    local out = "";
    for (local i = 0; i < 2; i++) {
        try { out += t.call(1, 2); } catch (e) { out += "E:" + e; }
        out += ";";
    }
    return out;
});
s("stack_call_ok_args_twice", function() {
    local t = newthread(function(a) { return "r" + a; });
    return "" + t.call(1) + "/" + t.call(2) + "/" + t.getstatus();
});
s("stack_call_ok_varargs_twice", function() {
    local t = newthread(function(...) { return vargv.len(); });
    return "" + t.call(1, 2, 3) + "/" + t.call() + "/" + t.call(9);
});
s("stack_wakeup_then_throw", function() {
    local t = newthread(function() { suspend(1); throw "e2"; });
    local a = t.call();
    local out = "" + a + "/" + t.getstatus();
    try { out += "/" + t.wakeup(); } catch (e) { out += "/E:" + e; }
    out += "/" + t.getstatus();
    try { out += "/" + t.call(); } catch (e) { out += "/E:" + e; }
    return out;
});
s("stack_call_then_throw_via_wakeup", function() {
    local t = newthread(function() { suspend(1); throw "e3"; });
    local out = "" + t.call();
    try { out += "/" + t.wakeup(); } catch (e) { out += "/E:" + e; }
    try { out += "/" + t.wakeup(); } catch (e) { out += "/E:" + e; }
    return out;
});
s("stack_wakeup_after_normal_death", function() {
    local t = newthread(function() { suspend(1); return "end"; });
    local out = "" + t.call() + "/" + t.getstatus();
    try { out += "/" + t.wakeup(); } catch (e) { out += "/E:" + e; }
    return out;
});
s("stack_suspended_then_error", function() {
    local t = newthread(function() { local v = suspend("s"); if (v == "bad") throw "ok-bad"; return "v:" + v; });
    t.call();
    try { return "" + t.wakeup("bad"); } catch (e) { return "E:" + e + "/" + t.getstatus(); }
});
s("stack_gen_from_dead_thread", function() {
    local t = newthread(function() { local g = (function() { yield 1; yield 2; })(); resume g; return g; });
    local g = t.call();
    return resume g + "/" + resume g + "/" + g.getstatus();
});
s("stack_call_after_suspend_value", function() {
    local t = newthread(function() { local v = suspend("first"); return "got:" + v; });
    local a = t.call();
    local c = t.wakeup();
    return "" + a + "/" + c + "/" + t.getstatus();
});

print("GENMATRIX-DONE\n");

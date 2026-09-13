// 广度语料 3：线程 / 生成器 / weakref+GC / 调试钩子 / 根表与常量表 / 数值格式
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

// ---- 线程 ----
t("th_new_getstatus", function() { return newthread(function() {}).getstatus(); });
t("th_call_ret", function() { local th = newthread(function() { return 42; }); th.call(); return th.getstatus(); });
t("th_call_args", function() { local th = newthread(function(a, b) { return a + b; }); th.call(1, 2); return th.getstatus(); });
t("th_suspend_wakeup", function() {
    local th = newthread(function() { local v = suspend("first"); return "got:" + v; });
    th.call();
    local s1 = th.getstatus();
    local r = th.wakeup("second");
    return s1 + "/" + r + "/" + th.getstatus();
});
t("th_wakeup_idle", function() { local th = newthread(function() {}); return th.wakeup(); });
t("th_wakeup_running", function() { local th = newthread(function() { return 1; }); th.call(); return th.wakeup(); });
t("th_wakeupthrow", function() {
    local th = newthread(function() { try { suspend(1); } catch (e) { return "caught:" + e; } return "no"; });
    th.call();
    local r = th.wakeupthrow("boom");
    return r;
});
t("th_getstackinfos", function() {
    local th = newthread(function() { local z = 5; suspend(1); return z; });
    th.call();
    local i = th.getstackinfos(0);
    return typeof i + "/" + (i != null);
});
t("th_weakref", function() { local th = newthread(function() {}); return typeof th.weakref(); });
t("th_call_nonfunc", function() { local th = newthread(1); return th.getstatus(); });

// ---- 生成器 ----
t("gen_basic", function() {
    local g = function() { yield 1; yield 2; return 3; }();
    local out = "" + resume g + resume g;
    return out + "/" + g.getstatus();
});
t("gen_return_value", function() {
    local g = function() { yield 1; return "end"; }();
    resume g;
    local last = resume g;
    return "" + last + "/" + g.getstatus();
});
t("gen_foreach", function() {
    local g = function() { for (local i = 0; i < 3; i++) yield i * 2; }();
    local s = 0;
    foreach (v in g) s += v;
    return s;
});
t("gen_nested", function() {
    local outer = function() { local inner = function() { yield "i"; }(); foreach (v in inner) yield v; yield "o"; }();
    local out = "";
    foreach (v in outer) out += v;
    return out;
});
t("gen_dead_resume", function() {
    local g = function() { yield 1; }();
    resume g; resume g;
    return g.getstatus();
});
t("gen_tostring", function() { local g = function() { yield 1; }(); return typeof ("" + g); });
t("gen_closure_outers", function() {
    local x = 7;
    local g = function() { yield x; x = 8; yield x; }();
    return "" + resume g + resume g;
});

// ---- weakref / GC ----
t("weak_ref_alive", function() { local o = {}; local w = o.weakref(); return typeof w.ref(); });
t("weak_ref_dead", function() { local w; { local o = {}; w = o.weakref(); } collectgarbage(); return w.ref(); });
t("weak_ref_of_class", function() { class C {} return typeof C.weakref(); });
t("weak_ref_of_string", function() { return typeof "x".weakref(); });
t("gc_collect", function() { collectgarbage(); return "ok"; });
t("gc_resurrect_type", function() { local r = resurrectunreachable(); return typeof r + "/" + r.len(); });
t("gc_cycle", function() {
    local a = {}; local b = { a = a }; a.b <- b;
    local w = b.weakref();
    a = null; b = null;
    collectgarbage();
    return (w.ref() == null) + "";
});
t("weakref_weakref", function() { local w = ({}).weakref(); return typeof w.weakref(); });

// ---- 调试钩子 / 调试信息 ----
t("enabledebuginfo", function() { enabledebuginfo(true); enabledebuginfo(false); return "ok"; });
t("debughook", function() {
    local events = "";
    setdebughook(function (event, source, line, fn) { events += event; });
    local a = 1 + 1;
    setdebughook(null);
    return events;
});
t("debughook_stackinfos", function() {
    local got = "";
    setdebughook(function (event, source, line, fn) {
        if (got == "") { local i = getstackinfos(1); got = typeof i; }
    });
    local x = 1;
    setdebughook(null);
    return got;
});

// ---- 根表 / 常量表 / 环境 ----
t("roottable_roundtrip", function() { local rt = getroottable(); setroottable(rt); return typeof getroottable(); });
t("consttable", function() { local ct = getconsttable(); setconsttable(ct); return typeof getconsttable(); });
t("const_lookup", function() { const K = 5; return K; });
t("callee", function() { return typeof callee(); });
t("vargv_main", function() { return typeof vargv + ":" + vargv.len(); });
t("vargv_varargs", function() { local f = function(...) { return vargv.len(); }; return f(1, 2, 3); });
t("vargv_default", function() { local f = function(...) { return typeof vargv; }; return f(); });

// ---- 数值格式与运算 ----
t("int_max", function() { return 9223372036854775807; });
t("int_overflow", function() { return 9223372036854775807 + 1; });
t("int_mul_overflow", function() { return 3037000500 * 3037000500; });
t("int_div_zero", function() { return 1 / 0; });
t("int_mod_zero", function() { return 1 % 0; });
t("float_div_zero", function() { return 1.0 / 0.0; });
t("float_zero_div", function() { return 0.0 / 0.0; });
t("float_big", function() { return 1e300; });
t("float_small", function() { return 1e-300; });
t("float_prec", function() { return 1.0 / 3.0; });
t("float_int_mix", function() { return 1 + 0.5; });
t("shift_left", function() { return 1 << 62; });
t("shift_right", function() { return -8 >> 1; });
t("shift_unsigned", function() { return -8 >>> 1; });
t("bit_and", function() { return 0xFF & 0x0F; });
t("bit_or_xor", function() { return (0xF0 | 0x0F) + "" + (0xFF ^ 0x0F); });
t("bitnot", function() { return ~0; });
t("neg_int", function() { return -(-9223372036854775807); });
t("cmp_mixed", function() { return (1 == 1.0) + "" + (1 < 1.5) + (2 > 1.5); });
t("bool_arith", function() { return true + true; });
t("null_cmp", function() { return (null == null) + "" + (null == 0) + (null == false); });
t("string_num_add", function() { return "a" + 1; });
t("num_string_cmp", function() { return "1" == 1; });
t("typeof_all", function() { return typeof 1 + typeof 1.5 + typeof "s" + typeof {} + typeof [] + typeof null + typeof true + typeof print + typeof newthread(function() {}); });

// ---- NaN / Inf 打印（↔ MSVC printf 拼写）----
t("nan_pos", function() { return "" + casti2f(0x7FC00000); });
t("nan_neg", function() { return "" + casti2f(0xFFC00000); });
t("nan_neg_other", function() { return "" + casti2f(0xFF800001); });
t("nan_zero_div", function() { return "" + (0.0 / 0.0); });
t("nan_fmt", function() { return format("%g|%f|%e", 0.0 / 0.0, 0.0 / 0.0, 0.0 / 0.0); });
t("inf_fmt", function() { return format("%g|%f|%e", 1.0 / 0.0, -1.0 / 0.0, 1.0 / 0.0); });
t("nan_bits_all", function() {
    local out = "";
    foreach (b in [0x7FC00000, 0xFFC00000, 0x7F800001, 0xFF800001, 0xFFFFFFFF, 0x7FFFFFFF]) out += "" + casti2f(b) + " ";
    return out;
});

// ---- resurrectunreachable（↔ ResurrectUnreachable）----
t("res_empty", function() { return typeof resurrectunreachable(); });
t("res_ring", function() {
    local a = {};
    local b = { a = a };
    a.b <- b;
    a = null; b = null;
    local r = resurrectunreachable();
    return typeof r + "/" + (r == null ? -1 : r.len());
});
t("res_ring_types", function() {
    local a = {};
    local b = { a = a };
    a.b <- b;
    a = null; b = null;
    local r = resurrectunreachable();
    if (r == null) return "null";
    local out = "";
    foreach (v in r) out += typeof v;
    return out;
});
t("res_after", function() { return typeof resurrectunreachable(); });

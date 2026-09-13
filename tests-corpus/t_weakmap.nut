// 自动生成：python tools/gen_weakmap.py —— 请勿手改
// weakref / GC / 回收语义矩阵（weakprobe 两轮探针的并集）

// weakref / 回收语义探测
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
function mk_tbl() { return { a = 1 }; }
function mk_arr() { return [1, 2]; }
function mk_cls() { return class { x = 1 }; }
function mk_inst() { local C = class { x = 1 }; return C(); }
function mk_clo() { return function() { return 1; }; }
function mk_gen() { return (function() { yield 1; })(); }
function mk_thr() { return newthread(function() { return 1; }); }
function mk_str() { return "hello"; }

// 1) 每种类型能不能取 weakref，typeof 是什么

// ---- 第一轮：类型 × weakref、ref/getweakrefval、容器、环 ----
s("w_tbl", function() { local o = mk_tbl(); local w = o.weakref(); return typeof w; });
s("w_arr", function() { local o = mk_arr(); local w = o.weakref(); return typeof w; });
s("w_cls", function() { local o = mk_cls(); local w = o.weakref(); return typeof w; });
s("w_inst", function() { local o = mk_inst(); local w = o.weakref(); return typeof w; });
s("w_clo", function() { local o = mk_clo(); local w = o.weakref(); return typeof w; });
s("w_gen", function() { local o = mk_gen(); local w = o.weakref(); return typeof w; });
s("w_thr", function() { local o = mk_thr(); local w = o.weakref(); return typeof w; });
s("w_str", function() { local o = mk_str(); local w = o.weakref(); return typeof w; });
s("w_int", function() { local o = 5; local w = o.weakref(); return typeof w; });
s("w_null", function() { local o = null; local w = o.weakref(); return typeof w; });
s("w_bool", function() { local o = true; local w = o.weakref(); return typeof w; });
s("w_userdata", function() { local b = blob(4); local w = b.weakref(); return typeof w; });
s("w_weakref", function() { local o = mk_tbl(); local w = o.weakref(); local w2 = w.weakref(); return typeof w2; });

// 2) ref / getweakrefval 取回对象
s("w_ref_tbl", function() { local o = mk_tbl(); local w = o.weakref(); local r = w.ref(); return typeof r + "/" + (r.a == 1); });
s("w_ref_alive", function() { local o = mk_tbl(); local w = o.weakref(); return typeof w.ref(); });
s("w_ref_dead", function() {
    local w;
    { local o = mk_tbl(); w = o.weakref(); }
    collectgarbage();
    return typeof w.ref();
});
s("w_ref_dead_nogc", function() { local w; { local o = mk_tbl(); w = o.weakref(); } return typeof w.ref(); });
s("w_self_eq", function() { local o = mk_tbl(); local w = o.weakref(); return w.ref() == o; });
s("w_two_refs", function() { local o = mk_tbl(); local a = o.weakref(); local b = o.weakref(); return a.ref() == b.ref(); });
s("w_ref_int", function() { local o = 5; local w = o.weakref(); return typeof w.ref(); });

// 3) weakref 本身的行为
s("w_typeof_str", function() { local o = mk_tbl(); return "" + o.weakref(); });
s("w_tostring", function() { local o = mk_tbl(); try { return o.weakref().tostring(); } catch (e) { return "E:" + e; } });
s("w_in_table", function() { local o = mk_tbl(); local t = { w = o.weakref() }; return typeof t.w + "/" + typeof t.w.ref(); });
s("w_keeps_container", function() { local t = {}; local o = mk_tbl(); t.w <- o.weakref(); o = null; collectgarbage(); return typeof t.w.ref(); });
s("w_getweakrefval_global", function() { local o = mk_tbl(); local w = o.weakref(); return typeof getweakrefval(w) + "/" + (getweakrefval(w) == o); });
s("w_getweakrefval_noref", function() { try { return "" + getweakrefval(5); } catch (e) { return "E:" + e; } });
s("w_getweakrefval_null", function() { try { return "" + getweakrefval(null); } catch (e) { return "E:" + e; } });

// 4) collectgarbage 的返回值与副作用
s("gc_ret_empty", function() { return typeof collectgarbage(); });
s("gc_ret_after_garbage", function() { for (local i = 0; i < 5; i++) { local t = { i = i }; } return typeof collectgarbage(); });
s("gc_twice", function() { collectgarbage(); return typeof collectgarbage(); });
s("gc_with_arg", function() { try { return typeof collectgarbage(1); } catch (e) { return "E:" + e; } });
s("gc_alive_kept", function() { local t = { a = 1 }; collectgarbage(); return t.a; });
s("gc_weakref_cycle", function() {
    local a = {}; local b = {};
    a.b <- b; b.a <- a;               // 环
    local w = a.weakref();
    a = null; b = null;
    collectgarbage();
    return typeof w.ref();
});
s("gc_selfref", function() {
    local a = {}; a.self <- a;
    local w = a.weakref();
    a = null;
    collectgarbage();
    return typeof w.ref();
});
s("gc_nested_cycle", function() {
    local outer = {};
    local inner = { p = outer };
    outer.i <- inner;
    local w = inner.weakref();
    outer = null; inner = null;
    collectgarbage();
    return typeof w.ref();
});

// 5) 弱引用与 string 实习
s("w_string_identity", function() { local a = "ab" + "c"; local b = "abc"; return a == b; });
s("w_string_weakref", function() { local a = "abc"; local w = a.weakref(); return typeof w.ref(); });

// ---- 第二轮：是否保活、collectgarbage 计数、各类对象 ----
function mk() { return { a = 1 }; }

// weakref 是否让对象活下来（C++ 语义：不保活）
s("keep_plain", function() { local w; { local o = mk(); w = o.weakref(); } return typeof w.ref(); });
s("keep_after_gc", function() { local w; { local o = mk(); w = o.weakref(); } collectgarbage(); return typeof w.ref(); });
s("keep_self_weak", function() { local t = {}; t.self <- t.weakref(); local w = t.weakref(); t = null; collectgarbage(); return typeof w.ref(); });
s("keep_in_array", function() { local a = []; local o = mk(); a.append(o.weakref()); o = null; collectgarbage(); return typeof a[0].ref(); });
s("keep_strong_in_array", function() { local a = []; local o = mk(); a.append(o); o = null; collectgarbage(); return typeof a[0].a; });

// collectgarbage 的**计数**（可观察）
s("gc_count_cycle1", function() {
    local a = {}; local b = {}; a.b <- b; b.a <- a; a = null; b = null;
    return collectgarbage();
});
s("gc_count_cycle2", function() {
    local a = {}; local b = {}; a.b <- b; b.a <- a;
    local c = {}; c.self <- c;
    a = null; b = null; c = null;
    return collectgarbage();
});
s("gc_count_plain", function() { local t = mk(); t = null; return collectgarbage(); });
s("gc_count_after_ref", function() { local o = mk(); local w = o.weakref(); collectgarbage(); return typeof w.ref(); });

// 弱引用 + 生成器 / 线程
s("gen_weak_alive", function() { local g = (function() { yield 1; })(); local w = g.weakref(); return typeof w.ref() + "/" + typeof g; });
s("thr_weak_alive", function() { local t = newthread(function() { return 1; }); local w = t.weakref(); return typeof w.ref() + "/" + t.getstatus(); });
s("thr_weak_dead", function() { local w; { local t = newthread(function() { return 1; }); w = t.weakref(); t.call(); } collectgarbage(); return typeof w.ref(); });

// 实例 / 类的弱引用与成员
s("inst_weak_member", function() {
    local C = class { v = 7; };
    local o = C(); local w = o.weakref();
    return typeof w.ref() + "/" + w.ref().v;
});
s("cls_weak_member", function() {
    local C = class { v = 7; }; local w = C.weakref();
    return typeof w.ref() + "/" + (w.ref() == C);
});
s("blob_weak", function() { local b = blob(4); local w = b.weakref(); return typeof w.ref(); });
s("regexp_weak", function() { local r = regexp("a"); local w = r.weakref(); return typeof w.ref(); });
s("func_weak", function() { local f = function() { return 3; }; local w = f.weakref(); return typeof w.ref() + "/" + w.ref()(); });
s("natfunc_weak", function() { local w = print.weakref(); return typeof w; });
s("str_weak_alive", function() { local s = "abc"; local w = s.weakref(); return typeof w.ref() + "/" + (w.ref() == s); });
s("str_weak_dead", function() { local w; { local s = "abcdef" + "gh"; w = s.weakref(); } collectgarbage(); return typeof w.ref(); });

// 弱引用的比较与复制
s("w_eq_w", function() { local o = mk(); local a = o.weakref(); local b = a; return a == b; });
s("w_neq_w", function() { local o = mk(); local p = mk(); return o.weakref() == p.weakref(); });
s("w_eq_ref", function() { local o = mk(); local w = o.weakref(); return w == o; });
s("w_clone", function() { local o = mk(); local w = o.weakref(); return typeof clone w; });
s("w_in_weak", function() { local o = mk(); local w = o.weakref(); return "ref" in w; });
s("w_foreach_delegate", function() {
    local o = mk(); local w = o.weakref(); local out = "";
    local d = w.getdelegate();
    foreach (k, v in d) out += k + ",";
    return out;
});

print("WEAKMAP-DONE\n");

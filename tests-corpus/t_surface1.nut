// 广度语料 1：类/实例/表/数组/字符串的成员与边界
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

// ---- 类特性 ----
class Base {
    static s = "S";
    field = 1;
    constructor(v = 10) { field = v; }
    function get() { return field; }
    static function smake() { return "static"; }
}
class Derived extends Base {
    extra = 2;
    constructor(v = 20) { base.constructor(v); }
    function get() { return base.get() + extra; }
}
t("static_field", function() { return Base.s + "/" + Derived.s; });
t("static_method", function() { return Base.smake(); });
t("inst_field", function() { return Derived().get(); });
t("ctor_default", function() { return Base().get(); });
t("instanceof", function() { local d = Derived(); return (d instanceof Base) + "" + (d instanceof Derived) + (Base() instanceof Derived); });
t("getbase", function() { return typeof Derived.getbase(); });
t("class_instance", function() { return typeof Base.instance(); });
t("newmember", function() { local c = class {}; c.newmember("m", 7); return c.m; });
t("rawnewmember", function() { local c = class {}; c.rawnewmember("m", 8); return c.rawget("m"); });
t("class_attrs", function() { local c = class {}; c.setattributes({ a = 1 }); return c.getattributes().a; });
t("member_attrs", function() { class C { x = 1; } return typeof C.getattributes(); });
t("rawget_class", function() { local c = class { v = 5 }; return c.rawget("v"); });
t("class_rawin", function() { class C { x = 1; } return "x" in C; });
t("inst_rawget", function() { class C { x = 3; } return C().rawget("x"); });
t("inst_rawin", function() { class C { x = 3; } return "x" in C(); });
t("inst_getclass", function() { class C {} return typeof C().getclass(); });
t("clone_instance", function() { class C { x = 1; } local c = C(); local d = clone c; d.x = 9; return c.x + "/" + d.x; });
t("clone_class", function() { class C { x = 1; } local D = clone C; return typeof D; });
t("attrs_missing", function() { class C {} return typeof C.getattributes(); });

// ---- 元方法全表 ----
class MM {
    v = 0;
    constructor(x) { v = x; }
    function _add(o) { return MM(v + o.v); }
    function _sub(o) { return MM(v - o.v); }
    function _mul(o) { return MM(v * o.v); }
    function _div(o) { return MM(v / o.v); }
    function _modulo(o) { return MM(v % o.v); }
    function _unm() { return MM(-v); }
    function _typeof() { return "MM"; }
    function _cmp(o) { return v <=> o.v; }
    function _tostring() { return "MM" + v; }
    function _get(k) { return "get:" + k; }
    function _set(k, val) { return "set:" + k + "=" + val; }
    function _call(a) { return "call:" + a; }
    function _cloned(o) { return "cloned"; }
    function _nexti(prev) { return null; }
}
t("mm_add", function() { return (MM(1) + MM(2)).v; });
t("mm_sub", function() { return (MM(5) - MM(2)).v; });
t("mm_mul", function() { return (MM(3) * MM(4)).v; });
t("mm_div", function() { return (MM(8) / MM(2)).v; });
t("mm_mod", function() { return (MM(7) % MM(4)).v; });
t("mm_unm", function() { return (-MM(5)).v; });
t("mm_typeof", function() { return typeof MM(1); });
t("mm_cmp", function() { return (MM(1) < MM(2)) + "" + (MM(2) < MM(1)); });
t("mm_tostring", function() { return "" + MM(3); });
t("mm_get", function() { return MM(1).whatever; });
t("mm_call", function() { return MM(1)(5); });
t("mm_clone", function() { local m = MM(1); local c = clone m; return typeof c; });
t("mm_len", function() { return MM(1).len(); });

// ---- 数组边界 ----
t("arr_slice_neg", function() { local a = [1,2,3,4,5]; return a.slice(-2).len(); });
t("arr_slice_rev", function() { local a = [1,2,3]; return a.slice(0, -1).len(); });
t("arr_insert_neg", function() { local a = [1,2,3]; a.insert(-1, 9); return a[2]; });
t("arr_remove_neg", function() { local a = [1,2,3]; a.remove(-1); return a.len(); });
t("arr_find", function() { return [1,2,3].find(2); });
t("arr_find_missing", function() { return [1,2,3].find(9); });
t("arr_reduce_noinit", function() { return [1,2,3,4].reduce(function(a,b){ return a+b; }); });
t("arr_sort_cmp", function() { local a = [3,1,2]; a.sort(function(x,y){ return y <=> x; }); return "" + a[0] + a[1] + a[2]; });
t("arr_apply", function() { local s = 0; [1,2,3].apply(function(i,v){ s += v; }); return s; });
t("arr_extend", function() { local a = [1]; a.extend([2,3]); return a.len(); });
t("arr_insert_oob", function() { local a = [1]; a.insert(5, 9); return a.len(); });
t("arr_resize_grow", function() { local a = [1]; a.resize(3); return "" + a.len() + typeof a[2]; });
t("arr_top_empty", function() { local a = []; return a.top(); });
t("arr_pop_empty", function() { local a = []; return a.pop(); });
t("arr_len", function() { return [].len(); });

// ---- 表边界 ----
t("tbl_rawexists", function() { local o = { a = 1 }; return o.rawin("a") + "" + o.rawin("b"); });
t("tbl_rawdelete", function() { local o = { a = 1 }; o.rawdelete("a"); return o.len(); });
t("tbl_delete", function() { local o = { a = 1, b = 2 }; delete o.a; return o.len(); });
t("tbl_keys_values", function() { local o = { a = 1 }; return o.keys().len() + o.values().len(); });
t("tbl_filter", function() { local o = { a = 1, b = 2 }; local r = o.filter(function(k,v){ return v > 1; }); return r.len(); });
t("tbl_map", function() { local o = { a = 1 }; local r = o.map(function(k,v){ return v * 2; }); return r.a; });
t("tbl_setdelegate", function() { local p = { x = 1 }; local c = {}; c.setdelegate(p); return c.x; });
t("tbl_delegate_cycle", function() { local o = {}; o.setdelegate(o); return "ok"; });
t("tbl_nested_newslot", function() { local o = {}; o.a <- { b <- 2 }; return o.a.b; });
t("tbl_len_after_set", function() { local o = {}; o.a <- 1; o.a = 2; return o.len(); });

// ---- 字符串边界 ----
t("str_slice_neg", function() { return "hello".slice(-3); });
t("str_find_start", function() { return "abcabc".find("abc", 1); });
t("str_find_missing", function() { return "abc".find("z"); });
t("str_tointeger_radix", function() { return "ff".tointeger(16); });
t("str_tointeger_bad", function() { return "zz".tointeger(); });
t("str_tofloat_bad", function() { return "zz".tofloat(); });
t("str_index", function() { return "abc"[1]; });
t("str_index_neg", function() { return "abc"[-1]; });
t("str_empty_len", function() { return "".len(); });
t("str_escape_roundtrip", function() { return escape("a\\b\"c"); });
t("str_toupper_unicode", function() { return "aBc".toupper(); });
t("str_strip_custom", function() { return strip("xxaxx", "x"); });
t("str_format_star", function() { return format("%*d", 5, 1); });
t("str_format_pct", function() { return format("100%%"); });
t("num_tochar", function() { return (65).tochar(); });
t("num_tochar_zero", function() { return (0).tochar(); });

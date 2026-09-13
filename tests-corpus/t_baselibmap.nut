// 自动生成：base 库零覆盖全局函数矩阵（tools/gen_baselibmap.py）
// assert/error/array/type/dummy —— 62 个全局里仅有的 5 个零覆盖项
function show(r) {
    local t = typeof r;
    if (t == "array") {
        local s = "[";
        for (local i = 0; i < r.len(); i++) { if (i > 0) s += ","; s += show(r[i]); }
        return s + "]";
    }
    if (t == "table") { local s = "{"; foreach (k, v in r) s += "" + k + ":" + show(v) + ","; return s + "}"; }
    if (t == "string") return "\"" + r + "\"";
    return "" + r;
}
function t(tag, f) {
    try { print(tag + "=" + show(f()) + "|" + typeof f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
// class 声明只能是语句，不能当表达式（`type(class C {})` 是语法错），所以先声明
class TypeProbeClass {}

t("a_true", function() { assert(true); return "passed"; });
t("a_false", function() { assert(false); return "passed"; });
t("a_null", function() { assert(null); return "passed"; });
t("a_i0", function() { assert(0); return "passed"; });
t("a_i1", function() { assert(1); return "passed"; });
t("a_f0", function() { assert(0.0); return "passed"; });
t("a_f1", function() { assert(1.5); return "passed"; });
t("a_es", function() { assert(""); return "passed"; });
t("a_s", function() { assert("x"); return "passed"; });
t("a_arr_e", function() { assert([]); return "passed"; });
t("a_arr_1", function() { assert([0]); return "passed"; });
t("a_tbl_e", function() { assert({}); return "passed"; });
t("a_fn", function() { assert(function() {}); return "passed"; });
t("a_blob", function() { assert(blob(0)); return "passed"; });
t("a_msg_s", function() { assert(false, "boom"); return "passed"; });
t("a_msg_i", function() { assert(false, 123); return "passed"; });
t("a_msg_f", function() { assert(false, 1.5); return "passed"; });
t("a_msg_null", function() { assert(false, null); return "passed"; });
t("a_msg_true", function() { assert(false, true); return "passed"; });
t("a_msg_arr", function() { assert(false, [1, 2]); return "passed"; });
t("a_msg_tbl", function() { assert(false, { a = 1 }); return "passed"; });
t("a_msg_fn", function() { assert(false, function() {}); return "passed"; });
t("a_msgs_i", function() { assert(0, 123); return "passed"; });
t("a_msg_undef", function() { assert(false, null); return "passed"; });
t("a_noargs", function() { assert(); return "passed"; });
t("a_manyargs", function() { assert(true, "a", "b", "c"); return "passed"; });
t("a_msg_tostring", function() { local o = { _tostring = function() { return "custom"; } }; assert(false, o); return "passed"; });
t("a_ret", function() { return assert(true); });
t("a_ret_msg", function() { return assert(true, "unused"); });
t("e_s", function() { local r = error("boom"); return "" + r; });
t("e_es", function() { local r = error(""); return "" + r; });
t("e_i", function() { local r = error(123); return "" + r; });
t("e_f", function() { local r = error(1.5); return "" + r; });
t("e_null", function() { local r = error(null); return "" + r; });
t("e_true", function() { local r = error(true); return "" + r; });
t("e_arr", function() { local r = error([1, 2]); return "" + r; });
t("e_tbl", function() { local r = error({ a = 1 }); return "" + r; });
t("e_blob", function() { local r = error(blob(2)); return "" + r; });
t("e_fn", function() { local r = error(function() {}); return "" + r; });
t("e_none", function() { return error(); });
t("e_two", function() { return error("a", "b"); });
t("ar_z0", function() { return array(0); });
t("ar_z1", function() { return array(1); });
t("ar_z3", function() { return array(3); });
t("ar_fill_i", function() { return array(3, 7); });
t("ar_fill_s", function() { return array(2, "x"); });
t("ar_fill_null", function() { return array(2, null); });
t("ar_fill_arr", function() { return array(2, [1]); });
t("ar_fill_tbl", function() { return array(1, { a = 1 }); });
t("ar_frac", function() { return array(2.9); });
t("ar_frac_small", function() { return array(0.5); });
t("ar_int_arg", function() { return array(3); });
t("ar_extra", function() { return array(2, 5, 6); });
t("ar_big", function() { return array(1000); });
t("ar_none", function() { return array(); });
t("ar_str", function() { return array("x"); });
t("ar_null", function() { return array(null); });
t("ar_bool", function() { return array(true); });
t("ar_len", function() { return array(3, 7).len(); });
t("ar_type", function() { return typeof array(1); });
t("ar_getclass", function() { return array(1).getclass(); });
t("ty_null", function() { return type(null); });
t("ty_int", function() { return type(1); });
t("ty_float", function() { return type(1.5); });
t("ty_bool", function() { return type(true); });
t("ty_str", function() { return type("x"); });
t("ty_arr", function() { return type([]); });
t("ty_tbl", function() { return type({}); });
t("ty_fn", function() { return type(function() {}); });
t("ty_natfn", function() { return type(print); });
t("ty_blob", function() { return type(blob(1)); });
t("ty_cls", function() { return type(TypeProbeClass); });
t("ty_inst", function() { return type(TypeProbeClass()); });
t("ty_gen", function() { return type((function() { yield 1; }())); });
t("ty_th", function() { return type(newthread(function() {})); });
t("ty_wref", function() { return type(((1).weakref())); });
t("ty_re", function() { return type(regexp("a")); });
t("ty_wref_real", function() { local tt = {}; return type(tt.weakref()); });
t("ty_wref_int", function() { local i = 5; return type(i.weakref()) + "/" + i.weakref(); });
t("ty_weakref_weakref", function() { local tt = {}; local w = tt.weakref(); return type(w.weakref()); });
t("ty_bare_fn", function() { return type(); });
t("ty_extra", function() { return type(1, 2); });
t("ty_vs_typeof", function() { local a = [1], b = {}; return type(a) + "/" + type(b) + "/" + typeof(a) + "/" + typeof(b); });
t("d_none", function() { return dummy(); });
t("d_one", function() { return dummy(1); });
t("d_many", function() { return dummy(1, 2, 3, "x", null); });
t("d_type", function() { return typeof dummy; });
t("d_ret_type", function() { return typeof dummy(1); });
t("d_in_expr", function() { return "" + (dummy(1) == null); });
print("BASELIBMAP-DONE\n");

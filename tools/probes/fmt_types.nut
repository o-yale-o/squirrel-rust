// format 类型接受的边界（每个用例都捕获错误文本）
function s(tag, f) {
    try { print(tag + "=[" + f() + "]\n"); }
    catch (e) { print(tag + "![" + e + "]\n"); }
}
s("f_true", function() { return format("%f", true); });
s("f_false", function() { return format("%f", false); });
s("d_15", function() { return format("%d", 1.5); });
s("d_n15", function() { return format("%d", -1.5); });
s("d_25", function() { return format("%d", 2.5); });
s("x_25", function() { return format("%x", 2.5); });
s("d_bigf", function() { return format("%d", 1.0e30); });
s("d_nbigf", function() { return format("%d", -1.0e30); });
s("d_null", function() { return format("%d", null); });
s("d_str", function() { return format("%d", "5"); });
s("f_int", function() { return format("%f", 5); });
s("f_bigint", function() { return format("%f", 9223372036854775807); });
s("g_int", function() { return format("%g", 5); });
s("e_int", function() { return format("%e", 5); });
s("c_int", function() { return format("%c", 65); });
s("c_float", function() { return format("%c", 65.9); });
s("c_bool", function() { return format("%c", true); });
s("c_str", function() { return format("%c", "A"); });
s("c_null", function() { return format("%c", null); });
s("s_int", function() { return format("%s", 5); });
s("s_bool", function() { return format("%s", true); });
s("s_tbl", function() { return format("%s", {}); });
s("u_negf", function() { return format("%u", -1.5); });
s("o_negf", function() { return format("%o", -1.5); });
s("d_nan", function() { return format("%d", 0.0/0.0); });
s("d_inf", function() { return format("%d", 1.0/0.0); });
s("f_min", function() { return format("%f", -9223372036854775807 - 1); });
s("f_max", function() { return format("%f", 9223372036854775807); });
s("pct_only_arg", function() { return format("%", 1); });
s("pct_flag_arg", function() { return format("%-", 1); });
s("dot_arg", function() { return format("%.", 1); });
s("many_flags", function() { return format("%+++++++++++++++++++++d", 1); });
// 注意：`%` + 20 个 '+' + `d`（整段恰好 20 字符）会让参考实现越界写 fmt[20]，
// 实测 STATUS_STACK_BUFFER_OVERRUN 崩溃（复现见 STATUS.md「残留差异」）。
// 本实现报可捕获的 "format too long"，因此这一条不能放进对拍探针。
s("width99", function() { return format("%99d", 1); });
s("width100", function() { return format("%100d", 1); });
s("w2p2", function() { return format("%99.99f", 1.5); });
s("long_flag", function() { return format("%-+ #0-.3f", 1.5); });

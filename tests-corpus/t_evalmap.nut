// 自动生成：编译期折叠 / switch 值形态 / 短路求值顺序矩阵（tools/gen_evalmatrix.py）
//
// 本轮修复：链式赋值 x = y = z 的 DMOVE 执行顺序（见生成器 docstring 与
// MIGRATION.md）。其余为验证型，全部与参考逐字节一致。
function d(x) {
    local t = typeof x;
    if (t == "table") { local o = "{"; local first = true; foreach (k, v in x) { if (!first) o += ","; first = false; o += k + "=" + v; } return o + "}"; }
    return "" + x;
}
function side(x) { print("S:" + x + ";"); return x; }
function t(tag, f) {
    try { local v = f(); print(tag + "=" + (typeof v == "array" ? "arr" : (typeof v == "string" ? v : "" + v)) + "|" + typeof v + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

t("sw_neg", function() { local x = -3; switch (x) { case -3: return "n3"; case 3: return "p3"; } return "none"; });
t("sw_dup", function() { local x = 2; switch (x) { case 2: return "first"; case 2: return "second"; } return "none"; });
t("sw_min", function() { local x = -9223372036854775808; switch (x) { case -9223372036854775808: return "min"; } return "none"; });
t("sw_max", function() { local x = 9223372036854775807; switch (x) { case 9223372036854775807: return "max"; } return "none"; });
t("sw_float", function() { local x = 1.5; switch (x) { case 1.5: return "f15"; case 1: return "i1"; } return "none"; });
t("sw_float_eq_int", function() { local x = 2.0; switch (x) { case 2: return "i2"; } return "none"; });
t("sw_default_last", function() { local x = 9; switch (x) { case 1: return "one"; default: return "def"; } });
t("sw_default_first", function() { return compilestring("local x = 9; switch (x) { default: return 42; }", "sdf") == null ? "compile-null" : "compiled"; });
t("sw_empty", function() { local x = 1; switch (x) { } return "ok"; });
t("sw_only_default", function() { local x = 1; switch (x) { default: return "d"; } });
t("sw_nested", function() { local x = 2, y = 20; switch (x) { case 2: switch (y) { case 20: return "2/20"; } return "2/?"; } return "none"; });
t("sw_break_in_loop", function() { local out = ""; for (local i = 0; i < 3; i++) { switch (i) { case 1: break; default: out += i; } out += "|"; } return out; });
t("sw_local_in_case", function() { local x = 1; switch (x) { case 1: local v = "lv"; return "got:" + v; } return "none"; });
t("sw_case_expr", function() { local x = 5, y = 5; switch (x) { case y: return "var-case"; } return "none"; });
t("sw_sparse_big", function() { local x = 1000000; switch (x) { case 1: return "1"; case 1000000: return "1m"; case 2: return "2"; } return "none"; });
t("sw_bool", function() { local x = true; switch (x) { case true: return "T"; case false: return "F"; } return "none"; });
t("sw_null", function() { local x = null; switch (x) { case null: return "N"; } return "none"; });
t("sw_return_in_loop", function() { local a = [1, 2, 3]; foreach (v in a) { switch (v) { case 2: return "found2"; } } return "none"; });
t("cf_div0_int", function() { return 1 / 0; });
t("cf_mod0_int", function() { return 7 % 0; });
t("cf_div0_float", function() { return 1.0 / 0.0; });
t("cf_mod0_float", function() { return 1.0 % 0.0; });
t("cf_ovf_add", function() { return 9223372036854775807 + 1; });
t("cf_ovf_mul", function() { return 3037000500 * 3037000500; });
t("cf_shift64", function() { return 1 << 64; });
t("cf_shift_neg", function() { return 1 << -1; });
t("cf_float_add", function() { return 0.1 + 0.2; });
t("cf_mixed", function() { return 1 + 2.5; });
t("cf_unary", function() { return -(-5); });
t("cf_not", function() { return !(1 < 2); });
t("cf_cmp_chain", function() { return 1 < 2 == true; });
t("cf_str_concat", function() { return "a" + "b"; });
t("cf_div_prec", function() { return 1.0 / 3 * 3; });
t("sc_and_vals", function() { return (1 && 2) + "/" + (0 && 2) + "/" + (null && 2) + "/" + ("" && 2); });
t("sc_or_vals", function() { return (1 || 2) + "/" + (0 || 2) + "/" + (null || "x") + "/" + ("" || 2); });
t("sc_and_short", function() { local s = ""; local r = false && side(1); return s + "/" + r; });
t("sc_or_short", function() { local s = ""; local r = true || side(1); return s + "/" + r; });
t("sc_and_calls", function() { local s = ""; local r = side(true) && side(2); return s + "/" + r; });
t("sc_or_calls", function() { local s = ""; local r = side(false) || side(3); return s + "/" + r; });
t("sc_chain", function() { local s = ""; local r = side(1) && side(2) || side(3) && side(4); return s + "/" + r; });
t("sc_mixed_types", function() { return ("" && 5) + "/" + typeof ([] || "arr") + "/" + typeof ({} || "tbl"); });
t("sc_not_not", function() { return !!5 + "/" + !!0 + "/" + !!"x"; });
t("asg_value", function() { local a = 1; local b = (a = 5); return a + "/" + b; });
t("asg_chain", function() { local a = 0; local b = 0; local c = 3; a = b = c; return a + "/" + b; });
t("asg_in_cond", function() { local a = 0, r = ""; if ((a = 7) == 7) r = "yes"; return r + "/" + a; });
t("asg_in_call", function() { local a = 0; local f = function(x) { return x * 2; }; local r = f(a = 4); return r + "/" + a; });
t("tern_nested", function() { local x = 2; return x == 1 ? "one" : x == 2 ? "two" : "many"; });
t("tern_side_once", function() { local n = 0; local f = function() { n++; return true; }; local r = f() ? "a" : "b"; return r + "/" + n; });
t("tern_assign_branch", function() { local a = 0, b = 0; local x = 1; x ? a = 1 : b = 2; return a + "/" + b; });
t("tern_in_args", function() { local f = function(p, q) { return p + ":" + q; }; return f(1 ? "t" : "f", 0 ? "t" : "f"); });
print("EVALMAP-DONE\n");

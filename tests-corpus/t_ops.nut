// 运算符与自增/自减的槽位形态（↔ INC/INCL/PINC/PINCL、NOT、AND/OR 等指令）
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

// 本地变量：INCL / PINCL
t("local_postinc", function() { local i = 1; local r = i++; return "" + r + "/" + i; });
t("local_preinc", function() { local i = 1; local r = ++i; return "" + r + "/" + i; });
t("local_postdec", function() { local i = 2; local r = i--; return "" + r + "/" + i; });
t("local_predec", function() { local i = 2; local r = --i; return "" + r + "/" + i; });

// 非本地（表字段 / 数组元素 / 全局）：INC / PINC
g_counter <- 10;
t("global_postinc", function() { local r = g_counter++; return "" + r + "/" + g_counter; });
t("global_preinc", function() { local r = ++g_counter; return "" + r + "/" + g_counter; });
t("global_postdec", function() { local r = g_counter--; return "" + r + "/" + g_counter; });
t("global_predec", function() { local r = --g_counter; return "" + r + "/" + g_counter; });
t("field_postinc", function() { local o = { v = 1 }; local r = o.v++; return "" + r + "/" + o.v; });
t("field_preinc", function() { local o = { v = 1 }; local r = ++o.v; return "" + r + "/" + o.v; });
t("field_postdec", function() { local o = { v = 3 }; local r = o.v--; return "" + r + "/" + o.v; });
t("field_predec", function() { local o = { v = 3 }; local r = --o.v; return "" + r + "/" + o.v; });
t("index_postinc", function() { local a = [1]; local r = a[0]++; return "" + r + "/" + a[0]; });
t("index_preinc", function() { local a = [1]; local r = ++a[0]; return "" + r + "/" + a[0]; });
t("nested_field_inc", function() { local o = { p = { q = 0 } }; ++o.p.q; return o.p.q; });

// 逻辑非（单目 !）
t("not_true", function() { return !true; });
t("not_false", function() { return !false; });
t("not_int", function() { return !0; });
t("not_null", function() { return !null; });
t("not_expr", function() { local i = 0; local r = !(i++); return "" + r + "/" + i; });
t("not_in_if", function() { if (!false) return "yes"; return "no"; });
t("not_double", function() { return !!5; });
t("not_table", function() { local o = {}; return !o; });

// 逻辑与/或（AND / OR 指令 + 短路）
t("and_literal", function() { local r = true && false; return r; });
t("and_shortcircuit", function() { local n = 0; local f = function() { n++; return true; }; local r = false && f(); return "" + r + "/" + n; });
t("or_shortcircuit", function() { local n = 0; local f = function() { n++; return true; }; local r = true || f(); return "" + r + "/" + n; });
t("and_chain", function() { local a = 1; local b = 2; local c = 3; return a < b && b < c; });
t("or_chain", function() { return false || 0 || "x"; });

// 位运算 / 移位 / 比较 / 复合赋值
t("bitwise_all", function() { return (0xF0 & 0x3C) + ":" + (0xF0 | 0x0F) + ":" + (0xFF ^ 0x0F); });
t("shift_all", function() { return (1 << 10) + ":" + (1024 >> 3) + ":" + ((-8) >>> 1); });
t("bw_not", function() { return ~0; });
t("cmp_all", function() { return (1 == 1) + "" + (1 != 2) + (1 < 2) + (2 <= 2) + (3 > 2) + (3 >= 4); });
t("cmp_threeway", function() { return (1 <=> 2) + ":" + (2 <=> 2) + ":" + (3 <=> 2); });
// 复合赋值只有 += -= *= /= %=（&= |= ^= <<= >>= 不存在，两边都报 expression expected）
t("compound_assign", function() { local x = 10; x += 5; x -= 3; x *= 2; x /= 4; x %= 4; return x; });
t("bitwise_assign_manual", function() { local x = 0xF0; x = x & 0x3C; x = x | 1; x = x ^ 0xFF; x = x << 2; x = x >> 1; return x; });
t("unary_neg", function() { local x = 5; return -x + -(-x); });
t("float_ops", function() { return 1.5 + 2.25 + 0.25 * 4 - 1.0; });
t("int_div_trunc", function() { return 10 / 4; });
t("mod_ops", function() { return 10 % 3; });
t("in_operator", function() { local o = { a = 1 }; return ("a" in o) + "" + ("b" in o); });
t("ternary_chain", function() { local x = 2; return x == 1 ? "a" : x == 2 ? "b" : "c"; });
t("delete_slot", function() { local o = { a = 1, b = 2 }; delete o.a; return o.len(); });
t("exists_operator", function() { local o = {}; return ("x" in o); });
t("typeof_chain", function() { return typeof 1 + typeof "s"; });
t("clone_operator", function() { local a = [1, 2]; local b = clone a; b[0] = 9; return "" + a[0] + b[0]; });
t("instanceof_op", function() { class A {} class B extends A {} local b = B(); return (b instanceof A) + "" + (b instanceof B); });

// 浮点/整数字面量、转义、verbatim 字符串、比较与 typeof 的边界
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
local s1 = "tab\there";
t("len", function() { return s1.len(); });
t("find_tab", function() { return s1.find("\t"); });
t("hex_escape", function() { return "\x41\x42"; });
t("quote_escape", function() { return "quote:\"q\""; });
local v = @"raw\nstring";
t("verbatim_len", function() { return v.len(); });
// 注意：65.tochar() 会被词法成浮点 65. 紧跟标识符 → 两边都是**编译错误**
// （end of statement expected），所以这里只能写 (65).tochar()
t("tochar", function() { return (65).tochar() + (66).tochar(); });
// bool 参与算术在原版是错误（不是当 1/0 用）
t("bool_add", function() { return true + 1; });
t("bool_add_false", function() { return false + 1; });
t("null_eq_null", function() { return null == null; });
t("int_eq_float", function() { return 1 == 1.0; });
t("zero_eq_false", function() { return 0 == false; });
t("empty_eq_null", function() { return "" == null; });
t("cmp_3way", function() { return (1 <=> 2) + "," + (2 <=> 1) + "," + (1 <=> 1); });
t("null_cmp", function() { local x = null; return x == null; });
t("typeof_typeof", function() { return typeof typeof 1; });
t("float_literal_forms", function() { return 1. + 0.5 + 1e2 + 0x10; });
t("int_literal_forms", function() { return 0x1F + 010; }); // 0b/1_000 不是 Squirrel 字面量
t("negative_zero", function() { return "" + (-0.0) + "/" + (-0); });
t("big_literals", function() { return 9223372036854775807; });
t("hex_big", function() { return 0x7FFFFFFFFFFFFFFF; });
t("escape_octal_like", function() { return "\0".len(); });

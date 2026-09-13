// 自动生成：python tools/gen_lexmatrix.py —— 请勿手改
// 词法 / 优先级 / 作用域 组合矩阵

function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

// ---- 1. 字符串转义（ASCII 转义用 escape() 输出；高位字节改用字节转储）
// 注：`escape()` 遇到 ≥0x80 的字节时，参考实现用 `(char)c` 传给 `%02x`，
// 写出 8 位补码（`\xffffffe9`）**并越界写自己的 scratchpad**——实测输出会串行、
// 还混进别的错误消息片段（STATUS 第四节）。所以高位字节只比字节值，不比 escape()。
s("esc_n", function() { local v = "\n"; return v.len() + ":" + escape(v); });
s("esc_t", function() { local v = "\t"; return v.len() + ":" + escape(v); });
s("esc_r", function() { local v = "\r"; return v.len() + ":" + escape(v); });
s("esc_a", function() { local v = "\a"; return v.len() + ":" + escape(v); });
s("esc_b", function() { local v = "\b"; return v.len() + ":" + escape(v); });
s("esc_f", function() { local v = "\f"; return v.len() + ":" + escape(v); });
s("esc_v", function() { local v = "\v"; return v.len() + ":" + escape(v); });
s("esc_bs", function() { local v = "\\"; return v.len() + ":" + escape(v); });
s("esc_dq", function() { local v = "\""; return v.len() + ":" + escape(v); });
s("esc_sq", function() { local v = "\'"; return v.len() + ":" + escape(v); });
s("esc_zero", function() { local v = "\0"; return v.len() + ":" + escape(v); });
s("esc_x41", function() { local v = "\x41"; return v.len() + ":" + escape(v); });
s("esc_x7f", function() { local v = "\x7f"; return v.len() + ":" + escape(v); });
s("esc_mix", function() { return escape("a\tb\nc\\d\"e"); });
s("esc_empty", function() { return escape(""); });

// ---- 1b. \x / \u / \U 的**字节值**与字符串下标的有符号性
// ↔ 参考实现是非 UNICODE 构建（SQChar = char）：
//   * `\u00e9` 编码成 UTF-8 的两字节 C3 A9；
//   * 但 `"é"[0]` 取到的是 SQChar（**有符号**）→ -61，不是 195；blob 则是 0..255。
function bytes(v) {
    local out = v.len() + "[";
    for (local i = 0; i < v.len(); i++) { if (i) out += ","; out += v[i]; }
    return out + "]";
}
s("bslash_x41", function() { return bytes("\x41"); });
s("bslash_xff", function() { return bytes("\xff"); });
s("uni_41", function() { return bytes("\u0041"); });
s("uni_e9", function() { return bytes("\u00e9"); });
s("uni_ff", function() { return bytes("\u00ff"); });
s("uni_100", function() { return bytes("\u0100"); });
s("uni_4e2d", function() { return bytes("\u4e2d"); });
s("uni_20ac", function() { return bytes("\u20ac"); });
s("uni_U0001F600", function() { return bytes("\U0001F600"); });
s("uni_U00000041", function() { return bytes("\U00000041"); });
s("idx_ascii", function() { local v = "abc"; return v[0] + "/" + v[2] + "/" + v[-1]; });
s("idx_high", function() { local v = "\u00e9"; return v[0] + "/" + v[1]; });
s("idx_ff", function() { local v = "\xff"; return v[0]; });
s("idx_out_of_range", function() { local v = "abc"; try { return v[3]; } catch (e) { return "E:" + e; } });
s("foreach_high", function() { local out = ""; foreach (c in "\u00e9") out += c + ","; return out; });
s("uni_eq_utf8", function() { return ("\u00e9" == "\xc3\xa9") + "/" + ("\u00e9" == "\xe9"); });
s("blob_high_byte", function() { local b = blob(1); b[0] = 233; return b[0] + "/" + (b[0] > 128); });
s("blob_ff_byte", function() { local b = blob(1); b[0] = 255; return b[0]; });

// ---- 2. verbatim 字符串（@"..."，内部不处理转义；"" 表示一个引号）
s("verb_plain", function() { local v = @"abc"; return v.len() + ":" + v; });
s("verb_esc", function() { local v = @"a\nb"; return v.len() + ":" + escape(v); });
s("verb_quote", function() { local v = @"a""b"; return v.len() + ":" + v; });
s("verb_backslash", function() { local v = @"a\"; return v.len() + ":" + v; });
s("verb_empty", function() { local v = @""; return v.len(); });
s("verb_tab", function() { local v = @"a	b"; return escape(v); });

// ---- 3. 运算符优先级与结合性（两两/三元组合）
s("prec_add_mul", function() { return 1 + 2 * 3; });
s("prec_mul_add", function() { return 2 * 3 + 1; });
s("prec_sub_assoc", function() { return 10 - 3 - 2; });
s("prec_div_assoc", function() { return 100 / 5 / 2; });
s("prec_mod_mul", function() { return 7 % 4 * 2; });
s("prec_shift_add", function() { return 1 << 2 + 3; });
s("prec_add_shift", function() { return 1 + 2 << 3; });
s("prec_and_or", function() { return 1 | 2 & 3; });
s("prec_xor_and", function() { return 1 ^ 3 & 1; });
s("prec_eq_lt", function() { return 1 < 2 == true; });
s("prec_lt_eq", function() { return (1 < 2) == true; });
s("prec_cmp_chain", function() { return 1 <=> 2 < 3; });
s("prec_and_lt", function() { return 1 < 2 && 3 < 4; });
s("prec_or_and", function() { return false || true && false; });
s("prec_not_eq", function() { return !true == false; });
s("prec_neg_pow", function() { return -2 * 3; });
s("prec_unm_chain", function() { return - - 5; });
s("prec_bnot_add", function() { return ~1 + 1; });
s("prec_typeof_eq", function() { return typeof 1 == "integer"; });
s("prec_in_prec", function() { return "a" in { a = 1 } == true; });
s("prec_ternary_prec", function() { return true ? 1 : 2 + 3; });
s("prec_ternary_nest", function() { return true ? false ? 1 : 2 : 3; });
s("prec_ternary_assign", function() { return true ? 1 : 2; });
s("prec_assign_chain", function() { local a = 1, b = 2; a = b = 3; return a + b; });
s("prec_assign_expr", function() { local a = 1; local b = (a = 5); return b; });
s("prec_cmp_assign", function() { local a = 0; a += 1 + 2 * 3; return a; });
s("prec_logic_short", function() { local n = 0; (false && (n = 1)); return n; });
s("prec_logic_short2", function() { local n = 0; (true || (n = 1)); return n; });
s("prec_logic_ret", function() { return false && true; });
s("prec_logic_ret2", function() { return true || false; });
s("prec_in_chain", function() { return ("a" in { a = 1 }) && ("b" in { a = 1 }); });
s("prec_instanceof_prec", function() { local C = class {}; local o = C(); return (o instanceof C) == true; });
s("prec_instanceof_and", function() { local C = class {}; local o = C(); return o instanceof C && 1 == 1; });
s("prec_shift_prec", function() { return 1 << 1 << 2; });
s("prec_ushift", function() { return -8 >>> 1; });
s("prec_mod_neg_prec", function() { return -7 % 3 + 1; });
s("prec_mul_div_mod", function() { return 8 / 2 % 3 * 2; });

s("assoc_p", function() { return 2 + 3 + 4; });
s("assoc_s", function() { return 2 - 3 - 4; });
s("assoc_t", function() { return 2 * 3 * 4; });
s("assoc_d", function() { return 2 / 3 / 4; });
s("assoc_m", function() { return 2 % 3 % 4; });
s("assoc_a", function() { return 2 & 3 & 4; });
s("assoc_o", function() { return 2 | 3 | 4; });
s("assoc_^", function() { return 2 ^ 3 ^ 4; });
s("assoc_ll", function() { return 2 << 3 << 4; });
s("assoc_gg", function() { return 2 >> 3 >> 4; });
s("assoc_ggg", function() { return 2 >>> 3 >>> 4; });
s("assoc_aa", function() { return 2 && 3 && 4; });
s("assoc_oo", function() { return 2 || 3 || 4; });

s("cmp_ee_1_2", function() { return 1 == 2; });
s("cmp_ee_2_1", function() { return 2 == 1; });
s("cmp_ee_2_2", function() { return 2 == 2; });
s("cmp_ee_1p5_1", function() { return 1.5 == 1; });
s("cmp_ee_a_b", function() { return "a" == "b"; });
s("cmp_ne_1_2", function() { return 1 != 2; });
s("cmp_ne_2_1", function() { return 2 != 1; });
s("cmp_ne_2_2", function() { return 2 != 2; });
s("cmp_ne_1p5_1", function() { return 1.5 != 1; });
s("cmp_ne_a_b", function() { return "a" != "b"; });
s("cmp_l_1_2", function() { return 1 < 2; });
s("cmp_l_2_1", function() { return 2 < 1; });
s("cmp_l_2_2", function() { return 2 < 2; });
s("cmp_l_1p5_1", function() { return 1.5 < 1; });
s("cmp_l_a_b", function() { return "a" < "b"; });
s("cmp_le_1_2", function() { return 1 <= 2; });
s("cmp_le_2_1", function() { return 2 <= 1; });
s("cmp_le_2_2", function() { return 2 <= 2; });
s("cmp_le_1p5_1", function() { return 1.5 <= 1; });
s("cmp_le_a_b", function() { return "a" <= "b"; });
s("cmp_g_1_2", function() { return 1 > 2; });
s("cmp_g_2_1", function() { return 2 > 1; });
s("cmp_g_2_2", function() { return 2 > 2; });
s("cmp_g_1p5_1", function() { return 1.5 > 1; });
s("cmp_g_a_b", function() { return "a" > "b"; });
s("cmp_ge_1_2", function() { return 1 >= 2; });
s("cmp_ge_2_1", function() { return 2 >= 1; });
s("cmp_ge_2_2", function() { return 2 >= 2; });
s("cmp_ge_1p5_1", function() { return 1.5 >= 1; });
s("cmp_ge_a_b", function() { return "a" >= "b"; });
s("cmp_leg_1_2", function() { return 1 <=> 2; });
s("cmp_leg_2_1", function() { return 2 <=> 1; });
s("cmp_leg_2_2", function() { return 2 <=> 2; });
s("cmp_leg_1p5_1", function() { return 1.5 <=> 1; });
s("cmp_leg_a_b", function() { return "a" <=> "b"; });

// ---- 4. ++/-- 的返回值（前缀 vs 后缀）与作用对象
s("pp_local", function() { local x = 1; local y = x++; return x + "/" + y; });
s("pp_local_pre", function() { local x = 1; local y = ++x; return x + "/" + y; });
s("mm_local", function() { local x = 1; local y = x--; return x + "/" + y; });
s("mm_local_pre", function() { local x = 1; local y = --x; return x + "/" + y; });
s("pp_slot", function() { local t = { v = 1 }; local y = t.v++; return t.v + "/" + y; });
s("pp_slot_pre", function() { local t = { v = 1 }; local y = ++t.v; return t.v + "/" + y; });
s("pp_arr", function() { local a = [1]; local y = a[0]++; return a[0] + "/" + y; });
s("pp_global", function() { g1 <- 1; local y = g1++; return g1 + "/" + y; });
s("pp_root_pre", function() { g2 <- 1; local y = ++g2; return g2 + "/" + y; });
s("pp_this", function() { local C = class { v = 1; function f() { return this.v++; } }; local o = C(); local y = o.f(); return o.v + "/" + y; });
s("pp_expr_stmt", function() { local x = 1; x++; ++x; x--; --x; return x; });
s("pp_float", function() { local x = 1.5; x++; return x; });

// ---- 5. 作用域与遮蔽
s("scope_block", function() { local x = 1; { local x = 2; } return x; });
s("scope_inner_use", function() { local x = 1; { local y = x + 1; return y; } });
s("scope_shadow_assign", function() { local x = 1; { local x = 2; x = 3; } return x; });
s("scope_outer_assign", function() { local x = 1; { x = 5; } return x; });
s("scope_global_vs_local", function() { gv <- 1; local gv2 = 1; { local gv2 = 2; } return gv + gv2; });
s("scope_closure_capture", function() { local x = 1; local f = function() { return x; }; x = 2; return f(); });
s("scope_closure_block", function() { local f; { local x = 9; f = function() { return x; }; } return f(); });
s("scope_for_capture", function() { local fs = []; for (local i = 0; i < 3; i++) { fs.append(function() { return i; }); } return fs[0]() + "/" + fs[1]() + "/" + fs[2](); });
s("scope_foreach_capture", function() { local fs = []; foreach (v in [10, 20]) { fs.append(function() { return v; }); } return fs[0]() + "/" + fs[1](); });
s("scope_while_capture", function() { local fs = []; local i = 0; while (i < 2) { local j = i; fs.append(function() { return j; }); i++; } return fs[0]() + "/" + fs[1](); });
s("scope_fn_param_capture", function() { local mk = function(p) { return function() { return p; }; }; return mk(7)(); });
s("scope_nested_shadow", function() { local x = 1; local f = function() { local x = 2; return function() { return x; }; }; return f()(); });
s("scope_this_nested", function() { local C = class { v = 3; function f() { local g = function() { return v; }; return g(); } }; try { return C().f(); } catch (e) { return "E:" + e; } });
s("scope_local_multi", function() { local a = 1, b = 2, c; return a + "/" + b + "/" + typeof c; });
s("scope_redeclare", function() { local a = 1; local a = 2; return a; });

// ---- 6. 语句形态
s("st_if_chain", function() { local x = 2; if (x == 1) return "a"; else if (x == 2) return "b"; else return "c"; });
s("st_if_nobrace", function() { local x = 1; if (x) return "y"; return "n"; });
s("st_while", function() { local i = 0, n = 0; while (i < 4) { n += i; i++; } return n; });
s("st_while_false", function() { while (false) { return 1; } return 2; });
s("st_do_while", function() { local i = 0; do { i++; } while (i < 3); return i; });
s("st_do_while_once", function() { local i = 0; do { i++; } while (false); return i; });
s("st_for_empty", function() { local n = 0; for (;;) { n++; if (n > 2) break; } return n; });
s("st_for_nocond", function() { local n = 0; for (local i = 0;; i++) { if (i > 1) break; n++; } return n; });
s("st_for_noinit", function() { local i = 0; for (; i < 2; i++) {} return i; });
s("st_for_nostep", function() { local n = 0; for (local i = 0; i < 3;) { i++; n++; } return n; });
s("st_foreach_arr", function() { local out = ""; foreach (v in [1, 2, 3]) out += v; return out; });
s("st_foreach_arr_idx", function() { local out = ""; foreach (i, v in [7, 8]) out += i + ":" + v + ","; return out; });
s("st_foreach_tbl", function() { local t = {}; t.a <- 1; t.b <- 2; local out = ""; foreach (k, v in t) out += k + "=" + v + ","; return out; });
s("st_foreach_str", function() { local out = ""; foreach (c in "abc") out += c + ""; return out; });
s("st_foreach_gen", function() { local out = ""; foreach (v in (function() { yield 1; yield 2; })()) out += v; return out; });
s("st_foreach_break", function() { local out = ""; foreach (v in [1, 2, 3]) { if (v == 2) break; out += v; } return out; });
s("st_foreach_continue", function() { local out = ""; foreach (v in [1, 2, 3]) { if (v == 2) continue; out += v; } return out; });
s("st_switch", function() { local x = 2; switch (x) { case 1: return "a"; case 2: return "b"; default: return "c"; } });
s("st_switch_fall", function() { local x = 1, out = ""; switch (x) { case 1: out += "1"; case 2: out += "2"; default: out += "d"; } return out; });
s("st_switch_break", function() { local x = 1, out = ""; switch (x) { case 1: out += "1"; break; case 2: out += "2"; break; } return out; });
s("st_switch_str", function() { switch ("b") { case "a": return 1; case "b": return 2; } return 0; });
s("st_switch_nodefault", function() { local x = 9, out = "x"; switch (x) { case 1: out = "a"; } return out; });
s("st_nested_loops", function() { local n = 0; for (local i = 0; i < 3; i++) { for (local j = 0; j < 3; j++) { if (j == 1) continue; n++; } } return n; });
s("st_break_inner", function() { local n = 0; for (local i = 0; i < 3; i++) { for (local j = 0; j < 3; j++) { if (j == 1) break; n++; } } return n; });
s("st_multiline_expr", function() { local x = 1 +
        2 +
        3; return x; });
s("st_comment_inline", function() { local x = 1; // 注释
 return x; /* 块注释 */ });
s("line_marker", function() { local n = __LINE__; return typeof n; });
s("line_delta", function() { local a = __LINE__;


 local b = __LINE__; return b - a; });

// ---- 7. 函数与类语法面
s("fn_expr", function() { local f = function() { return 1; }; return f(); });
s("fn_named_stmt", function() { function inner() { return 2; } return inner(); });
s("fn_defaults", function() { local f = function(a, b = 2, c = 3) { return a + b + c; }; return f(1) + "/" + f(1, 10) + "/" + f(1, 10, 100); });
s("fn_default_expr", function() { local f = function(a, b = 1 + 1) { return a + b; }; return f(1); });
s("fn_varargs", function() { local f = function(...) { return vargv.len() + ":" + vargv[0]; }; return f(1, 2); });
s("fn_varargs_zero", function() { local f = function(...) { return vargv.len(); }; return f(); });
s("fn_callee", function() { local f = function() { return typeof callee; }; return f(); });
s("fn_tailcall", function() { local f = function(n) { if (n == 0) return "done"; return f(n - 1); }; return f(3); });
s("fn_recursion", function() { local f = function(n) { return n <= 1 ? 1 : n * f(n - 1); }; return f(5); });
s("cls_basic", function() { local C = class { v = 1; function get() { return this.v; } }; return C().get(); });
s("cls_extends", function() { local A = class { function f() { return "A"; } }; local B = class extends A { }; return B().f(); });
s("cls_base", function() { local A = class { function f() { return "A"; } }; local B = class extends A { function f() { return "B" + base.f(); } }; return B().f(); });
s("cls_ctor", function() { local C = class { v = 0; constructor(x) { v = x; } }; return C(5).v; });
s("cls_ctor_default", function() { local C = class { v = 0; constructor(x = 3) { v = x; } }; return C().v + "/" + C(7).v; });
s("cls_static", function() { local C = class { static s = 5; static function f() { return s; } }; return C.f(); });
s("cls_attr", function() { local C = class { </ a = 1 /> v = 2; }; return C.getattributes("v").a; });
s("cls_attr_multi", function() { local C = class { </ a = 1, b = "x" /> v = 2; }; local at = C.getattributes("v"); return at.a + "/" + at.b; });
s("cls_nested", function() { local Outer = class { Inner = null; constructor() { Inner = class { v = 9; }; } }; return Outer().Inner().v; });
s("cls_method_varargs", function() { local C = class { function f(...) { return vargv.len(); } }; return C().f(1, 2, 3); });
s("cls_method_default", function() { local C = class { function f(a, b = 2) { return a + b; } }; return C().f(1); });
s("cls_clone", function() { local C = class { v = 1; }; local D = clone C; D.newmember("w", 2); return C().v + "/" + D().w; });
s("cls_instanceof", function() { local A = class {}; local B = class extends A {}; return (B() instanceof A) + "/" + (A() instanceof B); });
s("ns_global", function() { nsg <- 1; return ::nsg + "/" + ::getroottable().nsg; });
s("nested_fn_capture_this", function() { local C = class { v = 4; function f() { local g = function() { return this.v; }; return g.call(this); } }; return C().f(); });

print("LEXMATRIX-DONE\n");

# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_lexmatrix.nut：词法 / 运算符优先级 / 作用域 组合矩阵。

范围（P1-3）：
- 字符串转义全表（用 escape() 打印，避免控制字符搅乱逐行对比）
- verbatim 字符串 `@"..."`
- 运算符优先级与结合性（枚举两两组合）
- 短路求值 / 赋值链 / 前后缀自增的**返回值**
- 作用域与遮蔽、循环里的闭包捕获
- 注释、多行语句、语句形态（if/while/do/for/foreach/switch/三元）
- __LINE__
- 类/函数形态的语法面（extends/base/static/attributes/默认参数/varargs）

用法：python tools/gen_lexmatrix.py
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "tests-corpus", "t_lexmatrix.nut")

L = []
A = L.append
A("// 自动生成：python tools/gen_lexmatrix.py —— 请勿手改")
A("// 词法 / 优先级 / 作用域 组合矩阵")
A("")
A("function s(tag, f) {")
A("    try { print(tag + \"=\" + f() + \"\\n\"); }")
A("    catch (e) { print(tag + \"!\" + e + \"\\n\"); }")
A("}")
A("")

# ---------------------------------------------------------------- 1. 字符串转义
A("// ---- 1. 字符串转义（ASCII 转义用 escape() 输出；高位字节改用字节转储）")
A("// 注：`escape()` 遇到 ≥0x80 的字节时，参考实现用 `(char)c` 传给 `%02x`，")
A("// 写出 8 位补码（`\\xffffffe9`）**并越界写自己的 scratchpad**——实测输出会串行、")
A("// 还混进别的错误消息片段（STATUS 第四节）。所以高位字节只比字节值，不比 escape()。")
ESCAPES = [
    ("n", r"\n"), ("t", r"\t"), ("r", r"\r"), ("a", r"\a"), ("b", r"\b"),
    ("f", r"\f"), ("v", r"\v"), ("bs", r"\\"), ("dq", r"\""), ("sq", r"\'"),
    ("zero", r"\0"), ("x41", r"\x41"), ("x7f", r"\x7f"),
    # 注：未知转义（\q \% \/ \$ \@）是**编译错误** `unrecognised escaper char`，
    # 归 tools/errmsg_matrix.py 对拍，不放进这个"必须能编译"的语料
]
for name, esc in ESCAPES:
    A('s("esc_%s", function() { local v = "%s"; return v.len() + ":" + escape(v); });' % (name, esc))
A('s("esc_mix", function() { return escape("a\\tb\\nc\\\\d\\"e"); });')
A('s("esc_empty", function() { return escape(""); });')
A("")
A("// ---- 1b. \\x / \\u / \\U 的**字节值**与字符串下标的有符号性")
A("// ↔ 参考实现是非 UNICODE 构建（SQChar = char）：")
A("//   * `\\u00e9` 编码成 UTF-8 的两字节 C3 A9；")
A("//   * 但 `\"é\"[0]` 取到的是 SQChar（**有符号**）→ -61，不是 195；blob 则是 0..255。")
A("function bytes(v) {")
A("    local out = v.len() + \"[\";")
A("    for (local i = 0; i < v.len(); i++) { if (i) out += \",\"; out += v[i]; }")
A("    return out + \"]\";")
A("}")
A('s("bslash_x41", function() { return bytes("\\x41"); });')
A('s("bslash_xff", function() { return bytes("\\xff"); });')
A('s("uni_41", function() { return bytes("\\u0041"); });')
A('s("uni_e9", function() { return bytes("\\u00e9"); });')
A('s("uni_ff", function() { return bytes("\\u00ff"); });')
A('s("uni_100", function() { return bytes("\\u0100"); });')
A('s("uni_4e2d", function() { return bytes("\\u4e2d"); });')
A('s("uni_20ac", function() { return bytes("\\u20ac"); });')
A('s("uni_U0001F600", function() { return bytes("\\U0001F600"); });')
A('s("uni_U00000041", function() { return bytes("\\U00000041"); });')
A('s("idx_ascii", function() { local v = "abc"; return v[0] + "/" + v[2] + "/" + v[-1]; });')
A('s("idx_high", function() { local v = "\\u00e9"; return v[0] + "/" + v[1]; });')
A('s("idx_ff", function() { local v = "\\xff"; return v[0]; });')
A('s("idx_out_of_range", function() { local v = "abc"; try { return v[3]; } catch (e) { return "E:" + e; } });')
A('s("foreach_high", function() { local out = ""; foreach (c in "\\u00e9") out += c + ","; return out; });')
A('s("uni_eq_utf8", function() { return ("\\u00e9" == "\\xc3\\xa9") + "/" + ("\\u00e9" == "\\xe9"); });')
A('s("blob_high_byte", function() { local b = blob(1); b[0] = 233; return b[0] + "/" + (b[0] > 128); });')
A('s("blob_ff_byte", function() { local b = blob(1); b[0] = 255; return b[0]; });')
A("")

# ---------------------------------------------------------------- 2. verbatim 字符串
A("// ---- 2. verbatim 字符串（@\"...\"，内部不处理转义；\"\" 表示一个引号）")
A('s("verb_plain", function() { local v = @"abc"; return v.len() + ":" + v; });')
A('s("verb_esc", function() { local v = @"a\\nb"; return v.len() + ":" + escape(v); });')
A('s("verb_quote", function() { local v = @"a""b"; return v.len() + ":" + v; });')
A('s("verb_backslash", function() { local v = @"a\\"; return v.len() + ":" + v; });')
A('s("verb_empty", function() { local v = @""; return v.len(); });')
A('s("verb_tab", function() { local v = @"a\tb"; return escape(v); });')
A("")

# ---------------------------------------------------------------- 3. 运算符优先级 / 结合性
A("// ---- 3. 运算符优先级与结合性（两两/三元组合）")
BINOPS = ["+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>", ">>>",
          "==", "!=", "<", "<=", ">", ">=", "<=>", "&&", "||", "in", "instanceof"]
PREC_CASES = [
    ("add_mul", "1 + 2 * 3"), ("mul_add", "2 * 3 + 1"), ("sub_assoc", "10 - 3 - 2"),
    ("div_assoc", "100 / 5 / 2"), ("mod_mul", "7 % 4 * 2"), ("shift_add", "1 << 2 + 3"),
    ("add_shift", "1 + 2 << 3"), ("and_or", "1 | 2 & 3"), ("xor_and", "1 ^ 3 & 1"),
    ("eq_lt", "1 < 2 == true"), ("lt_eq", "(1 < 2) == true"), ("cmp_chain", "1 <=> 2 < 3"),
    ("and_lt", "1 < 2 && 3 < 4"), ("or_and", "false || true && false"),
    ("not_eq", "!true == false"), ("neg_pow", "-2 * 3"), ("unm_chain", "- - 5"),
    ("bnot_add", "~1 + 1"), ("typeof_eq", "typeof 1 == \"integer\""),
    ("in_prec", "\"a\" in { a = 1 } == true"), ("ternary_prec", "true ? 1 : 2 + 3"),
    ("ternary_nest", "true ? false ? 1 : 2 : 3"), ("ternary_assign", "true ? 1 : 2"),
    ("assign_chain", "local a = 1, b = 2; a = b = 3; return a + b"),
    ("assign_expr", "local a = 1; local b = (a = 5); return b"),
    ("cmp_assign", "local a = 0; a += 1 + 2 * 3; return a"),
    ("logic_short", "local n = 0; (false && (n = 1)); return n"),
    ("logic_short2", "local n = 0; (true || (n = 1)); return n"),
    ("logic_ret", "return false && true"),
    ("logic_ret2", "return true || false"),
    ("in_chain", "(\"a\" in { a = 1 }) && (\"b\" in { a = 1 })"),
    ("instanceof_prec", "local C = class {}; local o = C(); return (o instanceof C) == true"),
    ("instanceof_and", "local C = class {}; local o = C(); return o instanceof C && 1 == 1"),
    ("shift_prec", "1 << 1 << 2"), ("ushift", "-8 >>> 1"),
    ("mod_neg_prec", "-7 % 3 + 1"), ("mul_div_mod", "8 / 2 % 3 * 2"),
]
for name, expr in PREC_CASES:
    body = expr if expr.strip().startswith(("return", "local")) else "return " + expr
    A('s("prec_%s", function() { %s; });' % (name, body))
A("")
# 结合性：同一运算符连写
for op in ["+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>", ">>>", "&&", "||"]:
    tag = op.replace("&", "a").replace("|", "o").replace("<", "l").replace(">", "g").replace("%", "m").replace("/", "d").replace("*", "t").replace("+", "p").replace("-", "s")
    A('s("assoc_%s", function() { return 2 %s 3 %s 4; });' % (tag, op, op))
A("")
# 比较运算返回值
for op in ["==", "!=", "<", "<=", ">", ">=", "<=>"]:
    for a, b in [("1", "2"), ("2", "1"), ("2", "2"), ("1.5", "1"), ("\"a\"", "\"b\"")]:
        tag = op.replace("<", "l").replace(">", "g").replace("=", "e").replace("!", "n")
        A('s("cmp_%s_%s_%s", function() { return %s %s %s; });' % (tag, a.replace(".", "p").replace('"', ""), b.replace(".", "p").replace('"', ""), a, op, b))
A("")

# ---------------------------------------------------------------- 4. 自增/自减返回值
A("// ---- 4. ++/-- 的返回值（前缀 vs 后缀）与作用对象")
A('s("pp_local", function() { local x = 1; local y = x++; return x + "/" + y; });')
A('s("pp_local_pre", function() { local x = 1; local y = ++x; return x + "/" + y; });')
A('s("mm_local", function() { local x = 1; local y = x--; return x + "/" + y; });')
A('s("mm_local_pre", function() { local x = 1; local y = --x; return x + "/" + y; });')
A('s("pp_slot", function() { local t = { v = 1 }; local y = t.v++; return t.v + "/" + y; });')
A('s("pp_slot_pre", function() { local t = { v = 1 }; local y = ++t.v; return t.v + "/" + y; });')
A('s("pp_arr", function() { local a = [1]; local y = a[0]++; return a[0] + "/" + y; });')
A('s("pp_global", function() { g1 <- 1; local y = g1++; return g1 + "/" + y; });')
A('s("pp_root_pre", function() { g2 <- 1; local y = ++g2; return g2 + "/" + y; });')
A('s("pp_this", function() { local C = class { v = 1; function f() { return this.v++; } }; local o = C(); local y = o.f(); return o.v + "/" + y; });')
A('s("pp_expr_stmt", function() { local x = 1; x++; ++x; x--; --x; return x; });')
A('s("pp_float", function() { local x = 1.5; x++; return x; });')
A("")

# ---------------------------------------------------------------- 5. 作用域与遮蔽
A("// ---- 5. 作用域与遮蔽")
A('s("scope_block", function() { local x = 1; { local x = 2; } return x; });')
A('s("scope_inner_use", function() { local x = 1; { local y = x + 1; return y; } });')
A('s("scope_shadow_assign", function() { local x = 1; { local x = 2; x = 3; } return x; });')
A('s("scope_outer_assign", function() { local x = 1; { x = 5; } return x; });')
A('s("scope_global_vs_local", function() { gv <- 1; local gv2 = 1; { local gv2 = 2; } return gv + gv2; });')
A('s("scope_closure_capture", function() { local x = 1; local f = function() { return x; }; x = 2; return f(); });')
A('s("scope_closure_block", function() { local f; { local x = 9; f = function() { return x; }; } return f(); });')
A('s("scope_for_capture", function() { local fs = []; for (local i = 0; i < 3; i++) { fs.append(function() { return i; }); } return fs[0]() + "/" + fs[1]() + "/" + fs[2](); });')
A('s("scope_foreach_capture", function() { local fs = []; foreach (v in [10, 20]) { fs.append(function() { return v; }); } return fs[0]() + "/" + fs[1](); });')
A('s("scope_while_capture", function() { local fs = []; local i = 0; while (i < 2) { local j = i; fs.append(function() { return j; }); i++; } return fs[0]() + "/" + fs[1](); });')
A('s("scope_fn_param_capture", function() { local mk = function(p) { return function() { return p; }; }; return mk(7)(); });')
A('s("scope_nested_shadow", function() { local x = 1; local f = function() { local x = 2; return function() { return x; }; }; return f()(); });')
A('s("scope_this_nested", function() { local C = class { v = 3; function f() { local g = function() { return v; }; return g(); } }; try { return C().f(); } catch (e) { return "E:" + e; } });')
A('s("scope_local_multi", function() { local a = 1, b = 2, c; return a + "/" + b + "/" + typeof c; });')
A('s("scope_redeclare", function() { local a = 1; local a = 2; return a; });')
A("")

# ---------------------------------------------------------------- 6. 语句形态
A("// ---- 6. 语句形态")
A('s("st_if_chain", function() { local x = 2; if (x == 1) return "a"; else if (x == 2) return "b"; else return "c"; });')
A('s("st_if_nobrace", function() { local x = 1; if (x) return "y"; return "n"; });')
A('s("st_while", function() { local i = 0, n = 0; while (i < 4) { n += i; i++; } return n; });')
A('s("st_while_false", function() { while (false) { return 1; } return 2; });')
A('s("st_do_while", function() { local i = 0; do { i++; } while (i < 3); return i; });')
A('s("st_do_while_once", function() { local i = 0; do { i++; } while (false); return i; });')
A('s("st_for_empty", function() { local n = 0; for (;;) { n++; if (n > 2) break; } return n; });')
A('s("st_for_nocond", function() { local n = 0; for (local i = 0;; i++) { if (i > 1) break; n++; } return n; });')
A('s("st_for_noinit", function() { local i = 0; for (; i < 2; i++) {} return i; });')
A('s("st_for_nostep", function() { local n = 0; for (local i = 0; i < 3;) { i++; n++; } return n; });')
A('s("st_foreach_arr", function() { local out = ""; foreach (v in [1, 2, 3]) out += v; return out; });')
A('s("st_foreach_arr_idx", function() { local out = ""; foreach (i, v in [7, 8]) out += i + ":" + v + ","; return out; });')
A('s("st_foreach_tbl", function() { local t = {}; t.a <- 1; t.b <- 2; local out = ""; foreach (k, v in t) out += k + "=" + v + ","; return out; });')
A('s("st_foreach_str", function() { local out = ""; foreach (c in "abc") out += c + ""; return out; });')
A('s("st_foreach_gen", function() { local out = ""; foreach (v in (function() { yield 1; yield 2; })()) out += v; return out; });')
A('s("st_foreach_break", function() { local out = ""; foreach (v in [1, 2, 3]) { if (v == 2) break; out += v; } return out; });')
A('s("st_foreach_continue", function() { local out = ""; foreach (v in [1, 2, 3]) { if (v == 2) continue; out += v; } return out; });')
A('s("st_switch", function() { local x = 2; switch (x) { case 1: return "a"; case 2: return "b"; default: return "c"; } });')
A('s("st_switch_fall", function() { local x = 1, out = ""; switch (x) { case 1: out += "1"; case 2: out += "2"; default: out += "d"; } return out; });')
A('s("st_switch_break", function() { local x = 1, out = ""; switch (x) { case 1: out += "1"; break; case 2: out += "2"; break; } return out; });')
A('s("st_switch_str", function() { switch ("b") { case "a": return 1; case "b": return 2; } return 0; });')
A('s("st_switch_nodefault", function() { local x = 9, out = "x"; switch (x) { case 1: out = "a"; } return out; });')
A('s("st_nested_loops", function() { local n = 0; for (local i = 0; i < 3; i++) { for (local j = 0; j < 3; j++) { if (j == 1) continue; n++; } } return n; });')
A('s("st_break_inner", function() { local n = 0; for (local i = 0; i < 3; i++) { for (local j = 0; j < 3; j++) { if (j == 1) break; n++; } } return n; });')
A('s("st_multiline_expr", function() { local x = 1 +\n        2 +\n        3; return x; });')
A('s("st_comment_inline", function() { local x = 1; // 注释\n return x; /* 块注释 */ });')
A('s("line_marker", function() { local n = __LINE__; return typeof n; });')
A('s("line_delta", function() { local a = __LINE__;\n\n\n local b = __LINE__; return b - a; });')
A("")

# ---------------------------------------------------------------- 7. 函数/类语法面
A("// ---- 7. 函数与类语法面")
A('s("fn_expr", function() { local f = function() { return 1; }; return f(); });')
A('s("fn_named_stmt", function() { function inner() { return 2; } return inner(); });')
A('s("fn_defaults", function() { local f = function(a, b = 2, c = 3) { return a + b + c; }; return f(1) + "/" + f(1, 10) + "/" + f(1, 10, 100); });')
A('s("fn_default_expr", function() { local f = function(a, b = 1 + 1) { return a + b; }; return f(1); });')
A('s("fn_varargs", function() { local f = function(...) { return vargv.len() + ":" + vargv[0]; }; return f(1, 2); });')
A('s("fn_varargs_zero", function() { local f = function(...) { return vargv.len(); }; return f(); });')
A('s("fn_callee", function() { local f = function() { return typeof callee; }; return f(); });')
A('s("fn_tailcall", function() { local f = function(n) { if (n == 0) return "done"; return f(n - 1); }; return f(3); });')
A('s("fn_recursion", function() { local f = function(n) { return n <= 1 ? 1 : n * f(n - 1); }; return f(5); });')
A('s("cls_basic", function() { local C = class { v = 1; function get() { return this.v; } }; return C().get(); });')
A('s("cls_extends", function() { local A = class { function f() { return "A"; } }; local B = class extends A { }; return B().f(); });')
A('s("cls_base", function() { local A = class { function f() { return "A"; } }; local B = class extends A { function f() { return "B" + base.f(); } }; return B().f(); });')
A('s("cls_ctor", function() { local C = class { v = 0; constructor(x) { v = x; } }; return C(5).v; });')
A('s("cls_ctor_default", function() { local C = class { v = 0; constructor(x = 3) { v = x; } }; return C().v + "/" + C(7).v; });')
A('s("cls_static", function() { local C = class { static s = 5; static function f() { return s; } }; return C.f(); });')
A('s("cls_attr", function() { local C = class { </ a = 1 /> v = 2; }; return C.getattributes("v").a; });')
A('s("cls_attr_multi", function() { local C = class { </ a = 1, b = "x" /> v = 2; }; local at = C.getattributes("v"); return at.a + "/" + at.b; });')
A('s("cls_nested", function() { local Outer = class { Inner = null; constructor() { Inner = class { v = 9; }; } }; return Outer().Inner().v; });')
A('s("cls_method_varargs", function() { local C = class { function f(...) { return vargv.len(); } }; return C().f(1, 2, 3); });')
A('s("cls_method_default", function() { local C = class { function f(a, b = 2) { return a + b; } }; return C().f(1); });')
A('s("cls_clone", function() { local C = class { v = 1; }; local D = clone C; D.newmember("w", 2); return C().v + "/" + D().w; });')
A('s("cls_instanceof", function() { local A = class {}; local B = class extends A {}; return (B() instanceof A) + "/" + (A() instanceof B); });')
A('s("ns_global", function() { nsg <- 1; return ::nsg + "/" + ::getroottable().nsg; });')
A('s("nested_fn_capture_this", function() { local C = class { v = 4; function f() { local g = function() { return this.v; }; return g.call(this); } }; return C().f(); });')
A("")

A('print("LEXMATRIX-DONE\\n");')
A("")

open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(L))
print("wrote %s (%d 行, %d 用例)" % (OUT, len(L), len([x for x in L if x.startswith("s(")])))

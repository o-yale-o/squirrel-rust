# -*- coding: utf-8 -*-
"""生成 printf 风格格式串矩阵语料 tests-corpus/t_formatmx.nut。

参考实现（sqstdlib/sqstdstring.cpp:sqstd_format）的语义要点：

- `%s` 走 `sq_getstring`（**只接受字符串**，不做 ToString）；
  `%i/%d/%o/%u/%x/%X/%c` 走 `sq_getinteger`（只接受 integer）；`%f/%g/%G/%e/%E` 走
  `sq_getfloat`（只接受 float）——类型不符时分别报
  "string/integer/float expected for the specified format"。
- 实际转换交给 CRT `scsprintf`，所以 `%#x`、`%+d`、`% d`、`%.5x`、宽度/精度/标志
  的行为都以 MSVC printf 为准。
- `validate_format` 限制：宽度最多 2 位数字（否则 "width format too long"）、
  精度同样最多 2 位、整段最长 20 字符（否则 "format too long"）、不支持 `*`。

用法：
    python tools/gen_formatmx.py            # 写出 tests-corpus/t_formatmx.nut
"""
import os

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_formatmx.nut")

# 浮点值（SQFloat 是 f32，取值控制在 f32 范围内）
FLOATS = [
    ("zero", "0.0"),
    ("negzero", "-0.0"),
    ("one", "1.0"),
    ("negone", "-1.0"),
    ("half", "0.5"),
    ("two5", "2.5"),
    ("third", "1.0/3.0"),
    ("nthird", "-1.0/3.0"),
    ("em5", "0.00001"),
    ("em4", "0.0001"),
    ("e5", "100000.0"),
    ("e6", "1000000.0"),
    ("e15", "1.0e15"),
    ("e16", "1.0e16"),
    ("e21", "1.0e21"),
    ("mixed", "123456.789"),
    ("negmixed", "-123456.789"),
    ("nines", "0.999999999"),
    ("fmax", "3.4028235e38"),
    ("fmin", "1.1754944e-38"),
    ("denorm", "1.0e-45"),
    ("round", "0.5"),
    ("r15", "1.5"),
    ("r25", "2.5"),
    ("r35", "3.5"),
    ("inf", "1.0/0.0"),
    ("ninf", "-1.0/0.0"),
    ("nan", "0.0/0.0"),
    ("nnan", "-(0.0/0.0)"),
    ("posnan", "-(0.0/0.0)"),
]

# 整数类值
INTS = [
    ("i0", "0"),
    ("i1", "1"),
    ("im1", "-1"),
    ("i7", "7"),
    ("i255", "255"),
    ("ineg", "-123456789"),
    ("imax", "9223372036854775807"),
    ("imin", "(-9223372036854775807 - 1)"),
    ("i65", "65"),
    ("i300", "300"),
]

FVERBS = ["%f", "%.0f", "%.1f", "%.10f", "%e", "%.0e", "%.15e", "%E",
          "%g", "%.1g", "%.10g", "%G", "%#g", "%#.0f", "%#e", "%#.0e", "%#.1g", "%#.3g",
          "%10.3f", "%-10.3f", "%+.3f", "% .3f", "%010.3f", "%.2g", "%.0g",
          "%+f", "% f", "%010f", "%-10f|", "%5.3f", "%+5.3f"]
IVERBS = ["%d", "%i", "%5d", "%-5d", "%05d", "%+d", "% d", "%x", "%X", "%#x",
          "%o", "%#o", "%u", "%c", "%.5d", "%5.2d", "%.5x", "%#010x", "%+.5d",
          "%05.3d", "%-05d", "%+u", "%08.3x", "%.0x", "%#.0x", "%#.0o", "%.d", "%.0d"]
SVERBS = ["%s", "%5s", "%-5s", "%.2s", "%10.2s", "%s|%s"]

EXTRA = [
    ("pct_trailing", 'return format("abc%");'),
    ("pct_only", 'return format("%");'),
    ("pct_pct", 'return format("a%%b");'),
    ("pct_pct_d", 'return format("%%d", 1);'),
    ("invalid_z", 'return format("%z", 1);'),
    ("invalid_star", 'return format("%*d", 5, 1);'),
    ("not_enough", 'return format("%d");'),
    ("not_enough2", 'return format("%d %d", 1);'),
    ("extra_args", 'return format("%d", 1, 2);'),
    ("width_3digits", 'return format("%100d", 1);'),
    ("width_2digits", 'return format("%99d", 1);'),
    ("width_3digits_f", 'return format("%1000f", 1.0);'),
    ("prec_3digits", 'return format("%.100f", 1.0);'),
    ("prec_2digits", 'return format("%.99f", 1.0);'),
    ("format_too_long", 'return format("%---------.99d", 1);'),
    ("format_long_ok", 'return format("%---------.9d", 1);'),
    ("flags_only", 'return format("%-", 1);'),
    ("dot_only", 'return format("%.", 1);'),
    ("s_int", 'return format("%s", 1);'),
    ("s_float", 'return format("%s", 1.5);'),
    ("s_null", 'return format("%s", null);'),
    ("s_bool", 'return format("%s", true);'),
    ("s_table", 'return format("%s", {});'),
    ("s_mt_tostring", 'local t = {}; t.setdelegate({ _tostring = function() { return "T"; } }); return format("%s", t);'),
    ("d_float", 'return format("%d", 1.5);'),
    ("d_string", 'return format("%d", "1");'),
    ("f_int", 'return format("%f", 1);'),
    ("f_string", 'return format("%f", "1.5");'),
    ("g_int", 'return format("%g", 1);'),
    ("x_float", 'return format("%x", 1.5);'),
    ("c_float", 'return format("%c", 65.0);'),
    ("c_string", 'return format("%c", "a");'),
    ("c_big", 'return format("%c", 300);'),
    ("c_zero", 'return format("%c", 0);'),
    ("c_newline", 'return format("%c", 10);'),
    ("s_nul", 'return format("%s", "a" + (0).tochar() + "b");'),
    ("s_empty", 'return format("%s", "");'),
    ("fmt_not_string", 'return format(1, 2);'),
    ("fmt_null", 'return format(null);'),
    ("printf_ok", 'printf("%s=%d|", "x", 99); return "ok";'),
    ("printf_type_err", 'printf("%d", 1.5); return "ok";'),
    ("many_args", 'return format("%d,%d,%d,%d,%d,%d", 1, 2, 3, 4, 5, 6);'),
    ("pct_at_end_of_args", 'return format("%d%%", 5);'),
    ("nonascii", 'return format("[%s]", "\\xe4\\xb8\\xad");'),
    # ---- 类型接受：整型动词收 float/bool，浮点动词只收数值（bool 不行）----
    ("d_float_true", 'return format("%d", true);'),
    ("d_float_false", 'return format("%d", false);'),
    ("d_float_trunc", 'return format("%d|%d|%d", 1.5, -1.5, 2.5);'),
    ("d_float_huge", 'return format("%d", 1.0e30);'),
    ("d_nan", 'return format("%d", 0.0/0.0);'),
    ("d_inf", 'return format("%d", 1.0/0.0);'),
    ("f_int_arg", 'return format("%f", 5);'),
    ("f_bigint_arg", 'return format("%f", 9223372036854775807);'),
    ("f_bool", 'return format("%f", true);'),
    ("e_int_arg", 'return format("%e", 5);'),
    ("g_int_arg", 'return format("%g", 5);'),
    ("c_float_arg", 'return format("%c", 65.9);'),
    ("c_bool_arg", 'return format("%c", true);'),
    ("c_str_arg", 'return format("%c", "A");'),
    ("c_null_arg", 'return format("%c", null);'),
    ("c_255", 'return format("%c", 255);'),
    ("c_5width", 'return format("[%5c]", 65);'),
    ("c_05width", 'return format("[%05c]", 65);'),
    # ---- 标志与宽度细节 ----
    ("s_05width", 'return format("[%05s]", "abc");'),
    ("s_flags", 'return format("[%+s][%#s][%-5s]", "a", "a", "a");'),
    ("u_plus", 'return format("%+u|% u", 1, 1);'),
    ("pct_with_arg", 'return format("%", 1);'),
    ("dot_with_arg", 'return format("%.", 1);'),
    ("flag_with_arg", 'return format("%-", 1);'),
    ("many_flags_ok", 'return format("%-+ #0.3f", 1.5);'),
    ("flags_21", 'return format("%+++++++++++++++++++++d", 1);'),
    ("width99", 'return format("%99d", 1);'),
    ("prec99_f", 'return format("%.99f", 1.5);'),
    ("w99p99", 'return format("%99.99f", 1.5);'),
    ("x_m1", 'return format("%#x|%x", -1, -1);'),
    ("o_m1", 'return format("%#o", -1);'),
    ("u_m1", 'return format("%u", -1);'),
    ("d_imin", 'return format("%d|%+d|%5d", -9223372036854775807 - 1, -9223372036854775807 - 1, -9223372036854775807 - 1);'),
]

lines = [
    "// 自动生成：format/printf 格式串矩阵（tools/gen_formatmx.py）",
    "function s(tag, f) {",
    '    try { print(tag + "=" + f() + "\\n"); }',
    '    catch (e) { print(tag + "!" + e + "\\n"); }',
    "}",
    "",
]
n = 0
for verb in FVERBS:
    for vname, vlit in FLOATS:
        n += 1
        lines.append('s("f%d_%s_%s", function() { return format("%s", %s); });' % (n, verb.replace("%", "p").replace(".", "d"), vname, verb, vlit))
for verb in IVERBS:
    for vname, vlit in INTS:
        n += 1
        lines.append('s("i%d_%s_%s", function() { return format("%s", %s); });' % (n, verb.replace("%", "p").replace(".", "d"), vname, verb, vlit))
for verb in SVERBS:
    for sname, slit in [("abc", '"abc"'), ("empty", '""'), ("long", '"0123456789"')]:
        n += 1
        lines.append('s("s%d_%s_%s", function() { return format("%s", %s); });' % (n, verb.replace("%", "p").replace(".", "d").replace("|", "_"), sname, verb, slit))
for tag, body in EXTRA:
    lines.append('s("x_%s", function() { %s });' % (tag, body))
lines.append('print("FORMATMATRIX-DONE\\n");')

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 行" % (OUT, len(lines)))

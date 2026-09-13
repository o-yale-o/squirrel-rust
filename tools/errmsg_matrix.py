# -*- coding: utf-8 -*-
"""编译错误对拍：把一组**语法/词法非法**的源码逐条编译（`-c` 只编译），
比较两侧的 stderr（编译器错误消息 + 行列号）。

为什么单独做：语法错的脚本会中断整个文件，不能塞进普通语料。

用法：
    python tools/errmsg_matrix.py            # 跑内置用例
    python tools/errmsg_matrix.py -v         # 打印每条的原文与两侧输出
"""
import os
import subprocess
import sys

try:  # 关掉系统错误对话框：参考实现在 UB 形态上会真崩溃并弹模态框
    import os as _os, sys as _sys
    _d = _os.path.dirname(_os.path.abspath(__file__))
    for _p in (_d, _os.path.join(_d, "..", "tools"), _os.path.join(_d, "tools")):
        if _os.path.isdir(_p):
            _sys.path.insert(0, _p)
    from safe_run import suppress_crash_dialogs as _scd
    _scd()
    del _d, _p, _os, _sys, _scd
except Exception as _e:  # pragma: no cover
    import sys as _sys2
    print("warning: 无法关闭崩溃对话框: %r" % (_e,), file=_sys2.stderr)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RUST = os.path.join(ROOT, "target", "release", "sq.exe")
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")

# (tag, 源码) —— 每条都应该是**编译期**报错
CASES = [
    ("unterminated_string", 'local a = "abc;\n'),
    ("unterminated_verbatim", 'local a = @"abc;\n'),
    ("unterminated_comment", '/* abc\n'),
    ("bad_escape_x", 'local a = "\\xZZ";\n'),
    ("bad_escape_u", 'local a = "\\uZZZZ";\n'),
    ("bad_escape_q", 'local a = "\\q";\n'),
    ("bad_escape_pct", 'local a = "\\%";\n'),
    ("bad_escape_slash", 'local a = "\\/";\n'),
    ("bad_escape_at", 'local a = "\\@";\n'),
    ("bad_escape_dollar", 'local a = "\\$";\n'),
    ("stray_backslash", 'local a = "a\\";\n'),
    ("dot_number", "local a = .5;\n"),
    ("digit_sep", "local a = 1_000;\n"),
    ("hex_empty", "local a = 0x;\n"),
    ("hex_bad", "local a = 0xG;\n"),
    ("exp_bad", "local a = 1e;\n"),
    ("minus_minus_int", "local a = 9223372036854775808;\n"),
    ("unclosed_paren", "local a = (1 + 2;\n"),
    ("unclosed_brace", "function f() { local a = 1;\n"),
    ("unclosed_bracket", "local a = [1, 2;\n"),
    ("extra_close_paren", "local a = 1);\n"),
    ("extra_close_brace", "}\n"),
    ("if_no_paren", "if true { }\n"),
    ("if_empty_cond", "if () { }\n"),
    ("while_no_cond", "while { }\n"),
    ("for_bad", "for (local i = 0; i < 3) { }\n"),
    ("foreach_bad", "foreach (x) { }\n"),
    ("switch_bad", "switch (1) { case }\n"),
    ("function_no_name", "function () { }\n"),
    ("function_no_body", "function f()\n"),
    ("class_no_body", "class C\n"),
    ("class_bad_member", "class C { x }\n"),
    ("local_no_name", "local = 5;\n"),
    ("local_no_value", "local a;\n"),
    ("assign_to_literal", "1 = 2;\n"),
    ("reserved_word", "local class = 5;\n"),
    ("bad_operator", "local a = 1 ** 2;\n"),
    ("bad_operator2", "local a = 1 <> 2;\n"),
    ("ternary_bad", "local a = 1 ? 2;\n"),
    ("unclosed_ternary", "local a = 1 ? 2 : ;\n"),
    ("return_outside", "return 1;\n"),
    ("yield_outside", "yield 1;\n"),
    ("resume_bad", "local a = resume;\n"),
    ("stray_operator", "local a = *5;\n"),
    ("colon_stray", "local a = 1 : 2;\n"),
    ("semicolon_double", "local a = 1;;\n"),
    ("nested_comment", "/* a /* b */ local c = 1;\n"),
    ("bad_unicode_ident", "local \u00e9 = 5;\n"),
    ("bad_char_literal", "local a = 'ab';\n"),
    ("bad_class_attr", "class C { </ x = > y = 1; }\n"),
    ("bad_default_param", "function f(a, b = 1, ...) { }\n"),
    ("bad_varargs_pos", "function f(..., a) { }\n"),
    ("bad_static", "class C { static x; }\n"),
    ("bad_base", "class C { function f() { base(); } }\n"),
    ("bad_newslot_expr", "local a = 1; a <- ;\n"),
]


def compile_only(exe, path):
    out = path[:-4] + ".%s.cnut" % ("ref" if "ref" in exe else "rst")
    p = subprocess.run([exe, "-c", "-o", out, path], capture_output=True, cwd=ROOT, timeout=60)
    err = p.stderr.decode("utf8", "replace").replace("\r\n", "\n")
    return p.returncode, err.strip()


def main():
    verbose = "-v" in sys.argv
    same = diff = 0
    for tag, src in CASES:
        p = os.path.join(ROOT, "tmp-c2", "err_%s.nut" % tag)
        open(p, "w", encoding="utf-8", newline="\n").write(src)
        rc_a, err_a = compile_only(REF, p)
        rc_b, err_b = compile_only(RUST, p)
        ok = (err_a == err_b) and (rc_a == rc_b)
        if ok:
            same += 1
            if verbose:
                print("== %-24s 一致 rc=%s %s" % (tag, rc_a, err_a.splitlines()[0][:70] if err_a else ""))
        else:
            diff += 1
            print("** %-24s 不一致" % tag)
            print("     源码  %r" % src)
            print("     ref   rc=%s %s" % (rc_a, err_a.replace("\n", " | ")[:150]))
            print("     rst   rc=%s %s" % (rc_b, err_b.replace("\n", " | ")[:150]))
    print("\n共 %d 条：一致 %d，不一致 %d" % (len(CASES), same, diff))


if __name__ == "__main__":
    main()

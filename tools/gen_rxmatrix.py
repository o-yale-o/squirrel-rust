# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_rxmatrix.nut：正则语义组合矩阵。

设计原则：
- 每个用例 `s(tag, f)` 包 try/catch，只输出一行：语料必须整跑完（超时=FAIL）。
- 参考实现在正则上的若干形态是 **UB**，一律排除（见 STATUS 第四节）：
  * `regexp("")`：写 NULL 崩溃；
  * 主语不是字符串：`sq_getstring` 的返回值被忽略，`str` 未初始化就被解引用；
  * `search(subj, <负 start>)` / `start > len`：`str+start` 直接越界指针运算。
  语料里 start 只取 `0..len`（含 len）区间，且 start 类型非法由 typemask 拦下。
- 产品式组合：模式 × 主语（每个组合同时给 search / capture / subexpcount），
  外加非法模式 × 错误文本、API 边界、start 偏移。

用法：python tools/gen_rxmatrix.py  → 写 tests-corpus/t_rxmatrix.nut
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "tests-corpus", "t_rxmatrix.nut")


def nut(s):
    """转成 Squirrel 字符串字面量（显式转义，便于控制反斜杠）"""
    out = []
    for ch in s:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "\t":
            out.append("\\t")
        elif ord(ch) < 0x20 or ord(ch) > 0x7E:
            out.append("\\x%02x" % ord(ch))
        else:
            out.append(ch)
    return '"' + "".join(out) + '"'


L = []
A = L.append
A("// 自动生成：python tools/gen_rxmatrix.py —— 请勿手改")
A("// 正则语义组合矩阵（↔ sqstdlib/sqstdrex.cpp 引擎 + sqstdstring.cpp 的 rexobj 方法）")
A("")
A("function s(tag, f) {")
A("    try { print(tag + \"=\" + f() + \"\\n\"); }")
A("    catch (e) { print(tag + \"!\" + e + \"\\n\"); }")
A("}")
A("function rx(p) { return regexp(p); }")
A("// 结构化描述：{b,e} 表 / [{b,e},…] 数组 / 裸值")
A("function d(x) {")
A("    local t = typeof x;")
A("    if (t == \"table\") return \"{\" + x.begin + \",\" + x.end + \"}\";")
A("    if (t == \"array\") {")
A("        local o = \"[\";")
A("        for (local i = 0; i < x.len(); i++) {")
A("            if (i > 0) o += \",\";")
A("            o += \"{\" + x[i].begin + \",\" + x[i].end + \"}\";")
A("        }")
A("        return o + \"]\";")
A("    }")
A("    return \"\" + x;")
A("}")
A("// 一个模式对一个主语：search / capture / subexpcount / match 四件套")
A("function probe(p, subj) {")
A("    local r = rx(p);")
A("    local out = \"N\" + r.subexpcount();")
A("    out += \" S\" + d(r.search(subj));")
A("    out += \" C\" + d(r.capture(subj));")
A("    out += \" M\" + r.match(subj);")
A("    return out;")
A("}")
A("")

# ------------------------------------------------------------------ 1. 模式 × 主语
PATTERNS = [
    ("lit", "abc"),
    ("dot", "a.c"),
    ("bol", "^abc"),
    ("eol", "abc$"),
    ("class", "[abc]+"),
    ("nclass", "[^abc]+"),
    ("range", "[a-z]+"),
    ("cclass_d", r"\d+"),
    ("cclass_w", r"\w+"),
    ("cclass_s", r"\s"),
    ("nclass_D", r"\D+"),
    ("nclass_W", r"\W"),
    ("nclass_S", r"\S"),
    ("star", "ab*"),
    ("plus", "ab+"),
    ("opt", "ab?"),
    ("rep_n", "a{2}"),
    ("rep_nm", "a{1,3}"),
    ("rep_nopen", "a{2,}"),
    ("nongreedy", "a*?b"),
    ("alt", "abc|xyz"),
    ("group", "(ab)+"),
    ("grp_cap", "(a)(b)"),
    ("grp_opt", "(a)(b)?"),
    ("grp_nested", "((a)b)"),
    ("noncap", "(?:ab)+"),
    ("wb", r"\bfoo\b"),
    ("esc_dot", r"a\.c"),
    ("cclassexpr", "[[:alpha:]]+"),
    ("cclassexpr_d", "[[:digit:]]+"),
    ("anchor_both", "^a+$"),
    ("star_all", ".*"),
    ("empty_alt", "a|"),
]
SUBJECTS = [
    ("empty", ""),
    ("abc", "abc"),
    ("xabcy", "xabcy"),
    ("ABC", "ABC"),
    ("nl", "a\nb"),
    ("digit", "a1b22c333"),
    ("spaces", "  a  b "),
    ("dots", "a.c abc"),
    ("brackets", "[abc]"),
    ("high", "\xe9a\xe9"),
    ("many", "aaaaaaaaaa"),
    ("fooled", "a foo b"),
    ("xyz", "xyz"),
]
for pname, pat in PATTERNS:
    for sname, subj in SUBJECTS:
        A('s("m_%s_%s", function() { return probe(%s, %s); });' % (pname, sname, nut(pat), nut(subj)))
A("")

# ------------------------------------------------------------------ 2. 非法模式 → 错误文本
BAD = [
    ("open_paren", "("),
    ("close_paren", ")"),
    ("open_bracket", "["),
    ("empty_class", "[]"),
    ("unfinished_range", "[a-"),
    ("invalid_range", "[z-a]"),
    ("class_in_range", r"[\d-z]"),
    ("same_balanced", r"\p{aa}"),
    ("bad_balanced", r"\p{a"),
    ("escape_eof", "\\"),
    ("bad_escape", r"\q"),
    ("brace_open", "a{"),
    ("brace_comma", "a{,"),
    ("brace_badend", "a{2,1}"),
    ("brace_overflow", "a{1000000000}"),
    ("bad_group_prefix", "(?<a>"),
    ("unexpected_char", "(?a)"),
]
for name, pat in BAD:
    A('s("bad_%s", function() { local r = rx(%s); return "ok:" + r.subexpcount(); });' % (name, nut(pat)))
A("")

# ------------------------------------------------------------------ 3. API 边界
A("// ---- 3. API 边界：start 偏移、类型、constructor 重用、typeof/tostring")
A("s(\"api_subexpcount0\", function() { return regexp(\"abc\").subexpcount(); });")
A("s(\"api_subexpcount3\", function() { return regexp(\"(a)(b(c))?\").subexpcount(); });")
A("s(\"api_subexpcount_noncap\", function() { return regexp(\"(?:a)(b)\").subexpcount(); });")
A("s(\"api_typeof\", function() { return typeof regexp(\"a\"); });")
A("s(\"api_tostring\", function() { local r = regexp(\"a\"); try { return r.tostring(); } catch (e) { return \"E:\" + e; } });")
A("s(\"api_reconstruct\", function() { local r = regexp(\"a\"); try { return \"\" + r.constructor(\"b\"); } catch (e) { return \"E:\" + e; } });")
A("s(\"api_classof\", function() { return regexp(\"a\").getclass(); });")
A("s(\"api_search_nonstr_subject\", function() { try { return \"\" + regexp(\"a\").search(5); } catch (e) { return \"E:\" + e; } });")
A("s(\"api_search_null_subject\", function() { try { return \"\" + regexp(\"a\").search(null); } catch (e) { return \"E:\" + e; } });")
A("s(\"api_search_badstart_type\", function() { try { return \"\" + regexp(\"a\").search(\"abc\", null); } catch (e) { return \"E:\" + e; } });")
A("s(\"api_search_strstart\", function() { try { return \"\" + regexp(\"a\").search(\"abc\", \"1\"); } catch (e) { return \"E:\" + e; } });")
A("s(\"api_search_floatstart\", function() { return d(regexp(\"a\").search(\"aaa\", 1.7)); });")
A("s(\"api_match_nonstr\", function() { try { return \"\" + regexp(\"a\").match(1); } catch (e) { return \"E:\" + e; } });")
A("s(\"api_match_extra_args\", function() { try { return \"\" + regexp(\"a\").match(\"a\", 0); } catch (e) { return \"E:\" + e; } });")
A("s(\"api_capture_nonstr\", function() { try { return \"\" + regexp(\"a\").capture(1); } catch (e) { return \"E:\" + e; } });")
A("s(\"api_search_notable\", function() { local r = regexp(\"z\"); return typeof r.search(\"abc\"); });")
A("s(\"api_capture_notable\", function() { local r = regexp(\"z\"); return typeof r.capture(\"abc\"); });")
A("s(\"api_match_nomatch\", function() { return regexp(\"z\").match(\"abc\"); });")
A("s(\"api_two_instances\", function() { local a = regexp(\"a\"); local b = regexp(\"b\"); return d(a.search(\"ab\")) + \"/\" + d(b.search(\"ab\")); });")
A("s(\"api_ctor_wrong_type\", function() { try { return \"\" + regexp(5); } catch (e) { return \"E:\" + e; } });")
A("s(\"api_ctor_null\", function() { try { return \"\" + regexp(null); } catch (e) { return \"E:\" + e; } });")
A("")
# start 偏移：只在 0..len 内（参考实现里 start>len 或负数都是越界指针运算）
A("// ---- 4. start 偏移（0..len；越界区间在参考实现里是 UB，已排除）")
for sname, subj in [("abc", "abc"), ("aab", "aab"), ("spaces", "a b c")]:
    for st in range(0, len(subj) + 1):
        A('s("off_%s_%d", function() { return d(regexp("a").search(%s, %d)) + "/" + d(regexp("a").capture(%s, %d)); });'
          % (sname, st, nut(subj), st, nut(subj), st))
A("")

# ------------------------------------------------------------------ 5. 引擎细节
A("// ---- 5. 引擎细节：量词贪婪/非贪婪、嵌套分组、回溯、大小写、高位字节")
details = [
    ("greedy_vs_lazy", 'd(regexp("<.*>").search("<a><b>")) + "/" + d(regexp("<.*?>").search("<a><b>"))'),
    ("backtrack", 'd(regexp("a*a*b").search("aaab")) + "/" + d(regexp("(a+)+b").search("aaab"))'),
    ("group_repeat_capture", 'd(regexp("(a)+").capture("aaa"))'),
    ("nested_optional", 'd(regexp("(a(b)?c)").capture("ac")) + "/" + d(regexp("(a(b)?c)").capture("abc"))'),
    ("case_sensitive", 'd(regexp("abc").search("ABC"))'),
    ("dot_no_newline", 'd(regexp("a.b").search("a\\nb")) + "/" + d(regexp("(?s)a.b").search("a\\nb"))'),
    ("high_byte_class", 'd(regexp("[\\xe9]").search("\\xe9"))'),
    ("high_byte_dot", 'd(regexp("a.c").search("a\\xe9c"))'),
    ("anchor_after_start", 'd(regexp("^b").search("ab", 1))'),
    ("eol_with_start", 'd(regexp("a$").search("aba", 0))'),
    ("empty_match", 'd(regexp("a*").search("bbb"))'),
    ("empty_pattern_group", 'd(regexp("()").capture("x"))'),
    ("alternation_order", 'd(regexp("a|ab").search("ab")) + "/" + d(regexp("ab|a").search("ab"))'),
    ("brace_exact_fail", 'd(regexp("a{3}").search("aa"))'),
    ("cclass_neg_digit", 'd(regexp("\\\\D+").search("12ab34"))'),
    ("wb_edges", 'd(regexp("\\\\bfoo").search("xfoo")) + "/" + d(regexp("foo\\\\b").search("foox"))'),
    ("cclassexpr_neg", 'd(regexp("[[:^digit:]]+").search("12ab"))'),
    ("ccclass_upper", 'd(regexp("[[:upper:]]+").search("aBCd"))'),
    ("space_class_exact", 'd(regexp("[ \\\\t]+").search("a \\tb"))'),
]
for name, expr in details:
    A('s("det_%s", function() { return %s; });' % (name, expr))
A("")

A('print("RXMATRIX-DONE\\n");')
A("")

open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(L))
print("wrote %s (%d lines, %d 用例)" % (OUT, len(L), len([x for x in L if x.startswith("s(")])))

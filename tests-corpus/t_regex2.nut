// 正则引擎细节（↔ sqstdrex.cpp）：量词、{n,m}、^ 位置、subexpcount、错误文本
function m(tag, pat, s) {
    try {
        print(tag + " => " + regexp(pat).match(s) + "\n");
    } catch (e) {
        print(tag + " !! " + e + "\n");
    }
}
m("lit", "abc", "xabcx");
m("star", "ab*c", "ac");
m("opt", "ab?c", "ac");
// 参考实现的量词 switch 只走一次：`*?` 里的 `?` 是字面问号
m("q_after_star_hit", "a*?", "aaa?");
m("q_after_star_miss", "a*?", "aaa");
m("q_after_plus", "a+?", "aa?");
m("rep_exact", "a{2}", "aa");
m("rep_exact_miss", "a{2}", "a");
m("rep_range", "a{2,3}", "aaa");
m("rep_range_miss", "a{2,3}", "aaaa");
m("rep_atleast", "a{2,}", "aaaa");
m("rep_zero", "a{0,1}b", "b");
m("rep_zero2", "a{0}b", "b");
m("class", "[a-c]+", "zzabc");
m("negclass", "[^a-c]+", "abcxyz");
m("alt", "a|b", "b");
m("anchor_bol", "^abc", "abcx");
m("anchor_bol_miss", "^abc", "xabc");
m("caret_mid_literal", "a^b", "a^b");
m("caret_after_alt", "x|^ab", "ab");
m("dollar_literal_mid", "a$b", "a$b");
m("wordbound", "\\bfoo\\b", "a foo b");
m("digit", "\\d+", "a123b");
m("space", "\\s+", "a  b");
m("escaped_dot", "a\\.c", "a.c");
m("balanced2", "\\m<>", "<abc>");
m("dollar_in_class", "[$]", "$");
m("badpat_open", "a[", "a");
m("badpat_paren", "a(", "a");
m("badpat_flag", "(?i)abc", "ABC");
m("badpat_brace", "a{", "a");
m("badpat_brace2", "a{1x", "a");
print("sub(a)=" + regexp("(a)").subexpcount() + " sub((a)(b))=" + regexp("((a)(b))").subexpcount()
    + " sub(?:a)=" + regexp("(?:a)").subexpcount() + " sub(abc)=" + regexp("abc").subexpcount() + "\n");
print("typeof=" + typeof regexp("a") + "\n");
try { regexp("abc", 'i'); } catch (e) { print("flags: " + e + "\n"); }
// capture：未参与匹配的组给空匹配表（不是 null）
local r = regexp("(a)?b").capture("b");
print("caplen=" + r.len() + " t0=" + typeof r[0] + " t1=" + typeof r[1] + "\n");
print("b0=" + r[1].begin + " e0=" + r[1].end + "\n");
local r2 = regexp("(\\d+)-(\\d+)").capture("ab 12-34 cd");
print("caplen2=" + r2.len() + " " + r2[0].begin + "," + r2[0].end + " " + r2[1].begin + "," + r2[1].end
    + " " + r2[2].begin + "," + r2[2].end + "\n");
local s = regexp("(\\d+)").search("ab123cd");
print("search=" + s.begin + "," + s.end + "\n");
print("search_null=" + (regexp("(\\d+)").search("abc") == null) + "\n");
// 更多编译期错误与边界（↔ sqstd_rex_charnode / class）
// 注：空模式 `regexp("")` 会让参考实现写 NULL 崩溃（弹模态崩溃框），不作为语料
m("tab_pattern", "a\tb", "a\tb");
m("nonascii", "\xe9", "\xe9");
m("empty_class", "[]", "]");
m("esc_at_end", "a\\", "a\\");
m("class_unterm", "[abc", "a");

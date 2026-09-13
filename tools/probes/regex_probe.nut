// 正则引擎行为对照（↔ sqstdrex.cpp）
function m(tag, pat, s) {
    try {
        print(tag + " => " + regexp(pat).match(s) + "\n");
    } catch (e) {
        print(tag + " !! " + e + "\n");
    }
}
m("lit", "abc", "xabcx");
m("dot", "a.c", "abc");
m("star", "ab*c", "ac");
m("plus", "ab+c", "abbc");
m("opt", "ab?c", "ac");
m("nongreedy_star", "a.*?b", "axxbxxb");
m("nongreedy_plus", "a.+?b", "axxb");
m("rep_exact", "a{2}", "aa");
m("rep_range", "a{2,3}", "aaa");
m("rep_atleast", "a{2,}", "aaaa");
m("rep_zero", "a{0,1}b", "b");
m("class", "[a-c]+", "zzabc");
m("negclass", "[^a-c]+", "abcxyz");
m("alt", "a|b", "b");
m("group", "(ab)+", "abab");
m("anchor_bol", "^abc", "abcx");
m("anchor_eol", "abc$", "xabc");
m("wordbound", "\\bfoo\\b", "a foo b");
m("digit", "\\d+", "a123b");
m("nondigit", "\\D+", "12ab34");
m("word", "\\w+", " a_b ");
m("space", "\\s+", "a  b");
m("escaped_dot", "a\\.c", "a.c");
m("backref", "(a)\\1", "aa");
m("balanced", "\\m<>.+?\\m<>", "<a<b>c>");
m("balanced2", "\\m<>", "<abc>");
m("i_flag_pattern", "(?i)abc", "ABC");
m("dollar_in_class", "[$]", "$");
function c(tag, pat, s) {
    try {
        local r = regexp(pat).capture(s);
        if (r == null) print(tag + " => null\n");
        else {
            local out = "";
            foreach (v in r) out += "[" + v + "]";
            print(tag + " => " + out + "\n");
        }
    } catch (e) {
        print(tag + " !! " + e + "\n");
    }
}
c("cap_group", "(\\d+)-(\\d+)", "ab 12-34 cd");
c("cap_nested", "((a)(b))", "ab");
c("cap_opt", "(a)?b", "b");
c("cap_star", "(ab)*", "ababab");
function s(tag, pat, str) {
    try {
        local r = regexp(pat).search(str);
        if (r == null) print(tag + " => null\n");
        else print(tag + " => " + r[0] + "|" + r[1] + "\n");
    } catch (e) {
        print(tag + " !! " + e + "\n");
    }
}
s("search", "(\\d+)", "ab123cd");
s("search_none", "(\\d+)", "abc");
s("subexpcount", "((a)(b))", "ab");
t <- regexp("(a)");
print("subexp=" + t.subexpcount() + " type=" + typeof t + "\n");
try { regexp("abc", 'i'); } catch (e) { print("flags: " + e + "\n"); }
try { regexp("a(", 'x'); } catch (e) { print("badpat: " + e + "\n"); }

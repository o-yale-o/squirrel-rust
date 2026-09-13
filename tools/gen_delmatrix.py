# -*- coding: utf-8 -*-
"""生成「委托方法 × 参数形态」矩阵语料 tests-corpus/t_delmatrix.nut。

前几轮的矩阵只覆盖了运算符 / metamethod / 格式串；**委托方法的语义**（尤其是有
合法参数时）一直只有零散语料。这里按官方 `*_default_delegate_funcz` 表逐个成员
生成调用（成员名直接从 C++ 表解析，保证不漏），参数形态覆盖空参 / 整数 / 负数 /
浮点 / null / 布尔 / 字符串 / 空字符串 / 数组 / 表 / 闭包 / 比较器 / 多参数等。

两个必要的安全约束（否则参考实现会崩，属于它自己的 UB）：

- `string.tointeger(base)`：base 传给 CRT `strtol`，1/负数/>36 是未定义行为
  （实测 STATUS_STACK_BUFFER_OVERRUN），因此这个成员只喂「非数值首参」形态；
- `file` 类型整体排除（会碰 stdin/stdout、有文件系统副作用）。

用法：
    python tools/gen_delmatrix.py           # 写出 tests-corpus/t_delmatrix.nut
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_delmatrix.nut")

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import audit_delegates as ad  # noqa: E402

# 参数形态：名字 → 实参文本
SHAPES = [
    ("none", ""),
    ("i0", "0"),
    ("i1", "1"),
    ("im1", "-1"),
    ("f", "1.5"),
    ("null", "null"),
    ("bool", "true"),
    ("s", '"s"'),
    ("es", '""'),
    ("arr", "[1, 2]"),
    ("tbl", "{ k = 1 }"),
    ("fn0", "function() {}"),
    ("fn1", "function(x) { return x; }"),
    ("cmp", "@(x, y) x <=> y"),
    ("ii", "1, 2"),
    ("si", '"a", 1'),
    ("is", '1, "a"'),
    ("n99", "-99"),
    ("p99", "99"),
    ("bl", "blob(2)"),
]

# 紧凑形态（次要接收者用，控制语料体积）
SHORT_SHAPES = ["none", "i1", "im1", "f", "null", "s", "arr", "tbl", "fn1", "cmp"]

# (标签, 接收者表达式, C++ 表名, 是否主接收者)
RECEIVERS = [
    ("tbl_e", "{}", "_table_default_delegate_funcz", True),
    ("tbl_f", "{ a = 1, b = 2 }", "_table_default_delegate_funcz", False),
    ("arr_e", "[]", "_array_default_delegate_funcz", True),
    ("arr_f", "[3, 1, 2]", "_array_default_delegate_funcz", False),
    ("str_e", '""', "_string_default_delegate_funcz", True),
    ("str_f", '"Hello, World"', "_string_default_delegate_funcz", False),
    ("num_i", "(42)", "_number_default_delegate_funcz", True),
    ("num_f", "(2.5)", "_number_default_delegate_funcz", False),
    ("clo_s", "(function(a, b) { return a; })", "_closure_default_delegate_funcz", True),
    ("clo_n", "(print)", "_closure_default_delegate_funcz", False),
    ("gen", "((function() { yield 1; yield 2; })())", "_generator_default_delegate_funcz", True),
    ("thr", "(newthread(function() { return 1; }))", "_thread_default_delegate_funcz", True),
    ("cls", "(class { a = 1; static s = 2; function f() { return 1; } })",
     "_class_default_delegate_funcz", True),
    ("inst", "((class { a = 1; function f() { return 1; } })())",
     "_instance_default_delegate_funcz", True),
    ("wk", "(({}).weakref())", "_weakref_default_delegate_funcz", True),
]

# blob / regexp 不在默认委托表里（它们有自己的类），单独按成员列表生成
EXTRA_TYPES = [
    ("blob", "(blob(4))", "resize swap2 swap4 swapfloat seek tell len eos readblob readn writeblob writen flush getdelegate"),
    ("re", '(regexp("a(b)c"))', "search match capture subexpcount constructor getdelegate"),
]

# 参考实现会崩的参数形态（它自己的 UB），逐成员排除
BLOCK = {
    # CRT strtol 收到非法进制 → STATUS_STACK_BUFFER_OVERRUN
    ("str_e", "tointeger"): {"i0", "i1", "im1", "f", "ii", "is", "n99", "p99"},
    ("str_f", "tointeger"): {"i0", "i1", "im1", "f", "ii", "is", "n99", "p99"},
    # split(<非串>, <非串>)：参考实现不检查 sq_getstring 的返回值 → 读未初始化指针崩溃
    ("str_e", "split"): {"null", "bool", "arr", "tbl", "fn0", "fn1", "cmp", "ii", "is", "n99", "p99", "bl"},
    ("str_f", "split"): {"null", "bool", "arr", "tbl", "fn0", "fn1", "cmp", "ii", "is", "n99", "p99", "bl"},
    # blob.resize(负数)：SQBlob::Resize 直接 realloc(负长度) → 访问违例（实测 0xC0000005）
    ("blob", "resize"): {"im1", "n99"},
    # blob.readblob(负数)：`sq_getscratchpad(v,-1)` + `fread(buf,1,(size_t)-1)` → 访问违例
    ("blob", "readblob"): {"im1", "n99"},
    # thread.getstackinfos(负数)：`cssize > level` 对负 level 恒真 → 读空的调用栈
    # （ASan 实测 `sq_stackinfos` sqdebug.cpp:35 heap-buffer-overflow，偶发访问违例）
    ("thr", "getstackinfos"): {"im1", "n99"},
}


def receiver_names(label, table):
    """某接收者对应的委托成员（按 C++ 表，保证不漏）+ 额外类型的手写成员表"""
    for lab, _, members in EXTRA_TYPES:
        if label.startswith(lab + "_") or label == lab:
            return [m for m in members.split() if m]
    return sorted(table)


lines = [
    "// 自动生成：委托方法 × 参数形态矩阵（tools/gen_delmatrix.py）",
    "// 成员名取自 C++ 的 *_default_delegate_funcz 表，参数形态覆盖空参/数值/字符串/容器/闭包/比较器",
    "// show()：容器渲染成可比较的文本（否则打印出来只有 (array : 0xPTR)）",
    "function show(r) {",
    "    local t = typeof r;",
    "    if (t == \"array\") {",
    "        local s = \"[\";",
    "        for (local i = 0; i < r.len(); i++) { if (i > 0) s += \",\"; s += show(r[i]); }",
    "        return s + \"]\";",
    "    }",
    "    if (t == \"table\") {",
    "        local s = \"{\";",
    "        foreach (k, v in r) s += \"\" + k + \":\" + show(v) + \",\";",
    "        return s + \"}\";",
    "    }",
    "    if (t == \"blob\") {",
    "        local s = \"<\";",
    "        for (local i = 0; i < r.len(); i++) { if (i > 0) s += \",\"; s += r[i]; }",
    "        return s + \">\";",
    "    }",
    "    if (t == \"string\") return \"\\\"\" + r + \"\\\"\";",
    "    return \"\" + r;",
    "}",
    "function s(tag, f) {",
    "    try { print(tag + \"=\" + show(f()) + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "// 具名函数只能声明（function name() {}），不能当表达式赋值给 local",
    "function named_probe() {}",
    "",
]

cpp = ad.cpp_tables()
n_auto = 0
for label, recv, table, primary in RECEIVERS:
    members = receiver_names(label, cpp[table] or {})
    shapes = SHAPES if primary else [s for s in SHAPES if s[0] in SHORT_SHAPES]
    for mem in members:
        for sname, args in shapes:
            if sname in BLOCK.get((label, mem), set()):
                continue
            n_auto += 1
            lines.append(
                's("d_%s_%s_%s", function() { return (%s).%s(%s); });' % (label, mem, sname, recv, mem, args)
            )
for label, recv, members in EXTRA_TYPES:
    for mem in members.split():
        for sname, args in SHAPES:
            if sname in BLOCK.get((label, mem), set()):
                continue
            n_auto += 1
            lines.append(
                's("d_%s_%s_%s", function() { return (%s).%s(%s); });' % (label, mem, sname, recv, mem, args)
            )

# ---------------------------------------------------------------- 语义用例
SEM = [
    # ---- table
    ("t_keys", 'return { a = 1, b = 2, c = 3 }.keys();'),
    ("t_values", 'return { a = 1, b = 2, c = 3 }.values();'),
    ("t_len", 'return { a = 1, b = 2 }.len();'),
    ("t_rawin", 'local t = { a = 1 }; return "" + t.rawin("a") + t.rawin("z");'),
    ("t_rawget", 'return { a = 7 }.rawget("a");'),
    ("t_rawset", 'local t = {}; t.rawset("k", 5); return t.rawget("k");'),
    ("t_rawdelete", 'local t = { a = 1 }; t.rawdelete("a"); return t.len();'),
    ("t_rawdelete_missing", 'local t = { a = 1 }; return t.rawdelete("z");'),
    ("t_clear", 'local t = { a = 1, b = 2 }; t.clear(); return t.len();'),
    ("t_filter", 'return { a = 1, b = 2, c = 3 }.filter(@(k, v) v > 1);'),
    ("t_filter_keys", 'return { a = 1, b = 2 }.filter(@(k, v) k == "b");'),
    ("t_map", 'return { a = 1, b = 2 }.map(@(k, v) v * 10);'),
    ("t_setdelegate", 'local d = { z = 9 }; local t = {}; t.setdelegate(d); return t.z;'),
    ("t_getdelegate", 'local d = {}; local t = {}; t.setdelegate(d); return t.getdelegate() == d;'),
    ("t_tostring_mt", 'local t = {}; t.setdelegate({ _tostring = function() { return "T"; } }); return t.tostring();'),
    ("t_weakref_alive", 'local t = { a = 1 }; local w = t.weakref(); return w.ref().a;'),
    ("t_rawget_chain", 'local d = { a = 1 }; local t = {}; t.setdelegate(d); return t.rawget("a");'),
    ("t_len_after_del", 'local t = { a = 1, b = 2 }; delete t.a; return t.len();'),

    # ---- array
    ("a_append", 'local a = [1]; a.append(2); return a;'),
    ("a_push", 'local a = [1]; a.push(2); return a;'),
    ("a_pop", 'local a = [1, 2]; return a.pop();'),
    ("a_pop_empty", 'local a = []; return a.pop();'),
    ("a_top", 'return [1, 2].top();'),
    ("a_insert", 'local a = [1, 3]; a.insert(1, 2); return a;'),
    ("a_insert_head", 'local a = [2, 3]; a.insert(0, 1); return a;'),
    ("a_insert_end", 'local a = [1, 2]; a.insert(2, 3); return a;'),
    ("a_insert_oob", 'local a = [1]; a.insert(9, 2); return a;'),
    ("a_remove", 'local a = [1, 2, 3]; return a.remove(1) + "," + a;'),
    ("a_remove_last", 'local a = [1, 2]; return a.remove(1) + "," + a;'),
    ("a_remove_oob", 'local a = [1]; return a.remove(9);'),
    ("a_resize_grow", 'local a = [1]; a.resize(3); return a.len();'),
    ("a_resize_shrink", 'local a = [1, 2, 3]; a.resize(1); return a;'),
    ("a_reverse", 'local a = [1, 2, 3]; a.reverse(); return a;'),
    ("a_sort", 'local a = [3, 1, 2]; a.sort(); return a;'),
    ("a_sort_cmp", 'local a = [3, 1, 2]; a.sort(@(x, y) y <=> x); return a;'),
    ("a_sort_strings", 'local a = ["b", "a", "c"]; a.sort(); return a;'),
    ("a_slice", 'return [1, 2, 3, 4].slice(1, 3);'),
    ("a_slice_neg", 'return [1, 2, 3, 4].slice(-2);'),
    ("a_slice_oob", 'return [1, 2, 3].slice(0, 99);'),
    ("a_map", 'return [1, 2, 3].map(@(v) v * 2);'),
    ("a_apply", 'local a = [1, 2, 3]; a.apply(@(v) v * 2); return a;'),
    ("a_reduce", 'return [1, 2, 3].reduce(@(acc, v) acc + v);'),
    ("a_reduce_init", 'return [1, 2, 3].reduce(@(acc, v) acc + v, 10);'),
    ("a_reduce_empty", 'return [].reduce(@(acc, v) acc + v);'),
    ("a_filter", 'return [1, 2, 3, 4].filter(@(v) v % 2 == 0);'),
    ("a_find", 'return [1, 2, 3].find(2);'),
    ("a_find_missing", 'return [1, 2, 3].find(9);'),
    ("a_extend", 'local a = [1]; a.extend([2, 3]); return a;'),
    ("a_extend_str", 'local a = [1]; a.extend("ab"); return a;'),
    ("a_clear", 'local a = [1, 2]; a.clear(); return a.len();'),
    ("a_len", 'return [1, 2, 3].len();'),
    ("a_tostring_mt", 'local a = [1]; local d = { _tostring = function() { return "A"; } }; a.setdelegate(d); return a.tostring();'),

    # ---- string
    ("s_len", 'return "hello".len();'),
    ("s_slice", 'return "hello".slice(1, 3);'),
    ("s_slice_neg", 'return "hello".slice(-3);'),
    ("s_slice_neg_both", 'return "hello".slice(-4, -1);'),
    ("s_slice_oob", 'return "hello".slice(0, 99);'),
    ("s_find", 'return "hello".find("ll");'),
    ("s_find_start", 'return "hello".find("l", 3);'),
    ("s_find_missing", 'return "hello".find("z");'),
    ("s_tointeger", 'return "42abc".tointeger();'),
    ("s_tointeger_hex", 'return "0x1f".tointeger(16);'),
    ("s_tointeger_auto", 'return "0x1f".tointeger(0);'),
    ("s_tofloat", 'return "3.5xyz".tofloat();'),
    ("s_tolower", 'return "HeLLo".tolower();'),
    ("s_toupper", 'return "HeLLo".toupper();'),
    ("s_tochar", 'return "A".tochar();'),
    ("s_tochar_long", 'return "AB".tochar();'),
    ("s_index_neg", 'return "hello"[-1];'),
    ("s_weakref", 'local s2 = "x"; local w = s2.weakref(); return "" + (w.ref() == s2);'),

    # ---- number
    ("n_tointeger", 'return (3.9).tointeger();'),
    ("n_tointeger_neg", 'return (-3.9).tointeger();'),
    ("n_tofloat", 'return (3).tofloat();'),
    ("n_tochar", 'return (65).tochar();'),
    ("n_tostring", 'return (2.5).tostring();'),
    ("n_len", 'return (1).len();'),

    # ---- closure
    ("c_call", 'local f = function(a, b) { return a + b; }; return f.call(1, 2);'),
    ("c_call_this", 'local f = function() { return this; }; return typeof f.call({});'),
    ("c_pcall_ok", 'local f = function() { return 7; }; local ok = f.pcall(); return "" + ok;'),
    ("c_pcall_err", 'local f = function() { throw "x"; }; local ok = f.pcall(); return "" + ok;'),
    ("c_acall", 'local f = function(a) { return a * 2; }; return f.acall([21]);'),
    ("c_pacall", 'local f = function(a) { return a * 2; }; local ok = f.pacall([21]); return "" + ok;'),
    ("c_bindenv", 'local e = { v = 10 }; local f = function() { return v; }.bindenv(e); return f();'),
    ("c_getroot", 'return typeof (function() {}).getroot();'),
    ("c_setroot", 'local f = function() { return 1; }; local old = f.getroot(); f.setroot(old); return f();'),
    ("c_getinfos_name", 'return named_probe.getinfos().name;'),
    ("c_getinfos_params", 'local f = function(a, b) {}; return f.getinfos().parameters;'),
    ("c_getinfos_varargs", 'local f = function(a, ...) {}; return "" + f.getinfos().varargs;'),
    ("c_default_params", 'local f = function(a, b = 5) { return a + b; }; return f(1);'),

    # ---- generator / thread
    ("g_status_new", 'local g = (function() { yield 1; }()); return g.getstatus();'),
    ("g_status_dead", 'local g = (function() { yield 1; }()); resume g; return g.getstatus();'),
    ("th_status_idle", 'local t = newthread(function() {}); return t.getstatus();'),
    ("th_call_ret", 'local t = newthread(function() { return 5; }); return t.call();'),
    ("th_call_arg", 'local t = newthread(function(a) { return a * 2; }); return t.call(21);'),
    ("th_getstackinfos", 'local t = newthread(function() { return 1; }); return typeof t.getstackinfos(0);'),

    # ---- class / instance
    ("cl_getbase", 'class B { } class D extends B { } return D.getbase() == B;'),
    ("cl_getbase_none", 'class B { } return B.getbase();'),
    ("cl_attrs", 'class C { </ doc = "d" /> x = 1; } return C.getattributes("x").doc;'),
    ("cl_setattrs", 'class C { x = 1; } C.setattributes("x", { doc = "d" }); return C.getattributes("x").doc;'),
    ("cl_rawget", 'class C { static s = 3; } return C.rawget("s");'),
    ("cl_rawset", 'class C { static s = 3; } C.rawset("s", 4); return C.s;'),
    ("cl_rawin", 'class C { static s = 3; } return "" + C.rawin("s") + C.rawin("z");'),
    ("cl_newmember", 'class C { } C.newmember("f", function() { return 9; }); return C().f();'),
    ("cl_rawnewmember", 'class C { } C.rawnewmember("f", function() { return 9; }); return C().f();'),
    ("cl_instance", 'class C { } local c = C(); return c.getclass() == C;'),
    ("cl_instanceof", 'class C { } local c = C(); return "" + (c instanceof C);'),
    ("cl_static_shared", 'class C { static n = 0; function inc() { n++; } } local a = C(), b = C(); a.inc(); b.inc(); return C.n;'),
    ("cl_field_per_instance", 'class C { x = 0; } local a = C(), b = C(); a.x = 5; return "" + b.x;'),
    ("cl_base_call", 'class B { function f() { return "B"; } } class D extends B { function f() { return "D" + base.f(); } } return D().f();'),
    ("cl_ctor_default", 'class C { x = 1; constructor(a = 5) { x = a; } } return C().x;'),
    ("cl_ctor_varargs", 'class C { n = 0; constructor(...) { n = vargv.len(); } } return C(1, 2, 3).n;'),
    ("cl_tostring_mt", 'class C { function _tostring() { return "C!"; } } return "" + C();'),
    ("cl_setdelegate_mt", 'local t = {}; t.setdelegate({ _get = function(k) { return "g"; } }); return t.anything;'),

    # ---- weakref
    ("w_ref_live", 'local t = {}; local w = t.weakref(); return "" + (w.ref() == t);'),
    ("w_ref_dead", 'local t = {}; local w = t.weakref(); t = null; collectgarbage(); return w.ref();'),
    ("w_weakref_self", 'local t = {}; local w = t.weakref(); return w.weakref() == w;'),
    ("w_tostring", 'local t = {}; return typeof t.weakref().tostring();'),

    # ---- blob / stream
    ("b_len", 'return blob(4).len();'),
    ("b_tell", 'return blob(4).tell();'),
    ("b_eos_fresh", 'return "" + blob(4).eos();'),
    ("b_resize", 'local b = blob(2); b.resize(4); return b.len();'),
    ("b_write_read", 'local b = blob(2); b.seek(0); b.writen(65, 0x62); b.seek(0); return b.readn(0x62);'),
    ("b_writeblob", 'local b = blob(4); local src = blob(2); src.seek(0); src.writen(1, 0x62); src.writen(2, 0x62); src.seek(0); b.seek(0); b.writeblob(src); b.seek(0); local r = b.readblob(2); return r[0] + "," + r[1];'),
    ("b_swap2", 'local b = blob(2); b.seek(0); b.writen(0x0102, 0x77); b.seek(0); b.swap2(); b.seek(0); return b.readn(0x77);'),
    ("b_swap4", 'local b = blob(4); b.seek(0); b.writen(0x01020304, 0x69); b.seek(0); b.swap4(); b.seek(0); return b.readn(0x69);'),
    ("b_index", 'local b = blob(2); b[0] = 7; return b[0];'),
    ("b_foreach", 'local b = blob(3); b[0] = 1; b[1] = 2; local s = 0; foreach (v in b) s += v; return s;'),
    ("b_clone", 'local b = blob(2); b[0] = 9; local c = clone b; return c[0];'),
    ("b_readblob_past_end", 'local b = blob(1); return b.readblob(4).len();'),
    ("b_readn_past_end", 'local b = blob(1); return b.readn(0x62);'),
    ("b_seek_end", 'local b = blob(4); b.seek(0, 0x65); return b.tell();'),
    ("b_flush", 'return "" + blob(1).flush();'),

    # ---- regexp
    ("re_search", 'local r = regexp("b"); return r.search("abc");'),
    ("re_search_missing", 'local r = regexp("z"); return r.search("abc");'),
    ("re_search_start", 'local r = regexp("a"); return r.search("aaa", 1);'),
    ("re_match", 'local r = regexp("a(b)c"); local m = r.match("xabcx"); return m[0] + "," + m[1];'),
    ("re_match_missing", 'local r = regexp("z"); return r.match("abc");'),
    ("re_capture", 'local r = regexp("a(b)(c)"); local m = r.match("abc"); local c = r.capture("abc"); return c[0] + c[1];'),
    ("re_subexpcount", 'return regexp("a(b)(c)").subexpcount();'),
    ("re_subexpcount_plain", 'return regexp("abc").subexpcount();'),
    ("re_constructor", 'return regexp("a").getclass();'),
    ("re_typeof", 'return typeof regexp("a");'),
    ("re_tostring", 'return typeof regexp("a").tostring();'),
    ("re_classes", 'local r = regexp("[a-z]+"); return r.search("123abc");'),
    ("re_anchors", 'local r = regexp("^ab"); return r.search("xab");'),
    ("re_alt", 'local r = regexp("cat|dog"); return r.search("hotdog");'),
    ("re_quant", 'local r = regexp("a{2,3}"); return r.search("aaaa");'),
]

for tag, body in SEM:
    lines.append('s("m_%s", function() { %s });' % (tag, body))

lines.append('print("DELMATRIX-DONE\\n");')

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")

tags = [t for t, _ in SEM]
assert len(tags) == len(set(tags)), "语义用例 tag 重复"
print("已写出 %s：自动 %d 条 + 语义 %d 条" % (OUT, n_auto, len(SEM)))

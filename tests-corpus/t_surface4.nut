// 广度语料 4：继承元方法 / 字符串转义 / 脚本继承内建类 / 杂项语义
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

// ---- 类的继承与元方法 ----
t("mm_inherited", function() {
    local log = "";
    class A {
        static function _inherited(attributes) { log += "I"; }
    }
    class B extends A {}
    return log;
});
t("mm_newslot", function() {
    class A { function _newslot(k, v) { return "ns:" + k; } }
    local a = A();
    return "ok";
});
t("static_inherit", function() {
    class A { static v = 1; }
    class B extends A {}
    return B.v;
});
t("override_field", function() {
    class A { x = 1; function f() { return "A"; } }
    class B extends A { x = 2; function f() { return "B" + base.f(); } }
    local b = B();
    return "" + b.x + b.f();
});
t("base_chain", function() {
    class A { function who() { return "A"; } }
    class B extends A { function who() { return "B" + base.who(); } }
    class C extends B { function who() { return "C" + base.who(); } }
    return C().who();
});
t("ctor_chain", function() {
    local log = "";
    class A { constructor() { log += "A"; } }
    class B extends A { constructor() { base.constructor(); log += "B"; } }
    B();
    return log;
});
t("ctor_implicit", function() {
    local log = "";
    class A { constructor() { log += "A"; } }
    class B extends A {}
    B();
    return log;
});
t("class_no_ctor", function() { class A { x = 1; } return A().x; });
t("metamethod_inherit", function() {
    class A { function _add(o) { return "added"; } }
    class B extends A {}
    return B() + B();
});
t("instance_weakref_typeof", function() { class A {} return typeof A().weakref(); });
t("deep_inherit", function() {
    class A { function who() { return "A"; } }
    class B extends A {}
    class C extends B {}
    class D extends C {}
    return D().who() + (D() instanceof A);
});

// ---- 字符串字面量与转义 ----
t("esc_u", function() { return "\u0041\u00e9"; });
t("esc_x", function() { return "\x41\x42"; });
t("esc_tn", function() { return "a\tb\nc".len(); });
t("esc_quote", function() { return "a\"b".len(); });
t("verbatim", function() { return @"a\nb".len(); });
t("verbatim_quote", function() { return @"a""b".len(); });
t("verbatim_multiline", function() { return @"line1
line2".len(); });
t("comment_line", function() { return 1; /* block */ });
t("comment_nested", function() { /* a /* b */ return 2; });
t("string_adjacent", function() { return "a" + "b"; });
t("char_class_string", function() { return "[a-z]".len(); });
t("ident_dollar", function() { local _a1 = 1; return _a1; });
t("escape_in_ident", function() { local a = 1; return a; });

// ---- 脚本继承内建类 ----
t("extends_blob", function() { class MyBlob extends blob {} return typeof MyBlob); });
t("extends_stream", function() { class MyStream extends stream {} return typeof MyStream); });
t("extends_array_err", function() { class MyArr extends array {} return "no"; });
t("blob_subclass_use", function() {
    class B extends blob { }
    local b = B(4);
    return typeof b + ":" + b.len();
});
t("stream_subclass_use", function() {
    class S extends stream { }
    local s = S();
    s.writen(1, 'b');
    return s.len();
});
t("file_subclass_use", function() {
    class F extends file { }
    local f = F("robust_surface6.txt", "wb+");
    f.writen(65, 'b');
    local n = f.len();
    f.close();
    remove("robust_surface6.txt");
    return n;
});

// ---- 杂项语义 ----
t("switch_int", function() { local x = 2; switch (x) { case 1: return "one"; case 2: return "two"; default: return "def"; } });
t("switch_str", function() { local x = "b"; switch (x) { case "a": return "A"; case "b": return "B"; } return "none"; });
t("switch_default_first", function() { switch (9) { default: return "d"; case 1: return "one"; } });
t("ternary_nested", function() { return 1 ? (2 ? "a" : "b") : "c"; });
t("logical_short", function() { local n = 0; local f = function() { n++; return true; }; local r = false && f(); local r2 = true || f(); return n; });
t("comma_expr", function() { local a = (1, 2, 3); return a; });
t("nested_func_default", function() { local f = function(a, b = a + 1) { return b; }; return f(1); });
t("varargs_foreach", function() { local f = function(...) { local s = 0; foreach (v in vargv) s += v; return s; }; return f(1, 2, 3); });
t("closure_this", function() { local f = function() { return typeof this; }; return f(); });
t("closure_bindenv", function() { local env = { v = 5 }; local f = function() { return this.v; }; return f.bindenv(env)(); });
t("closure_getroot", function() { local f = function() {}; return typeof f.getroot(); });
t("closure_setroot", function() { local f = function() { return 1; }; f.setroot(getroottable()); return f(); });
t("closure_getinfos", function() { local f = function(a, b) {}; local i = f.getinfos(); return i.native + "/" + i.parameters.len() + "/" + i.name; });
t("native_getinfos", function() { local i = print.getinfos(); return i.native + "/" + i.name; });
t("enumerate_negative", function() { enum E { A = -1, B = 1 }; return E.A + "/" + E.B; });
t("enum_auto", function() { enum E2 { A, B, C }; return E2.A + E2.B + E2.C; });
t("const_expr", function() { const C = 1 + 2; return C; });
t("rawcall", function() { local f = function() { return 1; }; return f.rawcall(); });
t("if_no_braces", function() { if (true) return 1; else return 2; });
t("while_break", function() { local i = 0; while (true) { i++; if (i > 3) break; } return i; });
t("do_while", function() { local i = 0; do { i++; } while (i < 3); return i; });
t("for_foreach_kv", function() { local s = ""; foreach (i, v in [10, 20]) s += i + ":" + v + " "; return s; });

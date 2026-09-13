// 自动生成：类/实例语义矩阵（tools/gen_oopmatrix.py）
function show(r) {
    local t = typeof r;
    if (t == "array") {
        local s = "[";
        for (local i = 0; i < r.len(); i++) { if (i > 0) s += ","; s += show(r[i]); }
        return s + "]";
    }
    if (t == "table") {
        local s = "{";
        foreach (k, v in r) s += "" + k + ":" + show(v) + ",";
        return s + "}";
    }
    if (t == "string") return "\"" + r + "\"";
    return "" + r;
}
function s(tag, f) {
    try { print(tag + "=" + show(f()) + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

// ---- 类夹具 ----
class A {
    x = 1;
    static s = 2;
    </ doc = "fx" /> y = 3;
    function m() { return 10; }
    static function sm() { return 20; }
    constructor() { }
}

class B extends A {
    z = 30;
    function m() { return 11 + base.m(); }
    constructor() { base.constructor(); }
}

class C { a = 1; b = 2; static t = 3; function f() { return a + b; } }

class D extends C { }

class E extends D { a = 100; function f() { return a; } }

s("o_instA_read_field", function() { local o = A(); return o.x; });
s("o_instA_read_static", function() { local o = A(); return o.s; });
s("o_instA_read_missing", function() { local o = A(); return o.nope; });
s("o_instA_read_attr_field", function() { local o = A(); return o.y; });
s("o_instA_call_method", function() { local o = A(); return o.m(); });
s("o_instA_call_static", function() { local o = A(); return o.sm(); });
s("o_instA_call_missing", function() { local o = A(); return o.nomethod(); });
s("o_instA_typeof_field", function() { local o = A(); return typeof o.x; });
s("o_instA_typeof_method", function() { local o = A(); return typeof o.m; });
s("o_instA_rawget_field", function() { local o = A(); return o.rawget("x"); });
s("o_instA_rawget_missing", function() { local o = A(); return o.rawget("nope"); });
s("o_instA_rawin_field", function() { local o = A(); return "" + o.rawin("x"); });
s("o_instA_rawin_missing", function() { local o = A(); return "" + o.rawin("nope"); });
s("o_instA_in_field", function() { local o = A(); return "" + ("x" in o); });
s("o_instA_in_missing", function() { local o = A(); return "" + ("nope" in o); });
s("o_instA_getclass", function() { local o = A(); return typeof o.getclass(); });
s("o_instA_getbase", function() { local o = A(); return typeof o.getbase(); });
s("o_instA_attrs_field", function() { local o = A(); return o.getattributes("y"); });
s("o_instA_attrs_missing", function() { local o = A(); return o.getattributes("nope"); });
s("o_instA_delete_field", function() { local o = A(); return delete o.x; });
s("o_instA_set_field", function() { local o = A(); o.x = 99; return o.x; });
s("o_instA_newslot", function() { local o = A(); o.newone <- 1; return "ok"; });
s("o_instA_add_field", function() { local o = A(); o.extra = 5; return "ok"; });
s("o_instA_tostring", function() { local o = A(); return typeof ("" + o); });
s("o_instA_len", function() { local o = A(); return o.len(); });
s("o_instA_weakref", function() { local o = A(); return typeof o.weakref(); });
s("o_instA_keys_of_class", function() { local o = A(); return o.getclass().len(); });
s("o_instA_instances_share_class", function() { local o = A(); return "" + (o.getclass() == o.getclass()); });
s("o_classA_read_field", function() { local o = A; return o.x; });
s("o_classA_read_static", function() { local o = A; return o.s; });
s("o_classA_read_missing", function() { local o = A; return o.nope; });
s("o_classA_read_attr_field", function() { local o = A; return o.y; });
s("o_classA_call_method", function() { local o = A; return o.m(); });
s("o_classA_call_static", function() { local o = A; return o.sm(); });
s("o_classA_call_missing", function() { local o = A; return o.nomethod(); });
s("o_classA_typeof_field", function() { local o = A; return typeof o.x; });
s("o_classA_typeof_method", function() { local o = A; return typeof o.m; });
s("o_classA_rawget_field", function() { local o = A; return o.rawget("x"); });
s("o_classA_rawget_missing", function() { local o = A; return o.rawget("nope"); });
s("o_classA_rawin_field", function() { local o = A; return "" + o.rawin("x"); });
s("o_classA_rawin_missing", function() { local o = A; return "" + o.rawin("nope"); });
s("o_classA_in_field", function() { local o = A; return "" + ("x" in o); });
s("o_classA_in_missing", function() { local o = A; return "" + ("nope" in o); });
s("o_classA_getclass", function() { local o = A; return typeof o.getclass(); });
s("o_classA_getbase", function() { local o = A; return typeof o.getbase(); });
s("o_classA_attrs_field", function() { local o = A; return o.getattributes("y"); });
s("o_classA_attrs_missing", function() { local o = A; return o.getattributes("nope"); });
s("o_classA_delete_field", function() { local o = A; return delete o.x; });
s("o_classA_set_field", function() { local o = A; o.x = 99; return o.x; });
s("o_classA_newslot", function() { local o = A; o.newone <- 1; return "ok"; });
s("o_classA_add_field", function() { local o = A; o.extra = 5; return "ok"; });
s("o_classA_tostring", function() { local o = A; return typeof ("" + o); });
s("o_classA_len", function() { local o = A; return o.len(); });
s("o_classA_weakref", function() { local o = A; return typeof o.weakref(); });
s("o_classA_keys_of_class", function() { local o = A; return o.getclass().len(); });
s("o_classA_instances_share_class", function() { local o = A; return "" + (o.getclass() == o.getclass()); });
s("o_instB_read_field", function() { local o = B(); return o.x; });
s("o_instB_read_static", function() { local o = B(); return o.s; });
s("o_instB_read_missing", function() { local o = B(); return o.nope; });
s("o_instB_read_attr_field", function() { local o = B(); return o.y; });
s("o_instB_call_method", function() { local o = B(); return o.m(); });
s("o_instB_call_static", function() { local o = B(); return o.sm(); });
s("o_instB_call_missing", function() { local o = B(); return o.nomethod(); });
s("o_instB_typeof_field", function() { local o = B(); return typeof o.x; });
s("o_instB_typeof_method", function() { local o = B(); return typeof o.m; });
s("o_instB_rawget_field", function() { local o = B(); return o.rawget("x"); });
s("o_instB_rawget_missing", function() { local o = B(); return o.rawget("nope"); });
s("o_instB_rawin_field", function() { local o = B(); return "" + o.rawin("x"); });
s("o_instB_rawin_missing", function() { local o = B(); return "" + o.rawin("nope"); });
s("o_instB_in_field", function() { local o = B(); return "" + ("x" in o); });
s("o_instB_in_missing", function() { local o = B(); return "" + ("nope" in o); });
s("o_instB_getclass", function() { local o = B(); return typeof o.getclass(); });
s("o_instB_getbase", function() { local o = B(); return typeof o.getbase(); });
s("o_instB_attrs_field", function() { local o = B(); return o.getattributes("y"); });
s("o_instB_attrs_missing", function() { local o = B(); return o.getattributes("nope"); });
s("o_instB_delete_field", function() { local o = B(); return delete o.x; });
s("o_instB_set_field", function() { local o = B(); o.x = 99; return o.x; });
s("o_instB_newslot", function() { local o = B(); o.newone <- 1; return "ok"; });
s("o_instB_add_field", function() { local o = B(); o.extra = 5; return "ok"; });
s("o_instB_tostring", function() { local o = B(); return typeof ("" + o); });
s("o_instB_len", function() { local o = B(); return o.len(); });
s("o_instB_weakref", function() { local o = B(); return typeof o.weakref(); });
s("o_instB_keys_of_class", function() { local o = B(); return o.getclass().len(); });
s("o_instB_instances_share_class", function() { local o = B(); return "" + (o.getclass() == o.getclass()); });
s("o_classB_read_field", function() { local o = B; return o.x; });
s("o_classB_read_static", function() { local o = B; return o.s; });
s("o_classB_read_missing", function() { local o = B; return o.nope; });
s("o_classB_read_attr_field", function() { local o = B; return o.y; });
s("o_classB_call_method", function() { local o = B; return o.m(); });
s("o_classB_call_static", function() { local o = B; return o.sm(); });
s("o_classB_call_missing", function() { local o = B; return o.nomethod(); });
s("o_classB_typeof_field", function() { local o = B; return typeof o.x; });
s("o_classB_typeof_method", function() { local o = B; return typeof o.m; });
s("o_classB_rawget_field", function() { local o = B; return o.rawget("x"); });
s("o_classB_rawget_missing", function() { local o = B; return o.rawget("nope"); });
s("o_classB_rawin_field", function() { local o = B; return "" + o.rawin("x"); });
s("o_classB_rawin_missing", function() { local o = B; return "" + o.rawin("nope"); });
s("o_classB_in_field", function() { local o = B; return "" + ("x" in o); });
s("o_classB_in_missing", function() { local o = B; return "" + ("nope" in o); });
s("o_classB_getclass", function() { local o = B; return typeof o.getclass(); });
s("o_classB_getbase", function() { local o = B; return typeof o.getbase(); });
s("o_classB_attrs_field", function() { local o = B; return o.getattributes("y"); });
s("o_classB_attrs_missing", function() { local o = B; return o.getattributes("nope"); });
s("o_classB_delete_field", function() { local o = B; return delete o.x; });
s("o_classB_set_field", function() { local o = B; o.x = 99; return o.x; });
s("o_classB_newslot", function() { local o = B; o.newone <- 1; return "ok"; });
s("o_classB_add_field", function() { local o = B; o.extra = 5; return "ok"; });
s("o_classB_tostring", function() { local o = B; return typeof ("" + o); });
s("o_classB_len", function() { local o = B; return o.len(); });
s("o_classB_weakref", function() { local o = B; return typeof o.weakref(); });
s("o_classB_keys_of_class", function() { local o = B; return o.getclass().len(); });
s("o_classB_instances_share_class", function() { local o = B; return "" + (o.getclass() == o.getclass()); });
s("o_instE_read_field", function() { local o = E(); return o.x; });
s("o_instE_read_static", function() { local o = E(); return o.s; });
s("o_instE_read_missing", function() { local o = E(); return o.nope; });
s("o_instE_read_attr_field", function() { local o = E(); return o.y; });
s("o_instE_call_method", function() { local o = E(); return o.m(); });
s("o_instE_call_static", function() { local o = E(); return o.sm(); });
s("o_instE_call_missing", function() { local o = E(); return o.nomethod(); });
s("o_instE_typeof_field", function() { local o = E(); return typeof o.x; });
s("o_instE_typeof_method", function() { local o = E(); return typeof o.m; });
s("o_instE_rawget_field", function() { local o = E(); return o.rawget("x"); });
s("o_instE_rawget_missing", function() { local o = E(); return o.rawget("nope"); });
s("o_instE_rawin_field", function() { local o = E(); return "" + o.rawin("x"); });
s("o_instE_rawin_missing", function() { local o = E(); return "" + o.rawin("nope"); });
s("o_instE_in_field", function() { local o = E(); return "" + ("x" in o); });
s("o_instE_in_missing", function() { local o = E(); return "" + ("nope" in o); });
s("o_instE_getclass", function() { local o = E(); return typeof o.getclass(); });
s("o_instE_getbase", function() { local o = E(); return typeof o.getbase(); });
s("o_instE_attrs_field", function() { local o = E(); return o.getattributes("y"); });
s("o_instE_attrs_missing", function() { local o = E(); return o.getattributes("nope"); });
s("o_instE_delete_field", function() { local o = E(); return delete o.x; });
s("o_instE_set_field", function() { local o = E(); o.x = 99; return o.x; });
s("o_instE_newslot", function() { local o = E(); o.newone <- 1; return "ok"; });
s("o_instE_add_field", function() { local o = E(); o.extra = 5; return "ok"; });
s("o_instE_tostring", function() { local o = E(); return typeof ("" + o); });
s("o_instE_len", function() { local o = E(); return o.len(); });
s("o_instE_weakref", function() { local o = E(); return typeof o.weakref(); });
s("o_instE_keys_of_class", function() { local o = E(); return o.getclass().len(); });
s("o_instE_instances_share_class", function() { local o = E(); return "" + (o.getclass() == o.getclass()); });
s("o_classE_read_field", function() { local o = E; return o.x; });
s("o_classE_read_static", function() { local o = E; return o.s; });
s("o_classE_read_missing", function() { local o = E; return o.nope; });
s("o_classE_read_attr_field", function() { local o = E; return o.y; });
s("o_classE_call_method", function() { local o = E; return o.m(); });
s("o_classE_call_static", function() { local o = E; return o.sm(); });
s("o_classE_call_missing", function() { local o = E; return o.nomethod(); });
s("o_classE_typeof_field", function() { local o = E; return typeof o.x; });
s("o_classE_typeof_method", function() { local o = E; return typeof o.m; });
s("o_classE_rawget_field", function() { local o = E; return o.rawget("x"); });
s("o_classE_rawget_missing", function() { local o = E; return o.rawget("nope"); });
s("o_classE_rawin_field", function() { local o = E; return "" + o.rawin("x"); });
s("o_classE_rawin_missing", function() { local o = E; return "" + o.rawin("nope"); });
s("o_classE_in_field", function() { local o = E; return "" + ("x" in o); });
s("o_classE_in_missing", function() { local o = E; return "" + ("nope" in o); });
s("o_classE_getclass", function() { local o = E; return typeof o.getclass(); });
s("o_classE_getbase", function() { local o = E; return typeof o.getbase(); });
s("o_classE_attrs_field", function() { local o = E; return o.getattributes("y"); });
s("o_classE_attrs_missing", function() { local o = E; return o.getattributes("nope"); });
s("o_classE_delete_field", function() { local o = E; return delete o.x; });
s("o_classE_set_field", function() { local o = E; o.x = 99; return o.x; });
s("o_classE_newslot", function() { local o = E; o.newone <- 1; return "ok"; });
s("o_classE_add_field", function() { local o = E; o.extra = 5; return "ok"; });
s("o_classE_tostring", function() { local o = E; return typeof ("" + o); });
s("o_classE_len", function() { local o = E; return o.len(); });
s("o_classE_weakref", function() { local o = E; return typeof o.weakref(); });
s("o_classE_keys_of_class", function() { local o = E; return o.getclass().len(); });
s("o_classE_instances_share_class", function() { local o = E; return "" + (o.getclass() == o.getclass()); });
s("m_share_array", function() { class S { arr = [] } local a = S(), b = S(); a.arr.append(1); return b.arr.len(); });
s("m_share_table", function() { class S2 { t = {} } local a = S2(), b = S2(); a.t.k <- 1; return b.t.len(); });
s("m_each_instance_assign", function() { class S3 { n = 0 } local a = S3(), b = S3(); a.n = 5; return "" + b.n; });
s("m_class_expr_default", function() { class S4 { v = 1 + 2 } return S4().v; });
s("m_static_init_order", function() { class S5 { static a = 1; static b = a + 1; } return S5.b; });
s("m_static_shared", function() { class S6 { static n = 0 } local a = S6(), b = S6(); a.getclass().n = 7; return S6.n; });
s("m_ctor_default", function() { class C1 { v = 0; constructor(a = 5) { v = a; } } return C1().v; });
s("m_ctor_default_given", function() { class C2 { v = 0; constructor(a = 5) { v = a; } } return C2(9).v; });
s("m_ctor_varargs", function() { class C3 { n = 0; constructor(...) { n = vargv.len(); } } return C3(1, 2, 3).n; });
s("m_ctor_varargs_zero", function() { class C4 { n = -1; constructor(...) { n = vargv.len(); } } return C4().n; });
s("m_ctor_this_ret", function() { class C5 { v = 1; constructor() { return this; } } return C5().v; });
s("m_ctor_ret_value", function() { class C6 { constructor() { return 5; } } return typeof C6(); });
s("m_ctor_call_base", function() { class P6 { p = 1; constructor() { p = 42; } } class Q6 extends P6 { constructor() { base.constructor(); } } return Q6().p; });
s("m_ctor_no_base_call", function() { class P7 { p = 1; constructor() { p = 42; } } class Q7 extends P7 { constructor() { } } return Q7().p; });
s("m_ctor_nested_call", function() { class P8 { p = 0; constructor(n) { p = n; } } class Q8 extends P8 { q = 0; constructor(n) { base.constructor(n * 2); q = n; } } local o = Q8(5); return o.p + "," + o.q; });
s("m_ctor_static_access", function() { class C9 { static s = 3; v = 0; constructor() { v = s * 2; } } return C9().v; });
s("m_ctor_field_access", function() { class C10 { a = 1; b = 0; constructor() { b = a + 1; } } return C10().b; });
s("m_ctor_recursion_bounded", function() { class C11 { constructor(n) { if (n > 0) { local x = C11(n - 1); } } } C11(50); return "ok"; });
s("m_ctor_missing_args", function() { class C12 { constructor(a) { } } return C12(); });
s("m_ctor_extra_args", function() { class C13 { constructor() { } } return C13(1, 2); });
s("m_ctor_returns_instance_of_other", function() { class C14 { } class C15 { constructor() { return C14(); } } return typeof C15(); });
s("m_inherit_3_levels", function() { class L1 { function f() { return 1; } } class L2 extends L1 { } class L3 extends L2 { } return L3().f(); });
s("m_inherit_override_base", function() { class M1 { function f() { return "M1"; } } class M2 extends M1 { function f() { return "M2" + base.f(); } } return M2().f(); });
s("m_inherit_base_chain", function() { class N1 { function f() { return 1; } } class N2 extends N1 { function f() { return 2 + base.f(); } } class N3 extends N2 { function f() { return 3 + base.f(); } } return N3().f(); });
s("m_inherit_static", function() { class O1 { static s = "a"; } class O2 extends O1 { } return O2.s; });
s("m_inherit_static_override", function() { class O3 { static s = "a"; } class O4 extends O3 { static s = "b"; } return O3.s + O4.s; });
s("m_inherit_getbase_chain", function() { class P1 { } class P2 extends P1 { } class P3 extends P2 { } return "" + (P3.getbase() == P2) + (P2.getbase() == P1) + (P1.getbase() == null); });
s("m_inherit_instanceof_chain", function() { class Q1 { } class Q2 extends Q1 { } class Q3 extends Q2 { } local o = Q3(); return "" + (o instanceof Q3) + (o instanceof Q2) + (o instanceof Q1); });
s("m_inherit_ctor_implicit", function() { class R1 { v = 1; } class R2 extends R1 { } return R2().v; });
s("m_inherit_field_shadow", function() { class T1 { v = 1; } class T2 extends T1 { v = 2; } local o = T2(); return o.v + "," + T1().v; });
s("m_inherit_method_shadow", function() { class U1 { function f() { return 1; } } class U2 extends U1 { function f() { return 2; } } return U2().f() + U1().f(); });
s("m_inherit_base_of_base", function() { class V1 { function f() { return "V1"; } } class V2 extends V1 { } class V3 extends V2 { function f() { return "V3" + base.f(); } } return V3().f(); });
s("m_inherit_from_instance", function() { class W1 { } local o = W1(); try { class W2 extends o { } } catch (e) { return "E:" + e; } return "ok"; });
s("m_base_outside_method", function() { class X1 { } return typeof X1; });
s("m_newmember_before_inst", function() { class Y1 { } Y1.newmember("f", function() { return 1; }); return Y1().f(); });
s("m_newmember_after_inst", function() { class Y2 { } local o = Y2(); Y2.newmember("f", function() { return 1; }); return "ok"; });
s("m_newmember_dup", function() { class Y3 { f = 1; } Y3.newmember("f", 2); return "ok"; });
s("m_rawnewmember_dup", function() { class Y4 { f = 1; } Y4.rawnewmember("f", 2); return Y4.f; });
s("m_newslot_after_inst", function() { class Y5 { } local o = Y5(); Y5.g <- 1; return "ok"; });
s("m_newslot_before_inst", function() { class Y6 { } Y6.g <- 1; return Y6().g; });
s("m_delete_class_member", function() { class Y7 { f = 1; } return delete Y7.f; });
s("m_modify_static_after_inst", function() { class Y8 { static s = 1; } local o = Y8(); Y8.s = 2; return Y8.s; });
s("m_member_count", function() { class Y9 { a = 1; b = 2; function f() {} } return Y9.len(); });
s("m_attr_member", function() { class Z1 { </ doc = "d" /> f = 1; } return Z1.getattributes("f").doc; });
s("m_attr_static", function() { class Z2 { </ doc = "s" /> static f = 1; } return Z2.getattributes("f").doc; });
s("m_attr_method", function() { class Z3 { </ doc = "m" /> function f() {} } return Z3.getattributes("f").doc; });
s("m_attr_class", function() { class Z4 </ doc = "c" /> { } return Z4.getattributes(null).doc; });
s("m_attr_none_class", function() { class Z5 { } return Z5.getattributes(null); });
s("m_attr_missing", function() { class Z6 { } return Z6.getattributes("nope"); });
s("m_attr_set", function() { class Z7 { f = 1; } Z7.setattributes("f", { doc = "x" }); return Z7.getattributes("f").doc; });
s("m_attr_set_new", function() { class Z8 { } Z8.setattributes("f", { doc = "x" }); return "ok"; });
s("m_attr_on_instance", function() { class Z9 { </ doc = "d" /> f = 1; } local o = Z9(); return o.getclass().getattributes("f").doc; });
s("m_attr_table_value", function() { class Z10 { </ num = 5 /> f = 1; } return Z10.getattributes("f").num; });
s("m_this_in_method", function() { class A1 { v = 7; function f() { return this.v; } } return A1().f(); });
s("m_this_in_lambda", function() { class A2 { v = 8; function f() { local g = @() this.v; return g(); } } return A2().f(); });
s("m_this_in_static", function() { class A3 { static function f() { return typeof this; } } return A3.f(); });
s("m_base_in_lambda", function() { class B1 { function f() { return 1; } } class B2 extends B1 { function f() { local g = @() base.f(); return g(); } } return B2().f(); });
s("m_base_in_ctor_lambda", function() { class B3 { v = 1; } class B4 extends B3 { constructor() { local g = @() base; return; } } return B4().v; });
s("m_method_as_value", function() { class B5 { function f() { return 9; } } local o = B5(); local m = o.f; return m(); });
s("m_method_bound_this", function() { class B6 { v = 4; function f() { return v; } } local o = B6(); local m = o.f; return m(); });
s("m_static_method_via_instance", function() { class B7 { static function f() { return 1; } } return B7().f(); });
s("m_instance_field_via_class", function() { class B8 { v = 1; } return B8.v; });
s("m_class_in_table", function() { local t = { cls = class { v = 5 } }; return t.cls().v; });
s("m_class_as_arg", function() { function mk(c) { return c(); } class C1 { v = 3 } return mk(C1).v; });
s("m_class_anon", function() { local c = class { v = 6 }; return c().v; });
s("m_class_typeof", function() { class C2 { } return typeof C2; });
s("m_class_tostring", function() { class C3 { } return typeof ("" + C3); });
s("m_class_weakref", function() { class C4 { } return typeof C4.weakref(); });
s("m_class_weakref_ref", function() { class C5 { } return "" + (C5.weakref().ref() == C5); });
s("m_class_call_twice", function() { class C6 { static n = 0; constructor() { n++; } } local a = C6(), b = C6(); return C6.n; });
s("m_class_instance_field_of_class", function() { class C7 { } local c = clone C7; return typeof c; });
s("m_class_clone_instance", function() { class C8 { v = 1; } local a = C8(); local b = clone a; b.v = 2; return a.v + "," + b.v; });
s("m_class_clone_instance_class", function() { class C9 { v = 1; } local a = C9(); local b = clone a; return "" + (a.getclass() == b.getclass()); });
s("m_class_instance_set_field_dyn", function() { class C10 { } local o = C10(); o.newf <- 5; return o.newf; });
s("m_class_instance_del_dyn", function() { class C11 { } local o = C11(); o.newf <- 5; delete o.newf; return "" + ("newf" in o); });
s("m_inst_eq_self", function() { class D1 { } local o = D1(); return "" + (o == o); });
s("m_inst_ne_other", function() { class D2 { } return "" + (D2() == D2()); });
s("m_inst_cmp_self", function() { class D3 { } local o = D3(); return o <=> o; });
s("m_inst_tostring_default", function() { class D4 { } return typeof ("" + D4()); });
s("m_inst_getclass_chain", function() { class D5 { } class D6 extends D5 { } return "" + (D6().getclass().getbase() == D5); });
s("m_inst_field_after_method", function() { class D7 { v = 1; function f() { v = 2; return v; } } local o = D7(); local r = o.f(); return r + "," + o.v; });
print("OOPSMATRIX-DONE\n");

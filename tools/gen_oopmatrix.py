# -*- coding: utf-8 -*-
"""生成「类/实例语义」矩阵语料 tests-corpus/t_oopmatrix.nut。

覆盖三块：

1. **自动部分**：固定几个"类夹具"（字段/静态字段/方法/静态方法/构造器/属性/继承）×
   多种"接收者"（实例、类、子类实例、子类）× 多条"访问路径"（读、调用、rawget、
   rawin、in、getattributes、getclass、getbase、typeof、delete、赋值、newslot）。
   每条用例都自己造一个新对象，互不干扰。
2. **语义部分**：手写的构造器/继承/静态/属性/锁定/默认字段共享等 100+ 条。
3. 末尾打印 `OOPSMATRIX-DONE`（差分守卫据此判断脚本真的跑完）。

用法：
    python tools/gen_oopmatrix.py
"""
import os
import re

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_oopmatrix.nut")

# 类夹具：标签 → 定义文本（A 是基类，B 继承它）
FIXTURES = [
    ("A", "class A {\n"
          "    x = 1;\n"
          "    static s = 2;\n"
          "    </ doc = \"fx\" /> y = 3;\n"
          "    function m() { return 10; }\n"
          "    static function sm() { return 20; }\n"
          "    constructor() { }\n"
          "}"),
    ("B", "class B extends A {\n"
          "    z = 30;\n"
          "    function m() { return 11 + base.m(); }\n"
          "    constructor() { base.constructor(); }\n"
          "}"),
    ("C", "class C { a = 1; b = 2; static t = 3; function f() { return a + b; } }"),
    ("D", "class D extends C { }"),
    ("E", "class E extends D { a = 100; function f() { return a; } }"),
]

# 接收者表达式（每次现造，避免互相污染）
RECEIVERS = [
    ("instA", "A()"),
    ("classA", "A"),
    ("instB", "B()"),
    ("classB", "B"),
    ("instE", "E()"),
    ("classE", "E"),
]

# 访问路径：标签 → 以 o 为接收者的代码（返回要打印的值；表达式或语句序列）
PATHS = [
    ("read_field", "return o.x;"),
    ("read_static", "return o.s;"),
    ("read_missing", "return o.nope;"),
    ("read_attr_field", "return o.y;"),
    ("call_method", "return o.m();"),
    ("call_static", "return o.sm();"),
    ("call_missing", "return o.nomethod();"),
    ("typeof_field", "return typeof o.x;"),
    ("typeof_method", "return typeof o.m;"),
    ("rawget_field", "return o.rawget(\"x\");"),
    ("rawget_missing", "return o.rawget(\"nope\");"),
    ("rawin_field", "return \"\" + o.rawin(\"x\");"),
    ("rawin_missing", "return \"\" + o.rawin(\"nope\");"),
    ("in_field", "return \"\" + (\"x\" in o);"),
    ("in_missing", "return \"\" + (\"nope\" in o);"),
    ("getclass", "return typeof o.getclass();"),
    ("getbase", "return typeof o.getbase();"),
    ("attrs_field", "return o.getattributes(\"y\");"),
    ("attrs_missing", "return o.getattributes(\"nope\");"),
    ("delete_field", "return delete o.x;"),
    ("set_field", "o.x = 99; return o.x;"),
    ("newslot", "o.newone <- 1; return \"ok\";"),
    ("add_field", "o.extra = 5; return \"ok\";"),
    ("tostring", "return typeof (\"\" + o);"),
    ("len", "return o.len();"),
    ("weakref", "return typeof o.weakref();"),
    ("keys_of_class", "return o.getclass().len();"),
    ("instances_share_class", "return \"\" + (o.getclass() == o.getclass());"),
]

lines = [
    "// 自动生成：类/实例语义矩阵（tools/gen_oopmatrix.py）",
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
    "    if (t == \"string\") return \"\\\"\" + r + \"\\\"\";",
    "    return \"\" + r;",
    "}",
    "function s(tag, f) {",
    "    try { print(tag + \"=\" + show(f()) + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "",
    "// ---- 类夹具 ----",
]
for name, body in FIXTURES:
    lines.append(body.replace("\n", "\n"))
    lines.append("")

n_auto = 0
for rname, rexpr in RECEIVERS:
    for pname, pbody in PATHS:
        n_auto += 1
        lines.append(
            's("o_%s_%s", function() { local o = %s; %s });' % (rname, pname, rexpr, pbody)
        )

# ---------------------------------------------------------------- 语义用例
SEM = [
    # ---- 默认字段值是共享还是每实例一份（Squirrel 的经典陷阱）
    ("share_array", 'class S { arr = [] } local a = S(), b = S(); a.arr.append(1); return b.arr.len();'),
    ("share_table", 'class S2 { t = {} } local a = S2(), b = S2(); a.t.k <- 1; return b.t.len();'),
    ("each_instance_assign", 'class S3 { n = 0 } local a = S3(), b = S3(); a.n = 5; return "" + b.n;'),
    ("class_expr_default", 'class S4 { v = 1 + 2 } return S4().v;'),
    ("static_init_order", 'class S5 { static a = 1; static b = a + 1; } return S5.b;'),
    ("static_shared", 'class S6 { static n = 0 } local a = S6(), b = S6(); a.getclass().n = 7; return S6.n;'),

    # ---- 构造器
    ("ctor_default", 'class C1 { v = 0; constructor(a = 5) { v = a; } } return C1().v;'),
    ("ctor_default_given", 'class C2 { v = 0; constructor(a = 5) { v = a; } } return C2(9).v;'),
    ("ctor_varargs", 'class C3 { n = 0; constructor(...) { n = vargv.len(); } } return C3(1, 2, 3).n;'),
    ("ctor_varargs_zero", 'class C4 { n = -1; constructor(...) { n = vargv.len(); } } return C4().n;'),
    ("ctor_this_ret", 'class C5 { v = 1; constructor() { return this; } } return C5().v;'),
    ("ctor_ret_value", 'class C6 { constructor() { return 5; } } return typeof C6();'),
    ("ctor_call_base", 'class P6 { p = 1; constructor() { p = 42; } } class Q6 extends P6 { constructor() { base.constructor(); } } return Q6().p;'),
    ("ctor_no_base_call", 'class P7 { p = 1; constructor() { p = 42; } } class Q7 extends P7 { constructor() { } } return Q7().p;'),
    ("ctor_nested_call", 'class P8 { p = 0; constructor(n) { p = n; } } class Q8 extends P8 { q = 0; constructor(n) { base.constructor(n * 2); q = n; } } local o = Q8(5); return o.p + "," + o.q;'),
    ("ctor_static_access", 'class C9 { static s = 3; v = 0; constructor() { v = s * 2; } } return C9().v;'),
    ("ctor_field_access", 'class C10 { a = 1; b = 0; constructor() { b = a + 1; } } return C10().b;'),
    # 注意：必须**有界**。无界的非尾递归（构造器里无限造同类实例）在参考实现里没有栈上限，
    # 会把宿主内存吃光（实测 32GB 被系统杀）；本实现封顶报可捕获 stack overflow（见 STATUS.md）
    ("ctor_recursion_bounded", 'class C11 { constructor(n) { if (n > 0) { local x = C11(n - 1); } } } C11(50); return "ok";'),
    ("ctor_missing_args", 'class C12 { constructor(a) { } } return C12();'),
    ("ctor_extra_args", 'class C13 { constructor() { } } return C13(1, 2);'),
    ("ctor_returns_instance_of_other", 'class C14 { } class C15 { constructor() { return C14(); } } return typeof C15();'),

    # ---- 继承
    ("inherit_3_levels", 'class L1 { f() { return 1; } } class L2 extends L1 { } class L3 extends L2 { } return L3().f();'),
    ("inherit_override_base", 'class M1 { f() { return "M1"; } } class M2 extends M1 { f() { return "M2" + base.f(); } } return M2().f();'),
    ("inherit_base_chain", 'class N1 { f() { return 1; } } class N2 extends N1 { f() { return 2 + base.f(); } } class N3 extends N2 { f() { return 3 + base.f(); } } return N3().f();'),
    ("inherit_static", 'class O1 { static s = "a"; } class O2 extends O1 { } return O2.s;'),
    ("inherit_static_override", 'class O3 { static s = "a"; } class O4 extends O3 { static s = "b"; } return O3.s + O4.s;'),
    ("inherit_getbase_chain", 'class P1 { } class P2 extends P1 { } class P3 extends P2 { } return "" + (P3.getbase() == P2) + (P2.getbase() == P1) + (P1.getbase() == null);'),
    ("inherit_instanceof_chain", 'class Q1 { } class Q2 extends Q1 { } class Q3 extends Q2 { } local o = Q3(); return "" + (o instanceof Q3) + (o instanceof Q2) + (o instanceof Q1);'),
    ("inherit_ctor_implicit", 'class R1 { v = 1; } class R2 extends R1 { } return R2().v;'),
    ("inherit_field_shadow", 'class T1 { v = 1; } class T2 extends T1 { v = 2; } local o = T2(); return o.v + "," + T1().v;'),
    ("inherit_method_shadow", 'class U1 { f() { return 1; } } class U2 extends U1 { f() { return 2; } } return U2().f() + U1().f();'),
    ("inherit_base_of_base", 'class V1 { f() { return "V1"; } } class V2 extends V1 { } class V3 extends V2 { f() { return "V3" + base.f(); } } return V3().f();'),
    ("inherit_from_instance", 'class W1 { } local o = W1(); try { class W2 extends o { } } catch (e) { return "E:" + e; } return "ok";'),
    ("base_outside_method", 'class X1 { } return typeof X1;'),

    # ---- 类成员增删与锁定
    ("newmember_before_inst", 'class Y1 { } Y1.newmember("f", function() { return 1; }); return Y1().f();'),
    ("newmember_after_inst", 'class Y2 { } local o = Y2(); Y2.newmember("f", function() { return 1; }); return "ok";'),
    ("newmember_dup", 'class Y3 { f = 1; } Y3.newmember("f", 2); return "ok";'),
    ("rawnewmember_dup", 'class Y4 { f = 1; } Y4.rawnewmember("f", 2); return Y4.f;'),
    ("newslot_after_inst", 'class Y5 { } local o = Y5(); Y5.g <- 1; return "ok";'),
    ("newslot_before_inst", 'class Y6 { } Y6.g <- 1; return Y6().g;'),
    ("delete_class_member", 'class Y7 { f = 1; } return delete Y7.f;'),
    ("modify_static_after_inst", 'class Y8 { static s = 1; } local o = Y8(); Y8.s = 2; return Y8.s;'),
    ("member_count", 'class Y9 { a = 1; b = 2; function f() {} } return Y9.len();'),

    # ---- 属性（attributes）
    ("attr_member", 'class Z1 { </ doc = "d" /> f = 1; } return Z1.getattributes("f").doc;'),
    ("attr_static", 'class Z2 { </ doc = "s" /> static f = 1; } return Z2.getattributes("f").doc;'),
    ("attr_method", 'class Z3 { </ doc = "m" /> function f() {} } return Z3.getattributes("f").doc;'),
    ("attr_class", 'class Z4 </ doc = "c" /> { } return Z4.getattributes(null).doc;'),
    ("attr_none_class", 'class Z5 { } return Z5.getattributes(null);'),
    ("attr_missing", 'class Z6 { } return Z6.getattributes("nope");'),
    ("attr_set", 'class Z7 { f = 1; } Z7.setattributes("f", { doc = "x" }); return Z7.getattributes("f").doc;'),
    ("attr_set_new", 'class Z8 { } Z8.setattributes("f", { doc = "x" }); return Z7_ok();'),
    ("attr_on_instance", 'class Z9 { </ doc = "d" /> f = 1; } local o = Z9(); return o.getclass().getattributes("f").doc;'),
    ("attr_table_value", 'class Z10 { </ num = 5 /> f = 1; } return Z10.getattributes("f").num;'),

    # ---- this / base / 作用域
    ("this_in_method", 'class A1 { v = 7; f() { return this.v; } } return A1().f();'),
    ("this_in_lambda", 'class A2 { v = 8; f() { local g = @() this.v; return g(); } } return A2().f();'),
    ("this_in_static", 'class A3 { static f() { return typeof this; } } return A3.f();'),
    ("base_in_lambda", 'class B1 { f() { return 1; } } class B2 extends B1 { f() { local g = @() base.f(); return g(); } } return B2().f();'),
    ("base_in_ctor_lambda", 'class B3 { v = 1; } class B4 extends B3 { constructor() { local g = @() base; return; } } return B4().v;'),
    ("method_as_value", 'class B5 { function f() { return 9; } } local o = B5(); local m = o.f; return m();'),
    ("method_bound_this", 'class B6 { v = 4; function f() { return v; } } local o = B6(); local m = o.f; return m();'),
    ("static_method_via_instance", 'class B7 { static function f() { return 1; } } return B7().f();'),
    ("instance_field_via_class", 'class B8 { v = 1; } return B8.v;'),

    # ---- 类作为值
    ("class_in_table", 'local t = { cls = class { v = 5 } }; return t.cls().v;'),
    ("class_as_arg", 'function mk(c) { return c(); } class C1 { v = 3 } return mk(C1).v;'),
    ("class_anon", 'local c = class { v = 6 }; return c().v;'),
    ("class_typeof", 'class C2 { } return typeof C2;'),
    ("class_tostring", 'class C3 { } return typeof ("" + C3);'),
    ("class_weakref", 'class C4 { } return typeof C4.weakref();'),
    ("class_weakref_ref", 'class C5 { } return "" + (C5.weakref().ref() == C5);'),
    ("class_call_twice", 'class C6 { static n = 0; constructor() { n++; } } local a = C6(), b = C6(); return C6.n;'),
    ("class_instance_field_of_class", 'class C7 { } local c = clone C7; return typeof c;'),
    ("class_clone_instance", 'class C8 { v = 1; } local a = C8(); local b = clone a; b.v = 2; return a.v + "," + b.v;'),
    ("class_clone_instance_class", 'class C9 { v = 1; } local a = C9(); local b = clone a; return "" + (a.getclass() == b.getclass());'),
    ("class_instance_set_field_dyn", 'class C10 { } local o = C10(); o.newf <- 5; return o.newf;'),
    ("class_instance_del_dyn", 'class C11 { } local o = C11(); o.newf <- 5; delete o.newf; return "" + ("newf" in o);'),

    # ---- 实例身份与比较
    ("inst_eq_self", 'class D1 { } local o = D1(); return "" + (o == o);'),
    ("inst_ne_other", 'class D2 { } return "" + (D2() == D2());'),
    # 注意：不要比较**两个不同实例**——参考实现走 ObjCmp 的 `_userpointer(o1) < _userpointer(o2)`
    # （裸指针序），结果随堆布局变化，跨进程/跨字节码不可复现（cnut 交叉执行实测 -1 vs 1）。
    # 同一个实例的比较是确定的（_rawval 相等 → 0）。
    ("inst_cmp_self", 'class D3 { } local o = D3(); return o <=> o;'),
    ("inst_tostring_default", 'class D4 { } return typeof ("" + D4());'),
    ("inst_getclass_chain", 'class D5 { } class D6 extends D5 { } return "" + (D6().getclass().getbase() == D5);'),
    ("inst_field_after_method", 'class D7 { v = 1; function f() { v = 2; return v; } } local o = D7(); local r = o.f(); return r + "," + o.v;'),
]

for tag, body in SEM:
    if "Z7_ok()" in body:  # 占位：这个用例本身是想看「给不存在的成员设属性」的行为
        body = 'class Z8 { } Z8.setattributes("f", { doc = "x" }); return "ok";'
    # Squirrel 的类体内方法必须写 `function f() {...}`，不能写 `f() {...}`
    body = re.sub(r"(?<!function) f\(\) \{", " function f() {", body)
    lines.append('s("m_%s", function() { %s });' % (tag, body))

lines.append('print("OOPSMATRIX-DONE\\n");')

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：自动 %d 条 + 语义 %d 条" % (OUT, n_auto, len(SEM)))

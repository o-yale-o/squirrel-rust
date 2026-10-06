// 自动生成：类属性时序/共享/继承边界矩阵（tools/gen_attrtiming.py）
//
// 与 t_attrmap.nut（形态矩阵）互补：这里测**动力学**——实例化后改属性、
// 继承后改父类、属性表共享引用、newmember 4 参、clone 边界、参数个数。
// 全部用例已在参考实现上实测，两侧逐字节一致。
function d(x) {
    local t = typeof x;
    if (t == "table") { local o = "{"; local f = true; foreach (k, v in x) { if (!f) o += ","; f = false; o += k + "=" + v; } return o + "}"; }
    return "" + x;
}
function t(tag, f) {
    try { local v = f(); print(tag + "=" + d(v) + "|" + typeof v + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

t("t_after_inst", function() { local C = class { </ a = 1 /> v = 1; }; local o = C(); C.setattributes("v", { a = 99 }); return d(o.getclass().getattributes("v")) + "/" + d(C.getattributes("v")); });
t("t_parent_after_extend", function() { local A = class { </ a = 1 /> v = 1; }; local B = class extends A { }; A.setattributes("v", { a = 77 }); return "A=" + d(A.getattributes("v")) + " B=" + d(B.getattributes("v")); });
t("set_inherited_member", function() { local A = class { v = 1; }; local B = class extends A { }; return d(B.setattributes("v", { a = 5 })) + "/" + d(B.getattributes("v")); });
t("get_inherited_member", function() { local A = class { </ a = 1 /> v = 1; }; local B = class extends A { }; return d(B.getattributes("v")); });
t("mid_override", function() { local A = class { </ a = "A" /> v = 1; }; local B = class extends A { </ a = "B" /> v = 2; }; local C = class extends B { }; return "A=" + d(A.getattributes("v")) + " B=" + d(B.getattributes("v")) + " C=" + d(C.getattributes("v")); });
t("cl_inherit", function() { local A = class { }; A.setattributes(null, { lvl = "A" }); local B = class extends A { }; return "A=" + d(A.getattributes(null)) + " B=" + d(B.getattributes(null)); });
t("cl_child", function() { local A = class { }; local B = class extends A { }; B.setattributes(null, { lvl = "B" }); return "A=" + d(A.getattributes(null)) + " B=" + d(B.getattributes(null)); });
t("shared_mutate", function() { local C = class { </ a = { n = 1 } /> v = 1; }; C.getattributes("v").a.n = 42; return C.getattributes("v").a.n; });
t("shared_newkey", function() { local C = class { </ a = { n = 1 } /> v = 1; }; local x = C.getattributes("v"); x.same = "yes"; return d(x); });
t("shared_same_view", function() { local C = class { </ a = { n = 1 } /> v = 1; }; local x = C.getattributes("v"); local y = C.getattributes("v"); x.a.n = 5; return x.a.n + "/" + y.a.n; });
t("nm4", function() { local C = class { v = 1; }; C.newmember("w", 2, { a = 8 }, false); return d(C.getattributes("w")) + "/" + C.w; });
t("nm4_static", function() { local C = class { v = 1; }; C.newmember("sw", 3, { a = 6 }, true); return d(C.getattributes("sw")) + "/" + C.sw + "/" + typeof C.rawget("sw"); });
t("nm3", function() { local C = class { v = 1; }; C.newmember("w3", 4, { a = 7 }); return d(C.getattributes("w3")); });
t("nm_noattrs", function() { local C = class { v = 1; }; C.newmember("w4", 5); return d(C.getattributes("w4")); });
t("clone_class", function() { local A = class { </ a = 1 /> v = 1; }; return clone A; });
t("inst_set", function() { local C = class { </ a = 1 /> v = 1; }; local o = C(); return d(o.setattributes("v", { a = 3 })); });
t("inst_get_null", function() { local C = class { </ a = 1 /> v = 1; }; local o = C(); return d(o.getattributes(null)); });
t("ga_zero", function() { local C = class { v = 1; }; return C.getattributes(); });
t("ga_two", function() { local C = class { v = 1; }; return C.getattributes("v", 2); });
t("sa_one", function() { local C = class { v = 1; }; return C.setattributes("v"); });
t("sa_three", function() { local C = class { v = 1; }; return C.setattributes("v", {}, 3); });
t("ctor_attr", function() { local C = class { </ a = 1 /> constructor() { } }; return d(C.getattributes("constructor")); });
t("meta_attr", function() { local C = class { </ a = 1 /> function _tostring() { return "C"; } }; return d(C.getattributes("_tostring")) + "/" + ("" + C()); });
t("delete_member", function() { local C = class { </ a = 1 /> v = 1; }; local r = delete C.v; return "" + r + " attr=" + d(C.getattributes("v")) + " member=" + d(C.rawget("v")); });
t("rawset_new", function() { local C = class { </ a = 1 /> v = 1; }; C.rawset("w", 2); return "w=" + d(C.getattributes("w")) + " v=" + d(C.getattributes("v")); });
t("closure_value", function() { local C = class { </ a = function() { return 1; } /> v = 1; }; local a = C.getattributes("v").a; return typeof a + "/" + a(); });
t("class_value", function() { local Inner = class { q = 1 }; local C = class { v = 1; }; C.setattributes("v", Inner); return typeof C.getattributes("v"); });
t("static_shadow", function() { local C = class { </ a = 1 /> static v = 10; }; local o = C(); return "" + C.v + "/" + o.v + "/" + d(C.getattributes("v")); });
print("ATTRTIMING-DONE\n");

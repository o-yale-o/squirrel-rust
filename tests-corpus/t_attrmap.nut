// 自动生成：python tools/gen_attrmap.py —— 请勿手改
// 类成员属性矩阵：</ ... /> 形式、getattributes/setattributes、继承/clone、可见性

// 类成员属性（</ ... />）与 getattributes / setattributes
// 语法规则（实测两侧一致）：空块 `</ />` 可以；**连续两个属性块**、**成员之后写属性**
// 都是语法错 `expected 'IDENTIFIER'`（归 tools/errmsg_matrix.py 对拍）。
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
function d(x) {
    local t = typeof x;
    if (t == "table") { local o = "{"; local first = true; foreach (k, v in x) { if (!first) o += ","; first = false; o += k + "=" + v; } return o + "}"; }
    if (t == "array") return "[" + x.len() + "]";
    return "" + x;
}
function at(cls, name) { return d(cls.getattributes(name)); }

// 1) 基本形式
s("attr_field", function() { local C = class { </ a = 1 /> v = 2; }; return at(C, "v"); });
s("attr_method", function() { local C = class { </ m = "x" /> function f() { return 1; } }; return at(C, "f"); });
s("attr_static", function() { local C = class { </ a = 1 /> static sv = 2; }; return at(C, "sv"); });
s("attr_multi", function() { local C = class { </ a = 1, b = "x", c = true /> v = 1; }; return at(C, "v"); });
s("attr_empty", function() { local C = class { </ /> v = 1; }; return at(C, "v"); });
s("attr_no_attr", function() { local C = class { v = 1; }; return "" + C.getattributes("v"); });
s("attr_types", function() {
    local C = class { </ i = 1, f = 1.5, s = "s", t = true, n = null /> v = 1; };
    return at(C, "v");
});
s("attr_table_value", function() { local C = class { </ a = { x = 1 } /> v = 1; }; return typeof C.getattributes("v").a + "/" + C.getattributes("v").a.x; });
s("attr_array_value", function() { local C = class { </ a = [1, 2] /> v = 1; }; return typeof C.getattributes("v").a + "/" + C.getattributes("v").a.len(); });
s("attr_expr_value", function() { local C = class { </ a = 1 + 1 /> v = 1; }; return at(C, "v"); });
// 注：`</ class = 1 />` 这类把**关键字当属性名**的写法是语法错，归 errmsg_matrix
s("attr_underscore_key", function() { local C = class { </ _a = 1 /> v = 1; }; return at(C, "v"); });
s("attr_several_members", function() {
    local C = class {
        </ a = 1 />
        x = 1;
        </ b = 2 />
        y = 2;
    };
    return at(C, "x") + "|" + at(C, "y");
});
s("attr_multi_line", function() {
    local C = class {
        </
            a = 1,
            b = 2
        />
        v = 1;
    };
    return at(C, "v");
});
s("attr_dup_key", function() { local C = class { </ a = 1, a = 2 /> v = 1; }; return at(C, "v"); });

// 2) 局部类 / 嵌套类 / 继承 / clone
s("attr_local_class", function() { local C = class { </ a = 1 /> v = 1; }; local o = C(); return at(C, "v") + "/" + o.v; });
s("attr_nested_class", function() { local O = class { I = null; constructor() { I = class { </ a = 1 /> v = 2; }; } }; return at(O().I, "v"); });
s("attr_derived", function() { local A = class { </ a = 1 /> v = 1; }; local B = class extends A { }; return at(B, "v"); });
s("attr_override", function() { local A = class { </ a = 1 /> v = 1; }; local B = class extends A { </ a = 2 /> v = 3; }; return at(B, "v"); });
s("attr_clone", function() { local A = class { </ a = 1 /> v = 1; }; local B = clone A; return at(B, "v"); });
s("attr_newmember", function() { local C = class { </ a = 1 /> v = 1; }; C.newmember("w", 2); return "" + C.getattributes("w"); });

// 3) getattributes 的边界
s("ga_null", function() { local C = class { </ a = 1 /> v = 1; }; return d(C.getattributes(null)); });
s("ga_class_no_attr", function() { local C = class { v = 1; }; return "" + C.getattributes(null); });
s("ga_missing", function() { local C = class { v = 1; }; return "" + C.getattributes("nope"); });
s("ga_method_attr", function() { local C = class { </ a = 1 /> function f() {} }; return d(C.getattributes("f")); });
s("ga_nonstring_key", function() { local C = class { v = 1; }; return "" + C.getattributes(5); });
s("ga_on_instance", function() { local C = class { </ a = 1 /> v = 1; }; local o = C(); return d(o.getattributes("v")); });

// 4) setattributes
s("sa_basic", function() { local C = class { v = 1; }; local r = C.setattributes("v", { a = 9 }); return typeof r + "/" + at(C, "v"); });
s("sa_replace", function() { local C = class { </ a = 1 /> v = 1; }; C.setattributes("v", { b = 2 }); return at(C, "v"); });
s("sa_null_value", function() { local C = class { </ a = 1 /> v = 1; }; C.setattributes("v", null); return "" + C.getattributes("v"); });
s("sa_missing_member", function() { local C = class { v = 1; }; return "" + C.setattributes("nope", { a = 1 }); });
s("sa_nontable", function() { local C = class { v = 1; }; return "" + C.setattributes("v", 5); });
s("sa_class_level", function() { local C = class { v = 1; }; C.setattributes(null, { a = 1 }); return d(C.getattributes(null)); });
s("sa_roundtrip", function() { local C = class { v = 1; }; C.setattributes("v", { x = "y" }); local a = C.getattributes("v"); return a.x + "/" + typeof a; });

// 5) 属性与成员可见性（属性不是成员）
s("vis_class_iter", function() { local C = class { </ a = 1 /> v = 1; }; local out = ""; foreach (k, val in C) out += k + ","; return out; });
s("vis_instance_iter", function() { local C = class { </ a = 1 /> v = 1; }; local o = C(); local out = ""; foreach (k, val in o) out += k + ","; return out; });
s("vis_in", function() { local C = class { </ a = 1 /> v = 1; }; return ("a" in C) + "/" + ("v" in C); });
s("vis_rawget", function() { local C = class { </ a = 1 /> v = 1; }; return C.rawget("v") + "/" + (C.rawget("a") == null); });
s("vis_read_attr_as_member", function() { local C = class { </ a = 1 /> v = 1; }; return "" + C.a; });
s("vis_member_count", function() { local C = class { </ a = 1, b = 2 /> v = 1; }; local n = 0; foreach (k, val in C) n++; return n; });
s("vis_attributes_of_attr_name", function() { local C = class { </ a = 1 /> v = 1; }; return "" + C.getattributes("a"); });

print("ATTRMAP-DONE\n");

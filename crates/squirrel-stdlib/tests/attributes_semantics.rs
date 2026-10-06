//! 类成员属性（`</ … />`）与 `getattributes`/`setattributes` 回归（P1-7）。
//!
//! 语料侧是 `tests-corpus/t_attrmap.nut`（40 条，两侧逐字节一致）。
//! 本轮修出一处真实差异：**`C.setattributes(null, {...})` 设类级属性**
//! （↔ `sq_setattributes` 的 `key == OT_NULL` 分支）——本实现原来报 `wrong index`；
//! 而且它返回的是**旧的** attributes（不是 null）。
//!
//! 顺带固化的语法事实（`tools/errmsg_matrix.py` 对拍，60/60 一致）：
//! * `</ class = 1 />`（关键字当属性名）→ `expected 'IDENTIFIER'`；
//! * 属性块**必须独占一行**：`v = 1; </ a = 1 />` 与单行里写第二个属性块都报错；
//! * 属性**不是成员**：`foreach`/`in`/`rawget` 都看不到它们。

use std::cell::RefCell;
use std::rc::Rc;

use squirrel::api::Vm;

fn run_capture(src: &str) -> (String, String) {
    let vm = Vm::new(1024);
    let out: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    let err: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let o = out.clone();
        let e = err.clone();
        vm.set_print_funcs(
            Some(Rc::new(move |_v, b: &[u8]| o.borrow_mut().extend_from_slice(b))),
            Some(Rc::new(move |_v, b: &[u8]| e.borrow_mut().extend_from_slice(b))),
        );
    }
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    let rc = vm.run_buffer(src.as_bytes(), b"attr.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 属性读取、继承、clone、可见性
#[test]
fn attribute_reading_matches_reference() {
    let src = r#"
        function s(tag, f) { try { print(tag + "=" + f() + "\n"); } catch (e) { print(tag + "!" + e + "\n"); } }
        function d(x) {
            if (typeof x == "table") { local o = "{"; local first = true; foreach (k, v in x) { if (!first) o += ","; first = false; o += k + "=" + v; } return o + "}"; }
            return "" + x;
        }
        s("field", function() { local C = class { </ a = 1 /> v = 2; }; return d(C.getattributes("v")); });
        s("method", function() { local C = class { </ m = "x" /> function f() {} }; return d(C.getattributes("f")); });
        s("static_member", function() { local C = class { </ a = 1 /> static sv = 2; }; return d(C.getattributes("sv")); });
        s("empty_block", function() { local C = class { </ /> v = 1; }; return d(C.getattributes("v")); });
        s("no_attr", function() { local C = class { v = 1; }; return "" + C.getattributes("v"); });
        s("class_level", function() { local C = class { </ a = 1 /> v = 1; }; return d(C.getattributes(null)); });
        s("missing", function() { local C = class { v = 1; }; return "" + C.getattributes("nope"); });
        s("derived", function() { local A = class { </ a = 1 /> v = 1; }; local B = class extends A { }; return d(B.getattributes("v")); });
        s("override", function() { local A = class { </ a = 1 /> v = 1; }; local B = class extends A { </ a = 2 /> v = 3; }; return d(B.getattributes("v")); });
        s("clone", function() { local A = class { </ a = 1 /> v = 1; }; local B = clone A; return d(B.getattributes("v")); });
        s("not_a_member", function() { local C = class { </ a = 1 /> v = 1; }; local n = 0; foreach (k, val in C) n++; return n + "/" + ("a" in C) + "/" + (C.rawget("a") == null); });
        s("multi_attr", function() { local C = class { </ i = 1, f = 1.5, s = "s", t = true, n = null /> v = 1; }; local a = C.getattributes("v"); return a.i + "/" + a.f + "/" + a.s + "/" + a.t + "/" + a.n; });
        s("table_value", function() { local C = class { </ a = { x = 1 } /> v = 1; }; return C.getattributes("v").a.x; });
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "field={a=1}\nmethod={m=x}\nstatic_member={a=1}\nempty_block={}\nno_attr=null\n\
         class_level=null\nmissing!wrong index\nderived={a=1}\noverride={a=2}\n\
         clone!cloning a class\nnot_a_member!the index doesn't exist\n\
         multi_attr=1/1.5/s/true/null\ntable_value=1\n"
    );
}

/// `setattributes`：返回值是**旧** attributes；`null` key 表示类级属性
#[test]
fn attribute_writing_matches_reference() {
    let src = r#"
        function s(tag, f) { try { print(tag + "=" + f() + "\n"); } catch (e) { print(tag + "!" + e + "\n"); } }
        function d(x) {
            if (typeof x == "table") { local o = "{"; local first = true; foreach (k, v in x) { if (!first) o += ","; first = false; o += k + "=" + v; } return o + "}"; }
            return "" + x;
        }
        s("replace_returns_old", function() {
            local C = class { </ a = 1 /> v = 1; };
            local old = C.setattributes("v", { b = 2 });
            return d(old) + "|" + d(C.getattributes("v"));
        });
        s("class_level_set", function() {
            local C = class { </ a = 1 /> v = 1; };
            local old = C.setattributes(null, { z = 9 });
            return d(old) + "|" + d(C.getattributes(null));
        });
        s("class_level_then_member", function() {
            local C = class { v = 1; };
            local old = C.setattributes(null, { z = 9 });
            return "" + old + "|" + d(C.getattributes(null)) + "|" + "" + C.getattributes("v");
        });
        s("null_value", function() {
            local C = class { </ a = 1 /> v = 1; };
            local old = C.setattributes("v", null);
            return d(old) + "|" + "" + C.getattributes("v");
        });
        s("missing_member", function() { local C = class { v = 1; }; return "" + C.setattributes("nope", { a = 1 }); });
        s("nontable_value", function() { local C = class { </ a = 1 /> v = 1; }; return d(C.setattributes("v", 5)); });
        s("roundtrip", function() { local C = class { v = 1; }; C.setattributes("v", { x = "y" }); return C.getattributes("v").x; });
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "replace_returns_old={a=1}|{b=2}\n\
         class_level_set=null|{z=9}\n\
         class_level_then_member=null|{z=9}|null\n\
         null_value={a=1}|null\n\
         missing_member!wrong index\n\
         nontable_value={a=1}\n\
         roundtrip=y\n"
    );
}

/// P22：属性的**时序/共享/继承边界**（语料 `tests-corpus/t_attrtiming.nut`，28 条，
/// 验证型——两侧本就逐字节一致）。钉住的语义：
///
/// 1. **时序**：`setattributes` 在实例化之后改，实例透过 `getclass()` 立即可见
///    （属性挂在类上，实例不快照）；**继承后改父类属性，子类不受影响**
///    （extend 时拷贝进子类）；给**只在父类**的成员 set/get 都直接可用。
/// 2. **类级属性（null 键）不随 extends 继承**，子类要自己设。
/// 3. **属性表是共享引用**：`getattributes("v").a.n = 42` 对后续读取可见；
///    而**给属性表加新键** `x.same = "yes"` 报 `the index 'same' does not exist`
///    —— 这是 Squirrel 核心语义"普通 `=` 不建新槽（要用 `<-`）"，不是属性特有。
/// 4. `newmember` 的 4 参形式 `(name, val, attrs, isstatic)` 属性表与 static 都生效；
///    不带 attrs 则为 null。
/// 5. 边界：`clone` 一个 **class** 报 `cloning a class`；实例**没有**
///    `setattributes/getattributes`；两者参数个数都是死规矩；
///    `delete C.v` 报 `attempt to delete a slot from a class`；
///    `rawset` 新成员属性为 null；构造函数可挂属性、`_tostring` 挂属性后
///    `getattributes("_tostring")` 报 `wrong index`（参考实现如此）。
#[test]
fn attribute_timing_sharing_and_inheritance() {
    let src = r#"
        function s(tag, f) { try { print(tag + "=" + f() + "\n"); } catch (e) { print(tag + "!" + e + "\n"); } }
        function d(x) {
            if (typeof x == "table") { local o = "{"; local first = true; foreach (k, v in x) { if (!first) o += ","; first = false; o += k + "=" + v; } return o + "}"; }
            return "" + x;
        }
        s("after_inst", function() { local C = class { </ a = 1 /> v = 1; }; local o = C(); C.setattributes("v", { a = 99 }); return d(o.getclass().getattributes("v")) + "/" + d(C.getattributes("v")); });
        s("parent_after_extend", function() { local A = class { </ a = 1 /> v = 1; }; local B = class extends A { }; A.setattributes("v", { a = 77 }); return "A=" + d(A.getattributes("v")) + " B=" + d(B.getattributes("v")); });
        s("set_inherited", function() { local A = class { v = 1; }; local B = class extends A { }; return d(B.setattributes("v", { a = 5 })) + "/" + d(B.getattributes("v")); });
        s("get_inherited", function() { local A = class { </ a = 1 /> v = 1; }; local B = class extends A { }; return d(B.getattributes("v")); });
        s("mid_override", function() { local A = class { </ a = "A" /> v = 1; }; local B = class extends A { </ a = "B" /> v = 2; }; local C = class extends B { }; return "A=" + d(A.getattributes("v")) + " B=" + d(B.getattributes("v")) + " C=" + d(C.getattributes("v")); });
        s("cl_inherit", function() { local A = class { }; A.setattributes(null, { lvl = "A" }); local B = class extends A { }; return "A=" + d(A.getattributes(null)) + " B=" + d(B.getattributes(null)); });
        s("shared_mutate", function() { local C = class { </ a = { n = 1 } /> v = 1; }; C.getattributes("v").a.n = 42; return C.getattributes("v").a.n; });
        s("shared_newkey", function() { local C = class { </ a = { n = 1 } /> v = 1; }; local x = C.getattributes("v"); x.same = "yes"; return d(x); });
        s("shared_same_view", function() { local C = class { </ a = { n = 1 } /> v = 1; }; local x = C.getattributes("v"); local y = C.getattributes("v"); x.a.n = 5; return x.a.n + "/" + y.a.n; });
        s("nm4", function() { local C = class { v = 1; }; C.newmember("w", 2, { a = 8 }, false); return d(C.getattributes("w")) + "/" + C.w; });
        s("nm4_static", function() { local C = class { v = 1; }; C.newmember("sw", 3, { a = 6 }, true); return d(C.getattributes("sw")) + "/" + C.sw + "/" + typeof C.rawget("sw"); });
        s("nm_noattrs", function() { local C = class { v = 1; }; C.newmember("w4", 5); return d(C.getattributes("w4")); });
        s("clone_class", function() { local A = class { </ a = 1 /> v = 1; }; return clone A; });
        s("inst_set", function() { local C = class { </ a = 1 /> v = 1; }; local o = C(); return d(o.setattributes("v", { a = 3 })); });
        s("inst_get", function() { local C = class { </ a = 1 /> v = 1; }; local o = C(); return d(o.getattributes(null)); });
        s("ga_two", function() { local C = class { v = 1; }; return C.getattributes("v", 2); });
        s("sa_three", function() { local C = class { v = 1; }; return C.setattributes("v", {}, 3); });
        s("ctor_attr", function() { local C = class { </ a = 1 /> constructor() { } }; return d(C.getattributes("constructor")); });
        s("meta_attr", function() { local C = class { </ a = 1 /> function _tostring() { return "C"; } }; return d(C.getattributes("_tostring")) + "/" + ("" + C()); });
        s("delete_member", function() { local C = class { </ a = 1 /> v = 1; }; local r = delete C.v; return "" + r + " attr=" + d(C.getattributes("v")); });
        s("rawset_new", function() { local C = class { </ a = 1 /> v = 1; }; C.rawset("w", 2); return "w=" + d(C.getattributes("w")) + " v=" + d(C.getattributes("v")); });
        s("class_value", function() { local Inner = class { q = 1 }; local C = class { v = 1; }; C.setattributes("v", Inner); return typeof C.getattributes("v"); });
        s("static_shadow", function() { local C = class { </ a = 1 /> static v = 10; }; local o = C(); return "" + C.v + "/" + o.v + "/" + d(C.getattributes("v")); });
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "after_inst={a=99}/{a=99}\n\
         parent_after_extend=A={a=77} B={a=1}\n\
         set_inherited=null/{a=5}\n\
         get_inherited={a=1}\n\
         mid_override=A={a=A} B={a=B} C={a=B}\n\
         cl_inherit=A={lvl=A} B=null\n\
         shared_mutate=42\n\
         shared_newkey!the index 'same' does not exist\n\
         shared_same_view=5/5\n\
         nm4={a=8}/2\n\
         nm4_static={a=6}/3/integer\n\
         nm_noattrs=null\n\
         clone_class!cloning a class\n\
         inst_set!the index 'setattributes' does not exist\n\
         inst_get!the index 'getattributes' does not exist\n\
         ga_two!wrong number of parameters\n\
         sa_three!wrong number of parameters\n\
         ctor_attr={a=1}\n\
         meta_attr!wrong index\n\
         delete_member!attempt to delete a slot from a class\n\
         rawset_new=w=null v={a=1}\n\
         class_value=class\n\
         static_shadow=10/10/{a=1}\n"
    );
}

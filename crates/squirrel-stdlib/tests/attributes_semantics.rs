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

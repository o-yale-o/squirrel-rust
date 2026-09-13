//! 序列化往返回归（P1-10）：脚本侧 `writeclosuretofile` / `loadfile`。
//!
//! 语料侧是 `tests-corpus/t_sermap.nut`（25 条，两侧只差指针文本）。
//! 这一轮修出一处错误文本差异：**native closure** 传进 `writeclosuretofile` 时，
//! 它过得了 `.sc` 类型掩码（掩码接受 closure|nativeclosure），但过不了
//! `sq_aux_gettypedarg(OT_CLOSURE)` 的检查，于是报
//!
//! ```text
//! wrong argument type, expected 'function' got 'function'
//! ```
//!
//! 那句"expected 'function' got 'function'"正是参考实现的原文：后半段是
//! `PrintObjVal`（对 native closure 就退化成类型名）。本实现原来只报 `closure expected`。
//!
//! 另外固化：带自由变量的闭包不能序列化（`a closure with free variables bound
//! cannot be serialized`）；`loadfile` 对文本文件/空文件会**当作源码编译**并返回闭包；
//! 往返后的闭包可以再写出（`double_roundtrip`）；`.cnut` 的**字节一致**另有
//! `tools/cnut_matrix.py` 守门（69/69 字节一致 + 双向交叉执行）。

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
    let rc = vm.run_buffer(src.as_bytes(), b"ser.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 往返：各种闭包形态写出去再读回来
#[test]
fn closure_roundtrip_matches_reference() {
    let dir = std::env::temp_dir().join("sqrust_ser_semantics");
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("t.cnut").to_string_lossy().replace('\\', "\\\\");
    let src = format!(
        r#"
        local P = "{p}";
        function s(tag, f) {{ try {{ print(tag + "=" + f() + "\n"); }} catch (e) {{ print(tag + "!" + e + "\n"); }} }}
        s("plain", function() {{ local f = function() {{ return 42; }}; writeclosuretofile(P, f); return typeof loadfile(P) + "/" + loadfile(P)(); }});
        s("args", function() {{ local f = function(a, b) {{ return a + b; }}; writeclosuretofile(P, f); return loadfile(P)(2, 3); }});
        s("defaults", function() {{ local f = function(a, b = 10) {{ return a + b; }}; writeclosuretofile(P, f); local g = loadfile(P); return g(1) + "/" + g(1, 2); }});        s("varargs", function() {{ local f = function(...) {{ return vargv.len(); }}; writeclosuretofile(P, f); return loadfile(P)(1, 2, 3); }});
        s("strings", function() {{ local f = function() {{ return "abc\u00e9\xff"; }}; writeclosuretofile(P, f); local v = loadfile(P)(); return v.len() + "/" + (v == "abc\u00e9\xff"); }});
        s("floats", function() {{ local f = function() {{ return 1.5 + 0.25; }}; writeclosuretofile(P, f); return loadfile(P)(); }});
        s("class", function() {{ local f = function() {{ local C = class {{ v = 7; function get() {{ return v; }} }}; return C().get(); }}; writeclosuretofile(P, f); return loadfile(P)(); }});
        s("nested", function() {{ local f = function() {{ local inner = function(q) {{ return q * 2; }}; return inner(21); }}; writeclosuretofile(P, f); return loadfile(P)(); }});
        s("control_flow", function() {{ local f = function(n) {{ local t = 0; for (local i = 0; i < n; i++) if (i % 2 == 0) t += i; return t; }}; writeclosuretofile(P, f); return loadfile(P)(10); }});
        s("generator_inside", function() {{ local f = function() {{ local g = (function() {{ yield 1; yield 2; }})(); return resume g + resume g; }}; writeclosuretofile(P, f); return loadfile(P)(); }});
        s("double_roundtrip", function() {{ local f = function(a) {{ return a * 3; }}; writeclosuretofile(P, f); local g = loadfile(P); writeclosuretofile(P, g); return loadfile(P)(4); }});
        s("with_outer", function() {{ local captured = 5; local f = function() {{ return captured; }}; return "" + writeclosuretofile(P, f); }});
        s("native_closure", function() {{ return "" + writeclosuretofile(P, print); }});
        s("non_closure", function() {{ return "" + writeclosuretofile(P, 5); }});
        s("table", function() {{ return "" + writeclosuretofile(P, {{}}); }});
        s("missing_dir", function() {{ local f = function() {{ return 1; }}; return "" + writeclosuretofile("no_such_dir_xyz/out.cnut", f); }});
        s("load_missing", function() {{ return "" + loadfile("no_such_file_xyz.cnut"); }});
    "#
    );
    let (out, _err) = run_capture(&src);
    // 注意 `defaults`：序列化**会丢默认参数值**（`g(1)` 里 b 是 null → arith 报错）——
    // 参考实现同样如此（语料里也已确认），这是被忠实复刻的怪癖。
    // `strings` 长度 6 = "abc" + C3 A9（\u00e9 的 UTF-8 两字节）+ FF ✓
    assert_eq!(
        out,
        "plain=function/42\nargs=5\ndefaults!arith op + on between 'integer' and 'null'\n\
         varargs=3\nstrings=6/true\nfloats=1.75\n\
         class=7\nnested=42\ncontrol_flow=20\ngenerator_inside=3\ndouble_roundtrip=12\n\
         with_outer!a closure with free variables bound cannot be serialized\n\
         native_closure!wrong argument type, expected 'function' got 'function'\n\
         non_closure!parameter 2 has an invalid type 'integer' ; expected: 'function|function'\n\
         table!parameter 2 has an invalid type 'table' ; expected: 'function|function'\n\
         missing_dir!cannot open the file\nload_missing!cannot open the file\n"
    );
}

/// `loadfile` 对**文本 / 空文件**会当作源码编译（↔ sqstd_loadfile 的嗅探逻辑）
#[test]
fn loadfile_sniffs_text_and_empty_files() {
    let dir = std::env::temp_dir().join("sqrust_ser_semantics");
    let _ = std::fs::create_dir_all(&dir);
    let txt = dir.join("t.nut");
    let empty = dir.join("e.bin");
    std::fs::write(&txt, b"return 7;").unwrap();
    std::fs::write(&empty, b"").unwrap();
    let (txt, empty) = (
        txt.to_string_lossy().replace('\\', "\\\\"),
        empty.to_string_lossy().replace('\\', "\\\\"),
    );
    let src = format!(
        r#"
        print(typeof loadfile("{txt}") + "/" + loadfile("{txt}")() + "\n");
        print(typeof loadfile("{empty}") + "\n");
    "#
    );
    let (out, _err) = run_capture(&src);
    assert_eq!(out, "function/7\nfunction\n");
}

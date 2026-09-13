//! 词法 / 优先级 / 作用域语义回归（P1-3）。
//!
//! 语料侧是 `tests-corpus/t_lexmatrix.nut`（203 条，两侧逐字节一致）；
//! 这里固化其中**最容易回归**的几条，其中两条是本轮修出来的真实差异：
//!
//! 1. **字符串下标是有符号的**：参考实现是非 UNICODE 构建（`SQChar` = `char`），
//!    `"\u00e9"[0]` 取到的是 `-61`（0xC3 符号扩展），不是 `195`；
//!    `foreach (c in "…")` 同一条路径。本实现原来返回 0..255。
//!    **blob 不同**：`b[0] = 255` 拿回 `255`（`unsigned char`）。
//! 2. **`escape()` 遇高位字节是参考实现的 UB**（用 `(char)c` 传给 `%02x`，
//!    写出 8 位补码并越界写 scratchpad，实测输出串行、混进别的错误消息片段），
//!    所以高位字节只比字节值、不比 `escape()`。
//!
//! 另外固化：转义表、verbatim 串、优先级/结合性、短路求值、作用域遮蔽与循环捕获、
//! `++`/`--` 的返回值、`__LINE__`、switch 贯穿。

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
    let rc = vm.run_buffer(src.as_bytes(), b"lex.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 字符串下标/foreach 的有符号性（blob 是无符号）
#[test]
fn string_bytes_are_signed_like_sqchar() {
    let src = r#"
        function bytes(v) {
            local out = v.len() + "[";
            for (local i = 0; i < v.len(); i++) { if (i) out += ","; out += v[i]; }
            return out + "]";
        }
        print(bytes("\x41") + "\n");
        print(bytes("\xff") + "\n");
        print(bytes("\u0041") + "\n");
        print(bytes("\u00e9") + "\n");
        print(bytes("\u4e2d") + "\n");
        print(bytes("\U0001F600") + "\n");
        print("abc"[0] + "/" + "abc"[2] + "/" + "abc"[-1] + "\n");
        local out = "";
        foreach (c in "\u00e9") out += c + ",";
        print(out + "\n");
        print(("\u00e9" == "\xc3\xa9") + "/" + ("\u00e9" == "\xe9") + "\n");
        local b = blob(2);
        b[0] = 233; b[1] = 255;
        print(b[0] + "/" + b[1] + "\n");
        try { print("abc"[3]); } catch (e) { print("E:" + e + "\n"); }
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "1[65]\n1[-1]\n1[65]\n2[-61,-87]\n3[-28,-72,-83]\n4[-16,-97,-104,-128]\n\
         97/99/99\n-61,-87,\ntrue/false\n233/255\nE:the index '3' does not exist\n"
    );
}

/// 优先级 / 结合性 / 短路 / 作用域 / 自增返回值
#[test]
fn precedence_scoping_and_side_effects_match_reference() {
    let src = r#"
        function p(tag, v) { print(tag + "=" + v + "\n"); }
        p("add_mul", 1 + 2 * 3);
        p("sub_assoc", 10 - 3 - 2);
        p("div_assoc", 100 / 5 / 2);
        p("shift_add", 1 << 2 + 3);
        p("and_or", 1 | 2 & 3);
        p("eq_lt", 1 < 2 == true);
        p("and_lt", 1 < 2 && 3 < 4);
        p("or_and", false || true && false);
        p("not_eq", !true == false);
        p("ushift_neg", -8 >>> 1);
        local n = 0;
        local r = false && (n = 1);
        p("short_and", n + "/" + r);
        local m = 0;
        local q = true || (m = 1);
        p("short_or", m + "/" + q);
        local x = 1;
        p("post", (x++) + "/" + x);
        p("pre", (++x) + "/" + x);
        local t = { v = 1 };
        p("slot_post", (t.v++) + "/" + t.v);
        local f;
        { local inner = 9; f = function() { return inner; }; }
        p("block_capture", f());
        local fs = [];
        for (local i = 0; i < 3; i++) fs.append(function() { return i; });
        p("for_capture", fs[0]() + "/" + fs[1]() + "/" + fs[2]());
        local gs = [];
        foreach (v in [10, 20]) gs.append(function() { return v; });
        p("foreach_capture", gs[0]() + "/" + gs[1]());
        local s2 = 1;
        { local s2 = 2; s2 = 3; }
        p("shadow", s2);
        p("switch_fall", (function() { local o = ""; switch (1) { case 1: o += "1"; case 2: o += "2"; default: o += "d"; } return o; })());
        p("verb_quote", @"a""b");
        p("verb_esc_len", @"a\nb".len());
        p("line_type", typeof __LINE__);
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "add_mul=7\nsub_assoc=5\ndiv_assoc=10\nshift_add=32\nand_or=3\neq_lt=true\nand_lt=true\n\
         or_and=false\nnot_eq=true\nushift_neg=9223372036854775804\nshort_and=0/false\n\
         short_or=0/true\npost=1/2\npre=3/3\nslot_post=1/2\nblock_capture=9\nfor_capture=3/3/3\n\
         foreach_capture=20/20\nshadow=1\nswitch_fall=12d\nverb_quote=a\"b\nverb_esc_len=4\n\
         line_type=integer\n"
    );
}

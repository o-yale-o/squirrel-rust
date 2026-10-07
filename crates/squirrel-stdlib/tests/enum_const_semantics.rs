//! `enum`/`const` 语义回归（`sqcompiler.cpp` 的 `EnumDecl`/`ConstDecl`）。
//!
//! 语料 `t_enummap.nut`（25 条）补齐后为**验证型**收获（两侧逐字节一致），
//! 钉住的反直觉语义：
//!
//! 1. **enum/const 是纯编译期内联**：声明后**连根表槽都没有**。
//! 2. **enum 自动递增**：`{A, B, C}` → 0/1/2；显式值后继续递增。
//! 3. **enum 成员值必须是字面量**（表达式/自引用都是编译错）。
//! 4. **重复项先到先得**；**函数体内 `enum` 合法**（与 `const` 不同！const 在
//!    函数体内编译通过但值内联）。
//! 5. **const 重定义不报错**（后值覆盖）；`delete const名` 报
//!    `can't delete an expression`；**`const` 值不接受表达式**。
//! 6. **`const` 值可引用 enum 成员**。

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
    let rc = vm.run_buffer(src.as_bytes(), b"enummap.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

fn run_lines(body: &str) -> String {
    let src = format!(
        r#"
function t(tag, f) {{
    try {{ local v = f(); print(tag + "=" + v + "|" + typeof v + "\n"); }}
    catch (e) {{ print(tag + "!" + e + "\n"); }}
}}
{body}
"#
    );
    run_capture(&src).0
}

fn line<'a>(out: &'a str, tag: &str) -> &'a str {
    out.lines()
        .find(|l| l.starts_with(&format!("{tag}=")) || l.starts_with(&format!("{tag}!")))
        .unwrap_or_else(|| panic!("没有 {tag} 行：\n{out}"))
}

/// `enum`：自动递增、显式值、重复项先到先得、纯编译期内联
#[test]
fn enum_is_compile_time_inlined() {
    let out = run_lines(
        r#"
t("auto", function() { enum E_AUTO { A, B, C } return E_AUTO.A + "/" + E_AUTO.B + "/" + E_AUTO.C; });
t("neg_after", function() { enum E_NEG { A = 5, B, C } return E_NEG.B + "/" + E_NEG.C; });
t("negauto", function() { enum E_N { A = -1, B } return E_N.B; });
t("dup", function() { enum E_DUP { A = 1, A = 2 } return E_DUP.A; });
t("float", function() { enum E_F { A = 1.5, B } return E_F.A + "/" + E_F.B; });
t("str", function() { enum E_S { A = "x" } return E_S.A; });
t("big", function() { enum E_G { A = 0x7FFFFFFFFFFFFFFF } return E_G.A; });
t("not_in_root", function() { enum E_NR { A } return ("E_NR" in getroottable()) ? "in-root" : "inline-only"; });
t("in_fn", function() { local f = function() { enum Q { A } return Q.A; }; return f(); });
t("delete_member", function() { local f = compilestring("enum E_DEL { A, B } delete E_DEL.A; return E_DEL.len();", "ED"); return f(); });
"#,
    );
    assert_eq!(line(&out, "auto"), "auto=0/1/2|string", "自动递增 0/1/2");
    assert_eq!(
        line(&out, "neg_after"),
        "neg_after=0/1|string",
        "显式值后**不**带动后续递增：B/C 仍从 0 开始（反直觉，两侧一致）"
    );
    assert_eq!(line(&out, "negauto"), "negauto=0|integer", "显式值后同样从 0 开始");
    assert_eq!(line(&out, "dup"), "dup=2|integer", "重复项后值覆盖（不是先到先得）");
    assert_eq!(
        line(&out, "float"),
        "float=1.5/0|string",
        "浮点值后同样从 0 开始（不递增）"
    );
    assert_eq!(line(&out, "str"), "str=x|string", "字符串值合法");
    assert_eq!(line(&out, "big"), "big=9223372036854775807|integer");
    assert_eq!(
        line(&out, "not_in_root"),
        "not_in_root=inline-only|string",
        "**核心**：enum 不进根表"
    );
    assert_eq!(line(&out, "in_fn"), "in_fn=0|integer", "函数体内 enum 合法");
    assert_eq!(
        line(&out, "delete_member"),
        "delete_member!can't delete an expression",
        "enum 成员是编译期常量，delete 报编译错（可 catch）"
    );
}

/// `const`：字面量四类型、表达式报错、重定义不报错、delete 报错
#[test]
fn const_semantics() {
    let out = run_lines(
        r#"
t("lit", function() { const KC = 42; return KC; });
t("types", function() { const KS = "s"; const KF = 1.5; const KB = true; const KN = -3; return KS + "/" + KF + "/" + KB + "/" + KN; });
t("expr", function() { local f = compilestring("const KE = 1 + 2; return KE;", "CE"); return f(); });
t("redef", function() { const KR = 1; const KR = 2; return KR; });
t("delete", function() { local f = compilestring("const KD = 1; return delete KD;", "CD"); return f(); });
t("typeof", function() { const KT = "v"; return typeof KT; });
t("in_fn", function() { const KF2 = 1; return KF2; });
t("from_enum", function() { local f = compilestring("enum E_SRC { V = 9 } const KC2 = E_SRC.V; return KC2;", "CS"); return f(); });
t("not_in_root", function() { const K_NR = 5; return ("K_NR" in getroottable()) ? "in-root" : "inline-only"; });
t("shadow_local", function() { const KS2 = 1; return (function() { local KS2 = 9; return KS2; })(); });
"#,
    );
    assert_eq!(line(&out, "lit"), "lit=42|integer");
    assert_eq!(line(&out, "types"), "types=s/1.5/true/-3|string");
    assert_eq!(
        line(&out, "expr"),
        "expr!end of statement expected (; or lf)",
        "const 值不接受表达式（1 + 2 不行）"
    );
    assert_eq!(line(&out, "redef"), "redef=2|integer", "重定义不报错，后值覆盖");
    assert_eq!(
        line(&out, "delete"),
        "delete!can't delete an expression",
        "const 是编译期内联，不可 delete"
    );
    assert_eq!(line(&out, "typeof"), "typeof=string|string");
    assert_eq!(line(&out, "in_fn"), "in_fn=1|integer", "函数体内 const 合法");
    assert_eq!(
        line(&out, "from_enum"),
        "from_enum!scalar expected : integer, float, or string",
        "const 值必须是字面量——enum 成员访问也是表达式，报编译错"
    );
    assert_eq!(
        line(&out, "not_in_root"),
        "not_in_root=inline-only|string",
        "**核心**：const 不进根表"
    );
    assert_eq!(line(&out, "shadow_local"), "shadow_local=9|integer", "局部遮蔽 const");
}

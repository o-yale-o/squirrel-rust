//! 序列化「值形状」回归：`writeclosuretofile` → `loadfile` 往返后值还是不是原来的值。
//!
//! 语料侧是 `tests-corpus/t_sershape.nut`（67 条，两侧逐字节一致）。原有的
//! `serialization_semantics.rs` 测的是"能不能写/读文件"；这里补的是**值本身的往返**。
//!
//! 关键点（都是实测，不是照文档想当然）：
//!
//! 1. **值必须是闭包里的字面量**才参与往返。用外部 `local` 会先撞
//!    "a closure with free variables bound cannot be serialized"，
//!    根本走不到值序列化 —— 第一版探针就是这么写错的。
//! 2. **默认参数只在"被直接序列化的那个闭包自己"上会丢**：`g(1)` 里 `b` 变 `null`；
//!    但闭包**内部**嵌套函数的默认值（`b=100`）、它**返回出去**的内部闭包
//!    （`inner=77`）、varargs（`vargv.len()==3`）全都完好。老文档只写"默认参数会丢"，
//!    实测范围窄得多。
//! 3. 生成器闭包往返后仍是生成器（`typeof g() == "generator"`、
//!    `getstatus() == "suspended"`、resume 依次给 1/2）。
//! 4. 参考实现两处**有趣的错误文本**（我们逐字复刻）：
//!    - `writeclosuretofile(P, print)` → `wrong argument type, expected 'function' got 'function'`
//!    - `writeclosuretofile(P, 1)` → `parameter 2 has an invalid type 'integer' ; expected: 'function|function'`
//! 5. 坏文件：只有一个字节的 `.cnut` → `unexpected character(control)`（可捕获）。
//!
//! 安全边界：自引用结构两侧都报同样的错（可进语料）；**深嵌套值到 ~10000 层时参考实现
//! 栈溢出崩溃**（`0xC00000FD`，属已记录的"深嵌套值释放"族），语料只用到几层。

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
    let rc = vm.run_buffer(src.as_bytes(), b"sershape.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// 每行 `tag=值|类型|err=...`（与语料 `t_sershape.nut` 同一口径）
///
/// `tag` 用来给临时 `.cnut` 取**独立文件名**：cargo 默认并行跑同一个文件里的多个
/// `#[test]`，共用一个路径会互相覆盖（实测出过 `null` 用例返回另一个用例的 120）。
fn run_lines(body: &str, tag: &str) -> String {
    let dir = std::env::temp_dir().join("sqrust_ser_shapes");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{tag}.cnut")).to_string_lossy().replace('\\', "\\\\");
    let src = format!(
        r#"
PATH <- "{path}";
function show(r, d = 0) {{
    if (d > 4) return "...";
    local t = typeof r;
    if (t == "array") {{ local s = "["; for (local i = 0; i < r.len(); i++) {{ if (i > 0) s += ","; s += show(r[i], d + 1); }} return s + "]"; }}
    if (t == "table") {{ local s = "{{"; foreach (k, v in r) s += show(k, d + 1) + ":" + show(v, d + 1) + ","; return s + "}}"; }}
    if (t == "string") return "\"" + r + "\"";
    if (t == "float") return format("%.17g", r);
    return "" + r;
}}
function rt(f) {{
    writeclosuretofile(PATH, f);
    local g = loadfile(PATH);
    local r = null; local err = null;
    try {{ r = g(); }} catch (e) {{ err = "" + e; }}
    return show(r) + "|" + typeof r + "|err=" + show(err);
}}
function t(tag, f) {{
    try {{ print(tag + "=" + f() + "\n"); }} catch (e) {{ print(tag + "!" + e + "\n"); }}
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

/// 标量与容器：往返后值与类型都不变
#[test]
fn scalar_and_container_shapes_round_trip() {
    let out = run_lines(
        r#"
t("null", function() { return rt(function() { return null; }); });
t("true", function() { return rt(function() { return true; }); });
t("int_min", function() { return rt(function() { return -9223372036854775807 - 1; }); });
t("float_neg0", function() { return rt(function() { return -0.0; }); });
t("str_escapes", function() { return rt(function() { return "a\tb\\c"; }); });
t("arr_mixed", function() { return rt(function() { return [1, "a", null, true, 1.5]; }); });
t("arr_nested", function() { return rt(function() { return [1, [2, [3, [4]]]]; }); });
t("tbl_int_key", function() { return rt(function() { return { [1] = "intkey" }; }); });
t("tbl_mixed_keys", function() { return rt(function() { return { a = 1, [2] = "b" }; }); });
t("tbl_nested", function() { return rt(function() { return { a = { b = { c = 3 } } }; }); });
"#,
        "scalar",
    );
    assert_eq!(line(&out, "null"), "null=null|null|err=null");
    assert_eq!(line(&out, "true"), "true=true|bool|err=null");
    assert_eq!(line(&out, "int_min"), "int_min=-9223372036854775808|integer|err=null");
    assert_eq!(line(&out, "float_neg0"), "float_neg0=-0|float|err=null", "-0.0 要保住符号");
    assert_eq!(line(&out, "str_escapes"), "str_escapes=\"a\tb\\c\"|string|err=null");
    assert_eq!(line(&out, "arr_mixed"), "arr_mixed=[1,\"a\",null,true,1.5]|array|err=null");
    assert_eq!(line(&out, "arr_nested"), "arr_nested=[1,[2,[3,[4]]]]|array|err=null");
    assert_eq!(line(&out, "tbl_int_key"), "tbl_int_key={1:\"intkey\",}|table|err=null");
    assert_eq!(line(&out, "tbl_mixed_keys"), "tbl_mixed_keys={\"a\":1,2:\"b\",}|table|err=null");
    assert_eq!(line(&out, "tbl_nested"), "tbl_nested={\"a\":{\"b\":{\"c\":3,},},}|table|err=null");
}

/// 代码形状：嵌套闭包、递归、类、生成器都能往返并可继续工作
#[test]
fn code_shapes_round_trip() {
    let out = run_lines(
        r#"
t("closure_recursive", function() {
    return rt(function() { local f = null; f = function(n) { return n <= 1 ? 1 : n * f(n - 1); }; return f(5); });
});
t("closure_varargs", function() {
    return rt(function() { local f = function(...) { return vargv.len(); }; return f(1, 2, 3); });
});
t("class_inherit", function() {
    return rt(function() { class B { function who() { return "B"; } } class D extends B { function who() { return "D" + base.who(); } } return D().who(); });
});
t("class_attrs", function() {
    return rt(function() { class C { </ doc = "d" /> x = 1; } return C.getattributes("x").doc; });
});
t("gen_roundtrip", function() {
    writeclosuretofile(PATH, function() { yield 1; yield 2; });
    local g = loadfile(PATH);
    local made = g();
    local a = resume made; local b = resume made;
    return typeof g + "/" + typeof made + "/" + made.getstatus() + "/" + a + "," + b;
});
t("blob_in_closure", function() {
    return rt(function() { local b = blob(2); b.writen(0x0102, 'w'); b.seek(0, 'b'); return b.readn('w'); });
});
"#,
        "code",
    );
    assert_eq!(line(&out, "closure_recursive"), "closure_recursive=120|integer|err=null");
    assert_eq!(line(&out, "closure_varargs"), "closure_varargs=3|integer|err=null");
    assert_eq!(line(&out, "class_inherit"), "class_inherit=\"DB\"|string|err=null");
    assert_eq!(line(&out, "class_attrs"), "class_attrs=\"d\"|string|err=null");
    assert_eq!(
        line(&out, "gen_roundtrip"),
        "gen_roundtrip=function/generator/suspended/1,2",
        "往返后仍要能被识别成生成器"
    );
    assert_eq!(line(&out, "blob_in_closure"), "blob_in_closure=258|integer|err=null");
}

/// 默认参数的丢失范围：**只有被直接序列化的那个闭包自己**（实测，比老文档窄得多）
#[test]
fn default_params_lost_only_on_serialized_closure_itself() {
    let src = r#"
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); } catch (e) { print(tag + "!" + e + "\n"); }
}
t("top_default_lost", function() {
    writeclosuretofile(PATH, function(a, b = 100) { return "b=" + b; });
    local g = loadfile(PATH);
    return g(1);
});
t("top_default_given", function() {
    writeclosuretofile(PATH, function(a, b = 100) { return "b=" + b; });
    local g = loadfile(PATH);
    return g(1, 2);
});
t("nested_default_kept", function() {
    writeclosuretofile(PATH, function() { local f = function(a, b = 100) { return "b=" + b; }; return f(1); });
    local g = loadfile(PATH);
    return g();
});
t("returned_inner_kept", function() {
    writeclosuretofile(PATH, function() { return function(a, b = 77) { return "inner=" + b; }; });
    local g = loadfile(PATH);
    return g()(1);
});
t("varargs_kept", function() {
    writeclosuretofile(PATH, function(...) { return "n=" + vargv.len(); });
    local g = loadfile(PATH);
    return g(1, 2, 3);
});
"#;
    let dir = std::env::temp_dir().join("sqrust_ser_shapes");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("t2.cnut").to_string_lossy().replace('\\', "\\\\");
    let out = run_capture(&format!("PATH <- \"{path}\";\n{src}")).0;
    assert_eq!(line(&out, "top_default_lost"), "top_default_lost=b=null");
    assert_eq!(line(&out, "top_default_given"), "top_default_given=b=2");
    assert_eq!(line(&out, "nested_default_kept"), "nested_default_kept=b=100", "嵌套函数的默认值保住");
    assert_eq!(line(&out, "returned_inner_kept"), "returned_inner_kept=inner=77");
    assert_eq!(line(&out, "varargs_kept"), "varargs_kept=n=3");
}

/// `writeclosuretofile` 的**检查顺序**（模糊测试在私有工作目录里跑出来的差异，已修）。
///
/// 参考实现 `sqstd_writeclosuretofile` 是：先 `fopen(filename, "wb+")`，再交给
/// `sq_writeclosure`。加上外层 native 的参数掩码 `.sc`，实际顺序是：
///   1. 掩码先查：第 3 参是整数 → `parameter 2 has an invalid type ...`，**不建文件**；
///   2. 过了掩码就 `fopen`：不可写 → `cannot open the file`（**哪怕第 3 参是 native
///      closure、后面还会类型不符**）；
///   3. 可写则文件立刻建出来，之后无论序列化失败（自由变量）还是 native closure 不匹配
///      `OT_CLOSURE`，那个 **0 字节文件都会留在磁盘上**（参考实现失败分支也 fclose，但不删）。
#[test]
fn writeclosuretofile_checks_order_like_reference() {
    let dir = std::env::temp_dir().join("sqrust_ser_wcf_order");
    let _ = std::fs::create_dir_all(&dir);
    let ok_path = dir.join("ok.cnut").to_string_lossy().replace('\\', "\\\\");
    let bad_path = "no_such_dir_zz/x.cnut";
    let src = format!(
        r#"
function exists(p) {{
    try {{ local fh = file(p, "rb"); local n = fh.len(); fh.close(); return "len=" + n; }}
    catch (e) {{ return "missing"; }}
}}
function t(tag, f) {{
    try {{ print(tag + "=" + f() + "\n"); }} catch (e) {{ print(tag + "!" + e + "\n"); }}
}}
t("int_badpath", function() {{ writeclosuretofile("{bad_path}", 1); return "no-error"; }});
t("int_badpath_exists", function() {{ return exists("{bad_path}"); }});
t("native_badpath", function() {{ writeclosuretofile("{bad_path}", print); return "no-error"; }});
t("native_ok", function() {{ writeclosuretofile("{ok_path}", print); return "no-error"; }});
t("native_ok_exists", function() {{ return exists("{ok_path}"); }});
t("freevar_ok", function() {{
    local fv = 5;
    local bad = function() {{ return fv; }};
    writeclosuretofile("{ok_path}", bad);
    return "no-error";
}});
t("freevar_ok_exists", function() {{ return exists("{ok_path}"); }});
t("closure_ok", function() {{ writeclosuretofile("{ok_path}", function() {{ return 1; }}); return "no-error"; }});
t("closure_ok_exists", function() {{ return exists("{ok_path}"); }});
"#
    );
    let (out, _err) = run_capture(&src);
    // 1. 掩码先查，不建文件
    assert_eq!(
        line(&out, "int_badpath"),
        "int_badpath!parameter 2 has an invalid type 'integer' ; expected: 'function|function'"
    );
    assert_eq!(line(&out, "int_badpath_exists"), "int_badpath_exists=missing");
    // 2. 不可写路径即使第 3 参类型不符，也先报打不开
    assert_eq!(line(&out, "native_badpath"), "native_badpath!cannot open the file");
    // 3. 可写：先建文件，失败也留着（0 字节）
    assert_eq!(
        line(&out, "native_ok"),
        "native_ok!wrong argument type, expected 'function' got 'function'"
    );
    assert_eq!(line(&out, "native_ok_exists"), "native_ok_exists=len=0");
    assert_eq!(
        line(&out, "freevar_ok"),
        "freevar_ok!a closure with free variables bound cannot be serialized"
    );
    assert_eq!(line(&out, "freevar_ok_exists"), "freevar_ok_exists=len=0");
    // 成功的写入自然留下非空文件
    assert_eq!(line(&out, "closure_ok"), "closure_ok=no-error");
    let last = line(&out, "closure_ok_exists");
    assert!(
        last.starts_with("closure_ok_exists=len=") && last != "closure_ok_exists=len=0",
        "成功写出后应该是非空文件：{last}"
    );
    let _ = std::fs::remove_file(dir.join("ok.cnut"));
}

#[test]
fn serialization_error_paths() {
    let out = run_lines(
        r#"
function terr(f) {
    local r = null; local err = null;
    try { r = f(); } catch (e) { err = "" + e; }
    return show(r) + "|err=" + show(err);
}
t("native_direct", function() { return terr(function() { writeclosuretofile(PATH, print); }); });
t("badarg", function() { return terr(function() { writeclosuretofile(PATH, 1); }); });
t("noargs", function() { return terr(function() { writeclosuretofile(); }); });
t("badpath", function() { return terr(function() { writeclosuretofile("no_such_dir/x.cnut", function() { return 1; }); }); });
t("freevar", function() {
    local n = 5;
    local f = function() { return n; };
    return terr(function() { writeclosuretofile(PATH, f); });
});
t("load_missing", function() { return terr(function() { return loadfile("tmp-c2/definitely_missing_xyz.cnut"); }); });
"#,
        "errs",
    );
    // 参考实现原文如此：'function' got 'function'
    assert_eq!(
        line(&out, "native_direct"),
        "native_direct=null|err=\"wrong argument type, expected 'function' got 'function'\""
    );
    // 参考实现原文如此：'function|function'
    assert_eq!(
        line(&out, "badarg"),
        "badarg=null|err=\"parameter 2 has an invalid type 'integer' ; expected: 'function|function'\""
    );
    assert_eq!(line(&out, "noargs"), "noargs=null|err=\"wrong number of parameters\"");
    assert_eq!(line(&out, "badpath"), "badpath=null|err=\"cannot open the file\"");
    assert_eq!(
        line(&out, "freevar"),
        "freevar=null|err=\"a closure with free variables bound cannot be serialized\""
    );
    assert_eq!(line(&out, "load_missing"), "load_missing=null|err=\"cannot open the file\"");
}

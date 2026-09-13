//! io / system / date 语义回归（P1-5）。
//!
//! 语料侧是 `tests-corpus/t_sysmap.nut`（45 条，两侧逐字节一致）。这里固化本轮
//! **修出来的两处真实差异**与几条易错语义：
//!
//! 1. **`remove()` / `rename()`**：↔ `_system_remove` / `_system_rename` ——
//!    失败要**抛错**（`remove() failed` / `rename() failed`），成功返回 **0 个值**
//!    （脚本看到 `null`）。本实现原来返回 `-1` / `0`，两者都不对。
//! 2. **`system()`**：↔ CRT `system()`，子进程**继承**父进程 stdio。本实现原来用
//!    `.output()` 捕获后丢掉 —— `system("echo X")` 在参考实现里会把 X 打到脚本
//!    stdout 上，我们什么都不打。
//! 3. **`clock()`**：是**进程 CPU 时间**（`clock()/CLOCKS_PER_SEC`），不是墙上时间。
//!    本实现原来用 `SystemTime` 差值（类型上测不出来，语义上是错的）。
//!
//! `date()` 只对 **UTC**（第二个参数传 `'u'` 的字符码 117）断言确切字段值——
//! 本地时间依赖机器时区；另外 `date(负数)` 两侧都报 `crt api failure`。

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
    let rc = vm.run_buffer(src.as_bytes(), b"sys.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(rc.is_ok(), "脚本本身不该失败: {:?} / stderr={}", rc.err(), e);
    (o, e)
}

/// `date()` 的 UTC 字段（↔ `_system_date` 的 8 个字段）
#[test]
fn date_utc_fields_match_reference() {
    let src = r#"
        function d(t) {
            local x = date(t, 117);   // 'u' = 117 → gmtime
            return x.year + "-" + x.month + "-" + x.day + " " + x.hour + ":" + x.min + ":" + x.sec
                + " w" + x.wday + " y" + x.yday + " n" + (function() { local n = 0; foreach (k, v in x) n++; return n; })();
        }
        print(d(0) + "\n");
        print(d(1234567890) + "\n");
        print(d(2147483647) + "\n");
        print(d(4102444800) + "\n");
        print(d(951782400) + "\n");
        print(typeof date(0) + "\n");
        try { date(-1); print("no\n"); } catch (e) { print("E:" + e + "\n"); }
        try { date(0, "u"); print("no\n"); } catch (e) { print("E:" + e + "\n"); }
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "1970-0-1 0:0:0 w4 y0 n8\n\
         2009-1-13 23:31:30 w5 y43 n8\n\
         2038-0-19 3:14:7 w2 y18 n8\n\
         2100-0-1 0:0:0 w5 y0 n8\n\
         2000-1-29 0:0:0 w2 y59 n8\n\
         table\nE:crt api failure\n\
         E:parameter 2 has an invalid type 'string' ; expected: 'integer|float'\n"
    );
}

/// `remove()` / `rename()`：失败抛错、成功返回 null；`system()` 返回子进程退出码
#[test]
fn remove_rename_and_system_match_reference() {
    let dir = std::env::temp_dir().join("sqrust_sys_semantics");
    let _ = std::fs::create_dir_all(&dir);
    let a = dir.join("a.bin").to_string_lossy().replace('\\', "\\\\");
    let b = dir.join("b.bin").to_string_lossy().replace('\\', "\\\\");
    let missing = dir.join("nope.bin").to_string_lossy().replace('\\', "\\\\");
    let src = format!(
        r#"
        function t(f) {{ try {{ return "" + f(); }} catch (e) {{ return "E:" + e; }} }}
        local f = file("{a}", "wb"); f.writen(7, 'b'); f.close();
        print("rename_ok=" + t(function() {{ return rename("{a}", "{b}"); }}) + "\n");
        local g = file("{b}", "rb"); print("content=" + g.readn('b') + "\n"); g.close();
        print("remove_ok=" + t(function() {{ return remove("{b}"); }}) + "\n");
        print("remove_missing=" + t(function() {{ return remove("{missing}"); }}) + "\n");
        print("rename_missing=" + t(function() {{ return rename("{missing}", "{b}"); }}) + "\n");
        print("system_exit3=" + system("exit 3") + "\n");
        print("system_exit0=" + system("exit 0") + "\n");
        print("getenv_shape=" + (typeof getenv("PATH")).len() + "/" + typeof getenv("__SQ_RUST_NOPE__") + "\n");
        print("time_type=" + typeof time() + "/" + typeof clock() + "\n");
    "#
    );
    let (out, _err) = run_capture(&src);
    assert_eq!(
        out,
        "rename_ok=null\ncontent=7\nremove_ok=null\nremove_missing=E:remove() failed\n\
         rename_missing=E:rename() failed\nsystem_exit3=3\nsystem_exit0=0\n\
         getenv_shape=6/null\ntime_type=integer/float\n"
    );
}

/// `loadfile` / `dofile`（↔ `_g_io_loadfile` / `_g_io_dofile`，注册项 `-2, ".sb"`）
///
/// 关键语义（**第 2 个参数是 `printerror` 布尔**，不是脚本实参；额外实参被忽略）：
/// `dofile(p)` 会**执行**脚本并返回其返回值、`loadfile(p)` 返回一个闭包；
/// 编译错误与 `cannot open the file` 都是可捕获错误。
/// 临时脚本文件由 Rust 写好 —— 脚本侧没有"往文件写字符串"的 API（见下条注释）。
#[test]
fn loadfile_and_dofile_match_reference() {
    let dir = std::env::temp_dir().join("sqrust_sys_semantics");
    let _ = std::fs::create_dir_all(&dir);
    let ok = dir.join("ok.nut");
    let bad = dir.join("bad.nut");
    std::fs::write(&ok, b"print(\"inner\\n\"); return 42;").unwrap();
    std::fs::write(&bad, b"local = ;").unwrap();
    let q = |p: &std::path::Path| p.to_string_lossy().replace('\\', "\\\\");
    let (ok, bad) = (q(&ok), q(&bad));
    let src = format!(
        r#"
        function s(tag, f) {{ try {{ print(tag + "=" + f() + "\n"); }} catch (e) {{ print(tag + "!" + e + "\n"); }} }}
        local OK = "{ok}";
        local BAD = "{bad}";
        s("dofile_ok", function() {{ return dofile(OK); }});
        s("dofile_ok_false", function() {{ return dofile(OK, false); }});
        s("dofile_str2nd", function() {{ return dofile(OK, "x"); }});
        s("dofile_bad", function() {{ return dofile(BAD); }});
        s("dofile_missing", function() {{ return dofile("no_such_xyz.nut"); }});
        s("loadfile_ok", function() {{ return typeof loadfile(OK); }});
        s("loadfile_call", function() {{ return loadfile(OK)(); }});
        s("loadfile_missing", function() {{ return typeof loadfile("no_such_xyz.nut"); }});
        s("loadfile_nonbool2", function() {{ return typeof loadfile(OK, 1); }});
    "#
    );
    let (out, _err) = run_capture(&src);
    assert_eq!(
        out,
        "inner\ndofile_ok=42\ninner\ndofile_ok_false=42\n\
         dofile_str2nd!parameter 2 has an invalid type 'string' ; expected: 'bool'\n\
         dofile_bad!expected 'IDENTIFIER'\ndofile_missing!cannot open the file\n\
         loadfile_ok=function\ninner\nloadfile_call=42\n\
         loadfile_missing!cannot open the file\n\
         loadfile_nonbool2!parameter 2 has an invalid type 'integer' ; expected: 'bool'\n"
    );
}

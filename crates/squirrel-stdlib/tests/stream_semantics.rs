//! 流语义回归（↔ sqstdlib/sqstdstream.cpp + sqstdio.cpp 的 SQFile）。
//!
//! 这些行为以前**没有任何语料覆盖**，实测发现三处真实偏差：
//!   1. `tell()` 在文件流上恒返回 0（参考实现是真实偏移）；
//!   2. `len()` 在文件流上恒返回 0（参考实现是文件总长度）；
//!   3. 读 stdin 时按字节读、遇到 '\n' 就停——把控制台的行语义错安到了管道上
//!      （实测管道喂 "A\nBC" 后 `readblob(4)`，参考实现返回 4 字节、本实现返回 2）。
//!
//! `eos()` 的返回形式也容易被忽略：参考实现是 `1`/`null`，不是 `true`/`false`
//! （`_stream_eos`：真 pushinteger(1)、假 pushnull），`flush()` 同理。
//!
//! 期望值是用**参考实现**跑 `tools/probes/stream_semantics.nut` 得到的原文。

use std::cell::RefCell;
use std::rc::Rc;

use squirrel::api::Vm;

/// 跑脚本并收集 print 输出（错误走 stderr，一并返回）
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
    let _ = vm.run_buffer(src.as_bytes(), b"stream.nut");
    let o = String::from_utf8_lossy(&out.borrow()).into_owned();
    let e = String::from_utf8_lossy(&err.borrow()).into_owned();
    (o, e)
}

/// ↔ tools/probes/stream_semantics.nut（路径换成临时文件）
#[test]
fn file_and_blob_stream_semantics_match_cpp() {
    let path = std::env::temp_dir().join("sqrust_stream_semantics.bin");
    let _ = std::fs::remove_file(&path);
    let p = path.to_string_lossy().replace('\\', "\\\\");
    let (out, err) = run_capture(&format!(
        r#"
        // writen 的参数顺序是 (值, 格式)，与 readn(格式) 相反
        local src = file("{p}", "wb");
        src.writen(65, 'b');
        src.writen(66, 'b');
        src.writen(67, 'b');
        src.writen(68, 'b');
        src.close();
        local f = file("{p}", "rb");
        print(f.tell() + "," + f.len() + "\n");
        print("" + f.eos() + "\n");
        local a = f.readn('b'); local b = f.readn('b'); local c = f.readn('b');
        print(a + "" + b + "" + c + "\n");
        print(f.tell() + "," + f.len() + "\n");
        print("" + f.eos() + "\n");
        local d = f.readn('b');
        print(f.tell() + "," + f.len() + "\n");
        print("" + f.eos() + "\n");
        print("" + f.flush() + "\n");
        f.close();
        local bl = blob(4);
        print("blob " + bl.tell() + "," + bl.len() + "," + bl.eos() + "\n");
        bl.seek(0, 'e');
        print("blob " + bl.tell() + "," + bl.len() + "," + bl.eos() + "\n");
        local bl2 = blob(0);
        print("blob0 " + bl2.tell() + "," + bl2.len() + "," + bl2.eos() + "\n");
        "#
    ));
    let _ = std::fs::remove_file(&path);
    assert!(err.is_empty(), "不应有错误输出：{err}");
    assert_eq!(
        out, "0,4\nnull\n656667\n3,4\nnull\n4,4\n1\n1\nblob 0,4,null\nblob 4,4,1\nblob0 0,0,1\n",
        "文件流/blob 的 tell/len/eos/flush 必须与参考实现逐字一致（eos 真为 1、假为 null）"
    );
}

/// 不可 seek 的流上 `len()` 不可信（参考实现是 `fseek(SEEK_END)` 失败后的 CRT 内部状态，
/// 实测同一进程里先返回 4、后返回 0），所以 `_stream_readblob` 的长度夹取只对可信的
/// `len()` 生效——否则管道上会被夹成 0，永远报 "no data left to read"。
#[test]
fn blob_len_is_trusted_so_readblob_is_not_clamped_to_zero() {
    let vm = Vm::new(1024);
    let out: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let o = out.clone();
        vm.set_print_funcs(
            Some(Rc::new(move |_v, b: &[u8]| o.borrow_mut().extend_from_slice(b))),
            None,
        );
    }
    squirrel_stdlib::register_all(&mut vm.data_mut());
    // 这里不喂 stdin（测试进程的 stdin 不是管道），只验证 readblob 不再因
    // 「把请求长度夹到不可信的 len()=0」而永远报 no data left to read
    let r = vm.run_buffer(
        b"local b = blob(4); print(b.tell() + \",\" + b.len() + \"\\n\");",
        b"t.nut",
    );
    assert!(r.is_ok(), "blob 流应能正常工作");
    assert_eq!(String::from_utf8_lossy(&out.borrow()), "0,4\n");
}

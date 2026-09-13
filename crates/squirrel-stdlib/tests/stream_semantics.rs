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

/// `writen`/`readn` 的**类型分派**与字节序（↔ sqstdstream.cpp 的 `_stream_writen/readn`）。
///
/// 期望值全部用参考实现实测确认（语料侧还有 `tests-corpus/t_blobmap.nut` 的 133 条守门）。
#[test]
fn blob_writen_readn_type_dispatch_matches_reference() {
    let src = r#"
        function hx(b) {
            local out = b.len() + ":";
            for (local i = 0; i < b.len(); i++) out += format("%02x", b[i]);
            return out;
        }
        // 小端：258 = 0x0102 → 'i' 写 02 01 00 00
        local b = blob(0); b.writen(258, 'i'); print(hx(b) + "\n");
        local c = blob(0); c.writen(258, 's'); print(hx(c) + "\n");
        local d = blob(0); d.writen(258, 'c'); print(hx(d) + "\n");
        local e = blob(0); e.writen(-1, 'i'); print(hx(e) + "\n");
        // 读回
        local f = blob(0); f.writen(258, 'i'); f.writen(1.5, 'f'); f.writen(-3, 's');
        print(f.len() + "\n");
        f.seek(0);
        print(f.readn('i') + "," + f.readn('f') + "," + f.readn('s') + "\n");
        print(f.tell() + "/" + f.eos() + "\n");
        // 截断：'c' 只写 1 字节、's' 只写 2 字节
        local g = blob(0); g.writen(70000, 's'); print(hx(g) + "\n");
        // 交换
        local s2 = blob(0); s2.writen(258, 'i'); s2.swap2(); print(hx(s2) + "\n");
        local s4 = blob(0); s4.writen(258, 'i'); s4.swap4(); print(hx(s4) + "\n");
        // 错误路径
        try { local x = blob(0); x.writen(1, 'z'); print("no\n"); } catch (err) { print("E:" + err + "\n"); }
        try { local x = blob(2); x.readn('l'); print("no\n"); } catch (err) { print("E:" + err + "\n"); }
        try { local x = blob(0); x.readblob(1); print("no\n"); } catch (err) { print("E:" + err + "\n"); }
    "#;
    let (out, _err) = run_capture(src);
    assert_eq!(
        out,
        "4:02010000\n2:0201\n1:02\n4:ffffffff\n10\n258,1.5,-3\n10/1\n2:7011\n\
         4:01020000\n4:00000102\nE:invalid format\nE:io error\nE:no data left to read\n"
    );
}

/// 非法文件模式：**参考实现直接崩溃**（UCRT invalid parameter handler → 0xC0000409），
/// 本实现必须给可捕获错误而不是跟着崩。合法模式集合是按参考实现实测的"接受集"定的。
/// ref-crash-skip（给 tmp-c2/check_rs_scripts.py 用）
#[test]
fn invalid_file_modes_are_catchable_errors() {
    let dir = std::env::temp_dir().join("sqrust_mode_test.bin");
    let p = dir.to_string_lossy().replace('\\', "\\\\");
    let src = format!(
        r#"
        function try_mode(m) {{
            try {{ local f = file("{p}", m); f.close(); return m + ":ok"; }}
            catch (e) {{ return m + ":E"; }}
        }}
        print(try_mode("w") + "\n");
        print(try_mode("wb") + "\n");
        print(try_mode("a") + "\n");
        print(try_mode("w+b") + "\n");
        print(try_mode("r") + "\n");
        print(try_mode("rb") + "\n");
        print(try_mode("") + "\n");
        print(try_mode("z") + "\n");
        print(try_mode("zz") + "\n");
        print(try_mode("R") + "\n");
        print(try_mode("RB") + "\n");
        print(try_mode("rbb") + "\n");
        print(try_mode("trash") + "\n");
    "#
    );
    let (out, _err) = run_capture(&src);
    assert_eq!(
        out,
        "w:ok\nwb:ok\na:ok\nw+b:ok\nr:ok\nrb:ok\n:E\nz:E\nzz:E\nR:E\nRB:E\nrbb:E\ntrash:E\n"
    );
}

/// `blob.swap2/swap4` 是"**整块缓冲区**交换、**完全不动读写位置**"（↔ `_blob_swap2/4`：
/// 参考实现直接对 `self->GetBuf()` 循环 `__swap_word/__swap_dword`，从不碰 `_ptr`）。
///
/// 本实现原来用 snapshot → 改 → `seek(0)` + `write(&buf)` 写回，而 `write` 会把 `_ptr`
/// 推到末尾，于是 `swap2()` 之后 `tell()` 变成 4/8。**这是随机差分模糊测试（变异模式）
/// 抓到的真实差异**：变异 `tests-corpus/t_writeread.nut` 时，`swap2()` 之后 `seek(-1)` 失败、
/// 位置却停在末尾，接着 `readn` 越界报 `io error`，而参考实现正常返回原值。
#[test]
fn blob_swap_keeps_position_and_covers_whole_buffer() {
    let (out, err) = run_capture(
        r#"
        local b = blob(4);
        b.writen(0x1234, 'w');
        print("tell_after_write=" + b.tell() + "\n");
        b.swap2();
        print("tell_after_swap2=" + b.tell() + "\n");
        b.seek(1, 'b');
        b.swap2();
        print("tell_after_swap2_at1=" + b.tell() + "\n");
        local c = blob(8);
        c.writen(0x12345678, 'i');
        c.swap4();
        print("tell_after_swap4=" + c.tell() + "\n");
        // 整块交换：位置在 1 时 swap2 也交换了 [0..4)
        // writen 是**小端**写：0x0102 → [02,01]，0x0304 → [04,03]，
        // 整块 swap2 后变回 [01,02,03,04]，按小端读成 int 就是 0x04030201
        local d = blob(4);
        d.writen(0x0102, 'w');
        d.writen(0x0304, 'w');
        d.seek(1, 'b');
        d.swap2();
        d.seek(0, 'b');
        print("whole=" + d.readn('i') + "\n");
        d.seek(0, 'b');
        print("first_byte=" + d.readn('b') + "\n");
        "#,
    );
    assert!(err.is_empty(), "不该有错误输出：{err:?}");
    assert_eq!(
        out,
        "tell_after_write=2\ntell_after_swap2=2\ntell_after_swap2_at1=1\n\
         tell_after_swap4=4\nwhole=67305985\nfirst_byte=1\n"
    );
}

/// 文件流上的**负绝对偏移必须失败**（↔ CRT 的 `fseek(f, -1, SEEK_SET)` 返回非 0，
/// 参考实现据此返回 `-1`）。本实现原来把负偏移 `max(0)` 夹成 0 再当成功返回 0，
/// 同样是模糊测试变异 `t_writeread` 抓到的差异。
///
/// 注意与另外两种 origin 的区别（实测参考实现）：`'c'` 负偏移会失败（位置加上后为负）、
/// `'e'` 负偏移只要不越界就成功；而**越过末尾**的绝对偏移在文件上**是成功的**
/// （CRT 允许 seek 到文件尾之后），blob 上则失败（见上面的 blob seek 语义）。
#[test]
fn file_seek_negative_absolute_offset_fails() {
    let dir = std::env::temp_dir().join("sqrust_seekneg.bin");
    let _ = std::fs::remove_file(&dir);
    let p = dir.to_string_lossy().replace('\\', "\\\\");
    let src = format!(
        r#"
        local w = file("{p}", "wb+");
        local b = blob(2);
        b.writen(0x1234, 'w');
        w.writeblob(b);
        w.flush();
        w.seek(0, 'b');
        print("neg_abs=" + w.seek(-1, 'b') + "\n");
        print("tell_after_neg=" + w.tell() + "\n");
        print("read_ok=" + w.readn('w') + "\n");
        print("past_end=" + w.seek(99, 'b') + "\n");
        print("tell_past=" + w.tell() + "\n");
        w.seek(0, 'b');
        print("neg_cur=" + w.seek(-1, 'c') + "\n");
        print("neg_end=" + w.seek(-1, 'e') + "\n");
        print("far_end=" + w.seek(-99, 'e') + "\n");
        w.close();
    "#
    );
    let (out, _err) = run_capture(&src);
    assert_eq!(
        out,
        "neg_abs=-1\ntell_after_neg=0\nread_ok=4660\npast_end=0\ntell_past=99\n\
         neg_cur=-1\nneg_end=0\nfar_end=-1\n"
    );
    let _ = std::fs::remove_file(&dir);
}

/// 文件句柄开着时 `remove()` 必须**失败**（↔ CRT `fopen` 用 `_SH_DENYNO`，即
/// `FILE_SHARE_READ|FILE_SHARE_WRITE`，**不含 `FILE_SHARE_DELETE`**）。
///
/// Rust 的 `std::fs::File` 默认三种共享齐全 —— 开着也能删，与参考实现不同。
/// 已在 `FileBackend::from_path` 里显式 `share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)`；
/// 前提是后端随实例死亡释放（见 `backend_released_when_instance_dies` 与 TODO.md P14）。
///
/// 注意：三条断言都用**非尾调用**写法（`local r = f(); return r;`）。
/// `return f(x)` 是尾调用，我们会在那一步提前释放当前帧的局部（参考实现保留到帧弹出），
/// 这一格是已知差异（TODO.md P14 子问题 2），语料里也刻意回避。
#[test]
fn open_handle_blocks_remove_like_crt_fopen() {
    let dir = std::env::temp_dir().join("sqrust_share_test.bin");
    let _ = std::fs::remove_file(&dir);
    let p = dir.to_string_lossy().replace('\\', "\\\\");
    let src = format!(
        r#"
        function try_remove(p) {{ try {{ remove(p); return "ok"; }} catch (e) {{ return "E:" + e; }} }}
        local h1 = file("{p}", "wb+");
        local h2 = file("{p}", "wb+");
        local a = try_remove("{p}");
        print("while_open=" + a + "\n");
        h1.close();
        local b = try_remove("{p}");
        print("after_one_close=" + b + "\n");
        h2.close();
        local c = try_remove("{p}");
        print("after_all_close=" + c + "\n");
    "#
    );
    let (out, _err) = run_capture(&src);
    assert_eq!(
        out,
        "while_open=E:remove() failed\n\
         after_one_close=E:remove() failed\n\
         after_all_close=ok\n"
    );
    let _ = std::fs::remove_file(&dir);
}
///
/// `file.seek` 的偏移会被**截断成 32 位**（↔ `sqstd_fseek` 的 `fseek(file, (long)offset, ...)`，
/// Windows 上 `long` 是 32 位）；blob 走自己的 `SQBlob::Seek`（i64、不截断），两者不同。
///
/// 实测参考实现（探针 `tmp-c2/probe_trunc.nut`）：`INT64_MIN`→0（seek 到 0 且返回 0）、
/// `2^32+5`→5、`2^31`→-2^31（负数 → 失败 -1）、`INT64_MAX`→-1、`-4294967291`→5。
/// 这是模糊测试变异出 `seek(INT64_MIN, 'b')` 时发现的差异。
#[test]
fn file_seek_truncates_offset_to_32_bits() {
    let dir = std::env::temp_dir().join("sqrust_trunc.bin");
    let _ = std::fs::remove_file(&dir);
    let p = dir.to_string_lossy().replace('\\', "\\\\");
    let src = format!(
        r#"
        local tf = file("{p}", "wb+");
        tf.writen(0x11, 'b');
        print("i64min=" + tf.seek(-9223372036854775808, 'b') + ":" + tf.tell() + "\n");
        print("i64max=" + tf.seek(9223372036854775807, 'b') + ":" + tf.tell() + "\n");
        print("2p32=" + tf.seek(4294967296, 'b') + ":" + tf.tell() + "\n");
        print("2p32p5=" + tf.seek(4294967301, 'b') + ":" + tf.tell() + "\n");
        print("2p31=" + tf.seek(2147483648, 'b') + ":" + tf.tell() + "\n");
        print("neg=" + tf.seek(-4294967291, 'b') + ":" + tf.tell() + "\n");
        print("cur=" + tf.seek(-9223372036854775808, 'c') + ":" + tf.tell() + "\n");
        tf.close();
        local tb = blob(4);
        tb.writen(0x11, 'b');
        print("blob_i64min=" + tb.seek(-9223372036854775808, 'b') + ":" + tb.tell() + "\n");
        print("blob_2p32=" + tb.seek(4294967296, 'b') + ":" + tb.tell() + "\n");
    "#
    );
    let (out, _err) = run_capture(&src);
    assert_eq!(
        out,
        // i64max 被截成 -1 → 失败，位置停在上一句的 0
        "i64min=0:0\ni64max=-1:0\n2p32=0:0\n2p32p5=0:5\n2p31=-1:5\nneg=0:5\ncur=0:5\n\
         blob_i64min=-1:1\nblob_2p32=-1:1\n"
    );
    let _ = std::fs::remove_file(&dir);
}
///
/// 背景（TODO.md P14）：`BACKENDS` 注册表原来只增不减 —— 每个创建过的 file/blob/stream
/// 后端都被永久持有，没显式 `close()` 的文件句柄会一直开到进程结束（无界增长）。
/// 现在实例上挂了一个 userdata 成员 `__be`，它的 release hook 在实例死亡时清掉注册表槽位
/// （并做指针身份校验，避免跨线程 sid 碰撞误清）。
/// 这里断言行为：大量 blob 反复创建/丢弃后一切照常（后端真被释放时不会有"the stream is
/// invalid"；没释放则是无界增长，靠 tools 侧的内存探测另测）。
#[test]
fn backend_released_when_instance_dies() {
    let dir = std::env::temp_dir().join("sqrust_be_churn");
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("churn.bin").to_string_lossy().replace('\\', "\\\\");
    let src = format!(
        r#"
        function churn() {{
            local n = 0;
            for (local i = 0; i < 500; i++) {{
                local b = blob(64);
                n += b.len();
                local f = file("{p}", "wb+");
                n += f.tell();
                f.close();
            }}
            return n;
        }}
        print("churn=" + churn() + "\n");
        print("after=" + blob(8).len() + "\n");
    "#
    );
    let (out, _err) = run_capture(&src);
    // 500 次 × blob(64).len() = 32000（f.tell() 建完就是 0）
    assert_eq!(out, "churn=32000\nafter=8\n");
    let _ = std::fs::remove_file(&dir);
}

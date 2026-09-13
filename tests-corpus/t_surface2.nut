// 广度语料 2：blob / stream / file / 序列化
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

// ---- blob ----
t("blob_new", function() { return blob(4).len(); });
t("blob_zero", function() { return blob(0).len(); });
t("blob_resize_up", function() { local b = blob(2); b.resize(5); return b.len(); });
t("blob_resize_down", function() { local b = blob(5); b.resize(1); return b.len(); });
t("blob_index", function() { local b = blob(3); b[0] = 255; b[1] = 0; b[2] = 128; return "" + b[0] + "," + b[1] + "," + b[2]; });
t("blob_index_neg", function() { local b = blob(3); b[0] = 7; return b[-1]; });
t("blob_index_oob", function() { local b = blob(1); return b[5]; });
t("blob_tostring", function() { local b = blob(3); b[0] = 65; b[1] = 66; b[2] = 67; return b.tostring(); });
t("blob_eos", function() { local b = blob(2); return b.eos(); });
t("blob_tell_seek", function() { local b = blob(4); b.seek(2); return b.tell(); });
t("blob_seek_end", function() { local b = blob(4); b.seek(-1, 'e'); return b.tell(); });
t("blob_len_stream", function() { return blob(7).len(); });
t("blob_writen", function() { local b = blob(0); b.writen(0x1234, 'w'); return b.len(); });
t("blob_readn", function() { local b = blob(0); b.writen(0x1234, 'w'); b.seek(0); return b.readn('w'); });
t("blob_swap2_odd", function() { local b = blob(1); b[0] = 1; b.swap2(); return b.len(); });
t("blob_swap4", function() { local b = blob(4); b[0] = 1; b[1] = 2; b[2] = 3; b[3] = 4; b.swap4(); return "" + b[0] + b[1] + b[2] + b[3]; });
t("blob_foreach", function() { local b = blob(3); b[0] = 1; b[1] = 2; b[2] = 3; local s = 0; foreach (v in b) s += v; return s; });
t("blob_clone", function() { local b = blob(2); b[0] = 9; local c = clone b; c[0] = 1; return "" + b[0] + c[0]; });
t("blob_writeblob", function() { local a = blob(2); local b = blob(2); b.writeblob(a); return b.len(); });
t("blob_readblob", function() { local a = blob(2); local b = blob(0); b.writeblob(a); b.seek(0); return b.readblob(2).len(); });
t("blob_readblob_too_much", function() { local s = stream(); return s.readblob(999).len(); });
t("blob_typeof", function() { return typeof blob(1); });

// ---- stream 基类 ----
t("stream_new", function() { return typeof stream(); });
t("stream_len", function() { return stream().len(); });
t("stream_writen_readn", function() {
    local s = stream();
    s.writen(65, 'b'); s.writen(0x0102, 'w'); s.writen(1.5, 'f'); s.writen(7, 'i');
    s.seek(0, 'b');
    return "" + s.readn('b') + "," + s.readn('w') + "," + s.readn('f') + "," + s.readn('i');
});
t("stream_flush", function() { local s = stream(); s.flush(); return "ok"; });
t("stream_eos_empty", function() { return stream().eos(); });
t("stream_seek_bad_whence", function() { local s = stream(); s.seek(0, 'z'); return s.tell(); });
t("stream_readn_bad_type", function() { local s = stream(); s.writen(1, 'b'); s.seek(0); return s.readn('z'); });

// ---- file ----
t("file_write_read", function() {
    local f = file("robust_surface.txt", "wb+");
    f.writen(0x41424344, 'i');
    f.seek(0, 'b');
    local v = f.readn('i');
    f.close();
    remove("robust_surface.txt");
    return v;
});
t("file_line", function() {
    local f = file("robust_surface2.txt", "wb+");
    f.writen(10, 'b'); f.writen(20, 'b');
    f.seek(0, 'b');
    local n = f.readn('b') + f.readn('b');
    f.close();
    remove("robust_surface2.txt");
    return n;
});
// 注意：这条会真的创建 robust_surface3.txt，所以必须自己删掉 —— 语料在仓库根目录跑，
// 漏删就会把垃圾留在工作区（曾经把 robust_surface3.txt 提交进仓库一次）。
// remove 对不存在的文件会抛错，所以用 try 包住，保证这条用例的输出与原来一致。
t("file_typeof", function() {
    local r = typeof file("robust_surface3.txt", "wb+").close();
    try { remove("robust_surface3.txt"); } catch (e) {}
    return r;
});
t("file_stdout_type", function() { return typeof stdout + typeof stderr + typeof stdin; });
t("file_len", function() {
    local f = file("robust_surface4.txt", "wb+");
    f.writen(1, 'b'); f.writen(2, 'b');
    local n = f.len();
    f.close(); remove("robust_surface4.txt");
    return n;
});
t("file_append", function() {
    local f = file("robust_surface5.txt", "wb"); f.writen(1, 'b'); f.close();
    local g = file("robust_surface5.txt", "ab"); g.writen(2, 'b'); g.close();
    local h = file("robust_surface5.txt", "rb"); local n = h.len(); h.close();
    remove("robust_surface5.txt");
    return n;
});
t("file_readmissing", function() { return file("no_such_file_zz.txt", "rb"); });
t("stream_methods_missing", function() { local s = stream(); return s.readstr(); });

// ---- 序列化 ----
t("closure_roundtrip", function() {
    local f = function(a, b) { return a * b; };
    writeclosuretofile("robust_surface.cnut", f);
    local g = loadfile("robust_surface.cnut");
    remove("robust_surface.cnut");
    return g(6, 7);
});
t("closure_roundtrip_closure_env", function() {
    // 注意：base 是关键字，不能作标识符（两边都会报 "expected '.'"）
    local captured = 100;
    local f = function(a) { return captured + a; };
    writeclosuretofile("robust_surface2.cnut", f);
    local g = loadfile("robust_surface2.cnut");
    remove("robust_surface2.cnut");
    return g(1);
});
t("dofile_env", function() {
    local holder = { marker = "H" };
    local f = file("robust_surface3.nut", "wb");
    f.writen(114, 'b'); // 'r'
    f.close();
    // 用自定义 this 调 dofile：环境表应作为 this
    local env = { marker = "ENV" };
    env.setdelegate(getroottable());
    local src = file("robust_surface4.nut", "wb");
    foreach (c in "return this.marker;") src.writen(c, 'b');
    src.close();
    local r = env.dofile("robust_surface4.nut");
    remove("robust_surface3.nut"); remove("robust_surface4.nut");
    return r;
});
t("loadfile_missing", function() { return loadfile("no_such_file_zz.nut"); });
t("compilestring_ok", function() { return compilestring("return 1+1", "cs")(); });
t("compilestring_bad", function() { return compilestring("return 1+", "cs"); });
t("compilestring_env", function() { local f = compilestring("return this", "cs"); local env = {}; env.setdelegate(getroottable()); return typeof f.bindenv(env)(); });

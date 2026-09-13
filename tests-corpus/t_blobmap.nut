// 自动生成：python tools/gen_blobmap.py —— 请勿手改
// blob / stream 二进制语义矩阵

// blob / stream 二进制语义探测
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
function hexdump(b) {
    local out = b.len() + ":";
    for (local i = 0; i < b.len(); i++) { out += (b[i] < 16 ? "0" : "") + format("%x", b[i]); }
    return out;
}

// 1) 构造与基本属性
s("ctor_int", function() { local b = blob(4); return b.len() + "/" + typeof b; });
s("ctor_zero", function() { local b = blob(0); return b.len(); });
s("ctor_none", function() { local b = blob(); return b.len(); });
s("ctor_str", function() { local b = blob("abcd"); return hexdump(b) + "/" + b.len(); });
s("ctor_neg", function() { try { local b = blob(-1); return "" + b.len(); } catch (e) { return "E:" + e; } });
s("is_stream", function() { return blob(1) instanceof stream; });
s("typeof_blob", function() { return typeof blob(1); });

// 2) _get / _set / resize / clone / foreach
s("get_set", function() { local b = blob(3); b[0] = 1; b[1] = 255; b[2] = 128; return hexdump(b); });
s("set_oob", function() { local b = blob(1); try { b[5] = 1; return "ok"; } catch (e) { return "E:" + e; } });
s("get_oob", function() { local b = blob(1); try { return "" + b[5]; } catch (e) { return "E:" + e; } });
s("resize_up", function() { local b = blob(2); b[0] = 1; b.resize(4); return hexdump(b); });
s("resize_down", function() { local b = blob(4); b.resize(2); return hexdump(b); });
s("clone", function() { local b = blob(3); b[0] = 9; local c = clone b; c[0] = 1; return hexdump(b) + "/" + hexdump(c); });
s("foreach", function() { local b = blob(3); b[0] = 1; b[1] = 2; b[2] = 3; local out = ""; foreach (v in b) out += v; return out; });
s("swap2", function() { local b = blob(4); b[0] = 0x11; b[1] = 0x22; b[2] = 0x33; b[3] = 0x44; b.swap2(); return hexdump(b); });
s("swap4", function() { local b = blob(4); b[0] = 0x11; b[1] = 0x22; b[2] = 0x33; b[3] = 0x44; b.swap4(); return hexdump(b); });
s("swap2_odd", function() { local b = blob(3); b.swap2(); return hexdump(b); });
s("swap4_odd", function() { local b = blob(5); b.swap4(); return hexdump(b); });

// 3) writen / readn 类型分派（小端，写入后读回）
foreach (fmt in ["l", "i", "s", "w", "c", "b"]) {
    s("rt_" + fmt, function() {
        local b = blob(0);
        b.writen(258, fmt);
        local n = b.len();
        b.seek(0);
        return n + ":" + b.readn(fmt);
    });
}
s("rt_f", function() { local b = blob(0); b.writen(1.5, 'f'); local n = b.len(); b.seek(0); return n + ":" + b.readn('f'); });
s("rt_d", function() { local b = blob(0); b.writen(1.5, 'd'); local n = b.len(); b.seek(0); return n + ":" + b.readn('d'); });
s("rt_neg", function() { local b = blob(0); b.writen(-1, 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i'); });
s("rt_big", function() { local b = blob(0); b.writen(9223372036854775807, 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l'); });
s("rt_trunc_i", function() { local b = blob(0); b.writen(258, 'c'); return hexdump(b); });
s("rt_trunc_s", function() { local b = blob(0); b.writen(70000, 's'); return hexdump(b); });
s("writen_badfmt", function() { local b = blob(0); try { b.writen(1, 'z'); return "ok"; } catch (e) { return "E:" + e; } });
s("writen_ret", function() { local b = blob(0); return typeof b.writen(1, 'i'); });

// 4) 游标与流状态
s("tell_init", function() { return blob(4).tell(); });
s("eos_init", function() { return blob(0).eos() + "/" + blob(2).eos(); });
s("seek_and_tell", function() { local b = blob(4); b.seek(2); return b.tell(); });
s("seek_end", function() { local b = blob(4); b.seek(0, 2); return b.tell(); });
s("seek_cur", function() { local b = blob(4); b.seek(1); b.seek(1, 1); return b.tell(); });
s("seek_beyond", function() { local b = blob(2); b.seek(5); return b.tell(); });
s("seek_neg", function() { local b = blob(2); try { b.seek(-1); return "" + b.tell(); } catch (e) { return "E:" + e; } });
s("len_blob", function() { return blob(7).len(); });
s("flush_blob", function() { return "" + blob(1).flush(); });

// 5) readblob / writeblob
s("writeblob", function() { local b = blob(0); local src = blob(3); src[0] = 1; return b.writeblob(src) + "/" + hexdump(b); });
s("readblob", function() { local b = blob("abcdef"); b.seek(1); local r = b.readblob(3); return hexdump(r) + "/" + b.tell(); });
s("readblob_all", function() { local b = blob("abcdef"); local r = b.readblob(99); return hexdump(r); });
s("readblob_zero", function() { local b = blob("ab"); try { return hexdump(b.readblob(0)); } catch (e) { return "E:" + e; } });
s("readblob_eof", function() { local b = blob(0); try { return hexdump(b.readblob(1)); } catch (e) { return "E:" + e; } });
s("readn_eof", function() { local b = blob(2); try { return "" + b.readn('l'); } catch (e) { return "E:" + e; } });
s("writeblob_bad", function() { local b = blob(0); try { return "" + b.writeblob(5); } catch (e) { return "E:" + e; } });

// 6) 往返：写进去再读出来是否字节一致
s("roundtrip_mixed", function() {
    local b = blob(0);
    b.writen(1, 'i'); b.writen(2.5, 'f'); b.writen(3, 's');
    local dump = hexdump(b);
    b.seek(0);
    local v1 = b.readn('i'), v2 = b.readn('f'), v3 = b.readn('s');
    return dump + "|" + v1 + "," + v2 + "," + v3 + "|" + b.tell() + "/" + b.len();
});

// 7) 文件流：同样的 writen/readn 走真实文件
s("file_rt", function() {
    local path = "tmp-c2/blobprobe_tmp.bin";
    local f = file(path, "wb");
    f.writen(258, 'i'); f.writen(1.5, 'd'); f.writen(7, 'b');
    f.close();
    local g = file(path, "rb");
    local out = g.readn('i') + "," + g.readn('d') + "," + g.readn('b');
    out += "|" + g.tell() + "/" + g.len() + "/" + g.eos();
    g.close();
    return out;
});
s("file_seek", function() {
    local path = "tmp-c2/blobprobe_tmp2.bin";
    local f = file(path, "wb"); f.writeblob(blob("0123456789")); f.close();
    local g = file(path, "rb");
    g.seek(3); local a = g.readn('c');
    g.seek(-2, 2); local b2 = g.readn('c');
    g.seek(1, 1); local c = g.readn('c');
    g.close();
    return a + "," + b2 + "," + c;
});
s("file_modes", function() {
    local out = "";
    foreach (m in ["rb", "wb", "ab", "r", "w"]) {
        try { local f = file("tmp-c2/blobprobe_tmp3.bin", m); out += m + ":ok;"; f.close(); }
        catch (e) { out += m + ":E:" + e + ";"; }
    }
    return out;
});
// 注：非法模式（"", "z", "R", "rbb", "trash"…）会让**参考实现崩溃**
//（UCRT invalid parameter handler → 0xC0000409），所以不进这份差分探针；
// 本实现的确定性行为由 crates/squirrel-stdlib/tests/stream_semantics.rs 守门。
s("file_close_twice", function() { local f = file("tmp-c2/blobprobe_tmp3.bin", "wb"); f.close(); try { f.close(); return "ok"; } catch (e) { return "E:" + e; } });
s("file_write_after_close", function() { local f = file("tmp-c2/blobprobe_tmp3.bin", "wb"); f.close(); try { f.writen(1, 'i'); return "ok"; } catch (e) { return "E:" + e; } });
s("file_open_missing", function() { try { local f = file("tmp-c2/definitely_missing_file.bin", "rb"); return "ok"; } catch (e) { return "E:" + e; } });
function hexdump(b) {
    local out = b.len() + ":";
    for (local i = 0; i < b.len(); i++) { out += (b[i] < 16 ? "0" : "") + format("%x", b[i]); }
    return out;
}

// ---- 扩充：writen/readn 格式 × 值 往返（写入长度 + 读回值 + 游标/长度）
s("rt_l_z", function() { local b = blob(0); b.writen(0, 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l') + ":" + b.tell() + "/" + b.eos(); });
s("rt_l_one", function() { local b = blob(0); b.writen(1, 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l') + ":" + b.tell() + "/" + b.eos(); });
s("rt_l_mone", function() { local b = blob(0); b.writen(-1, 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l') + ":" + b.tell() + "/" + b.eos(); });
s("rt_l_258", function() { local b = blob(0); b.writen(258, 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l') + ":" + b.tell() + "/" + b.eos(); });
s("rt_l_neg258", function() { local b = blob(0); b.writen(-258, 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l') + ":" + b.tell() + "/" + b.eos(); });
s("rt_l_big16", function() { local b = blob(0); b.writen(70000, 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l') + ":" + b.tell() + "/" + b.eos(); });
s("rt_l_b31", function() { local b = blob(0); b.writen(2147483647, 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l') + ":" + b.tell() + "/" + b.eos(); });
s("rt_l_nb31", function() { local b = blob(0); b.writen(-2147483648, 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l') + ":" + b.tell() + "/" + b.eos(); });
s("rt_l_max", function() { local b = blob(0); b.writen(9223372036854775807, 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l') + ":" + b.tell() + "/" + b.eos(); });
s("rt_l_min", function() { local b = blob(0); b.writen((-9223372036854775807 - 1), 'l'); local n = b.len(); b.seek(0); return n + ":" + b.readn('l') + ":" + b.tell() + "/" + b.eos(); });
s("rt_i_z", function() { local b = blob(0); b.writen(0, 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i') + ":" + b.tell() + "/" + b.eos(); });
s("rt_i_one", function() { local b = blob(0); b.writen(1, 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i') + ":" + b.tell() + "/" + b.eos(); });
s("rt_i_mone", function() { local b = blob(0); b.writen(-1, 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i') + ":" + b.tell() + "/" + b.eos(); });
s("rt_i_258", function() { local b = blob(0); b.writen(258, 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i') + ":" + b.tell() + "/" + b.eos(); });
s("rt_i_neg258", function() { local b = blob(0); b.writen(-258, 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i') + ":" + b.tell() + "/" + b.eos(); });
s("rt_i_big16", function() { local b = blob(0); b.writen(70000, 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i') + ":" + b.tell() + "/" + b.eos(); });
s("rt_i_b31", function() { local b = blob(0); b.writen(2147483647, 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i') + ":" + b.tell() + "/" + b.eos(); });
s("rt_i_nb31", function() { local b = blob(0); b.writen(-2147483648, 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i') + ":" + b.tell() + "/" + b.eos(); });
s("rt_i_max", function() { local b = blob(0); b.writen(9223372036854775807, 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i') + ":" + b.tell() + "/" + b.eos(); });
s("rt_i_min", function() { local b = blob(0); b.writen((-9223372036854775807 - 1), 'i'); local n = b.len(); b.seek(0); return n + ":" + b.readn('i') + ":" + b.tell() + "/" + b.eos(); });
s("rt_s_z", function() { local b = blob(0); b.writen(0, 's'); local n = b.len(); b.seek(0); return n + ":" + b.readn('s') + ":" + b.tell() + "/" + b.eos(); });
s("rt_s_one", function() { local b = blob(0); b.writen(1, 's'); local n = b.len(); b.seek(0); return n + ":" + b.readn('s') + ":" + b.tell() + "/" + b.eos(); });
s("rt_s_mone", function() { local b = blob(0); b.writen(-1, 's'); local n = b.len(); b.seek(0); return n + ":" + b.readn('s') + ":" + b.tell() + "/" + b.eos(); });
s("rt_s_258", function() { local b = blob(0); b.writen(258, 's'); local n = b.len(); b.seek(0); return n + ":" + b.readn('s') + ":" + b.tell() + "/" + b.eos(); });
s("rt_s_neg258", function() { local b = blob(0); b.writen(-258, 's'); local n = b.len(); b.seek(0); return n + ":" + b.readn('s') + ":" + b.tell() + "/" + b.eos(); });
s("rt_s_big16", function() { local b = blob(0); b.writen(70000, 's'); local n = b.len(); b.seek(0); return n + ":" + b.readn('s') + ":" + b.tell() + "/" + b.eos(); });
s("rt_s_b31", function() { local b = blob(0); b.writen(2147483647, 's'); local n = b.len(); b.seek(0); return n + ":" + b.readn('s') + ":" + b.tell() + "/" + b.eos(); });
s("rt_s_nb31", function() { local b = blob(0); b.writen(-2147483648, 's'); local n = b.len(); b.seek(0); return n + ":" + b.readn('s') + ":" + b.tell() + "/" + b.eos(); });
s("rt_s_max", function() { local b = blob(0); b.writen(9223372036854775807, 's'); local n = b.len(); b.seek(0); return n + ":" + b.readn('s') + ":" + b.tell() + "/" + b.eos(); });
s("rt_s_min", function() { local b = blob(0); b.writen((-9223372036854775807 - 1), 's'); local n = b.len(); b.seek(0); return n + ":" + b.readn('s') + ":" + b.tell() + "/" + b.eos(); });
s("rt_w_z", function() { local b = blob(0); b.writen(0, 'w'); local n = b.len(); b.seek(0); return n + ":" + b.readn('w') + ":" + b.tell() + "/" + b.eos(); });
s("rt_w_one", function() { local b = blob(0); b.writen(1, 'w'); local n = b.len(); b.seek(0); return n + ":" + b.readn('w') + ":" + b.tell() + "/" + b.eos(); });
s("rt_w_mone", function() { local b = blob(0); b.writen(-1, 'w'); local n = b.len(); b.seek(0); return n + ":" + b.readn('w') + ":" + b.tell() + "/" + b.eos(); });
s("rt_w_258", function() { local b = blob(0); b.writen(258, 'w'); local n = b.len(); b.seek(0); return n + ":" + b.readn('w') + ":" + b.tell() + "/" + b.eos(); });
s("rt_w_neg258", function() { local b = blob(0); b.writen(-258, 'w'); local n = b.len(); b.seek(0); return n + ":" + b.readn('w') + ":" + b.tell() + "/" + b.eos(); });
s("rt_w_big16", function() { local b = blob(0); b.writen(70000, 'w'); local n = b.len(); b.seek(0); return n + ":" + b.readn('w') + ":" + b.tell() + "/" + b.eos(); });
s("rt_w_b31", function() { local b = blob(0); b.writen(2147483647, 'w'); local n = b.len(); b.seek(0); return n + ":" + b.readn('w') + ":" + b.tell() + "/" + b.eos(); });
s("rt_w_nb31", function() { local b = blob(0); b.writen(-2147483648, 'w'); local n = b.len(); b.seek(0); return n + ":" + b.readn('w') + ":" + b.tell() + "/" + b.eos(); });
s("rt_w_max", function() { local b = blob(0); b.writen(9223372036854775807, 'w'); local n = b.len(); b.seek(0); return n + ":" + b.readn('w') + ":" + b.tell() + "/" + b.eos(); });
s("rt_w_min", function() { local b = blob(0); b.writen((-9223372036854775807 - 1), 'w'); local n = b.len(); b.seek(0); return n + ":" + b.readn('w') + ":" + b.tell() + "/" + b.eos(); });
s("rt_c_z", function() { local b = blob(0); b.writen(0, 'c'); local n = b.len(); b.seek(0); return n + ":" + b.readn('c') + ":" + b.tell() + "/" + b.eos(); });
s("rt_c_one", function() { local b = blob(0); b.writen(1, 'c'); local n = b.len(); b.seek(0); return n + ":" + b.readn('c') + ":" + b.tell() + "/" + b.eos(); });
s("rt_c_mone", function() { local b = blob(0); b.writen(-1, 'c'); local n = b.len(); b.seek(0); return n + ":" + b.readn('c') + ":" + b.tell() + "/" + b.eos(); });
s("rt_c_258", function() { local b = blob(0); b.writen(258, 'c'); local n = b.len(); b.seek(0); return n + ":" + b.readn('c') + ":" + b.tell() + "/" + b.eos(); });
s("rt_c_neg258", function() { local b = blob(0); b.writen(-258, 'c'); local n = b.len(); b.seek(0); return n + ":" + b.readn('c') + ":" + b.tell() + "/" + b.eos(); });
s("rt_c_big16", function() { local b = blob(0); b.writen(70000, 'c'); local n = b.len(); b.seek(0); return n + ":" + b.readn('c') + ":" + b.tell() + "/" + b.eos(); });
s("rt_c_b31", function() { local b = blob(0); b.writen(2147483647, 'c'); local n = b.len(); b.seek(0); return n + ":" + b.readn('c') + ":" + b.tell() + "/" + b.eos(); });
s("rt_c_nb31", function() { local b = blob(0); b.writen(-2147483648, 'c'); local n = b.len(); b.seek(0); return n + ":" + b.readn('c') + ":" + b.tell() + "/" + b.eos(); });
s("rt_c_max", function() { local b = blob(0); b.writen(9223372036854775807, 'c'); local n = b.len(); b.seek(0); return n + ":" + b.readn('c') + ":" + b.tell() + "/" + b.eos(); });
s("rt_c_min", function() { local b = blob(0); b.writen((-9223372036854775807 - 1), 'c'); local n = b.len(); b.seek(0); return n + ":" + b.readn('c') + ":" + b.tell() + "/" + b.eos(); });
s("rt_b_z", function() { local b = blob(0); b.writen(0, 'b'); local n = b.len(); b.seek(0); return n + ":" + b.readn('b') + ":" + b.tell() + "/" + b.eos(); });
s("rt_b_one", function() { local b = blob(0); b.writen(1, 'b'); local n = b.len(); b.seek(0); return n + ":" + b.readn('b') + ":" + b.tell() + "/" + b.eos(); });
s("rt_b_mone", function() { local b = blob(0); b.writen(-1, 'b'); local n = b.len(); b.seek(0); return n + ":" + b.readn('b') + ":" + b.tell() + "/" + b.eos(); });
s("rt_b_258", function() { local b = blob(0); b.writen(258, 'b'); local n = b.len(); b.seek(0); return n + ":" + b.readn('b') + ":" + b.tell() + "/" + b.eos(); });
s("rt_b_neg258", function() { local b = blob(0); b.writen(-258, 'b'); local n = b.len(); b.seek(0); return n + ":" + b.readn('b') + ":" + b.tell() + "/" + b.eos(); });
s("rt_b_big16", function() { local b = blob(0); b.writen(70000, 'b'); local n = b.len(); b.seek(0); return n + ":" + b.readn('b') + ":" + b.tell() + "/" + b.eos(); });
s("rt_b_b31", function() { local b = blob(0); b.writen(2147483647, 'b'); local n = b.len(); b.seek(0); return n + ":" + b.readn('b') + ":" + b.tell() + "/" + b.eos(); });
s("rt_b_nb31", function() { local b = blob(0); b.writen(-2147483648, 'b'); local n = b.len(); b.seek(0); return n + ":" + b.readn('b') + ":" + b.tell() + "/" + b.eos(); });
s("rt_b_max", function() { local b = blob(0); b.writen(9223372036854775807, 'b'); local n = b.len(); b.seek(0); return n + ":" + b.readn('b') + ":" + b.tell() + "/" + b.eos(); });
s("rt_b_min", function() { local b = blob(0); b.writen((-9223372036854775807 - 1), 'b'); local n = b.len(); b.seek(0); return n + ":" + b.readn('b') + ":" + b.tell() + "/" + b.eos(); });
s("rt_f_f0", function() { local b = blob(0); b.writen(0.0, 'f'); local n = b.len(); b.seek(0); local r = b.readn('f'); return n + ":" + r; });
s("rt_f_fnz", function() { local b = blob(0); b.writen(-0.0, 'f'); local n = b.len(); b.seek(0); local r = b.readn('f'); return n + ":" + r; });
s("rt_f_f15", function() { local b = blob(0); b.writen(1.5, 'f'); local n = b.len(); b.seek(0); local r = b.readn('f'); return n + ":" + r; });
s("rt_f_fm15", function() { local b = blob(0); b.writen(-1.5, 'f'); local n = b.len(); b.seek(0); local r = b.readn('f'); return n + ":" + r; });
s("rt_f_fpi", function() { local b = blob(0); b.writen(3.14159, 'f'); local n = b.len(); b.seek(0); local r = b.readn('f'); return n + ":" + r; });
s("rt_f_fbig", function() { local b = blob(0); b.writen(1.0e30, 'f'); local n = b.len(); b.seek(0); local r = b.readn('f'); return n + ":" + r; });
s("rt_f_fsmall", function() { local b = blob(0); b.writen(1.0e-30, 'f'); local n = b.len(); b.seek(0); local r = b.readn('f'); return n + ":" + r; });
s("rt_d_f0", function() { local b = blob(0); b.writen(0.0, 'd'); local n = b.len(); b.seek(0); local r = b.readn('d'); return n + ":" + r; });
s("rt_d_fnz", function() { local b = blob(0); b.writen(-0.0, 'd'); local n = b.len(); b.seek(0); local r = b.readn('d'); return n + ":" + r; });
s("rt_d_f15", function() { local b = blob(0); b.writen(1.5, 'd'); local n = b.len(); b.seek(0); local r = b.readn('d'); return n + ":" + r; });
s("rt_d_fm15", function() { local b = blob(0); b.writen(-1.5, 'd'); local n = b.len(); b.seek(0); local r = b.readn('d'); return n + ":" + r; });
s("rt_d_fpi", function() { local b = blob(0); b.writen(3.14159, 'd'); local n = b.len(); b.seek(0); local r = b.readn('d'); return n + ":" + r; });
s("rt_d_fbig", function() { local b = blob(0); b.writen(1.0e30, 'd'); local n = b.len(); b.seek(0); local r = b.readn('d'); return n + ":" + r; });
s("rt_d_fsmall", function() { local b = blob(0); b.writen(1.0e-30, 'd'); local n = b.len(); b.seek(0); local r = b.readn('d'); return n + ":" + r; });

// ---- 扩充：多值混写 + 定位读（先写后按偏移读）
s("mixed_offsets", function() {
    local b = blob(0);
    b.writen(1, 'b'); b.writen(258, 's'); b.writen(1.5, 'f'); b.writen(7, 'l'); b.writen(9, 'b');
    local total = b.len();
    local out = total + ":";
    b.seek(0); out += b.readn('b') + ",";
    b.seek(1); out += b.readn('s') + ",";
    b.seek(3); out += b.readn('f') + ",";
    b.seek(7); out += b.readn('l') + ",";
    b.seek(-1, 2); out += b.readn('b') + ",";
    return out + b.eos() + "/" + b.tell();
});
s("resize_keep", function() { local b = blob(0); b.writen(1, 'b'); b.writen(2, 'b'); b.resize(5); return b.len() + ":" + hexdump(b); });
s("resize_shrink_seek", function() { local b = blob(0); b.writen(1, 'b'); b.writen(2, 'b'); b.writen(3, 'b'); b.seek(3); b.resize(1); return b.len() + ":" + b.tell(); });
s("seek_roundtrip_reads", function() {
    local b = blob(0);
    for (local i = 0; i < 8; i++) b.writen(i, 'b');
    local out = "";
    for (local i = 7; i >= 0; i--) { b.seek(i); out += b.readn('b'); }
    return out;
});
s("writeblob_chain", function() { local b = blob(0); local s1 = blob("ab"); local s2 = blob("cd"); b.writeblob(s1); b.writeblob(s2); return hexdump(b) + "/" + b.tell(); });
s("readblob_partial", function() { local b = blob("abcdef"); b.seek(4); local r = b.readblob(10); return hexdump(r) + "/" + b.tell() + "/" + b.eos(); });
s("swap2_twice", function() { local b = blob(0); b.writen(258, 'i'); b.swap2(); b.swap2(); return hexdump(b); });
s("swap4_twice", function() { local b = blob(0); b.writen(258, 'i'); b.swap4(); b.swap4(); return hexdump(b); });
s("nested_blob_stream", function() { local b = blob(0); local inner = blob("xy"); b.writen(inner.len(), 'b'); b.writeblob(inner); return hexdump(b); });
s("blob_len_after_writen", function() { local b = blob(0); b.writen(0, 'l'); return b.len(); });

print("BLOBMAP-DONE\n");

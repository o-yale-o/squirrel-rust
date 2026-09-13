// 自动生成：索引值边界矩阵（tools/gen_idxmatrix.py）
//
// 观测口径：数组用 dump()、字符串用 len:escape()，**绝不直接 print 容器**
//（`(table : 0x…)` 的指针文本两侧本来就不同，别给差分添噪声）。
// **不含**超大的 resize/array/blob 尺寸：参考实现在那些输入上直接崩溃或
// 把系统拖死（本实现给可捕获的 container too big，见 MIGRATION.md）。
function dump(a) {
    local s = "[";
    for (local i = 0; i < a.len(); i++) {
        if (i > 0) s += ",";
        local v = a[i];
        s += (typeof v == "string" ? "\"" + v + "\"" : "" + v);
    }
    return s + "]";
}
function esc(s) { return s.len() + ":" + escape(s); }
function t(tag, f) {
    try { local v = f(); local o = typeof v; print(tag + "=" + (o == "array" ? dump(v) : (o == "string" ? esc(v) : "" + v)) + "|" + o + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
local S = "hello";
local A = [1, 2, 3, 4, 5];

t("ss_nn5", function() { return S.slice(-5, -5); });
t("as_nn5", function() { return A.slice(-5, -5); });
t("ss_nm1m5", function() { return S.slice(-5, -1); });
t("as_nm1m5", function() { return A.slice(-5, -1); });
t("ss_rev", function() { return S.slice(-1, -5); });
t("as_rev", function() { return A.slice(-1, -5); });
t("ss_z10", function() { return S.slice(1, 0); });
t("as_z10", function() { return A.slice(1, 0); });
t("ss_z21", function() { return S.slice(2, 1); });
t("as_z21", function() { return A.slice(2, 1); });
t("ss_bb", function() { return S.slice(-100, -99); });
t("as_bb", function() { return A.slice(-100, -99); });
t("ss_oe", function() { return S.slice(3, 100); });
t("as_oe", function() { return A.slice(3, 100); });
t("ss_eo", function() { return S.slice(100, 3); });
t("as_eo", function() { return A.slice(100, 3); });
t("ss_min0", function() { return S.slice(0, -9223372036854775808); });
t("as_min0", function() { return A.slice(0, -9223372036854775808); });
t("ss_0min", function() { return S.slice(-9223372036854775808, 0); });
t("as_0min", function() { return A.slice(-9223372036854775808, 0); });
t("ss_minmax", function() { return S.slice(-9223372036854775808, 9223372036854775807); });
t("as_minmax", function() { return A.slice(-9223372036854775808, 9223372036854775807); });
t("ss_maxmin", function() { return S.slice(9223372036854775807, -9223372036854775808); });
t("as_maxmin", function() { return A.slice(9223372036854775807, -9223372036854775808); });
t("ss_f", function() { return S.slice(1.9, 3.9); });
t("as_f", function() { return A.slice(1.9, 3.9); });
t("ss_fn", function() { return S.slice(-2.5, -1.5); });
t("as_fn", function() { return A.slice(-2.5, -1.5); });
t("ss_s0e", function() { return S.slice(0, 5); });
t("as_s0e", function() { return A.slice(0, 5); });
t("ss_s0x", function() { return S.slice(0, 6); });
t("as_s0x", function() { return A.slice(0, 6); });
t("ss_s5", function() { return S.slice(5, 5); });
t("as_s5", function() { return A.slice(5, 5); });
t("ss_s6", function() { return S.slice(6, 6); });
t("as_s6", function() { return A.slice(6, 6); });
t("s1_i", function() { return S.slice(0); });
t("a1_i", function() { return A.slice(0); });
t("s1_m1", function() { return S.slice(-1); });
t("a1_m1", function() { return A.slice(-1); });
t("s1_m6", function() { return S.slice(-6); });
t("a1_m6", function() { return A.slice(-6); });
t("s1_m100", function() { return S.slice(-100); });
t("a1_m100", function() { return A.slice(-100); });
t("s1_mn", function() { return S.slice(-9223372036854775808); });
t("a1_mn", function() { return A.slice(-9223372036854775808); });
t("s1_mx", function() { return S.slice(9223372036854775807); });
t("a1_mx", function() { return A.slice(9223372036854775807); });
t("s1_fl", function() { return S.slice(1.5); });
t("a1_fl", function() { return A.slice(1.5); });
t("s1_fln", function() { return S.slice(-1.5); });
t("a1_fln", function() { return A.slice(-1.5); });
t("fd_fn100", function() { return S.find("l", -100); });
t("fd_fm1", function() { return S.find("l", -1); });
t("fd_f0", function() { return S.find("l", 0); });
t("fd_f1", function() { return S.find("l", 1); });
t("fd_f2", function() { return S.find("l", 2); });
t("fd_f4", function() { return S.find("l", 4); });
t("fd_f5", function() { return S.find("l", 5); });
t("fd_f6", function() { return S.find("l", 6); });
t("fd_f100", function() { return S.find("l", 100); });
t("fd_fmin", function() { return S.find("l", -9223372036854775808); });
t("fd_fmax", function() { return S.find("l", 9223372036854775807); });
t("fd_ffl", function() { return S.find("l", 2.5); });
t("fd_ffln", function() { return S.find("l", -2.5); });
t("fd_empty", function() { return S.find(""); });
t("fd_empty2", function() { return S.find("", 2); });
t("fd_empty6", function() { return S.find("", 6); });
t("fd_miss", function() { return S.find("z"); });
t("fd_self", function() { return S.find("hello"); });
t("fd_self3", function() { return S.find("hello", 3); });
t("ins_m6", function() { local c = [1, 2, 3, 4, 5]; c.insert(-6, 99); return dump(c); });
t("rm_m6", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(-6) + ":" + dump(c); });
t("ins_m1", function() { local c = [1, 2, 3, 4, 5]; c.insert(-1, 99); return dump(c); });
t("rm_m1", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(-1) + ":" + dump(c); });
t("ins_z0", function() { local c = [1, 2, 3, 4, 5]; c.insert(0, 99); return dump(c); });
t("rm_z0", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(0) + ":" + dump(c); });
t("ins_p5", function() { local c = [1, 2, 3, 4, 5]; c.insert(5, 99); return dump(c); });
t("rm_p5", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(5) + ":" + dump(c); });
t("ins_p6", function() { local c = [1, 2, 3, 4, 5]; c.insert(6, 99); return dump(c); });
t("rm_p6", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(6) + ":" + dump(c); });
t("ins_p100", function() { local c = [1, 2, 3, 4, 5]; c.insert(100, 99); return dump(c); });
t("rm_p100", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(100) + ":" + dump(c); });
t("ins_min", function() { local c = [1, 2, 3, 4, 5]; c.insert(-9223372036854775808, 99); return dump(c); });
t("rm_min", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(-9223372036854775808) + ":" + dump(c); });
t("ins_max", function() { local c = [1, 2, 3, 4, 5]; c.insert(9223372036854775807, 99); return dump(c); });
t("rm_max", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(9223372036854775807) + ":" + dump(c); });
t("ins_fl", function() { local c = [1, 2, 3, 4, 5]; c.insert(1.5, 99); return dump(c); });
t("rm_fl", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(1.5) + ":" + dump(c); });
t("ins_fln", function() { local c = [1, 2, 3, 4, 5]; c.insert(-1.5, 99); return dump(c); });
t("rm_fln", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(-1.5) + ":" + dump(c); });
t("rz_r0", function() { local c = [1, 2, 3, 4, 5]; c.resize(0); return dump(c); });
t("rzf_r0", function() { local c = [1, 2]; c.resize(0, "x"); return dump(c); });
t("rz_r1", function() { local c = [1, 2, 3, 4, 5]; c.resize(1); return dump(c); });
t("rzf_r1", function() { local c = [1, 2]; c.resize(1, "x"); return dump(c); });
t("rz_r3", function() { local c = [1, 2, 3, 4, 5]; c.resize(3); return dump(c); });
t("rzf_r3", function() { local c = [1, 2]; c.resize(3, "x"); return dump(c); });
t("rz_r5", function() { local c = [1, 2, 3, 4, 5]; c.resize(5); return dump(c); });
t("rzf_r5", function() { local c = [1, 2]; c.resize(5, "x"); return dump(c); });
t("rz_r8", function() { local c = [1, 2, 3, 4, 5]; c.resize(8); return dump(c); });
t("rzf_r8", function() { local c = [1, 2]; c.resize(8, "x"); return dump(c); });
t("rz_rfl", function() { local c = [1, 2, 3, 4, 5]; c.resize(2.9); return dump(c); });
t("rzf_rfl", function() { local c = [1, 2]; c.resize(2.9, "x"); return dump(c); });
t("rz_rfln", function() { local c = [1, 2, 3, 4, 5]; c.resize(0.5); return dump(c); });
t("rzf_rfln", function() { local c = [1, 2]; c.resize(0.5, "x"); return dump(c); });
t("rz_null_fill", function() { local c = [1]; c.resize(3); return dump(c); });
t("ix_max", function() { return S[9223372036854775807]; });
t("ix_min", function() { return S[-9223372036854775808]; });
t("ix_fl", function() { return S[1.9]; });
t("ixa_max", function() { return A[9223372036854775807]; });
t("ixa_min", function() { return A[-9223372036854775808]; });
t("ixa_fl", function() { return A[2.9]; });
t("s3", function() { return S.slice(1, 2, 3); });
t("s_str", function() { return S.slice("1"); });
t("s_null", function() { return S.slice(null); });
t("a3", function() { return A.slice(1, 2, 3); });
t("bsk_past", function() { local b = blob(4); return b.seek(8, 'b'); });
t("bsk_neg", function() { local b = blob(4); return b.seek(-8, 'b'); });
t("bsk_max", function() { local b = blob(4); return b.seek(9223372036854775807, 'b'); });
t("bsk_end_past", function() { local b = blob(4); return b.seek(4, 'e'); });
t("bsk_end_neg", function() { local b = blob(4); return b.seek(-8, 'e'); });
t("bgrow_write", function() { local b = blob(4); b.seek(4, 'b'); b.writen(7, 'b'); return b.len() + ":" + b.tell(); });
t("bgrow_far", function() { local b = blob(4); b.seek(4, 'b'); b.writen(7, 'i'); return b.len() + ":" + b.tell(); });
t("bwdef", function() { local b = blob(); b.writen(1, 'b'); return b.len() + ":" + b.tell(); });
t("breadback", function() { local b = blob(4); b.seek(0, 'b'); b.writen(0x41, 'b'); b.seek(0, 'b'); return b.readn('b'); });
t("brb_zero", function() { local b = blob(4); return b.readblob(0); });
t("brb_two", function() { local b = blob(4); local r = b.readblob(2); return r.len() + ":" + b.tell(); });
t("brb_all", function() { local b = blob(4); local r = b.readblob(99); return r.len() + ":" + b.tell(); });
print("IDXMAP-DONE\n");

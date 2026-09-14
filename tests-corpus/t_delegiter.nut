// 自动生成：委托链 + 迭代期变更矩阵（tools/gen_delegiter.py）
//
// 两族都是现有生成器结构性地碰不到的：链的动力学（深度/循环/跨类型/raw*）
// 与迭代的动力学（foreach 期间增/删/缩/清）。全部用例已在参考实现上实测
// 跑通（tmp-c2/foreach_mut.py 逐条隔离），两侧逐字节一致。
function t(tag, f) {
    try { local v = f(); local o = typeof v; print(tag + "=" + (o == "string" ? v : "" + v) + "|" + o + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

class K_INST_H { y = 7 }
local K_INST = K_INST_H();


t("d1", function() { local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.x; });
t("d2", function() { local a = { x = 1 }; local b = {}; local c = {}; b.setdelegate(a); c.setdelegate(b); return c.x; });
t("d6", function() { local root = { x = "deep" }; local prev = root; for (local i = 0; i < 5; i++) { local n = {}; n.setdelegate(prev); prev = n; } return prev.x; });
t("d20", function() { local root = { x = "d20" }; local prev = root; for (local i = 0; i < 19; i++) { local n = {}; n.setdelegate(prev); prev = n; } return prev.x; });
t("shadow", function() { local a = { x = "a" }; local b = { x = "b" }; b.setdelegate(a); return b.x; });
t("mid_shadow", function() { local a = { x = "a" }; local b = {}; local c = {}; b.setdelegate(a); c.setdelegate(b); b.x = "b"; return c.x; });
t("t2c", function() { class K { y = 7 } local o = {}; o.setdelegate(K); return o.y; });
t("t2i", function() { local o = {}; o.setdelegate(K_INST); return o.y; });
t("c2t", function() { local tt = { z = 9 }; class K2 { }; local k = K2(); k.setdelegate(tt); return k.z; });
t("t2a", function() { local o = {}; o.setdelegate([1, 2]); return "ok"; });
t("a2t", function() { local a = [1, 2]; a.setdelegate({ q = 1 }); return "ok"; });
t("cyc2", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a);  return a.zzz; });
t("cyc1", function() { local a = {}; a.setdelegate(a);  return a.zzz; });
t("cyc3", function() { local a = {}; local b = {}; local c = {}; a.setdelegate(b); b.setdelegate(c); c.setdelegate(a);  return a.zzz; });
t("cyc_get", function() { local a = { existing = 1 }; local b = {}; a.setdelegate(b); b.setdelegate(a); return a.existing; });
t("cyc_set", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a);  a.foo <- 1; return a.foo; });
t("cyc_del", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a);  return delete a.foo; });
t("cyc_in", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a);  return "foo" in a; });
t("cyc_rawin", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a);  return a.rawin("foo"); });
t("cyc_len", function() { local a = { p = 1 }; local b = {}; a.setdelegate(b); b.setdelegate(a); return a.len(); });
t("cyc_foreach", function() { local a = { p = 1 }; local b = {}; a.setdelegate(b); b.setdelegate(a); local n = 0; foreach (k, v in a) n++; return n; });
t("cyc_tostr", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a);  return "" + a; });
t("cyc_typeof", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a);  return typeof a; });
t("cyc_clone", function() { local a = { p = 1 }; local b = {}; a.setdelegate(b); b.setdelegate(a); return clone a; });
t("cyc_getdelegate", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a);  return typeof a.getdelegate(); });
t("sd_ret", function() { local a = {}; local b = {}; return typeof a.setdelegate(b); });
t("sd_null", function() { local a = {}; a.setdelegate({ q = 1 }); a.setdelegate(null); return a.q; });
t("sd_self", function() { local a = {}; a.setdelegate({ q = 1 }); return a.setdelegate(a).q; });
t("sd_cyc_assign", function() { local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a); return "no-cycle-error"; });
t("raw_get", function() { local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.rawget("x"); });
t("raw_in", function() { local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.rawin("x"); });
t("raw_set", function() { local a = { x = 1 }; local b = {}; b.setdelegate(a); b.rawset("x", 9); return a.x + "/" + b.x; });
t("mm_before_chain", function() { local bt = { x = "chain" }; local d = { _get = function(k) { return "mm"; } }; local o = {}; o.setdelegate(d); d.setdelegate(bt); return o.x; });
t("mm_after_miss", function() { local bt = { }; local d = { _get = function(k) { return "mm"; } }; local o = {}; o.setdelegate(d); d.setdelegate(bt); return o.zzz; });
t("ns_chain", function() { local a = { }; local b = {}; b.setdelegate(a); b.q <- 5; return a.rawin("q") + "/" + b.rawin("q"); });
t("del_chain", function() { local a = { q = 1 }; local b = {}; b.setdelegate(a); return delete b.q; });
t("del_chain_mm", function() { local a = { q = 1 }; local d = { _delslot = function(k) { return "mm"; } }; local b = {}; b.setdelegate(d); d.setdelegate(a); return delete b.q; });
t("foreach_chain", function() { local a = { x = 1, y = 2 }; local b = { z = 3 }; b.setdelegate(a); local n = 0; foreach (k, v in b) n++; return n; });
t("keys_chain", function() { local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.len(); });
t("tbl_ins_self", function() { local tb = { a = 1, b = 2, c = 3 }; local n = 0; foreach (k, v in tb) { n++; if (n == 1) tb.newk <- 9; } return n + "/" + tb.len(); });
t("tbl_ins_many", function() { local tb = { a = 1, b = 2, c = 3 }; local n = 0; foreach (k, v in tb) { n++; tb["k" + n] <- n; if (n > 50) break; } return n + "/" + tb.len(); });
t("tbl_del_cur", function() { local tb = { a = 1, b = 2, c = 3, d = 4 }; local n = 0; foreach (k, v in tb) { n++; delete tb[k]; } return n + "/" + tb.len(); });
t("tbl_del_other", function() { local tb = { a = 1, b = 2, c = 3, d = 4 }; local seen = ""; foreach (k, v in tb) { seen += k; if (k == "a") delete tb.d; } return seen + "/" + tb.len(); });
t("tbl_set_cur", function() { local tb = { a = 1, b = 2, c = 3 }; local n = 0; foreach (k, v in tb) { tb[k] = v * 10; n++; } return n + "/" + tb.a + tb.b + tb.c; });
t("tbl_clear", function() { local tb = { a = 1, b = 2, c = 3 }; local seen = ""; foreach (k, v in tb) { seen += k; tb.clear(); } return seen + "/" + tb.len(); });
t("tbl_del_then_ins", function() { local tb = { a = 1, b = 2, c = 3 }; local n = 0; foreach (k, v in tb) { n++; delete tb[k]; tb[k + "2"] <- v; if (n > 30) break; } return n + "/" + tb.len(); });
t("arr_append", function() { local a = [1, 2, 3]; local n = 0; foreach (v in a) { n++; if (n < 10) a.append(n); } return n + "/" + a.len(); });
t("arr_remove_cur", function() { local a = [1, 2, 3, 4]; local n = 0; foreach (v in a) { n++; a.remove(0); } return n + "/" + a.len(); });
t("arr_set_cur", function() { local a = [1, 2, 3]; local n = 0; foreach (v in a) { a[0] = 9; n++; } return n + "/" + a[0] + a[1] + a[2]; });
t("arr_resize", function() { local a = [1, 2, 3]; local n = 0; foreach (v in a) { n++; a.resize(10); } return n + "/" + a.len(); });
t("arr_shrink", function() { local a = [1, 2, 3, 4, 5]; local seen = ""; foreach (v in a) { seen += v; a.resize(a.len() - 1); } return seen + "/" + a.len(); });
t("nested_same_tbl", function() { local tb = { a = 1, b = 2 }; local n = 0; foreach (k1, v1 in tb) foreach (k2, v2 in tb) n++; return "" + n; });
t("break_foreach", function() { local s = ""; foreach (v in [1, 2, 3]) { if (v == 2) break; s += v; } return s; });
t("continue_foreach", function() { local s = ""; foreach (v in [1, 2, 3]) { if (v == 2) continue; s += v; } return s; });
t("return_foreach", function() { foreach (v in [1, 2, 3]) { if (v == 2) return v; } return 0; });
t("throw_foreach", function() { foreach (v in [1, 2, 3]) { if (v == 2) throw "mid"; } return 0; });
t("str_foreach", function() { local s = ""; foreach (c in "abc") s += c + ","; return s; });
t("blob_foreach", function() { local b = blob(3); local n = 0; foreach (v in b) n++; return "" + n; });
t("gen_foreach", function() { local g = function() { yield 1; yield 2; }; local n = 0; foreach (v in g()) n++; return "" + n; });
print("DELEGITER-DONE\n");

// 自动生成：容器回调矩阵（tools/gen_cbmatrix.py）
//
// 观测口径：**一律用 dump() 打印内容**，绝不直接 print 容器 ——
// `(array : 0x…)` 里的指针值两侧本来就不同（diff_test 会归一化，但
// 一次性脚本里踩过一次假绿，索性从源头避免）。
function dump(a) {
    local s = "[";
    for (local i = 0; i < a.len(); i++) {
        if (i > 0) s += ",";
        local v = a[i];
        s += (typeof v == "string" ? v : "" + v);
    }
    return s + "]";
}
function t(tag, f) {
    try { local v = f(); print(tag + "=" + (typeof v == "string" ? v : "" + v) + "|" + typeof v + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

local A = [10, 20];
local tb = { a = 1, b = 2 };

// --- 1) sort 比较器返回值：类型与截断 ---
t("sr_int0", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return 0; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_int1", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return 1; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_intn1", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return -1; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_int2", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return 2; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_intn2", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return -2; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_float0", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return 0.0; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_f_half", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return 0.5; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_f_1_5", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return 1.5; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_f_n_half", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return -0.5; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_f_2_9", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return 2.9; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_f_n_1_9", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return -1.9; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_f_from_sub", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return a - b; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_f_big", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return 1e15; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_bool_true", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return true; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_bool_false", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return false; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_null", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return null; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_str", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return "x"; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_arr", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return []; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_tbl", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return {}; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_func", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { return function() {}; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_throw", function() {
    local arr = [4, 2, 5, 1, 3];
    try { arr.sort(function(a, b) { throw "sort-boom"; }); }
    catch (e) { return "E:" + e + " after " + dump(arr); }
    return dump(arr);
});
t("sr_float_array_sub", function() { local a = [2.5, 1.5, 3.5]; a.sort(function(x, y) { return x - y; }); return dump(a); });
t("sr_float_array_rev", function() { local a = [2.5, 1.5, 3.5]; a.sort(function(x, y) { return y - x; }); return dump(a); });
t("sr_float_bool_cmp", function() { local a = [2.5, 1.5, 3.5]; a.sort(function(x, y) { return x < y; }); return dump(a); });
t("sr_string_array", function() { local a = ["b", "a", "c"]; a.sort(function(x, y) { return x <=> y; }); return dump(a); });
t("sr_mixed_sizes", function() { local a = [5, 1, 4, 2, 8, 3, 9, 7, 6, 10, 0]; a.sort(function(x, y) { return x - y; }); return dump(a); });
t("sr_empty", function() { local a = []; a.sort(function(x, y) { throw "never"; }); return dump(a); });
t("sr_one", function() { local a = [7]; a.sort(function(x, y) { throw "never"; }); return dump(a); });
t("sr_two", function() { local a = [2, 1]; a.sort(function(x, y) { return x - y; }); return dump(a); });
t("sr_stability", function() { local a = [[1, "a"], [0, "b"], [1, "c"], [0, "d"]]; a.sort(function(x, y) { return x[0] - y[0]; }); local s = ""; foreach (e in a) s += e[1]; return s; });
t("sr_arity0", function() { local a = [3, 1, 2]; a.sort(function() { return 0; }); return dump(a); });
t("sr_arity1", function() { local a = [3, 1, 2]; a.sort(function(x) { return 0; }); return dump(a); });
t("sr_arity3", function() { local a = [3, 1, 2]; a.sort(function(x, y, z) { return 0; }); return dump(a); });
t("sr_native", function() { local a = [3, 1, 2]; a.sort(abs); return dump(a); });
t("sr_null", function() { local a = [3, 1, 2]; a.sort(null); return dump(a); });
t("sr_args", function() { local seen = ""; local a = [2, 1]; a.sort(function(x, y) { seen += typeof x + "/" + typeof y + ";"; return x <=> y; }); return seen; });
t("sr_this", function() { local a = [2, 1]; local seen = "?"; a.sort(function(x, y) { seen = typeof this; return x <=> y; }); return seen; });

// --- 2) 回调形参个数（map/apply 参考实现只支持 1~3 个）---
t("m_p0", function() { local r = A.map(function() { return "z"; }); return typeof r + ":" + r.len(); });
t("m_p1", function() { local r = A.map(function(a) { return "z"; }); return typeof r + ":" + r.len(); });
t("m_p2", function() { local r = A.map(function(a, b) { return "z"; }); return typeof r + ":" + r.len(); });
t("m_p3", function() { local r = A.map(function(a, b, c) { return "z"; }); return typeof r + ":" + r.len(); });
t("m_p4", function() { local r = A.map(function(a, b, c, d) { return "z"; }); return typeof r + ":" + r.len(); });
t("m_p5", function() { local r = A.map(function(a, b, c, d, e) { return "z"; }); return typeof r + ":" + r.len(); });
t("ap_p0", function() { local c = [1, 2]; c.apply(function() { return 9; }); return dump(c); });
t("ap_p1", function() { local c = [1, 2]; c.apply(function(a) { return 9; }); return dump(c); });
t("ap_p2", function() { local c = [1, 2]; c.apply(function(a, b) { return 9; }); return dump(c); });
t("ap_p3", function() { local c = [1, 2]; c.apply(function(a, b, c) { return 9; }); return dump(c); });
t("ap_p4", function() { local c = [1, 2]; c.apply(function(a, b, c, d) { return 9; }); return dump(c); });
t("ap_p5", function() { local c = [1, 2]; c.apply(function(a, b, c, d, e) { return 9; }); return dump(c); });
t("ft_p0", function() { local r = A.filter(function() { return true; }); return "n" + r.len(); });
t("ft_p1", function() { local r = A.filter(function(a) { return true; }); return "n" + r.len(); });
t("ft_p2", function() { local r = A.filter(function(a, b) { return true; }); return "n" + r.len(); });
t("ft_p3", function() { local r = A.filter(function(a, b, c) { return true; }); return "n" + r.len(); });
t("ft_p4", function() { local r = A.filter(function(a, b, c, d) { return true; }); return "n" + r.len(); });
t("ft_p5", function() { local r = A.filter(function(a, b, c, d, e) { return true; }); return "n" + r.len(); });
t("rd_p0", function() { return A.reduce(function() { return 1; }, 0); });
t("rd_p1", function() { return A.reduce(function(a) { return 1; }, 0); });
t("rd_p2", function() { return A.reduce(function(a, b) { return 1; }, 0); });
t("rd_p3", function() { return A.reduce(function(a, b, c) { return 1; }, 0); });
t("rd_p4", function() { return A.reduce(function(a, b, c, d) { return 1; }, 0); });
t("rd_p5", function() { return A.reduce(function(a, b, c, d, e) { return 1; }, 0); });
t("fd_p0", function() { return A.find(function() { return true; }); });
t("fd_p1", function() { return A.find(function(a) { return true; }); });
t("fd_p2", function() { return A.find(function(a, b) { return true; }); });
t("fd_p3", function() { return A.find(function(a, b, c) { return true; }); });
t("fd_p4", function() { return A.find(function(a, b, c, d) { return true; }); });
t("fd_p5", function() { return A.find(function(a, b, c, d, e) { return true; }); });
t("m_empty_p0", function() { local e = []; return typeof e.map(function() { return 1; }); });
t("ap_empty_p0", function() { local e = []; e.apply(function() { return 1; }); return "ok"; });
t("m_native", function() { local a = [1]; return typeof a.map(print); });
t("tb_map_p0", function() { return typeof tb.map(function() { return 1; }); });
t("tb_map_p1", function() { return typeof tb.map(function(k) { return 1; }); });
t("tb_map_p2", function() { return typeof tb.map(function(k, v) { return 1; }); });
t("tb_map_p4", function() { return typeof tb.map(function(k, v, x, y) { return 1; }); });
t("tb_filter_p0", function() { return tb.filter(function() { return true; }).len(); });
t("tb_filter_p2", function() { return tb.filter(function(k, v) { return true; }).len(); });
t("tb_keys_p0", function() { return tb.keys(function() { return true; }).len(); });
t("tb_keys_p2", function() { return tb.keys(function(k, v) { return true; }).len(); });
t("tb_len_cb", function() { return typeof tb.len(); });

// --- 3) 回调实参语义：this = 数组本身，第 1 个实参是值、第 2 个是下标、第 3 个又是数组 ---
t("as_m1", function() { local r = A.map(function(a) { return "" + typeof a + ":" + a; }); return dump(r); });
t("as_m2", function() { local r = A.map(function(a, b) { return "" + typeof a + ":" + a + "|" + typeof b + ":" + b; }); return dump(r); });
t("as_m3", function() { local r = A.map(function(a, b, c) { return "" + typeof a + ":" + a + "|" + typeof b + ":" + b + "|" + typeof c + ":" + c.len(); }); return dump(r); });
t("as_this_is_arr", function() { local r = A.map(function(a) { return (this == A) + "/" + this.len(); }); return dump(r); });
t("as_filter2", function() { local r = A.filter(function(a, b) { return b == 0; }); return dump(r); });
t("as_filter3", function() { local r = A.filter(function(a, b, c) { return typeof c == "array"; }); return dump(r); });
t("as_find2", function() { return A.find(function(a, b) { return b == 1; }); });
t("as_reduce2", function() { return A.reduce(function(acc, v) { return "" + acc + "/" + v; }, "S"); });
t("as_reduce3", function() { return A.reduce(function(acc, v, i) { return "" + acc + "/" + v + "/" + typeof i + i; }, "S"); });
t("as_reduce_noinit", function() { return A.reduce(function(acc, v) { return "" + acc + "/" + v; }); });
t("as_reduce_one", function() { local a = [5]; return a.reduce(function(acc, v) { return "" + acc + "/" + v; }); });
t("as_reduce_empty", function() { local a = []; return "R:" + a.reduce(function(acc, v) { return 1; }); });
t("as_reduce_null_init", function() { return "R:" + A.reduce(function(acc, v) { return acc; }, null); });
t("as_apply1", function() { local c = [1, 2]; c.apply(function(v) { return v * 10; }); return dump(c); });
t("as_apply_idx", function() { local c = [1, 2]; c.apply(function(v, i) { return "" + i; }); return dump(c); });
t("as_tb_map", function() { local r = tb.map(function(k, v) { return "" + k + "=" + v; }); return typeof r + ":" + r.len(); });
t("as_tb_keys", function() { return tb.keys(function(k) { return true; }).len(); });

// --- 4) 回调抛异常 / 回调不是函数 ---
t("th_map", function() { local a = [1, 2, 3]; try { a.map(function(v) { throw "map-boom"; }) } catch (e) { return "E:" + e; } return "no-error"; });
t("th_filter", function() { local a = [1, 2, 3]; try { a.filter(function(v) { throw "f"; }) } catch (e) { return "E:" + e; } return "no-error"; });
t("th_reduce", function() { local a = [1, 2, 3]; try { a.reduce(function(acc, v) { throw "r"; }, 0) } catch (e) { return "E:" + e; } return "no-error"; });
t("th_apply", function() { local a = [1, 2, 3]; try { a.apply(function(v) { throw "ap"; }) } catch (e) { return "E:" + e; } return "no-error"; });
t("th_find", function() { local a = [1, 2, 3]; try { a.find(function(v) { throw "fi"; }) } catch (e) { return "E:" + e; } return "no-error"; });
t("th_sort", function() { local a = [1, 2, 3]; try { a.sort(function(x, y) { throw "s"; }) } catch (e) { return "E:" + e; } return "no-error"; });
t("th_tmap", function() { local a = [1, 2, 3]; try { tb.map(function(k, v) { throw "tm"; }) } catch (e) { return "E:" + e; } return "no-error"; });
t("th_tfilter", function() { local a = [1, 2, 3]; try { tb.filter(function(k, v) { throw "tf"; }) } catch (e) { return "E:" + e; } return "no-error"; });
t("th_tkeys", function() { local a = [1, 2, 3]; try { tb.keys(function(k) { throw "tk"; }) } catch (e) { return "E:" + e; } return "no-error"; });
t("nf_int", function() { try { return "" + (A.map(5)); } catch (e) { return "E:" + e; } });
t("nf_null", function() { try { return "" + (A.map(null)); } catch (e) { return "E:" + e; } });
t("nf_str", function() { try { return "" + (A.filter("x")); } catch (e) { return "E:" + e; } });
t("nf_arr", function() { try { return "" + (A.reduce([], 0)); } catch (e) { return "E:" + e; } });
t("nf_int_sort", function() { try { return "" + ([2, 1].sort(5)); } catch (e) { return "E:" + e; } });
t("nf_arr_sort", function() { try { return "" + ([2, 1].sort([])); } catch (e) { return "E:" + e; } });
print("CBMAP-DONE\n");

// blob.resize 各参数形态单独跑（参考实现可能写到未初始化的 size）
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
s("r_i0", function() { return ((blob(4))).resize(0); });
s("r_i1", function() { return ((blob(4))).resize(1); });
s("r_im1", function() { return ((blob(4))).resize(-1); });
s("r_in99", function() { return ((blob(4))).resize(-99); });
s("r_f", function() { return ((blob(4))).resize(1.5); });
s("r_fn15", function() { return ((blob(4))).resize(-1.5); });
s("r_null", function() { return ((blob(4))).resize(null); });
s("r_bool", function() { return ((blob(4))).resize(true); });
s("r_s", function() { return ((blob(4))).resize("s"); });
s("r_es", function() { return ((blob(4))).resize(""); });
s("r_arr", function() { return ((blob(4))).resize([1, 2])); });
s("r_tbl", function() { return ((blob(4))).resize({ k = 1 }); });
s("r_p99", function() { return ((blob(4))).resize(99); });
s("r_p9999", function() { return ((blob(4))).resize(9999); });

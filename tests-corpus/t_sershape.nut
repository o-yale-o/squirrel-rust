
// 自动生成：闭包序列化 × 值形状矩阵（tools/gen_sermatrix.py）
// 每条：写出去 -> 读回来 -> 调用 -> show(返回值) + typeof
// 临时文件路径用 root 表的全局（local 声明在函数之后时，函数里解析不到 → 会报
// "the index 'PATH' does not exist"，生成器自检抓到过）
PATH <- "tmp-c2/sershape_tmp.cnut";
function show(r, d = 0) {
    if (d > 4) return "...";
    local t = typeof r;
    if (t == "array") { local s = "["; for (local i = 0; i < r.len(); i++) { if (i > 0) s += ","; s += show(r[i], d + 1); } return s + "]"; }
    if (t == "table") { local s = "{"; foreach (k, v in r) s += show(k, d + 1) + ":" + show(v, d + 1) + ","; return s + "}"; }
    if (t == "string") return "\"" + r + "\"";
    if (t == "float") return format("%.17g", r);
    if (t == "class" || t == "instance") { try { return r.tostring(); } catch (e) { return "(" + t + ")"; } }
    return "" + r;
}
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!outer:" + e + "\n"); }
}
// 往返：写闭包、读回、调用（可带实参）、渲染
function rt(f) {
    writeclosuretofile(PATH, f);
    local g = loadfile(PATH);
    local r = null; local err = null;
    try { r = g(); } catch (e) { err = "" + e; }
    return show(r) + "|" + typeof r + "|err=" + show(err);
}
function rt_args(f, a1) {
    writeclosuretofile(PATH, f);
    local g = loadfile(PATH);
    local r = null; local err = null;
    try { r = g(a1); } catch (e) { err = "" + e; }
    return show(r) + "|" + typeof r + "|err=" + show(err);
}
function rt_two(f, a1, a2) {
    writeclosuretofile(PATH, f);
    local g = loadfile(PATH);
    local r = null; local err = null;
    try { r = g(a1, a2); } catch (e) { err = "" + e; }
    return show(r) + "|" + typeof r + "|err=" + show(err);
}
// 错误路径也要自己吃掉错误再打印（否则会变成 t() 外层的 "!outer:"，
// 既让生成器自检失去意义，也让"用例是否真的执行到了"的信号变弱）
function terr(f) {
    local r = null; local err = null;
    try { r = f(); } catch (e) { err = "" + e; }
    return show(r) + "|err=" + show(err);
}
local PATH_UNUSED = 0;

t("v_null", function() { return rt(function() { return null; }); });
t("v_true", function() { return rt(function() { return true; }); });
t("v_false", function() { return rt(function() { return false; }); });
t("v_int0", function() { return rt(function() { return 0; }); });
t("v_int_neg", function() { return rt(function() { return -12345; }); });
t("v_int_max", function() { return rt(function() { return 9223372036854775807; }); });
t("v_int_min", function() { return rt(function() { return -9223372036854775807 - 1; }); });
t("v_float0", function() { return rt(function() { return 0.0; }); });
t("v_float_neg0", function() { return rt(function() { return -0.0; }); });
t("v_float_frac", function() { return rt(function() { return -1.5; }); });
t("v_float_big", function() { return rt(function() { return 1e30; }); });
t("v_float_tiny", function() { return rt(function() { return 1.1754944e-38; }); });
t("v_str_empty", function() { return rt(function() { return ""; }); });
t("v_str_ascii", function() { return rt(function() { return "hello"; }); });
t("v_str_escapes", function() { return rt(function() { return "a\tb\nc\\d\"e"; }); });
t("v_str_high", function() { return rt(function() { return "\xff\xfe"; }); });
t("v_str_long", function() { return rt(function() { return "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; }); });
t("v_arr_empty", function() { return rt(function() { return []; }); });
t("v_arr_ints", function() { return rt(function() { return [1, 2, 3]; }); });
t("v_arr_mixed", function() { return rt(function() { return [1, "a", null, true, 1.5]; }); });
t("v_arr_nested", function() { return rt(function() { return [1, [2, [3, [4]]]]; }); });
t("v_arr_of_arr", function() { return rt(function() { return [[1], [2], [3]]; }); });
t("v_tbl_empty", function() { return rt(function() { return {}; }); });
t("v_tbl_str_keys", function() { return rt(function() { return { a = 1, b = "two" }; }); });
t("v_tbl_int_key", function() { return rt(function() { return { [1] = "intkey" }; }); });
t("v_tbl_mixed_keys", function() { return rt(function() { return { a = 1, [2] = "b", [1.5] = true }; }); });
t("v_tbl_nested", function() { return rt(function() { return { a = { b = { c = 3 } } }; }); });
t("v_tbl_in_arr", function() { return rt(function() { return [{ a = 1 }, { b = [2] }]; }); });
t("v_closure_inner", function() { return rt(function() { local f = function(x) { return x * 2; }; return f(21); }); });
t("v_closure_nested", function() { return rt(function() { local f = function(x) { local g = function(y) { return y + 1; }; return g(x); }; return f(41); }); });
t("v_closure_returned", function() { return rt(function() { return function(x) { return x + 1; }; }); });
t("v_closure_varargs", function() { return rt(function() { local f = function(...) { return vargv.len(); }; return f(1, 2, 3); }); });
t("v_closure_default", function() { return rt(function() { local f = function(a, b = 7) { return a + b; }; return f(1); }); });
t("v_closure_recursive", function() { return rt(function() { local f = null; f = function(n) { return n <= 1 ? 1 : n * f(n - 1); }; return f(5); }); });
t("v_class_basic", function() { return rt(function() { class C { x = 1; function getx() { return x; } } return C().getx(); }); });
t("v_class_static", function() { return rt(function() { class C { static s = 3; function get() { return s; } } return C().get(); }); });
t("v_class_inherit", function() { return rt(function() { class B { function who() { return "B"; } } class D extends B { function who() { return "D" + base.who(); } } return D().who(); }); });
t("v_class_attrs", function() { return rt(function() { class C { </ doc = "d" /> x = 1; } return C.getattributes("x").doc; }); });
t("v_class_newmember", function() { return rt(function() { class C { } C.newmember("f", function() { return 9; }); return C().f(); }); });
t("v_class_tostring", function() { return rt(function() { class C { function _tostring() { return "C!"; } } return "" + C(); }); });
t("v_class_instanceof", function() { return rt(function() { class C { } local c = C(); return "" + (c instanceof C); }); });
t("v_gen_created", function() { return rt(function() { local g = (function() { yield 1; yield 2; }()); local a = resume g; return a; }); });
t("v_gen_status", function() { return rt(function() { local g = (function() { yield 1; }()); return g.getstatus(); }); });
t("v_blob_created", function() { return rt(function() { local b = blob(3); b.writen(65, 'b'); return b.readn('b'); }); });
t("v_blob_method", function() { return rt(function() { local b = blob(2); b.seek(0, 'b'); b.writen(0x0102, 'w'); b.seek(0, 'b'); return b.readn('w'); }); });
t("v_weakref_created", function() { return rt(function() { local tb = {}; local w = tb.weakref(); return "" + (w.ref() == tb); }); });
t("v_re_created", function() { return rt(function() { local r = regexp("a(b)c"); local m = r.match("xabcx"); return m[0] + "," + m[1]; }); });
t("v_re_search", function() { return rt(function() { local r = regexp("[0-9]+"); return r.search("ab123"); }); });
t("v_string_methods", function() { return rt(function() { return "Hello".tolower() + "/" + "Hello".toupper(); }); });
t("v_min_max", function() { return rt(function() { return [1, 2, 3].reduce(@(a, b) a > b ? a : b); }); });
t("v_sort_in_closure", function() { return rt(function() { local a = [3, 1, 2]; a.sort(); return a; }); });
t("v_try_inside", function() { return rt(function() { try { throw "inner"; } catch (e) { return "caught:" + e; } }); });
t("v_loop_inside", function() { return rt(function() { local s = 0; for (local i = 0; i < 5; i++) s += i; return s; }); });
t("v_gen_closure_roundtrip", function() {
    writeclosuretofile(PATH, function() { yield 1; yield 2; });
    local g = loadfile(PATH);
    local made = g();
    local a = null; local b = null; local err = null;
    try { a = resume made; b = resume made; } catch (e) { err = "" + e; }
    return typeof g + "/" + typeof made + "/" + made.getstatus()
         + "/" + show(a) + "," + show(b) + "/err=" + show(err);
});
t("v_env_current", function() { return rt(function() { return getroottable() == getroottable(); }); });
t("v_native_ref", function() { return rt(function() { return typeof print; }); });
t("a_args_two", function() { return rt_two(function(a, b) { return a + b; }, 3, 4); });
t("a_args_default_used", function() { return rt_two(function(a, b = 100) { return b; }, 1, 2); });
t("a_args_default_missing", function() { return rt_args(function(a, b = 100) { return "b=" + b; }, 1); });
t("a_args_varargs", function() { return rt_two(function(a, ...) { return vargv.len(); }, 1, 2); });
t("e_native_direct", function() { return terr(function() { writeclosuretofile(PATH, print); return "no-error"; }); });
t("e_freevar", function() { return terr(function() { local n = 5; local f = function() { return n; }; writeclosuretofile(PATH, f); return "no-error"; }); });
t("e_badarg", function() { return terr(function() { writeclosuretofile(PATH, 1); return "no-error"; }); });
t("e_noargs", function() { return terr(function() { writeclosuretofile(); return "no-error"; }); });
t("e_badpath", function() { return terr(function() { writeclosuretofile("no_such_dir/x.cnut", function() { return 1; }); return "no-error"; }); });
t("e_load_missing", function() { return terr(function() { return loadfile("tmp-c2/definitely_missing_xyz.cnut"); }); });
t("e_load_garbage", function() { return terr(function() { local f = file("tmp-c2/sershape_garbage.cnut", "wb"); f.writen(1, 'b'); f.close(); try { local g = loadfile("tmp-c2/sershape_garbage.cnut"); return "loaded:" + typeof g; } catch (e) { return "E:" + e; } }); });
t("e_wcf_native_badpath", function() { return terr(function() { writeclosuretofile("no_such_dir_zz/x.cnut", print); return "no-error"; }); });
t("e_wcf_int_badpath", function() { return terr(function() { writeclosuretofile("no_such_dir_zz/x.cnut", 1); return "no-error"; }); });
t("e_wcf_native_leftover", function() { return terr(function() { writeclosuretofile("tmp-c2/sershape_leftover.cnut", print); return "no-error"; }); });
t("e_wcf_native_leftover_check", function() { return terr(function() { local n = null; try { local fh = file("tmp-c2/sershape_leftover.cnut", "rb"); n = fh.len(); fh.close(); } catch (e) { n = "E:" + e; } try { remove("tmp-c2/sershape_leftover.cnut"); } catch (e) {} return "leftover=" + show(n); }); });
t("e_wcf_freevar_leftover", function() { return terr(function() { local fv = 5; local bad = function() { return fv; }; writeclosuretofile("tmp-c2/sershape_leftover2.cnut", bad); return "no-error"; }); });
t("e_wcf_freevar_leftover_check", function() { return terr(function() { local n = null; try { local fh = file("tmp-c2/sershape_leftover2.cnut", "rb"); n = fh.len(); fh.close(); } catch (e) { n = "E:" + e; } try { remove("tmp-c2/sershape_leftover2.cnut"); } catch (e) {} return "leftover=" + show(n); }); });
try { remove(PATH); } catch (e) {}
try { remove("tmp-c2/sershape_garbage.cnut"); } catch (e) {}
try { remove("tmp-c2/sershape_leftover.cnut"); } catch (e) {}
try { remove("tmp-c2/sershape_leftover2.cnut"); } catch (e) {}
print("SERMAP-DONE\n");

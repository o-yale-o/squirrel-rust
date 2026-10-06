// 自动生成：表键身份 + 循环变量闭包捕获矩阵（tools/gen_keymatrix.py）
//
// **表键严格按类型区分**：int/float/string/bool 是不同的槽（1.0 读不到 1 写的值），
// null 不能当键；数组下标做 tointeger 转换（a[1.9] → a[1]）；普通 `=` 不建新槽。
// **循环变量的闭包捕获**：for/foreach 的循环变量共享同一槽（闭包看到最终值），
// 循环体内 `local` 才逐次独立。两侧逐字节一致。
function d(x) {
    local t = typeof x;
    if (t == "table") { local o = "{"; local f = true; foreach (k, v in x) { if (!f) o += ","; f = false; o += "" + k + "=" + v; } return o + "}"; }
    if (t == "array") { local o = "["; for (local i = 0; i < x.len(); i++) { if (i) o += ","; o += "" + x[i]; } return o + "]"; }
    return "" + x;
}
function t(tag, f) {
    try { local v = f(); print(tag + "=" + d(v) + "|" + typeof v + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

t("k_int_float", function() { local x = {}; x[1] <- "i"; return x[1.0]; });
t("k_float_int", function() { local x = {}; x[1.0] <- "f"; return x[1]; });
t("k_str_vs_int", function() { local x = {}; x["1"] <- "s"; x[1] <- "i"; return d(x) + "/" + ("1" in x) + "/" + (1 in x); });
t("k_neg_zero", function() { local x = {}; x[-0.0] <- "nz"; return x[0.0] + "/" + d(x); });
t("k_zero_float", function() { local x = {}; x[0.0] <- "zf"; return x.len(); });
t("k_bool", function() { local x = {}; x[true] <- "T"; return x[true] + "/" + x.len(); });
t("k_null", function() { local x = {}; x[null] <- "N"; return x[null]; });
t("k_bool_vs_int", function() { local x = {}; x[true] <- "T"; return x[1] == null; });
t("k_float_frac", function() { local x = {}; x[1.5] <- "f"; return (1 in x) + "/" + x[1.5] + "/" + x.len(); });
t("k_big_float", function() { local x = {}; x[1e15] <- "big"; return x[1e15] + "/" + x.len(); });
t("k_delete_float", function() { local x = {}; x[1.0] <- "f"; return "" + (delete x[1.0]) + "/" + x.len(); });
t("k_overwrite_eq", function() { local x = {}; x[1] <- "first"; x[1] = "second"; return d(x); });
t("plain_eq_int", function() { local x = {}; x[1] = "i"; return d(x); });
t("plain_eq_str", function() { local x = {}; x.s = "v"; return d(x); });
t("a_float_idx", function() { local a = [10, 20, 30]; return a[1.0]; });
t("a_float_frac_idx", function() { local a = [10, 20, 30]; return a[1.9]; });
t("a_str_idx", function() { local a = [10, 20, 30]; return a["1"]; });
t("a_neg_zero_idx", function() { local a = [10, 20, 30]; return a[-0.0]; });
t("a_set_float", function() { local a = [10, 20, 30]; a[1.0] = 99; return d(a); });
t("a_set_frac", function() { local a = [10, 20, 30]; a[1.9] = 99; return d(a); });
t("a_neg_idx_set", function() { local a = [10, 20, 30]; a[-1] = 99; return d(a); });
t("iter_mixed", function() { local x = {}; x[1] <- "i"; x["s"] <- "s"; x[2.5] <- "f"; x[true] <- "b"; local o = ""; foreach (k, v in x) o += typeof k + ";"; return o; });
t("iter_keys_types", function() { local x = {}; x[1] <- "i"; x["s"] <- "s"; local ks = x.keys(); local o = ""; foreach (k in ks) o += typeof k + ";"; return o; });
t("cap_for", function() { local fs = []; for (local i = 0; i < 3; i++) fs.append(function() { return i; }); local s = ""; foreach (f in fs) s += f() + ","; return s; });
t("cap_foreach", function() { local fs = []; foreach (v in [10, 20, 30]) fs.append(function() { return v; }); local s = ""; foreach (f in fs) s += f() + ","; return s; });
t("cap_while", function() { local fs = []; local i = 0; while (i < 3) { local j = i; fs.append(function() { return j; }); i++; } local s = ""; foreach (f in fs) s += f() + ","; return s; });
t("cap_for_local_in_body", function() { local fs = []; for (local i = 0; i < 3; i++) { local j = i * 10; fs.append(function() { return j; }); } local s = ""; foreach (f in fs) s += f() + ","; return s; });
t("cap_generator", function() { local fs = []; for (local i = 0; i < 3; i++) fs.append((function() { yield i; })()); local s = ""; foreach (g in fs) { resume g; s += resume g; s += ","; } return s; });
t("cap_two_shared", function() { local fs = []; for (local i = 0; i < 2; i++) { fs.append(function() { return i; }); fs.append(function() { i = i + 100; return i; }); } local s = ""; foreach (f in fs) s += f() + ","; return s; });
t("cap_delegate_fn", function() { local fs = []; local tbl = { n = 0 }; for (local i = 0; i < 2; i++) { tbl.n = i; fs.append(function() { return tbl.n; }); } local s = ""; foreach (f in fs) s += f() + ","; return s; });
print("KEYMAP-DONE\n");

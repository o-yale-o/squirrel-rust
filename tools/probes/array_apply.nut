// array.apply 的返回值与就地修改
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
s("ret_type", function() { local a = [3, 1, 2]; local r = a.apply(@(x, y) x <=> y); return typeof r; });
s("ret_self", function() { local a = [3, 1, 2]; local r = a.apply(@(x, y) x <=> y); return r == a; });
s("arr_after", function() { local a = [3, 1, 2]; a.apply(@(x, y) x <=> y); return a[0] + "," + a[1] + "," + a[2]; });
s("ret_val", function() { local a = [3, 1, 2]; return "" + a.apply(@(x, y) x <=> y); });
s("argcount", function() { local a = [7]; a.apply(function(...) { print("args=" + vargv.len() + " "); return 0; }); return "ok"; });
s("map_ret", function() { local a = [1, 2]; return "" + a.map(@(v) v * 2); });
s("filter_ret", function() { local a = [1, 2, 3]; return "" + a.filter(@(v) v > 1); });

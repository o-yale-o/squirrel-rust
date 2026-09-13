// 定向补测：之前未覆盖的角落
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

// _delslot 元方法
t("mm_delslot", function() {
    local log = "";
    class D { function _delslot(k) { log += "del:" + k; return "handled"; } }
    local d = D();
    d.x <- 1;
    delete d.x;
    return log;
});
t("mm_delslot_missing", function() { class D { function _delslot(k) { return null; } } local d = D(); delete d.nope; return "ok"; });

// native 链中的 suspend 限制
t("suspend_in_native", function() {
    local th = newthread(function() { ::print(""); suspend(1); return 1; });
    th.call();
    return th.getstatus();
});
t("suspend_plain", function() { suspend(1); return "resumed"; });

// debug hook 的 fn 参数与多编译单元
t("debughook_fn", function() {
    local names = "";
    setdebughook(function (event, source, line, fn) { if (event == 'c' || event == 'r') names += (typeof fn) + ","; });
    local helper = function () { return 1; };
    helper();
    setdebughook(null);
    return names;
});
t("debughook_named", function() {
    local seen = "";
    setdebughook(function (event, source, line, fn) { if (event == 'c' && typeof fn == "string") seen += fn + "|"; });
    function top() { return 1; }
    top();
    setdebughook(null);
    return seen;
});
t("debughook_lines", function() {
    local n = 0;
    setdebughook(function (event, source, line, fn) { if (event == 'l') n++; });
    local a = 1;
    local b = 2;
    setdebughook(null);
    return (n > 0);
});

// getstackinfos 的 func 命名
t("stkinfo_named", function() {
    function named() { local i = getstackinfos(1); return i.func; }
    return named();
});
t("stkinfo_method", function() {
    class A { function m() { local i = getstackinfos(1); return i.func; } }
    return A().m();
});
t("stkinfo_method_called_plain", function() {
    class A { function m() { local i = getstackinfos(1); return i.func; } }
    local f = A().m;
    return f();
});
t("stkinfo_native_caller", function() {
    local out = "";
    local f = function() { local i = getstackinfos(1); out = i.src + ":" + i.line; };
    f();
    return (out.find("NATIVE") >= 0) + ":" + out.len();
});
t("stkinfo_lambda", function() {
    local l = function () { local i = getstackinfos(1); return i.func; };
    return l();
});

// 文本模式文件 I/O 与其它 file 细节
t("file_text_mode", function() {
    local f = file("robust_text.txt", "w");
    f.writen(65, 'b'); f.writen(10, 'b'); f.writen(66, 'b');
    f.close();
    local g = file("robust_text.txt", "rb");
    local n = g.len();
    g.close(); remove("robust_text.txt");
    return n;
});
t("file_read_str", function() {
    local f = file("robust_text2.txt", "wb");
    foreach (c in "hello") f.writen(c, 'b');
    f.close();
    local g = file("robust_text2.txt", "rb");
    local b = blob(5);
    local r = g.readblob(5);
    g.close(); remove("robust_text2.txt");
    return r.len();
});
t("file_seek_modes", function() {
    local f = file("robust_text3.txt", "wb+");
    foreach (c in "abcdef") f.writen(c, 'b');
    f.seek(-2, 'e');
    local c = f.readn('b');
    f.close(); remove("robust_text3.txt");
    return c;
});
t("file_double_close", function() { local f = file("robust_text4.txt", "wb+"); f.close(); f.close(); return "ok"; });

// 其它未测语义
t("attributes_inherit", function() {
    class A { x = 1; }
    class B extends A { y = 2; }
    return typeof B.getattributes();
});
t("static_after_instance", function() {
    class A { static s = 1; }
    local a = A();
    return a.s;
});
t("enumerate_enum_typeof", function() {
    // 枚举名只能以 `E.MEMBER` 形式出现：裸写 `E` 两边都报 "expected '.'"
    enum E { X, Y = 5 }
    return E.X + ":" + E.Y + ":" + typeof E.X;
});
t("weakref_after_clone", function() { local o = { a = 1 }; local c = clone o; local w = o.weakref(); o = null; collectgarbage(); return typeof w.ref(); });
t("array_nested_deep_tostring", function() { local a = [1, [2, [3, [4]]]]; return "" + a.len(); });
t("table_self_ref_foreach", function() { local o = {}; o.self <- o; local n = 0; foreach (k, v in o) n++; return n; });
t("closure_recursion_limit", function() { local f; f = function (n) { return n <= 0 ? 0 : f(n - 1) + 1; }; return f(100); });
t("getroottable_set_get", function() { local rt = getroottable(); rt.marker <- 1; return getroottable().marker; });

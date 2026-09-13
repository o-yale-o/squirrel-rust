// 生成器与 GC 的稳定性探针：参考实现在「未启动的生成器」上是否有析构问题
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
local mode = vargv.len() > 0 ? vargv[0] : "mu";
if (mode == "mu" || mode == "gen") {
    for (local i = 0; i < 2000; i++) {
        local g = (function() { yield 1; })();
    }
    print("created 2000 suspended\n");
}
if (mode == "mu" || mode == "gc") {
    for (local i = 0; i < 500; i++) {
        local g = (function() { yield 1; })();
        g = null;
        collectgarbage();
    }
    print("gc 500\n");
}
if (mode == "mu" || mode == "wk") {
    for (local i = 0; i < 500; i++) {
        s("wk" + i, function() { return ((function() { yield 1; })()).weakref(0); });
    }
    print("weakref-with-arg 500\n");
}
if (mode == "mu" || mode == "w0") {
    for (local i = 0; i < 500; i++) {
        local w = ((function() { yield 1; })()).weakref();
    }
    print("weakref 500\n");
}
print("PROBE-DONE\n");

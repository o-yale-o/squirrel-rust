// 递归路径的健壮性探针（**有界**版本，两侧可对拍）。
//
// 背景：参考实现的 EnterFrame 没有栈上限，非尾递归的无限递归会一直涨到进程被杀
// （实测一条 3 行脚本把宿主吃到 32GB）。本实现封顶报可捕获的 stack overflow。
// 但正因如此，**无界的递归用例没法放进差分探针**——参考实现会先 OOM；
// 无界版本放在 Rust 侧守门：crates/squirrel-stdlib/tests/robustness.rs 的
// `runaway_recursion_is_catchable_and_bounded`。
//
// 这份探针覆盖各条"能递归"的路径，深度有界：
// 用法：python tools/line_diff.py tools/probes/recursion_paths.nut
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

// 1) 通过 _call 元方法自递归（撞 native 调用深度上限，两边都很快报错）
s("mt_call", function() {
    local t = {};
    t.setdelegate({ _call = function(...) { return t(); } });
    return t();
});

// 2) 直接自递归，深度有界
s("direct", function() {
    local f;
    f = function(n) { if (n <= 0) return 0; local x = f(n - 1); return x + 1; };
    return f(5000);
});

// 3) 互递归，深度有界
s("mutual", function() {
    local f, g;
    f = function(n) { if (n <= 0) return "mutual-ok"; return g(n - 1); };
    g = function(n) { return f(n); };
    return f(5000);
});

// 4) 尾递归：帧复用，深度可以很大
s("tail", function() {
    local f;
    f = function(n) { if (n <= 0) return "tail-ok"; return f(n - 1); };
    return f(200000);
});

// 5) 生成器 resume
s("gen", function() {
    local mk = function() { yield 1; yield 2; };
    local g = mk();
    local a = resume g;
    local b = resume g;
    return "" + a + "," + b;
});

// 6) 深栈上取调用栈信息（不许越界/panic）
s("deep_stackinfo", function() {
    local f;
    f = function(n) {
        if (n <= 0) return typeof getstackinfos(3);
        local x = f(n - 1);
        return x;
    };
    return f(2000);
});

print("RECUR-PATHS-DONE\n");

// 固定的字节码夹具源码：覆盖闭包 / outer 值 / 循环里捕获局部变量。
// 两个引擎编译出的 .cnut 会被 tests/bytecode_interop.rs 直接执行（含参考实现产物）。
function mk(n) {
    local acc = n;
    return function(x) { acc += x; return acc; };
}
local f = mk(10);
print(f(1) + "," + f(2) + "\n");
local g = mk(100);
print(g(5) + "\n");
function outer2() {
    local a = 1, b = 2;
    return function() { return function() { return a + b; }; };
}
print(outer2()()() + "\n");
local t = {};
foreach (i in [1, 2, 3]) {
    local c = i * 2;
    t["k" + i] <- function() { return c; };
}
print(t.k1() + "," + t.k3() + "\n");

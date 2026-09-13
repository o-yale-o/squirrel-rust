// 错误钩子的 LOCALS 全类型打印（↔ sqstd_printcallstack）
function deep() {
    local i = 42;
    local neg = -7;
    local big = 5000000000;
    local negbig = -3000000000;
    local f = 0.1;
    local f2 = 1.0 / 3.0;
    local s = "str";
    local b = true;
    local bf = false;
    local t = { k = 1 };
    local a = [1, 2];
    local g = function() {};
    local nf = print;
    local gen = function() { yield 1; }();
    local ud = blob(4);
    local th = newthread(function() {});
    local w = t.weakref();
    class C {}
    local c = C;
    local inst = C();
    local nul = null;
    local up = 1;
    local clo = function() { return up; };
    throw "boom";
}
deep();

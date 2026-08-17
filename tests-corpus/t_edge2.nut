// 多层嵌套闭包捕获
function outer() {
    local a = 1;
    function middle() {
        local b = 2;
        function inner() {
            return a + b;
        }
        return inner;
    }
    return middle();
}
print(outer()() + "\n");

// 循环中的闭包
local fns = [];
for(local i = 0; i < 3; i++) {
    fns.append(function() { return i; });
}
foreach(f in fns) print(f());
print("\n");

// 生成器内的闭包
function gen() {
    local n = 10;
    yield function() { return n; };
    n = 20;
    yield function() { return n; };
}
local g = gen();
local f1 = resume g;
local f2 = resume g;
print(f1() + " " + f2() + "\n");

// 类方法互调与 this
class C {
    v = 5;
    function double() { return v * 2; }
    function quad() { return double() * 2; }
    function setv(nv) { v = nv; return this; }
}
local c = C();
print(c.quad() + "\n");
print(c.setv(10).quad() + "\n");

local f = @(x) x * 2;
print(f(21) + "\n");
local add = @(a, b) a + b;
print(add(1, 2) + "\n");
function kw(a, b) { return a + "-" + b; }
print(kw(1, 2) + "\n");
// 具名参数块：{ key = value } 是 SET 到**调用返回值**上（↔ FunctionCallArgs 的 '{' 分支），
// 所以必须配合返回实例/表的调用；写成 print(kw(1) { b = 2 }) 两边都会报错。
class Named { a = 0; b = 0; }
local named = Named() { a = 1, b = 2 };
print(named.a + "-" + named.b + "\n");
local env = {};
env.v <- 10;
local src = function() { return v; };
local bound = src.bindenv(env);
print(bound() + "\n");

local f = @(x) x * 2;
print(f(21) + "\n");
local add = @(a, b) a + b;
print(add(1, 2) + "\n");
function kw(a, b) { return a + "-" + b; }
print(kw(1) { b = 2 } + "\n");
local env = {};
env.v <- 10;
local src = function() { return v; };
local bound = src.bindenv(env);
print(bound() + "\n");

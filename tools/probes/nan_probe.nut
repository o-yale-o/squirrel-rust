// NaN 符号与 fmod 语义探针（两个引擎对拍）
function rt(x) { return x; }
local zero = rt(0.0);
local nzero = rt(-0.0);
local one = rt(1.0);
local none = rt(-1.0);
local twofive = rt(2.5);
local inf = rt(1.0) / zero;
local ninf = rt(-1.0) / zero;
local qnan = zero / zero;
local vals = [zero, nzero, one, none, twofive, inf, ninf, qnan];
local names = ["0", "-0", "1", "-1", "2.5", "inf", "-inf", "nan"];
for (local i = 0; i < vals.len(); i++) {
    for (local j = 0; j < vals.len(); j++) {
        local r = "err";
        try { r = format("%g", vals[i] % vals[j]); } catch (e) { r = "E:" + e; }
        print("mod " + names[i] + " " + names[j] + " = " + r + "\n");
    }
}
for (local i = 0; i < vals.len(); i++) {
    for (local j = 0; j < vals.len(); j++) {
        local r = "err";
        try { r = format("%g", vals[i] / vals[j]); } catch (e) { r = "E:" + e; }
        print("div " + names[i] + " " + names[j] + " = " + r + "\n");
    }
}
for (local i = 0; i < vals.len(); i++) {
    local r = "err";
    try { r = format("%g", -vals[i]); } catch (e) { r = "E:" + e; }
    local r2 = "err";
    try { r2 = format("%d", (vals[i] < 0).tointeger() + (vals[i] <= 0).tointeger() * 10); } catch (e) { r2 = "E:" + e; }
    print("unm " + names[i] + " = " + r + " flags=" + r2 + "\n");
}
// 字面量常量折叠路径
print("fold mod = " + format("%g", 1.0 % 0.0) + "\n");
print("fold div = " + format("%g", 0.0 / 0.0) + "\n");
print("fold neg = " + format("%g", -(0.0 / 0.0)) + "\n");
print("str nan = " + (0.0 / 0.0) + "\n");
print("str -nan = " + (-(0.0 / 0.0)) + "\n");
print("str fmod = " + (1.0 % 0.0) + "\n");
function tconv(s) { try { return format("%g", s.tofloat()); } catch (e) { return "E:" + e; } }
print("tofloat nan = " + tconv("nan") + "\n");
print("tofloat -nan = " + tconv("-nan") + "\n");
print("tofloat inf = " + tconv("inf") + "\n");
print("tofloat 1e999 = " + tconv("1e999") + "\n");

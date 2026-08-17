# -*- coding: utf-8 -*-
"""生成边界差分测试语料（tests-corpus/*.nut）。"""
import io
import os

OUT = r"D:\OpenSource\squirrel-rust\tests-corpus"

cases = {}

cases["t_float.nut"] = '''\
print(0.1 + "\\n");
print(1.0/3.0 + "\\n");
print(-1.0/3.0 + "\\n");
print(100000.0 + "\\n");
print(1000000.0 + "\\n");
print(999999.0 + "\\n");
print(0.0001 + "\\n");
print(0.00001 + "\\n");
print(1.5e300 + "\\n");
print(-0.5 + "\\n");
print(3.14159265358979 + "\\n");
print(1.0 + "\\n");
print(0.0 + "\\n");
local f = 2.5; print(f*f + "\\n");
'''

cases["t_int.nut"] = '''\
print(9223372036854775807 + 1); print("\\n");
print(-9223372036854775807 - 2); print("\\n");
print(7 / 2); print(" ");
print(-7 / 2); print(" ");
print(7 % 3); print(" ");
print(-7 % 3); print("\\n");
print(1 << 10); print(" ");
print(-8 >> 1); print(" ");
print(-1 >>> 60); print(" ");
print(0xFF ^ 0x0F); print(" ");
print(~0); print("\\n");
'''

cases["t_bit.nut"] = '''\
print(1 << 63); print("\\n");
print(255 & 15); print(" ");
print(128 | 7); print("\\n");
print(3 & 5.0);
'''

cases["t_format.nut"] = '''\
print(format("%d %i %s", 42, -7, "str") + "\\n");
print(format("%5d|%-5d|", 42, 42) + "\\n");
print(format("%05d", 42) + "\\n");
print(format("%x %X %o", 255, 255, 8) + "\\n");
print(format("%f", 3.14159) + "\\n");
print(format("%.2f", 3.14159) + "\\n");
print(format("%e", 12345.678) + "\\n");
print(format("%g", 0.00001234) + "\\n");
print(format("%c%c%c", 72, 105, 33) + "\\n");
print(format("%.3s", "abcdef") + "\\n");
printf("%s=%d\\n", "x", 99);
'''

cases["t_err1.nut"] = '''\
local t = {};
try { local x = t.missing; } catch(e) { print(e + "\\n"); }
local a = [1,2,3];
try { local x = a[10]; } catch(e) { print(e + "\\n"); }
try { local x = a["s"]; } catch(e) { print(e + "\\n"); }
local s = "abc";
try { local x = s[5]; } catch(e) { print(e + "\\n"); }
'''

cases["t_err2.nut"] = '''\
try { print({} + 1); } catch(e) { print(e + "\\n"); }
try { print("a" - 1); } catch(e) { print(e + "\\n"); }
try { print(1 / 0); } catch(e) { print(e + "\\n"); }
try { print(1 % 0); } catch(e) { print(e + "\\n"); }
try { print(-{}); } catch(e) { print(e + "\\n"); }
try { print(~1.5); } catch(e) { print(e + "\\n"); }
'''

cases["t_err3.nut"] = '''\
try { local x = 1; x(); } catch(e) { print(e + "\\n"); }
try { local x = "s"; x(); } catch(e) { print(e + "\\n"); }
function f(a, b) { return a+b; }
try { f(1); } catch(e) { print(e + "\\n"); }
try { f(1,2,3); } catch(e) { print(e + "\\n"); }
function g(a, ...) {}
try { g(); } catch(e) { print(e + "\\n"); }
'''

cases["t_deleg.nut"] = '''\
local base = { greet = function() { return "hello"; }, val = 10 };
local d = {};
d.setdelegate(base);
print(d.greet() + "\\n");
print(d.val + "\\n");
d.val = 20;
print(base.val + "\\n");
print(d.rawin("val") + " ");
print(d.rawin("greet") + "\\n");
print(d.getdelegate() == base);
'''

cases["t_meta.nut"] = '''\
class V {
    x = 0; y = 0;
    constructor(a, b) { x = a; y = b; }
    function _add(o) { return V(x + o.x, y + o.y); }
    function _sub(o) { return V(x - o.x, y - o.y); }
    function _mul(o) { return V(x * o, y * o); }
    function _div(o) { return V(x / o, y / o); }
    function _modulo(o) { return V(x % o, y % o); }
    function _unm() { return V(-x, -y); }
    function _tostring() { return "(" + x + "," + y + ")"; }
    function _cmp(o) { if (x < o.x) return -1; if (x > o.x) return 1; return 0; }
    function _typeof() { return "V"; }
}
local a = V(1,2), b = V(3,4);
print((a + b).tostring() + "\\n");
print((b - a).tostring() + "\\n");
print((a * 3).tostring() + "\\n");
print((b / 2).tostring() + "\\n");
print((-a).tostring() + "\\n");
print(a < b); print(" ");
print(a > b); print("\\n");
print(typeof a); print("\\n");
print(a <=> b); print("\\n");
'''

cases["t_meta2.nut"] = '''\
local proxy = {};
proxy.setdelegate({
    function _get(k) { return "got:" + k; }
    function _set(k, v) { ::print("set " + k + "=" + v + "\\n"); }
});
print(proxy.anything + "\\n");
proxy.foo = 5;
'''

cases["t_gen.nut"] = '''\
function g() {
    yield 1;
    try {
        yield 2;
    } catch(e) {
        yield "caught:" + e;
    }
    return "end";
}
local gen = g();
print(resume gen); print(" ");
print(resume gen); print(" ");
print(resume gen); print("\\n");
print(gen.getstatus() + "\\n");
try { resume gen; } catch(e) { print(e + "\\n"); }
'''

cases["t_clone.nut"] = '''\
local t = { a = 1, b = [1,2,3] };
local c = clone(t);
c.a = 99;
print(t.a + " " + c.a + "\\n");
local a2 = clone(t.b);
a2.append(4);
print(t.b.len() + " " + a2.len() + "\\n");
t.c <- 5;
print("c" in t); print(" ");
print("z" in t); print("\\n");
delete t.c;
print("c" in t); print("\\n");
print(delete t.a + "\\n");
'''

cases["t_try2.nut"] = '''\
for(local i = 0; i < 3; i++) {
    try {
        if (i == 1) continue;
        if (i == 2) break;
        print("i=" + i + "\\n");
    } catch(e) { print("c"); }
}
print("---\\n");
try {
    try {
        throw "inner";
    } catch(e) {
        throw "outer:" + e;
    }
} catch(e2) { print(e2 + "\\n"); }
'''

cases["t_order.nut"] = '''\
local t = {};
for(local i = 0; i < 50; i++) t["k" + i] <- i;
foreach(k, v in t) print(k + "=" + v + " ");
print("\\n");
print(t.len() + "\\n");
local keys = t.keys();
foreach(k in keys) print(k + ",");
print("\\n");
'''

cases["t_str.nut"] = '''\
print("".len()); print("\\n");
print("hello"[0] + " " + "hello"[-1] + "\\n");
print("hello world".slice(-5) + "\\n");
print("hello".slice(1, 3) + "\\n");
print("aaa".find("aa") + " ");
print("aaa".find("aa", 1) + " ");
print("aaa".find("b") + "\\n");
print("Hello".tolower() + " " + "Hello".toupper() + "\\n");
print("12abc".tointeger() + "\\n");
print("0x1A".tointeger(16) + "\\n");
print("3.14abc".tofloat() + "\\n");
try { "abc".slice(2, 1); } catch(e) { print(e + "\\n"); }
'''

cases["t_lambda.nut"] = '''\
local f = @(x) x * 2;
print(f(21) + "\\n");
local add = @(a, b) a + b;
print(add(1, 2) + "\\n");
function kw(a, b) { return a + "-" + b; }
print(kw(1) { b = 2 } + "\\n");
local env = { v = 10 };
local bound = function[](...) { return v; }.bindenv(env);
print(bound() + "\\n");
'''

cases["t_const.nut"] = '''\
const MAX = 100;
const NAME = "test";
const NEG = -5;
enum Color { RED, GREEN = 5, BLUE }
print(MAX + " " + NAME + " " + NEG + "\\n");
print(Color.RED + " " + Color.GREEN + " " + Color.BLUE + "\\n");
print(MAX * 2 + "\\n");
'''

cases["t_expr.nut"] = '''\
local x = 5;
x += 3; print(x + " ");
x -= 2; print(x + " ");
x *= 2; print(x + " ");
x /= 3; print(x + " ");
x %= 4; print(x + "\\n");
local i = 5;
print(i++ + " ");
print(i + " ");
print(++i + " ");
print(i-- + " ");
print(i + "\\n");
local t = { n = 1 };
t.n += 10; print(t.n + "\\n");
local a = [1];
a[0] += 5; print(a[0] + "\\n");
'''

cases["t_deep.nut"] = '''\
function rec(n) { return rec(n + 1); }
try { rec(0); } catch(e) { print(e + "\\n"); }
print("after\\n");
'''

cases["t_static.nut"] = '''\
class C {
    </ a = "attr" /> static version = 1;
    </ doc = "field" /> x = 5;
    </ m = "m1" /> function foo() { return version; }
}
print(C.version + "\\n");
local attrs = C.getattributes("x");
print(attrs.doc + "\\n");
local ma = C.getattributes("foo");
print(ma.m + "\\n");
local i = C();
print(i.foo() + "\\n");
'''

cases["t_gc.nut"] = '''\
for(local i = 0; i < 10; i++) {
    local t = {};
    t.self <- t;
    t.arr <- [t, {x = t}];
}
local n = collectgarbage();
print(n >= 0 ? "gc-ok" : "gc-fail");
print("\\n");
'''

cases["t_weak.nut"] = '''\
local t = { v = 1 };
local w = t.weakref();
print(typeof w + " ");
print(w.ref().v + "\\n");
t = null;
collectgarbage();
print(w.ref() == null ? "dead" : "alive");
print("\\n");
'''

for name, body in cases.items():
    io.open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n").write(body)
print("wrote", len(cases), "cases")

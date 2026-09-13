// getstackinfos（↔ base_getstackinfos + sq_getlocal）：func/src/line + locals 顺序与作用域
function dump(infos) {
    local s = "";
    foreach(k, v in infos.locals) {
        s += k + "=";
        if (typeof v == "table") s += "{table}";
        else if (typeof v == "string") s += "\"" + v + "\"";
        else if (typeof v == "function") s += "{func}";
        else s += v;
        s += " ";
    }
    return s;
}

// 1. 基本：形参 + 声明序 + this 在最后
function inner(a, b) {
    local x = a + b;
    local s = "str";
    local t = { k = 1 };
    local infos = getstackinfos(1);
    print(dump(infos) + "\n");
    print("func=" + infos.func + " line=" + infos.line + "\n");
    return x;
}
function outer() {
    local o = 42;
    return inner(o, o + 1);
}
print(outer() + "\n");

// 2. 原生帧（level 0）没有任何局部变量
function nativeFrame() {
    local infos = getstackinfos(0);
    local n = 0;
    foreach(k, v in infos.locals) n++;
    return infos.func + "/" + infos.src + "/" + infos.line + "/n=" + n;
}
print(nativeFrame() + "\n");

// 3. outer 变量排在局部变量之前
function makeClosure() {
    local captured = 7;
    local unused = 1;
    return function() {
        local inner_local = captured;
        local infos = getstackinfos(1);
        return dump(infos);
    };
}
print(makeClosure()() + "\n");

// 4. 已离开作用域的局部变量被过滤
function scopeEnd() {
    {
        local gone = 1;
    }
    local infos = getstackinfos(1);
    return dump(infos);
}
print(scopeEnd() + "\n");

// 5. 越界 level → null（负 level 在 C++ 侧是越界读 UB，不作为语料）
function levelOOB() {
    local a = getstackinfos(99);
    local b = getstackinfos(3);
    return (a == null) + "/" + (b == null);
}
print(levelOOB() + "\n");

// 6. main 帧（vargv/this）
function mainFrame() {
    local infos = getstackinfos(2);
    if (infos == null) return "null";
    local s = "";
    foreach(k, v in infos.locals) s += k + ":" + typeof v + " ";
    return infos.func + " " + s;
}
print(mainFrame() + "\n");

// 7. 生成器内
function genFrame() {
    local g = function() {
        local gv = 5;
        local infos = getstackinfos(1);
        yield dump(infos);
        return null;
    }();
    local out = "";
    foreach(v in g) out += v;
    return out;
}
print(genFrame() + "\n");

// 8. 线程的 getstackinfos + wakeup 恢复
local th = newthread(function() {
    local tv = 11;
    local s = suspend("s");
    return tv + "|" + s;
});
th.call();
print("state1=" + th.getstatus() + "\n");
local tinfo = th.getstackinfos(0);
print(typeof tinfo + " " + tinfo.func + " " + tinfo.line + "\n");
print("wake=" + th.wakeup("w") + "\n");
print("state2=" + th.getstatus() + "\n");

// 9. 异常路径仍可用（catch 内取栈信息）
try { throw "err"; } catch(e) { print("caught " + e + "\n"); }

// 微基准：`sq bench.nut <case>`（两个引擎跑同一个文件，保证代码完全一致）
//
// 计时用脚本内的 `clock()`（进程 CPU 秒，两个引擎语义一致，排除进程启动与
// stdout 缓冲差异）；外层再用墙钟复核。循环内不打印，避免 I/O 影响。
// 注意：`case` 是保留字，别拿它当变量名。

function bench(name, f) {
    local t0 = clock();
    local r = f();
    local dt = clock() - t0;
    print(name + "\t" + dt + "\t" + r + "\n");
}

function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
function mk_counter() { local n = 0; return function() { n = n + 1; return n; }; }
function mk_gen() { return (function() { for (local i = 0; i < 400; i++) yield i; })(); }
local METHOD_CLASS = class { v = 0; function inc(n) { v = v + n; return v; } };
local META_CLASS = class { function _get(k) { return 1; } };

local which = vargv.len() > 0 ? vargv[0] : "fib";

if (which == "fib") {
    bench("fib", function() { return fib(32); });
} else if (which == "loop") {
    bench("loop", function() {
        local s = 0;
        for (local i = 0; i < 8000000; i++) s = s + i % 7 - 3;
        return s;
    });
} else if (which == "table") {
    bench("table", function() {
        local t = {};
        for (local i = 0; i < 1000000; i++) t[i] <- i;
        local s = 0;
        for (local i = 0; i < 1000000; i++) s += t[i];
        return s;
    });
} else if (which == "array") {
    bench("array", function() {
        local a = [];
        for (local i = 0; i < 1000000; i++) a.append(i);
        local s = 0;
        for (local i = 0; i < 1000000; i++) s += a[i];
        return s;
    });
} else if (which == "method") {
    bench("method", function() {
        local o = METHOD_CLASS();
        for (local i = 0; i < 3000000; i++) o.inc(1);
        return o.v;
    });
} else if (which == "closure") {
    bench("closure", function() {
        local f = mk_counter();
        local s = 0;
        for (local i = 0; i < 3000000; i++) s = f();
        return s;
    });
} else if (which == "meta") {
    bench("meta", function() {
        local o = META_CLASS();
        local s = 0;
        for (local i = 0; i < 1000000; i++) s += o.nope;
        return s;
    });
} else if (which == "str") {
    bench("str", function() {
        local s = "";
        for (local i = 0; i < 80000; i++) s += "x";
        return s.len();
    });
} else if (which == "str_small") {
    // 对照组：同样 8 万次字符串创建，但每次都是短串（长度不增长）
    bench("str_small", function() {
        local s = "";
        for (local i = 0; i < 80000; i++) s = "v" + (i % 100);
        return s.len();
    });
} else if (which == "stringfind") {
    bench("stringfind", function() {
        local s = "abcdefghijklmnopqrstuvwxyz0123456789";
        local n = 0;
        for (local i = 0; i < 1000000; i++) if (s.find("xyz") != null) n++;
        return n;
    });
} else if (which == "generator") {
    bench("generator", function() {
        local n = 0;
        for (local k = 0; k < 5000; k++) {
            local g = mk_gen();
            while (g.getstatus() != "dead") { resume g; n++; }
        }
        return n;
    });
} else {
    print("unknown case: " + which + "\n");
}

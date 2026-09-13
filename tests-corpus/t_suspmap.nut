// 自动生成：python tools/gen_suspmap.py —— 请勿手改
// suspend 穿透 native/metamethod：错误文本与事后状态

// suspend 穿透 native / metamethod 的错误路径
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
function mkinner(kind) {
    if (kind == "plain") return function() { return "plain"; };
    if (kind == "suspend") return function(x) { suspend(x); return "after"; };
    if (kind == "meta") {
        local C = class { function _get(k) { suspend(1); return 1; } };
        return function() { local o = C(); return o.anything; };
    }
    if (kind == "call_meta") {
        local C = class { function _call(a) { suspend(1); return 1; } };
        return function() { local o = C(); return o(); };
    }
    if (kind == "native_cb") {
        return function() { local a = [1, 2]; local r = null; a.map(function(x) { suspend(1); return x; }); return "mapped"; };
    }
    if (kind == "each_cb") {
        return function() { local a = [1]; a.each(function(x) { suspend(1); }); return "eached"; };
    }
    if (kind == "gen") {
        return function() { local g = (function() { suspend(1); yield 1; })(); return resume g; };
    }
    if (kind == "in_try") {
        return function() { try { suspend(1); } catch (e) { return "caught:" + e; } return "no"; };
    }
    return function() { return "unknown"; };
}

// 1) 主脚本里直接 suspend：**两侧都会静默结束**（rc=1、无输出，见
// tests-corpus/t_suspend_main.nut 单独守门）——所以这里不能放，否则后面全跑不到。
// s("main_suspend", …) / s("main_suspend_in_fn", …) 见 t_suspend_main.nut

// 2) 线程里各种形态
s("thr_plain_suspend", function() { local t = newthread(mkinner("suspend")); return "" + t.call(1) + "/" + t.getstatus(); });
s("thr_meta_suspend", function() { local t = newthread(mkinner("meta")); try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("thr_callmeta_suspend", function() { local t = newthread(mkinner("call_meta")); try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("thr_native_cb_suspend", function() { local t = newthread(mkinner("native_cb")); try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("thr_each_cb_suspend", function() { local t = newthread(mkinner("each_cb")); try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("thr_gen_suspend", function() { local t = newthread(mkinner("gen")); try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("thr_try_suspend", function() { local t = newthread(mkinner("in_try")); try { return "" + t.call(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });
s("thr_suspend_nested_fn", function() {
    local t = newthread(function() { local inner = function() { return suspend("deep"); }; return "r:" + inner(); });
    return "" + t.call() + "/" + t.getstatus();
});

// 3) suspend 后再 suspend（同一线程重复挂起）
s("thr_suspend_twice_seq", function() {
    local t = newthread(function() { local a = suspend(1); local b = suspend(2); return "" + a + b; });
    local r1 = t.call();
    local r2 = t.wakeup("A");
    local r3 = t.wakeup("B");
    return "" + r1 + "/" + r2 + "/" + r3 + "/" + t.getstatus();
});
s("thr_suspend_after_wakeup_error", function() {
    local t = newthread(function() { suspend(1); throw "after-wake"; });
    t.call();
    try { return "" + t.wakeup("x"); } catch (e) { return "E:" + e + "/" + t.getstatus(); }
});

// 4) 嵌套线程：外层线程调内层线程（内层可以正常挂起）
s("nested_threads_suspend", function() {
    local inner = newthread(function() { suspend("inner-suspend"); return "inner-done"; });
    local outer = newthread(function() { local r = inner.call(); return "outer:" + r; });
    local a = outer.call();
    return "" + a + "/" + inner.getstatus() + "/" + outer.getstatus();
});

// 5) 生成器里 suspend、线程里 resume
// 注：在**主 VM** 里 resume 一个会 suspend 的生成器，同样会让主 VM 挂起、
// 脚本直接结束（rc=1）——参考实现还会因为 stdout 块缓冲丢掉之前的输出，
// 所以这条也不能放进同一个文件（见 t_suspend_main.nut 的说明）。
s("thread_resume_gen", function() {
    local t = newthread(function() { local g = (function() { yield "y"; return "r"; })(); return "" + resume g + resume g; });
    return t.call();
});

// 6) 错误路径后的状态
s("status_after_native_suspend_fail", function() {
    local t = newthread(function() { local a = [1]; a.map(function(x) { suspend(1); return x; }); return "no"; });
    try { t.call(); } catch (e) { }
    return t.getstatus();
});
s("reuse_after_suspend_fail", function() {
    local t = newthread(function() { local a = [1]; a.map(function(x) { suspend(1); return x; }); return "no"; });
    try { t.call(); } catch (e) { }
    try { return "" + t.call(); } catch (e) { return "E2:" + e; }
});
s("suspend_extra_args", function() { local t = newthread(function() { suspend(1, 2, 3); return "x"; }); try { return "" + t.call(); } catch (e) { return "E:" + e; } });
s("suspend_in_ctor", function() {
    local t = newthread(function() { local C = class { constructor() { suspend(1); } }; local o = C(); return "made"; });
    try { return "" + t.call() + "/" + t.getstatus(); } catch (e) { return "E:" + e + "/" + t.getstatus(); }
});

print("SUSPMAP-DONE\n");

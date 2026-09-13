// 自动生成：python tools/gen_gcx.py —— 请勿手改
// 多线程 × GC：挂起线程/生成器持有对象、线程里 collectgarbage、跨线程 outer

// 多线程 × GC 交叉面
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
function mk() { return { v = 1 }; }

// 1) 挂起的线程持有对象时，主线程 collectgarbage
s("gc_suspended_holds", function() {
    local t = newthread(function() {
        local obj = { v = 42 };
        suspend(obj);
        return obj.v;
    });
    local kept = t.call();          // 挂起，obj 通过 suspend 值带出来
    collectgarbage();
    return t.wakeup() + "/" + kept.v + "/" + t.getstatus();
});
s("gc_suspended_weak", function() {
    local t = newthread(function() {
        local obj = { v = 7 };
        local w = obj.weakref();
        suspend(w);
        return (w.ref() == null) ? "collected" : "alive";
    });
    local w = t.call();
    collectgarbage();
    return t.wakeup() + "/" + t.getstatus();
});
s("gc_suspended_weak_local_only", function() {
    local t = newthread(function() {
        local obj = { v = 7 };
        local w = obj.weakref();
        suspend(1);
        return (w.ref() == null) ? "collected" : "alive";
    });
    t.call();
    collectgarbage();
    return t.wakeup() + "/" + t.getstatus();
});

// 2) 空闲（已结束）线程持有的对象
s("gc_finished_thread", function() {
    local t = newthread(function() { local obj = { v = 9 }; return obj; });
    local w = null;
    local obj = t.call();
    w = obj.weakref();
    obj = null;
    collectgarbage();
    return (w.ref() == null) ? "collected" : "alive";
});
s("gc_thread_dead_after_return", function() {
    local t = newthread(function() { local big = { a = [1, 2, 3] }; return "done"; });
    local w = null;
    { local tmp = { z = 1 }; w = tmp.weakref(); }
    t.call();
    collectgarbage();
    return (w.ref() == null) ? "collected" : "alive";
});

// 3) 线程里调用 collectgarbage（当前 VM 是线程）
s("gc_from_thread", function() {
    local t = newthread(function() {
        local keep = { v = 5 };
        local w = { q = 1 }.weakref();
        collectgarbage();
        return keep.v + "/" + ((w.ref() == null) ? "collected" : "alive");
    });
    return t.call();
});
s("gc_from_thread_keeps_main", function() {
    local main_obj = { v = 3 };
    local w = main_obj.weakref();
    local t = newthread(function() { collectgarbage(); return "collected-in-thread"; });
    local r = t.call();
    return r + "/" + ((w.ref() == null) ? "collected" : "alive");
});
s("gc_from_nested_thread", function() {
    local main_obj = { v = 3 };
    local w = main_obj.weakref();
    local inner = newthread(function() { collectgarbage(); return "inner"; });
    local outer = newthread(function() { return inner.call(); });
    local r = outer.call();
    return r + "/" + ((w.ref() == null) ? "collected" : "alive");
});

// 4) 挂起的生成器持有对象
s("gc_suspended_gen", function() {
    local g = (function() {
        local obj = { v = 11 };
        local w = obj.weakref();
        yield w;
        return (w.ref() == null) ? "collected" : "alive";
    })();
    local w = resume g;
    collectgarbage();
    return resume g + "/" + ((w.ref() == null) ? "collected" : "alive");
});
s("gc_gen_outer", function() {
    local obj = { v = 12 };
    local g = (function() { local w = obj.weakref(); yield 1; return (w.ref() == null) ? "collected" : "alive"; })();
    resume g;
    collectgarbage();
    return resume g;
});

// 5) 跨线程 outer 值的回收
s("gc_cross_outer", function() {
    local shared = { v = 13 };
    local t = newthread(function() { local w = shared.weakref(); suspend(1); return (w.ref() == null) ? "collected" : "alive"; });
    t.call();
    collectgarbage();
    return t.wakeup();
});
s("gc_cross_outer_lost", function() {
    local t = null;
    {
        local shared = { v = 14 };
        t = newthread(function() { local w = shared.weakref(); suspend(1); return (w.ref() == null) ? "collected" : "alive"; });
        t.call();
        shared = null;
    }
    collectgarbage();
    return t.wakeup();
});

// 6) 多线程同时活着
s("gc_many_threads", function() {
    local threads = [];
    for (local i = 0; i < 5; i++) {
        local payload = { idx = i };
        local t = newthread(function() { local w = payload.weakref(); suspend(w); return (w.ref() == null) ? "collected" : "alive"; });
        t.call();
        threads.append(t);
    }
    collectgarbage();
    local out = "";
    foreach (t in threads) out += t.wakeup() + ",";
    return out;
});

// 7) 线程栈上的对象在 collectgarbage 后仍可用（值语义）
s("gc_thread_value_alive", function() {
    local t = newthread(function() {
        local a = [1, 2, 3];
        local b = { arr = a };
        suspend(1);
        local sum = 0;
        foreach (v in b.arr) sum += v;
        return sum;
    });
    t.call();
    collectgarbage();
    return "" + t.wakeup() + "/" + t.getstatus();
});

// 8) 线程对象本身被回收
s("gc_thread_object_weak", function() {
    local w = null;
    {
        local t = newthread(function() { return 1; });
        w = t.weakref();
    }
    collectgarbage();
    return (w.ref() == null) ? "collected" : "alive";
});
s("gc_thread_suspended_weak", function() {
    local w = null;
    {
        local t = newthread(function() { suspend(1); return 1; });
        t.call();
        w = t.weakref();
    }
    collectgarbage();
    return (w.ref() == null) ? "collected" : "alive";
});

print("GCX-DONE\n");

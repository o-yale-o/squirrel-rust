# -*- coding: utf-8 -*-
"""生成「线程/生成器 **状态 × API**」矩阵语料 tests-corpus/t_threadmap.nut。

动机与 P9 的 stream 矩阵同源：差异常常藏在**状态迁移**里，而按方法逐个生成的矩阵
看不到"同一个方法在不同状态下"的行为。线程尤其如此 —— 它是协作式的状态机
（IDLE / SUSPENDED / RUNNING / DEAD），历史 bug 也集中在这一带
（线程里 collectgarbage 撞 RefCell、wakeup 无值没写 null、二次 call 的帧处理）。

观测口径：**前置状态 | 返回值 | 错误文本 | 后置状态**，四者都要。

线程委托的 7 个成员（↔ `_thread_default_delegate_funcz`）：
`call` / `wakeup` / `wakeupthrow` / `getstatus` / `weakref` / `getstackinfos` / `tostring`。
参考实现的状态检查（`thread_wakeup`）：
  - IDLE → "cannot wakeup a idle thread"
  - RUNNING → "cannot wakeup a running thread"
  - 其它（含 DEAD）没有特判，直接进 `sq_wakeupvm` —— 这一格最值得测。

安全约束：`getstackinfos(<负 level>)` 是参考实现的越界读（ASan 报 sqdebug.cpp:35
heap-buffer-overflow），**只用非负 level**；线程体一律有界（最多 suspend 一次再返回），
避免死循环把两侧都挂住。

用法：
    python tools/gen_threadmatrix.py
"""
import os
import subprocess
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_threadmap.nut")

PREAMBLE = r"""
// 自动生成：线程/生成器 状态 × API 矩阵（tools/gen_threadmatrix.py）
// 观测口径：前置状态 | 返回值 | 错误 | 后置状态
function show(r, d = 0) {
    if (d > 2) return "...";
    local t = typeof r;
    if (t == "array") { local s = "["; for (local i = 0; i < r.len(); i++) { if (i > 0) s += ","; s += show(r[i], d + 1); } return s + "]"; }
    if (t == "table") { local s = "{"; foreach (k, v in r) s += show(k, d + 1) + ":" + show(v, d + 1) + ","; return s + "}"; }
    if (t == "string") return "\"" + r + "\"";
    if (t == "float") return format("%.17g", r);
    return "" + r;
}
function st(o) { try { return o.getstatus(); } catch (e) { return "E:" + e; } }
// 已知偏离（MIGRATION.md 第 3 节）：对**已挂起**线程再 call() 时报的类型名，
// 参考实现读的是**残留栈槽** —— 实测那个槽正好还是 suspend 传入的值：
//   42→'integer'、"s"→'string'、null→'null'、[1]→'array'、{}→'table'、1.5→'float'、true→'bool'
// 我们关帧后恒为 'table'（可捕获错误）。更糟的是：残留值若是闭包，参考实现会把它
// **当成函数调用并成功**（实测 err=null），我们只报错。
// 所以这里把类型名归一化，语料只比较"是否报了 attempt to call"，不钉那个不稳定的名字。
function nerr(e) {
    if (e == null) return null;
    local s = "" + e;
    if (s.find("attempt to call '") == 0) return "attempt to call <type>";
    return s;
}
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!outer:" + e + "\n"); }
}
// 跑一次调用并汇报：前置状态 | 返回值 | 错误 | 后置状态
function go(o, f) {
    local before = st(o);
    local r = null; local err = null;
    try { r = f(o); } catch (e) { err = nerr("" + e); }
    return "before=" + before + "|r=" + show(r) + "|err=" + show(err) + "|after=" + st(o);
}

// ---- 线程状态构造器（setup 里的失败一律吞掉，保证状态一定到达）
function th_idle() { return newthread(function() { suspend("s1"); return "after"; }); }
function th_suspended() {
    local o = newthread(function() { suspend("s1"); return "after"; });
    try { o.call(); } catch (e) {}
    return o;
}
function th_suspended_val() {
    local o = newthread(function() { local v = suspend("s1"); return "got:" + v; });
    try { o.call(); } catch (e) {}
    return o;
}
function th_dead_return() {
    local o = newthread(function() { return "done"; });
    try { local x = o.call(); } catch (e) {}
    return o;
}
function th_dead_throw() {
    local o = newthread(function() { throw "boom"; });
    try { local x = o.call(); } catch (e) {}
    return o;
}
function th_suspended_nested() {
    local o = newthread(function() { return th_inner(); });
    try { o.call(); } catch (e) {}
    return o;
}
function th_inner() { suspend("nested"); return "inner"; }
function th_suspended_twice() {
    local o = newthread(function() { suspend("a"); suspend("b"); return "twice"; });
    try { o.call(); } catch (e) {}
    try { o.wakeup(); } catch (e) {}
    return o;
}
function th_dead_after_wakeup() {
    local o = newthread(function() { suspend("a"); return "end"; });
    try { o.call(); } catch (e) {}
    try { o.wakeup(); } catch (e) {}
    return o;
}

// ---- 生成器状态构造器
function gen_new() { return (function() { yield 1; yield 2; }()); }
function gen_suspended() {
    local g = (function() { yield 1; yield 2; }());
    try { resume g; } catch (e) {}
    return g;
}
function gen_dead() {
    local g = (function() { yield 1; }());
    try { resume g; } catch (e) {}
    try { resume g; } catch (e) {}
    return g;
}
function gen_never_started_dead() { return (function() { return 1; }()); }
function gst(o) { try { return o.getstatus(); } catch (e) { return "E:" + e; } }
function ggo(o, f) {
    local before = gst(o);
    local r = null; local err = null;
    try { r = f(o); } catch (e) { err = "" + e; }
    return "before=" + before + "|r=" + show(r) + "|err=" + show(err) + "|after=" + gst(o);
}
"""

# (标签, 状态构造器)
TH_STATES = [
    ("idle", "th_idle()"),
    ("suspended", "th_suspended()"),
    ("suspended_val", "th_suspended_val()"),
    ("dead_return", "th_dead_return()"),
    ("dead_throw", "th_dead_throw()"),
    ("suspended_nested", "th_suspended_nested()"),
    ("suspended_twice", "th_suspended_twice()"),
    ("dead_after_wakeup", "th_dead_after_wakeup()"),
]

# (标签, 调用表达式模板 {o} = 线程)
TH_APIS = [
    ("call0", "o.call()"),
    ("call_args", "o.call(1, 2)"),
    ("call_thrice", "o.call(1, 2, 3)"),
    ("wakeup0", "o.wakeup()"),
    ("wakeup_val", "o.wakeup(9)"),
    ("wakeup_null", "o.wakeup(null)"),
    ("wakeup_str", 'o.wakeup("w")'),
    ("wakeupthrow_val", 'o.wakeupthrow("e")'),
    ("wakeupthrow_bool", 'o.wakeupthrow("e", true)'),
    ("wakeupthrow_3", 'o.wakeupthrow("e", true, 1)'),
    ("getstatus", "o.getstatus()"),
    ("stackinfos0", "o.getstackinfos(0)"),
    ("stackinfos_big", "o.getstackinfos(99)"),
    ("weakref", "o.weakref()"),
    ("tostring", "o.tostring()"),
    ("typeof", "typeof o"),
]

GEN_STATES = [
    ("new", "gen_new()"),
    ("suspended", "gen_suspended()"),
    ("dead", "gen_dead()"),
    ("dead_nostart", "gen_never_started_dead()"),
]

GEN_APIS = [
    ("resume", "resume o"),
    ("getstatus", "o.getstatus()"),
    ("weakref", "o.weakref()"),
    ("tostring", "o.tostring()"),
    ("typeof", "typeof o"),
    ("stackinfos0", "o.getstackinfos(0)"),
]


def case(tag, state, call, wrapper):
    return ('t("{tag}", function() {{ local o = {state}; '
            'return {wrapper}(o, function(o) {{ return {call}; }}); }});'
            .format(tag=tag, state=state, call=call, wrapper=wrapper))


def main():
    lines = [PREAMBLE]
    n = 0
    for sname, sexpr in TH_STATES:
        for aname, call in TH_APIS:
            lines.append(case("th_%s_%s" % (sname, aname), sexpr, call, "go"))
            n += 1
    for sname, sexpr in GEN_STATES:
        for aname, call in GEN_APIS:
            lines.append(case("gn_%s_%s" % (sname, aname), sexpr, call, "ggo"))
            n += 1

    # 多步序列：状态迁移的链（每一步都汇报）
    SEQ = [
        ("seq_wakeup_twice", 'local o = th_suspended(); local a = null; local b = null; '
                             'try { a = o.wakeup(1); } catch (e) { a = "E:" + e; } '
                             'try { b = o.wakeup(2); } catch (e) { b = "E:" + e; } '
                             'return show(a) + " / " + show(b) + " / " + st(o);'),
        ("seq_call_after_dead", 'local o = th_dead_return(); local r = null; '
                                'try { r = o.call(); } catch (e) { r = "E:" + e; } '
                                'return show(r) + " / " + st(o);'),
        ("seq_wakeup_then_call", 'local o = th_suspended(); '
                                 'try { o.wakeup(); } catch (e) {} '
                                 'local r = null; try { r = o.call(); } catch (e) { r = "E:" + e; } '
                                 'return show(r) + " / " + st(o);'),
        ("seq_call_twice", 'local o = th_suspended_val(); local a = null; local b = null; '
                           'try { a = o.call(); } catch (e) { a = nerr("" + e); } '
                           'local mid = st(o); '
                           'try { b = o.call(); } catch (e) { b = nerr("" + e); } '
                           'return show(a) + " / mid=" + mid + " / " + show(b) + " / " + st(o);'),
        ("seq_wakeupthrow_then_status", 'local o = th_suspended(); local r = null; '
                                        'try { r = o.wakeupthrow("x"); } catch (e) { r = "E:" + e; } '
                                        'return show(r) + " / " + st(o);'),
        ("seq_suspend_value_seen", 'local o = th_suspended_val(); local r = null; '
                                   'try { r = o.wakeup(42); } catch (e) { r = "E:" + e; } '
                                   'return show(r) + " / " + st(o);'),
        ("seq_gen_resume_twice", 'local g = gen_new(); local a = null; local b = null; '
                                 'try { a = resume g; } catch (e) { a = "E:" + e; } '
                                 'local mid = gst(g); '
                                 'try { b = resume g; } catch (e) { b = "E:" + e; } '
                                 'return show(a) + " / mid=" + mid + " / " + show(b) + " / " + gst(g);'),
        ("seq_gen_resume_dead", 'local g = gen_dead(); local r = null; '
                                'try { r = resume g; } catch (e) { r = "E:" + e; } '
                                'return show(r) + " / " + gst(g);'),
        ("seq_wakeup_stackinfos", 'local o = th_suspended(); '
                                  'local si = null; try { si = o.getstackinfos(0); } catch (e) { si = "E:" + e; } '
                                  'local r = null; try { r = o.wakeup(); } catch (e) { r = "E:" + e; } '
                                  'return show(si) + " / " + show(r) + " / " + st(o);'),
        ("seq_thread_in_thread", 'local inner = newthread(function() { suspend("i"); return "iv"; }); '
                                 'local outer = newthread(function() { return inner.call(); }); '
                                 'local r = null; try { r = outer.call(); } catch (e) { r = "E:" + e; } '
                                 'return show(r) + " / " + st(outer) + " / " + st(inner);'),
    ]
    for tag, body in SEQ:
        lines.append('t("%s", function() { %s });' % (tag, body))
        n += 1

    # 收尾：本矩阵不写文件，只做生成器自检
    lines.append('print("THREADMAP-DONE\\n");')
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")
    print("已写出 %s：%d 条用例（线程 %d 状态 / 生成器 %d 状态）"
          % (OUT, n, len(TH_STATES), len(GEN_STATES)))

    # 生成器自检：用例体必须自己吃掉错误（!outer: 说明用例体坏了 → 假绿，差分测不出来）
    exe = os.path.join(ROOT, "target", "release", "sq.exe")
    if os.path.exists(exe):
        p = subprocess.run([exe, OUT], capture_output=True, cwd=ROOT, timeout=180)
        text = p.stdout.decode("utf8", "replace")
        bad = [l for l in text.splitlines() if "!outer:" in l]
        done = "THREADMAP-DONE" in text
        print("自检：跑到 DONE=%s，!outer 行数=%d" % (done, len(bad)))
        for l in bad[:5]:
            print("   " + l[:140])
        if bad or not done:
            print("自检失败：用例体自己抛错了，矩阵没有真正测到东西")
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_genmatrix.nut：生成器 / 线程语义组合矩阵。

设计原则（从之前几轮踩坑总结）：
- 每个用例包在 try/catch 里，只输出 `tag=值` 或 `tag!错误`：语料必须能整跑完（超时=FAIL）。
- 只纳入**参考实现有确定语义**的形态；参考实现自身 UB 的形态（负 getstackinfos level、
  线程递归调用自己等）一律排除，另见 STATUS 第四节。
- 产品式组合：resume 次数 × 状态、线程状态 × 操作、suspend 值形态 × wakeup 返回值、
  resume 非生成器类型 × 类型、跨 VM outer 读/写 × 来源。

用法：python tools/gen_genmatrix.py  → 写 tests-corpus/t_genmatrix.nut
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "tests-corpus", "t_genmatrix.nut")

L = []
A = L.append

A("// 自动生成：python tools/gen_genmatrix.py —— 请勿手改")
A("// 生成器 / 线程语义组合矩阵")
A("")
A("function s(tag, f) {")
A("    try { print(tag + \"=\" + f() + \"\\n\"); }")
A("    catch (e) { print(tag + \"!\" + e + \"\\n\"); }")
A("}")
A("")
A("// 三 yield + return 的生成器工厂")
A("function mk3() { return function() { yield 1; yield 2; yield 3; return 99; } }")
A("function mk0() { return function() { return 7; } }")
A("function yieldv(v) { local h = function() { yield v; }; return h(); }")
A("")

# ---------------------------------------------------------------- 1. resume 次数 × 状态
A("// ---- 1. resume 次数 × getstatus（生成器状态机）")
A("function resume_seq(n) {")
A("    local g = mk3()();")
A("    local out = \"\";")
A("    for (local i = 0; i < n; i++) {")
A("        if (i > 0) out += \",\";")
A("        try { out += resume g; } catch (e) { out += \"E:\" + e; }")
A("        out += \"/\" + g.getstatus();")
A("    }")
A("    return out;")
A("}")
for n in range(1, 6):
    A('s("resume%d", function() { return resume_seq(%d); });' % (n, n))
A("")

A("// 返回值的生成器（无 yield）：resume 一次即死")
A("s(\"resume_ret_only\", function() { local g = mk0()(); return resume g + \"/\" + g.getstatus(); });")
A("s(\"resume_ret_twice\", function() {")
A("    local g = mk0()(); resume g;")
A("    try { return \"\" + resume g; } catch (e) { return \"E:\" + e; }")
A("});")
A("")

# ---------------------------------------------------------------- 2. resume 非生成器
A("// ---- 2. resume 作用于非生成器：类型 × 形态")
cases = [
    ("null", "", "null"), ("int", "", "5"), ("float", "", "1.5"), ("str", "", "\"a\""),
    ("bool", "", "true"), ("table", "", "{}"), ("array", "", "[1]"), ("func", "", "function(){}"),
    ("class", "", "class {}"), ("inst", "local C = class {};", "C()"),
    ("thread", "", "newthread(function(){})"), ("weakref", "local t = {};", "t.weakref()"),
    ("gen_itself", "", "mk0()()"),
]
for name, prelude, expr in cases:
    A('s("resume_non_%s", function() {' % name)
    if prelude:
        A("    %s" % prelude)
    A("    local v = %s;" % expr)
    A('    try { return "ok:" + resume v; } catch (e) { return "E:" + e; }')
    A("});")
A("")

# ---------------------------------------------------------------- 3. yield 值形态 × resume 返回
A("// ---- 3. yield 值形态 → resume 返回值")
yvals = [("int", "1"), ("float", "2.5"), ("str", "\"s\""), ("bool", "false"),
         ("null", "null"), ("table", "{a=1}"), ("array", "[1,2]"), ("func", "function(){}"),
         ("class", "class {}"), ("nogen", "mk3()()"), ("thread", "newthread(function(){})")]
for name, expr in yvals:
    A('s("yield_%s", function() { local g = yieldv(%s); local r = resume g; return typeof r + ":" + (typeof r == "table" || typeof r == "array" || typeof r == "function" || typeof r == "class" ? "T" : r) + "/" + g.getstatus(); });'
      % (name, expr))
A("")

# ---------------------------------------------------------------- 4. suspend 值形态 × wakeup 返回
A("// ---- 4. suspend 值形态 → wakeup 返回值（线程侧同构）")
svals = [("noval", "suspend()"), ("int", "suspend(1)"), ("float", "suspend(2.5)"),
         ("str", "suspend(\"s\")"), ("null", "suspend(null)"), ("table", "suspend({a=1})"),
         ("array", "suspend([1,2])"), ("func", "suspend(function(){})")]
for name, call in svals:
    A('s("suspend_%s", function() {' % name)
    A("    local t = newthread(function() { local v = %s; return \"w:\" + typeof v; });" % call)
    A('    local r = t.wakeup();')
    A('    return typeof r + "/" + t.getstatus();')
    A("});")
A("")

# ---------------------------------------------------------------- 5. 线程状态 × 操作
A("// ---- 5. 线程状态 × 操作（fresh / suspended / finished / error-finished）")
A("function mk_thread(kind) {")
A("    if (kind == \"plain\") return newthread(function() { return 1; });")
A("    if (kind == \"susp\") return newthread(function() { local v = suspend(\"s1\"); return \"got:\" + v; });")
A("    if (kind == \"throw\") return newthread(function() { throw \"boom\"; });")
A("    return newthread(function() { suspend(1); throw \"late\"; });")
A("}")
A("function state_prep(kind) {")
A("    local t = mk_thread(kind);")
A("    if (kind == \"done\") { try { t.wakeup(); } catch (e) {} }")
A("    if (kind == \"err\") { try { t.wakeup(); } catch (e) {} }")
A("    return t;")
A("}")
ops = [
    ("getstatus", "t.getstatus()"),
    ("call", "t.call()"),
    ("wakeup", "t.wakeup()"),
    ("wakeup_val", "t.wakeup(\"x\")"),
    ("getstackinfos0", "typeof t.getstackinfos(0)"),
]
for kind in ["plain", "susp", "done", "err"]:
    for opname, opexpr in ops:
        A('s("th_%s_%s", function() { local t = state_prep("%s"); try { return "ok:" + %s; } catch (e) { return "E:" + e; } });'
          % (kind, opname, kind, opexpr))
A("")

# 已 suspend 的线程：操作矩阵
# 注意：必须先 `call()` 才能真正跑到挂起态（fresh 线程的 wakeup() 一定报
# "cannot wakeup a idle thread"，那样整组用例都是退化的）。
# 也**不能**把 `call()` 放进这组：对已挂起的线程再 call，参考实现会拿挂起现场当
# 被调用者（GetUp(-(nparams+1))），两侧的报错文本不同（内部栈布局差异，见 STATUS 第四节）。
for opname, opexpr in ops:
    if opname == "call":
        continue
    A('s("th_suspended_%s", function() { local t = newthread(function() { local v = suspend(1); return "g:" + v; }); t.call(); try { return "ok:" + %s + "/" + t.getstatus(); } catch (e) { return "E:" + e + "/" + t.getstatus(); } });'
      % (opname, opexpr))
A("")
# 挂起态的合法操作：唤醒并传值 / 不传值（↔ sq_wakeupvm 里 target 槽写 null）
A("s(\"susp_wakeup_plain\", function() {")
A("    local t = newthread(function() { local v = suspend(\"first\"); return \"got:\" + v; });")
A("    local a = t.call();")
A("    return \"\" + a + \"/\" + t.wakeup() + \"/\" + t.getstatus();")
A("});")
A("s(\"susp_wakeup_val\", function() {")
A("    local t = newthread(function() { local v = suspend(\"first\"); return \"got:\" + v; });")
A("    t.call();")
A("    return \"\" + t.wakeup(\"x\") + \"/\" + t.getstatus();")
A("});")
A("s(\"susp_wakeup_null\", function() {")
A("    local t = newthread(function() { local v = suspend(\"first\"); return \"got:\" + v; });")
A("    t.call();")
A("    return \"\" + t.wakeup(null) + \"/\" + t.getstatus();")
A("});")
A("s(\"susp_wakeup_table\", function() {")
A("    local t = newthread(function() { local v = suspend(\"first\"); return typeof v; });")
A("    t.call();")
A("    return \"\" + t.wakeup({a=1}) + \"/\" + t.getstatus();")
A("});")
A("s(\"susp_suspend_noarg\", function() {")
A("    local t = newthread(function() { local v = suspend(); return typeof v; });")
A("    t.call();")
A("    return \"\" + t.wakeup(5);")
A("});")
A("s(\"susp_chain_wakeup\", function() {")
A("    local t = newthread(function() { local a = suspend(1); local b = suspend(2); return \"\" + a + b; });")
A("    local out = \"\" + t.call() + \"/\" + t.getstatus();")
A("    out += \"|\" + t.wakeup(\"A\") + \"/\" + t.getstatus();")
A("    out += \"|\" + t.wakeup(\"B\") + \"/\" + t.getstatus();")
A("    return out;")
A("});")
A("s(\"susp_from_other_thread\", function() {")
A("    local t = newthread(function() { local v = suspend(1); return v; });")
A("    t.call();")
A("    local u = newthread(function() { return t.wakeup(\"w\"); });")
A("    return u.call();")
A("});")
A("s(\"susp_status_names\", function() {")
A("    local t = newthread(function() { local v = suspend(1); return \"g:\" + v; });")
A("    local out = t.getstatus();")
A("    t.call();")
A("    out += \"/\" + t.getstatus();")
A("    t.wakeup(1);")
A("    out += \"/\" + t.getstatus();")
A("    return out;")
A("});")
A("")

# ---------------------------------------------------------------- 6. 参数/返回值边界
A("// ---- 6. 线程与生成器的参数、返回值边界")
A("function va(...) { return vargv.len(); }")
A("s(\"th_noargs\", function() { return newthread(function() { return 1; }).call(1, 2, 3); });")
A("s(\"th_missing_args\", function() { local t = newthread(function(a, b) { return typeof a + \"/\" + typeof b; }); return t.call(1); });")
A("s(\"th_varargs\", function() { return newthread(va).call(1, 2, 3, 4); });")
A("s(\"th_varargs_zero\", function() { return newthread(va).call(); });")
A("s(\"gen_noargs\", function() { local h = function() { return 5; }; return resume h(1, 2); });")
A("s(\"gen_missing_args\", function() { local h = function(a, b) { yield typeof a + \"/\" + typeof b; }; local g = h(1); return resume g; });")
A("s(\"gen_varargs\", function() { local h = function(...) { yield vargv.len(); }; return resume h(1, 2, 3); });")
A("s(\"gen_default\", function() { local h = function(a, b = 5) { yield a + b; }; return resume h(1); });")
A("s(\"gen_default_both\", function() { local h = function(a, b = 5) { yield a + b; }; return resume h(1, 2); });")
A("s(\"th_ret_gen\", function() { local t = newthread(function() { return mk3()(); }); local g = t.call(); return typeof g + \"/\" + resume g + \"/\" + g.getstatus(); });")
A("")

# ---------------------------------------------------------------- 7. 错误路径
A("// ---- 7. 错误路径：生成器/线程内的异常如何传播")
A("s(\"gen_throw_first\", function() { local g = (function() { throw \"e1\"; yield 1; })(); try { return \"\" + resume g; } catch (e) { return \"E:\" + e + \"/\" + g.getstatus(); } });")
A("s(\"gen_throw_mid\", function() { local g = (function() { yield 1; throw \"e2\"; })(); resume g; try { return \"\" + resume g; } catch (e) { return \"E:\" + e + \"/\" + g.getstatus(); } });")
A("s(\"gen_throw_after_runtime_err\", function() { local g = (function() { yield 1; return \"x\".nosuch(); })(); resume g; try { return \"\" + resume g; } catch (e) { return \"E:\" + e + \"/\" + g.getstatus(); } });")
A("s(\"th_throw_call\", function() { local t = newthread(function() { throw \"boom\"; }); try { return \"\" + t.call(); } catch (e) { return \"E:\" + e + \"/\" + t.getstatus(); } });")
A("s(\"th_throw_wakeup\", function() { local t = newthread(function() { suspend(1); throw \"boom2\"; }); t.wakeup(); try { return \"\" + t.call(); } catch (e) { return \"E:\" + e + \"/\" + t.getstatus(); } });")
A("s(\"th_throw_second_call\", function() { local t = newthread(function() { throw \"boom3\"; }); try { t.call(); } catch (e) {} try { return \"\" + t.call(); } catch (e) { return \"E:\" + e + \"/\" + t.getstatus(); } });")
A("s(\"gen_throw_inside_thread\", function() { local t = newthread(function() { local g = (function() { yield 1; throw \"in\"; })(); resume g; resume g; return \"no\"; }); try { return \"\" + t.call(); } catch (e) { return \"E:\" + e + \"/\" + t.getstatus(); } });")
A("s(\"gen_yield_in_thread\", function() { local t = newthread(function() { local g = (function() { yield \"y\"; return \"r\"; })(); return \"a\" + resume g; }); return t.call(); });")
A("")

# ---------------------------------------------------------------- 8. 跨 VM outer（本轮修复点）
A("// ---- 8. 跨 VM 的 outer 读写（闭包捕获的局部变量被别的线程读/改）")
A("s(\"xvm_read\", function() {")
A("    local v = \"host\";")
A("    local t = newthread(function() { return v; });")
A("    return t.call();")
A("});")
A("s(\"xvm_write\", function() {")
A("    local v = 0;")
A("    local t = newthread(function() { v = 7; return 0; });")
A("    t.call();")
A("    return v;")
A("});")
A("s(\"xvm_param\", function() {")
A("    local h = function(p) { return newthread(function() { return p * 2; }); };")
A("    return h(21).call();")
A("});")
A("s(\"xvm_nested_read\", function() {")
A("    local a = \"A\";")
A("    local h = function() { return function() { return a; }; };")
A("    local t = newthread(h());")
A("    return t.call();")
A("});")
A("s(\"xvm_after_frame\", function() {")
A("    local inner = function() { local q = \"Q\"; return function() { return q; }; };")
A("    local f = inner();")
A("    local t = newthread(f);")
A("    return t.call();")
A("});")
A("s(\"xvm_gen_in_thread\", function() {")
A("    local v = 5;")
A("    local t = newthread(function() { v = v + 1; return v; });")
A("    t.call();")
A("    return v;")
A("});")
A("s(\"xvm_both_ways\", function() {")
A("    local v = 1;")
A("    local t = newthread(function() { v = v * 10; return v; });")
A("    local r = t.call();")
A("    v = v + 1;")
A("    return r + \"/\" + v;")
A("});")
A("s(\"xvm_thread_closure_into_thread\", function() {")
A("    local t1 = newthread(function() { local z = 3; return newthread(function() { z = z + 4; return z; }); });")
A("    local t2 = t1.call();")
A("    return t2.call() + \"/\" + t2.getstatus();")
A("});")
A("s(\"xvm_suspend_keeps_outer\", function() {")
A("    local v = \"init\";")
A("    local t = newthread(function() { suspend(1); return v; });")
A("    t.wakeup();")
A("    v = \"changed\";")
A("    return t.call();")
A("});")
A("")

# ---------------------------------------------------------------- 9. 其它位置/形态
A("// ---- 9. 生成器/线程作为值：存表、存数组、类成员、再嵌套")
A("s(\"gen_in_table\", function() { local t = { g = mk3()() }; return resume t.g + \"/\" + resume t.g; });")
A("s(\"gen_in_array\", function() { local a = [mk3()()]; return resume a[0] + \"/\" + resume a[0]; });")
A("s(\"gen_in_class\", function() { local C = class { g = null; constructor() { g = mk3()(); } }; local c = C(); return resume c.g + \"/\" + resume c.g; });")
A("s(\"gen_reentrant\", function() { local inner = mk3()(); local outer = (function() { yield resume inner; yield resume inner; })(); return resume outer + \"/\" + resume outer + \"/\" + inner.getstatus(); });")
A("s(\"gen_yield_gen\", function() { local inner = mk3()(); local outer = yieldv(inner); local got = resume outer; return typeof got + \"/\" + resume got; });")
A("s(\"th_in_table\", function() { local t = { th = newthread(function() { return 3; }) }; return t.th.call(); });")
A("s(\"th_in_array\", function() { local a = [newthread(function() { return 4; })]; return a[0].call(); });")
A("s(\"th_newthread_inside\", function() { local t = newthread(function() { local u = newthread(function() { return \"u\"; }); return \"t:\" + u.call(); }); return t.call(); });")
A("s(\"gen_foreach\", function() { local out = \"\"; foreach (v in mk3()()) out += v + \",\"; return out; });")
A("s(\"gen_foreach_break\", function() { local out = \"\"; foreach (v in mk3()()) { out += v; if (v == 2) break; } return out; });")
A("s(\"gen_foreach_partial\", function() { local g = mk3()(); local out = \"\"; foreach (v in g) { out += v; if (v == 1) break; } return out + \"/\" + g.getstatus(); });")
A("s(\"gen_while_resume\", function() { local g = mk3()(); local out = \"\"; local n = 0; while (g.getstatus() != \"dead\" && n < 6) { out += resume g; n++; } return out + \"/\" + n; });")
A("s(\"gen_locals_kept\", function() { local g = (function() { local a = 1; yield a; a = a + 1; yield a; return a; })(); return resume g + \"/\" + resume g + \"/\" + resume g; });")
A("s(\"gen_this\", function() { local C = class { v = 5; function f() { yield this.v; } }; local c = C(); return resume c.f(); });")
A("s(\"gen_callee\", function() { local h = function() { yield typeof callee; }; return resume h(); });")
A("s(\"th_this\", function() { local C = class { v = 6; function f() { return this.v; } }; local c = C(); return newthread(c.f).call(); });")
A("s(\"th_typeof\", function() { local t = newthread(function() { return 1; }); return typeof t + \"/\" + t.getstatus(); });")
A("s(\"th_getstatus_after_suspend\", function() { local t = newthread(function() { suspend(\"x\"); suspend(\"y\"); return 0; }); t.wakeup(); return t.getstatus() + \"/\" + t.wakeup() + \"/\" + t.getstatus(); });")
A("s(\"shared_table_across\", function() { local tbl = {}; local t = newthread(function() { tbl.k <- 1; return 0; }); t.call(); return \"k\" in tbl; });")
A("s(\"th_many\", function() { local sum = 0; for (local i = 0; i < 5; i++) { local t = newthread(function() { return 1; }); sum += t.call(); } return sum; });")
A("s(\"gen_many\", function() { local sum = 0; for (local i = 0; i < 5; i++) { local g = (function() { yield 2; })(); sum += resume g; } return sum; });")
A("")

A("// ---- 10. 线程栈状态：失败/正常结束后再次 call、wakeup 的栈纪律")
A("// （↔ sq_call 的 `if(!_suspended) Pop(params)` 与 thread_call 的 move+pop：")
A("//   正常返回要把 roottable/实参弹干净，挂起则保留 suspend 现场）")
A("s(\"stack_call_err_twice\", function() {")
A("    local t = newthread(function() { throw \"e1\"; });")
A("    local out = \"\";")
A("    for (local i = 0; i < 3; i++) {")
A("        try { out += t.call(); } catch (e) { out += \"E:\" + e; }")
A("        out += \"/\" + t.getstatus() + \";\";")
A("    }")
A("    return out;")
A("});")
A("s(\"stack_call_err_args_twice\", function() {")
A("    local t = newthread(function(a, b) { throw \"a\" + a + b; });")
A("    local out = \"\";")
A("    for (local i = 0; i < 2; i++) {")
A("        try { out += t.call(1, 2); } catch (e) { out += \"E:\" + e; }")
A("        out += \";\";")
A("    }")
A("    return out;")
A("});")
A("s(\"stack_call_ok_args_twice\", function() {")
A("    local t = newthread(function(a) { return \"r\" + a; });")
A("    return \"\" + t.call(1) + \"/\" + t.call(2) + \"/\" + t.getstatus();")
A("});")
A("s(\"stack_call_ok_varargs_twice\", function() {")
A("    local t = newthread(function(...) { return vargv.len(); });")
A("    return \"\" + t.call(1, 2, 3) + \"/\" + t.call() + \"/\" + t.call(9);")
A("});")
A("s(\"stack_wakeup_then_throw\", function() {")
A("    local t = newthread(function() { suspend(1); throw \"e2\"; });")
A("    local a = t.call();")
A("    local out = \"\" + a + \"/\" + t.getstatus();")
A("    try { out += \"/\" + t.wakeup(); } catch (e) { out += \"/E:\" + e; }")
A("    out += \"/\" + t.getstatus();")
A("    try { out += \"/\" + t.call(); } catch (e) { out += \"/E:\" + e; }")
A("    return out;")
A("});")
A("s(\"stack_call_then_throw_via_wakeup\", function() {")
A("    local t = newthread(function() { suspend(1); throw \"e3\"; });")
A("    local out = \"\" + t.call();")
A("    try { out += \"/\" + t.wakeup(); } catch (e) { out += \"/E:\" + e; }")
A("    try { out += \"/\" + t.wakeup(); } catch (e) { out += \"/E:\" + e; }")
A("    return out;")
A("});")
A("s(\"stack_wakeup_after_normal_death\", function() {")
A("    local t = newthread(function() { suspend(1); return \"end\"; });")
A("    local out = \"\" + t.call() + \"/\" + t.getstatus();")
A("    try { out += \"/\" + t.wakeup(); } catch (e) { out += \"/E:\" + e; }")
A("    return out;")
A("});")
A("s(\"stack_suspended_then_error\", function() {")
A("    local t = newthread(function() { local v = suspend(\"s\"); if (v == \"bad\") throw \"ok-bad\"; return \"v:\" + v; });")
A("    t.call();")
A("    try { return \"\" + t.wakeup(\"bad\"); } catch (e) { return \"E:\" + e + \"/\" + t.getstatus(); }")
A("});")
A("s(\"stack_gen_from_dead_thread\", function() {")
A("    local t = newthread(function() { local g = (function() { yield 1; yield 2; })(); resume g; return g; });")
A("    local g = t.call();")
A("    return resume g + \"/\" + resume g + \"/\" + g.getstatus();")
A("});")
A("s(\"stack_call_after_suspend_value\", function() {")
A("    local t = newthread(function() { local v = suspend(\"first\"); return \"got:\" + v; });")
A("    local a = t.call();")
A("    local c = t.wakeup();")
A("    return \"\" + a + \"/\" + c + \"/\" + t.getstatus();")
A("});")
A("")

A('print("GENMATRIX-DONE\\n");')
A("")

open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(L))
print("wrote %s (%d lines)" % (OUT, len(L)))

// 自动生成：错误转储 × 复杂帧矩阵（tools/gen_errdumpframes.py）
//
// 错误**跨越线程/原生调用边界**时：即使外层 try/catch 接住，stderr 照样打印
// AN ERROR HAS OCCURRED + CALLSTACK + LOCALS（含脚本相对路径），且脚本继续跑完；
// 纯脚本帧之间被 catch 的抛错**不产生任何转储**（stderr 必须干净）。
// `mm_chain_overflow`：_get 元方法里解析 `boom4` 又落回委托链 → 无限递归 →
// `Native stack overflow`（两侧一致，照实入语料）。
// 两侧逐字节一致（diff_test 对拍 stdout + stderr + 退出码）。
function s(tag, f) {
    try { f(); print(tag + "=no-dump\n"); }
    catch (e) { print(tag + "=caught:" + e + "\n"); }
}
function boom4(k) { throw "mm:" + k; }

s("thread_throw", function() { local th = newthread(function() { local lv = "L"; throw "tb"; }); th.call(); th.wakeup(); });
s("thread_wakeupthrow", function() { local th = newthread(function() { suspend("s"); }); th.call(); th.wakeupthrow("inj"); });
s("gen_in_thread", function() { local th = newthread(function() { local g = (function() { local gg = 9; throw "git"; })(); resume g; }); th.call(); th.wakeup(); });
s("native_frame_sort", function() { local a = [1, 2]; a.sort(function(x, y) { local sc = 1; throw "sortcb"; }); });
s("native_frame_map", function() { local a = [1]; a.map(function(v) { local mv = 2; throw "mapcb"; }); });
s("plain_catch", function() { throw "plain"; });
s("rethrow", function() { try { throw "orig"; } catch (e) { local r = 1; throw "again:" + e; } });
s("nested_try", function() { try { try { throw "inner"; } catch (e) { throw "outer:" + e; } } catch (e2) { throw "again:" + e2; } });
s("many_locals", function() { local a1 = 1, a2 = 2, a3 = "three", a4 = [4], a5 = { k = 5 }, a6 = 1.5, a7 = true, a8 = null; throw "many"; });
s("ctor_throw", function() { local C = class { constructor() { local c = 1; throw "ctor"; } }; C(); });
s("foreach_nexti", function() { local d = { _nexti = function(i) { throw "nexti"; } }; local o = {}; o.setdelegate(d); foreach (k, v in o) { } });
s("varargs_throw", function() { local f = function(a, ...) { local n = vargv.len(); throw "va:" + n; }; f(1, 2, 3); });
s("mm_chain_overflow", function() { local d = { _get = function(k) { return boom4(k); } }; local o = {}; o.setdelegate(d); return o.zz; });
print("ERRDUMPFRAMES-DONE\n");

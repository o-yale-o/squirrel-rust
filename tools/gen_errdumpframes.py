# -*- coding: utf-8 -*-
"""生成"错误转储 × 复杂帧"矩阵语料 tests-corpus/t_errdumpframes.nut。

为什么单开一份：`t_errdump1-3` 各自只有**一个**顶层 throw 的转储（第一个抛错
就中止脚本，一个文件只能钉一条）；本轮钉的是**错误跨越边界时**的转储行为——
线程内抛错、生成器在线程内抛错、原生回调（sort 比较器/map 回调）内抛错、
wakeupthrow 注入、脚本帧被 catch。这些形态现有生成器结构性地碰不到（需要
"错误穿越的边界类型"当输入维度）。本轮为**验证型**：16 + 6 条探针（逐条隔离，
stdout+stderr+退出码三方对拍）两侧逐字节一致。

实测钉住的语义（都对着参考实现验过）：
1. **转储发生在错误跨越"线程/原生调用"边界时**——即使外层脚本 try/catch 接住了
   错误，stderr 照样打印 `AN ERROR HAS OCCURRED [msg]` + CALLSTACK + LOCALS，
   且脚本**继续跑完**（stdout 有 DONE 标记）。
2. **脚本帧之间的普通抛错被 catch 时，不产生任何转储**（stderr 干净）。
3. `wakeupthrow` 注入的错误同样触发转储；生成器在**线程内**抛错也触发。
4. CALLSTACK/LOCALS 里的**脚本路径**是相对路径（diff_test 用同一路径跑两侧），
   逐字节可比。
5. **`_get` 元方法里的名字解析会递归爆栈**：`d._get` 调用外部 local 函数
   `boom4(k)`，而 `boom4` 的名字解析又落回委托链上的 `_get` → 无限递归 →
   `Native stack overflow`。**两侧完全一致**（连被 catch 后继续跑都一样），
   所以照实入语料（`mm_chain_overflow`）。

安全约束：无（所有用例都不崩溃/挂死参考实现；逐条隔离实测）。

用法：
    python tools/gen_errdumpframes.py
    python tools/gen_errdumpframes.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_errdumpframes.nut")

CASES = [
    # --- 触发转储的（错误跨越线程/原生边界）---
    ("thread_throw", 'local th = newthread(function() { local lv = "L"; throw "tb"; }); th.call(); th.wakeup();'),
    ("thread_wakeupthrow", 'local th = newthread(function() { suspend("s"); }); th.call(); th.wakeupthrow("inj");'),
    ("gen_in_thread", 'local th = newthread(function() { local g = (function() { local gg = 9; throw "git"; })(); resume g; }); th.call(); th.wakeup();'),
    ("native_frame_sort", 'local a = [1, 2]; a.sort(function(x, y) { local sc = 1; throw "sortcb"; });'),
    ("native_frame_map", 'local a = [1]; a.map(function(v) { local mv = 2; throw "mapcb"; });'),
    # --- 不触发转储的（纯脚本帧被 catch，stderr 必须干净）---
    ("plain_catch", 'throw "plain";'),
    ("rethrow", 'try { throw "orig"; } catch (e) { local r = 1; throw "again:" + e; }'),
    ("nested_try", 'try { try { throw "inner"; } catch (e) { throw "outer:" + e; } } catch (e2) { throw "again:" + e2; }'),
    ("many_locals", 'local a1 = 1, a2 = 2, a3 = "three", a4 = [4], a5 = { k = 5 }, a6 = 1.5, a7 = true, a8 = null; throw "many";'),
    ("ctor_throw", 'local C = class { constructor() { local c = 1; throw "ctor"; } }; C();'),
    ("foreach_nexti", 'local d = { _nexti = function(i) { throw "nexti"; } }; local o = {}; o.setdelegate(d); foreach (k, v in o) { }'),
    ("varargs_throw", 'local f = function(a, ...) { local n = vargv.len(); throw "va:" + n; }; f(1, 2, 3);'),
    ("mm_chain_overflow", 'local d = { _get = function(k) { return boom4(k); } }; local o = {}; o.setdelegate(d); return o.zz;'),
]

lines = [
    "// 自动生成：错误转储 × 复杂帧矩阵（tools/gen_errdumpframes.py）",
    "//",
    "// 错误**跨越线程/原生调用边界**时：即使外层 try/catch 接住，stderr 照样打印",
    "// AN ERROR HAS OCCURRED + CALLSTACK + LOCALS（含脚本相对路径），且脚本继续跑完；",
    "// 纯脚本帧之间被 catch 的抛错**不产生任何转储**（stderr 必须干净）。",
    "// `mm_chain_overflow`：_get 元方法里解析 `boom4` 又落回委托链 → 无限递归 →",
    "// `Native stack overflow`（两侧一致，照实入语料）。",
    "// 两侧逐字节一致（diff_test 对拍 stdout + stderr + 退出码）。",
    "function s(tag, f) {",
    "    try { f(); print(tag + \"=no-dump\\n\"); }",
    "    catch (e) { print(tag + \"=caught:\" + e + \"\\n\"); }",
    "}",
    "function boom4(k) { throw \"mm:\" + k; }",
    "",
]
for name, body in CASES:
    lines.append('s("%s", function() { %s });' % (name, body))

ALL = [l for l in lines if l.startswith('s("')]
if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k][:110]))
    sys.exit(0)

lines += ['print("ERRDUMPFRAMES-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

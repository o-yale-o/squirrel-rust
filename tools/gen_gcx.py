# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_gcx.nut：多线程 × GC 交叉面。

= tmp-c2/gcxprobe.nut 的全部用例（18 条，两侧完全一致）。
这一轮抓到一个**真 bug**：线程里调用 `collectgarbage()` 会 panic
（`RefCell already mutably borrowed`），修完又暴露出"当前 VM 自己的根没被标"
（线程栈上的对象被误回收）。详见 STATUS 三·补17 与 gc_thread_semantics.rs。

用法：python tools/gen_gcx.py
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = os.path.join(ROOT, "tmp-c2", "gcxprobe.nut")
OUT = os.path.join(ROOT, "tests-corpus", "t_gcx.nut")

src = open(PROBE, encoding="utf-8").read()
body = src[: src.index('print("GCXPROBE-DONE')].rstrip()

out = [
    "// 自动生成：python tools/gen_gcx.py —— 请勿手改",
    "// 多线程 × GC：挂起线程/生成器持有对象、线程里 collectgarbage、跨线程 outer",
    "",
    body,
    "",
    'print("GCX-DONE\\n");',
    "",
]
open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(out))
n = sum(1 for l in open(OUT, encoding="utf-8") if l.startswith("s("))
print("wrote %s (%d 用例)" % (OUT, n))

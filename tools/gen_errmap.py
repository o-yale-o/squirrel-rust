# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_errmap.nut：异常机制矩阵。

= tmp-c2/errprobe.nut 的全部用例，**去掉 `si_level_neg`**：
`getstackinfos(-1)` 在参考实现里是越界读（`_callsstack[cssize]`，UB，实测返回一个表），
本实现返回 null —— 这是第四节的有意偏离，不能进差分语料，
由 crates/squirrel-stdlib/tests/exception_semantics.rs 单独守门。

另外 tests-corpus/t_errtail.nut 手写：脚本末尾制造一个**未捕获**错误，
用来对比完整 stderr（AN ERROR HAS OCCURRED / CALLSTACK / LOCALS）与退出码。

用法：python tools/gen_errmap.py
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = os.path.join(ROOT, "tmp-c2", "errprobe.nut")
OUT = os.path.join(ROOT, "tests-corpus", "t_errmap.nut")

src = open(PROBE, encoding="utf-8").read()
body = src[: src.index('print("ERRPROBE-DONE')].rstrip()
# 剔除已知 UB 用例（连同它的注释行）
lines = [l for l in body.splitlines() if "si_level_neg" not in l]
body = "\n".join(lines)

out = [
    "// 自动生成：python tools/gen_errmap.py —— 请勿手改",
    "// 异常机制矩阵：throw 各类型、VM 错误消息、嵌套 trap、错误处理器、getstackinfos",
    "// 注：getstackinfos(-1) 是参考实现的越界读（UB），语料里已去掉，见 exception_semantics.rs",
    "",
    body,
    "",
    'print("ERRMAP-DONE\\n");',
    "",
]
open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(out))
n = sum(1 for l in open(OUT, encoding="utf-8") if l.startswith("s("))
print("wrote %s (%d 用例)" % (OUT, n))

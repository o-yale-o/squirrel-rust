# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_sysmap.nut：io / system / date 语义矩阵。

= tmp-c2/sysprobe.nut 的全部用例（47 条）。
注意两处**不作为语义差异**的噪声，语料里已经规避：
* `system()` 子进程输出与脚本自身 print 的**行序**取决于 stdout 缓冲策略
  （参考实现块缓冲、退出时才 flush），所以子进程输出改为重定向到文件再读回；
* `clock()` 的值是 CPU 计时，两次运行必然不同 → 只比类型/非负。

用法：python tools/gen_sysmap.py
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = os.path.join(ROOT, "tmp-c2", "sysprobe.nut")
OUT = os.path.join(ROOT, "tests-corpus", "t_sysmap.nut")

src = open(PROBE, encoding="utf-8").read()
body = src[: src.index('print("SYSPROBE-DONE')].rstrip()

out = [
    "// 自动生成：python tools/gen_sysmap.py —— 请勿手改",
    "// io / system / date 语义矩阵（固定时间戳；同一台机器上两侧时区相同）",
    "",
    body,
    "",
    'print("SYSMAP-DONE\\n");',
    "",
]
open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(out))
n = sum(1 for l in open(OUT, encoding="utf-8") if l.startswith("s("))
print("wrote %s (%d 用例)" % (OUT, n))

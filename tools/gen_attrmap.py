# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_attrmap.nut：类成员属性（</ ... />）与 getattributes/setattributes。

= tmp-c2/attrprobe.nut 的全部用例（42 条，两侧逐字节一致）。
语法错形态（**关键字当属性名**、**单行里第二个属性块 / 成员之后写属性**）
归 tools/errmsg_matrix.py 对拍，不在这里。

用法：python tools/gen_attrmap.py
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = os.path.join(ROOT, "tmp-c2", "attrprobe.nut")
OUT = os.path.join(ROOT, "tests-corpus", "t_attrmap.nut")

src = open(PROBE, encoding="utf-8").read()
body = src[: src.index('print("ATTRPROBE-DONE')].rstrip()

out = [
    "// 自动生成：python tools/gen_attrmap.py —— 请勿手改",
    "// 类成员属性矩阵：</ ... /> 形式、getattributes/setattributes、继承/clone、可见性",
    "",
    body,
    "",
    'print("ATTRMAP-DONE\\n");',
    "",
]
open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(out))
n = sum(1 for l in open(OUT, encoding="utf-8") if l.startswith("s("))
print("wrote %s (%d 用例)" % (OUT, n))

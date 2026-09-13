# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_sermap.nut：序列化往返（脚本侧 writeclosuretofile / loadfile）。

= tmp-c2/serprobe.nut 的全部用例（28 条，两侧只差指针文本）。
临时文件写在 `tmp-c2/`（本地对拍工具目录，仓库里已 gitignore）。

用法：python tools/gen_ser.py
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = os.path.join(ROOT, "tmp-c2", "serprobe.nut")
OUT = os.path.join(ROOT, "tests-corpus", "t_sermap.nut")

src = open(PROBE, encoding="utf-8").read()
body = src[: src.index('print("SERPROBE-DONE')].rstrip()

out = [
    "// 自动生成：python tools/gen_ser.py —— 请勿手改",
    "// 序列化往返：writeclosuretofile 写 .cnut → loadfile 读回并调用",
    "//（写文件名里带 0x… 的用例靠语料的指针归一化对齐）",
    "",
    body,
    "",
    'print("SERMAP-DONE\\n");',
    "",
]
open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(out))
n = sum(1 for l in open(OUT, encoding="utf-8") if l.startswith("s("))
print("wrote %s (%d 用例)" % (OUT, n))

# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_suspmap.nut：suspend 穿透 native / metamethod 的错误路径。

= tmp-c2/suspprobe.nut 的全部用例（12 条 stdout + 47 行 stderr，两侧完全一致）。
主脚本里直接 `suspend` 会让进程静默结束（rc=1、无输出，两侧一致），
那一条单独放 tests-corpus/t_suspend_main.nut。

用法：python tools/gen_suspmap.py
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = os.path.join(ROOT, "tmp-c2", "suspprobe.nut")
OUT = os.path.join(ROOT, "tests-corpus", "t_suspmap.nut")
MAIN = os.path.join(ROOT, "tests-corpus", "t_suspend_main.nut")

src = open(PROBE, encoding="utf-8").read()
body = src[: src.index('print("SUSPPROBE-DONE')].rstrip()

out = [
    "// 自动生成：python tools/gen_suspmap.py —— 请勿手改",
    "// suspend 穿透 native/metamethod：错误文本与事后状态",
    "",
    body,
    "",
    'print("SUSPMAP-DONE\\n");',
    "",
]
open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(out))

main_src = [
    "// 主脚本里直接 suspend：两侧都**静默结束**（rc=1、无任何输出），",
    "// 所以必须单独一个文件（放在别的语料里会让后面的用例跑不到）。",
    "// 这个文件刻意不打印任何东西 —— diff_test 会因为 stderr 为空而通过，",
    "// 关键是被 kill/超时就不会 PASS。",
    'print("SUSPEND_MAIN-SHOULD-NOT-PRINT\\n");',
    "suspend(1);",
    'print("SUSPEND_MAIN-DONE\\n");',
    "",
]
open(MAIN, "w", encoding="utf-8", newline="\n").write("\n".join(main_src))

n = sum(1 for l in open(OUT, encoding="utf-8") if l.startswith("s("))
print("wrote %s (%d 用例) + %s" % (OUT, n, MAIN))

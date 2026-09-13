# -*- coding: utf-8 -*-
"""把 tmp-c2/weakprobe.nut + weakprobe2.nut 合并成 tests-corpus/t_weakmap.nut。

两个探针各自带了 `s()` 与 mk_* 辅助函数，合并时只取第二个的用例行（去掉重复定义）。
用法：python tools/gen_weakmap.py
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
P1 = os.path.join(ROOT, "tmp-c2", "weakprobe.nut")
P2 = os.path.join(ROOT, "tmp-c2", "weakprobe2.nut")
OUT = os.path.join(ROOT, "tests-corpus", "t_weakmap.nut")

a = open(P1, encoding="utf-8").read()
b = open(P2, encoding="utf-8").read()

head_a = a[: a.index('s("')]
# 用例体是多行的，不能按行过滤：直接从第一个 s( 取到结尾，只去掉各自的收尾打印
cases_a = a[a.index('s("'):].replace('print("WEAKPROBE-DONE\\n");', "").strip()
# probe2 的 `function s(...)` 与 head_a 重复，从它的 `function mk()` 开始取
cases_b = b[b.index("function mk()"):].replace('print("WEAK2-DONE\\n");', "").strip()

out = []
out.append("// 自动生成：python tools/gen_weakmap.py —— 请勿手改")
out.append("// weakref / GC / 回收语义矩阵（weakprobe 两轮探针的并集）")
out.append("")
out.append(head_a.rstrip())
out.append("")
out.append("// ---- 第一轮：类型 × weakref、ref/getweakrefval、容器、环 ----")
out.append(cases_a)
out.append("")
out.append("// ---- 第二轮：是否保活、collectgarbage 计数、各类对象 ----")
out.append(cases_b)
out.append("")
out.append('print("WEAKMAP-DONE\\n");')
out.append("")

open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(out))
n = len([1 for l in cases_a.splitlines() if l.startswith("s(")]) + len(
    [1 for l in cases_b.splitlines() if l.startswith("s(")]
)
print("wrote %s (%d 个 s( 起点，含多行用例)" % (OUT, n))

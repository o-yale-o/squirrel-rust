# -*- coding: utf-8 -*-
"""盘点 C 嵌入 API（sqapi.h 的 141 个 sq_* 导出）在 Rust 侧有没有对应的能力/引用。

做法：在每个 sq_ 名字出现在我们源码的注释（↔ sq_xxx）或实现里的次数上做统计，
把"从未出现过"的函数单独列出来，人工判断是不是真缺口。
用法：python tools/audit_capi.py [--verbose]
"""
import os
import re
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
SQ = r"D:\OpenSource\squirrel"
HEADER = os.path.join(SQ, "include", "squirrel.h")
SRC_DIRS = [
    os.path.join(ROOT, "crates", "squirrel", "src"),
    os.path.join(ROOT, "crates", "squirrel-stdlib", "src"),
    os.path.join(ROOT, "crates", "sq", "src"),
]

hdr = open(HEADER, encoding="utf-8", errors="replace").read()
api = re.findall(r"SQUIRREL_API\s+\S+\s+(sq_\w+)", hdr)

text = []
for d in SRC_DIRS:
    for dirpath, _dirs, files in os.walk(d):
        for f in files:
            if f.endswith(".rs"):
                p = os.path.join(dirpath, f)
                text.append((p, open(p, encoding="utf-8", errors="replace").read()))

missing, weak, ok = [], [], 0
for name in api:
    hits = sum(t.count(name) for _p, t in text)
    if hits == 0:
        missing.append(name)
    elif hits == 1:
        weak.append(name)
    else:
        ok += 1

print("C 嵌入 API: %d 个导出" % len(api))
print("  有 2+ 处引用（视为已覆盖/已对齐）: %d" % ok)
print("  仅 1 处引用（多半只是注释里提了一次）: %d" % len(weak))
print("  完全没出现（**候选缺口**）: %d" % len(missing))
print()
print("== 完全没出现 ==")
print(" ".join(missing))
print()
print("== 仅 1 处引用 ==")
print(" ".join(weak))
if "--verbose" in sys.argv:
    for name in missing + weak:
        for p, t in text:
            if name in t:
                i = t.index(name)
                line = t.count("\n", 0, i) + 1
                print("  %s -> %s:%d" % (name, os.path.relpath(p, ROOT), line))

# -*- coding: utf-8 -*-
"""审计语料/样本里的"写文件"用例：是否都配了删除。

为什么需要：语料是由差分测试在**仓库根目录**跑的（`python tools/diff_test.py tests-corpus`），
谁漏删就会把垃圾留在工作区。实际发生过两次：`robust_surface3.txt` 被提交进仓库，
`robust_surface2.cnut` 一直在根目录（靠 `*.cnut` 忽略才没被发现）。

判定口径：
  - 只统计**写**模式（`"w"`/`"wb"`/`"wb+"`/`"a"`/…）以及 `writeclosuretofile` 的目标；
    只读路径（`"r"`/`"rb"`）与"故意不存在"的路径不算；
  - 写到 `tmp-c2/` 或系统临时目录的算安全（那些目录本来就不入库）；
  - 仓库根目录（或其它入库目录）写的文件，必须在同一份语料里出现 `remove("同名")`。

用法：
    python tools/audit_corpus_writes.py
退出码非 0 表示有漏删。
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIRS = [os.path.join(ROOT, "tests-corpus"), os.path.join(ROOT, "samples")]

# file("path", "mode") —— 第 2 参是写模式才算"写"
FILE_CALL = re.compile(r'file\s*\(\s*"([^"]*)"\s*,\s*"([^"]*)"\s*\)')
WCF = re.compile(r'writeclosuretofile\s*\(\s*"([^"]*)"')
REMOVE = re.compile(r'remove\s*\(\s*"([^"]*)"')


def is_write_mode(mode: str) -> bool:
    # "r+" 也能写，但它不改长度、不创建文件；真正的创建/截断看 w/a
    return any(ch in mode for ch in "wa")


def is_safe_path(p: str) -> bool:
    if p.startswith("tmp-c2/") or p.startswith("/") or (len(p) > 2 and p[1] == ":"):
        return True
    # 父目录不存在时写必然失败（file() 不会建目录）——那些是"故意写坏路径"的错误用例
    parent = os.path.dirname(p)
    if parent and not os.path.isdir(os.path.join(ROOT, parent)):
        return True
    return False


def main():
    bad = []
    checked = 0
    for d in DIRS:
        if not os.path.isdir(d):
            continue
        for name in sorted(os.listdir(d)):
            if not name.endswith(".nut"):
                continue
            path = os.path.join(d, name)
            text = open(path, encoding="utf-8", errors="replace").read()
            written = set()
            for m in FILE_CALL.finditer(text):
                p, mode = m.group(1), m.group(2)
                if p and is_write_mode(mode) and not is_safe_path(p):
                    written.add(p)
            for m in WCF.finditer(text):
                p = m.group(1)
                if p and not is_safe_path(p):
                    written.add(p)
            if not written:
                continue
            checked += 1
            removed = set(m.group(1) for m in REMOVE.finditer(text))
            leaked = sorted(p for p in written if p not in removed)
            if leaked:
                bad.append((os.path.join(os.path.basename(d), name), leaked))
                print("%-26s 写了但没删：%s" % (name, leaked))

    print("\nchecked %d files with writes, %d leaking" % (checked, len(bad)))
    if bad:
        print("漏删会在每次差分测试后污染工作区（曾经把文件提交进仓库）")
        return 1
    print("没有仓库根目录的漏删")
    return 0


if __name__ == "__main__":
    sys.exit(main())

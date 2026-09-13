# -*- coding: utf-8 -*-
"""语料"假绿"体检：差分 PASS 不代表用例真的执行到了。

背景（真实教训）：生成器里一个引号写错，1150 条用例**全部**在同一个地方抛错，
两侧输出完全一致 → 差分测试报 PASS，但矩阵没验证任何东西。抽查输出才发现。

体检口径：跑一遍语料，按"用例标签"聚合结果：
  - 标签行形如 `tag=...` / `tag!错误` / `tag=值|err=...`；
  - 统计"整份文件里错误行占比"以及"是否所有用例都落在同一个错误上"；
  - 后者是最强的假绿信号（用例体自己坏了，比如调用了不存在的辅助函数）。

用法：
    python tools/corpus_health.py            # 全部语料
    python tools/corpus_health.py --terse    # 只打印有疑点的
"""
import os
import re
import subprocess
import sys
from collections import Counter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CORPUS = os.path.join(ROOT, "tests-corpus")
RUST = os.path.join(ROOT, "target", "release", "sq.exe")

sys.path.insert(0, os.path.join(ROOT, "tools"))
try:
    from safe_run import suppress_crash_dialogs
    suppress_crash_dialogs()
except Exception:
    pass

TAG_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)([=!])(.*)$")


def scan(path):
    p = subprocess.run([RUST, path], capture_output=True, cwd=ROOT, timeout=180)
    out = p.stdout.decode("utf8", "replace").replace("\r", "")
    cases = []
    for line in out.splitlines():
        m = TAG_RE.match(line)
        if not m:
            continue
        tag, sep, rest = m.groups()
        err = None
        if sep == "!":
            err = rest
        elif "|err=" in rest:
            e = rest.split("|err=", 1)[1].split("|", 1)[0]
            if e not in ("null", ""):
                err = e
        cases.append((tag, err))
    return p.returncode, cases, out


def main():
    terse = "--terse" in sys.argv
    files = sorted(f for f in os.listdir(CORPUS) if f.endswith(".nut"))
    suspicious = []
    print("%-26s %6s %6s %6s  %s" % ("file", "cases", "errors", "ratio", "top error"))
    print("-" * 96)
    for name in files:
        rc, cases, out = scan(os.path.join(CORPUS, name))
        if not cases:
            continue
        errs = [(t, e) for t, e in cases if e is not None]
        ratio = len(errs) / len(cases)
        top = Counter(e for _, e in errs).most_common(1)
        top_s = ("%d× %s" % (top[0][1], top[0][0][:44])) if top else ""
        flag = ""
        # 全部用例都出错，且集中在同一两个错误上 → 强烈的假绿信号
        if errs and len(errs) == len(cases) and top and top[0][1] >= len(errs) * 0.9:
            flag = "  <== 全部错误且集中，疑似用例体自己坏了"
            suspicious.append(name)
        if not terse or flag:
            print("%-26s %6d %6d %5.0f%%  %s%s" % (name, len(cases), len(errs),
                                                   100 * ratio, top_s, flag))
    print()
    # 汇总行保持 ASCII + 数字，方便 tools/check_all.py 提取摘要
    print("scanned %d files, %d suspicious" % (len(files), len(suspicious)))
    if suspicious:
        print("有疑点的文件：%s" % suspicious)
        return 1
    print("没有“全部用例集中报同一个错”的文件")
    return 0


if __name__ == "__main__":
    sys.exit(main())

# -*- coding: utf-8 -*-
"""二分定位：参考实现在生成的错误对拍语料里到底哪一行崩。"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import run  # noqa: E402

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
SRC = r"D:\OpenSource\squirrel-rust\tests-corpus\t_errmsg2.nut"
TMP = r"D:\OpenSource\squirrel-rust\tmp-c2\_bisect.nut"
OK_RC = (0, -2, 4294967294)


def run_lines(lines):
    with open(TMP, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(PRELUDE + lines) + "\n")
    _, _, rc = run(REF, [TMP], timeout=60)
    return rc


PRELUDE = [
    "function t(tag, f) {",
    '    try { print(tag + "=" + f() + "\\n"); }',
    '    catch (e) { print(tag + "!" + e + "\\n"); }',
    "}",
]


def main():
    src = open(SRC, encoding="utf-8").read().split("\n")
    calls = [l for l in src if l.startswith('t("')]
    print("调用点总数 %d" % len(calls))
    if run_lines(calls) in OK_RC:
        print("整份都不崩（可能只在 diff 环境下崩）")
        return 0
    lo, hi = 0, len(calls)
    while hi - lo > 1:
        mid = (lo + hi) // 2
        rc = run_lines(calls[:mid])
        if rc in OK_RC:
            lo = mid
        else:
            hi = mid
    print("崩溃点（第 %d 条）: %s" % (hi, calls[hi - 1].strip()))
    if hi < len(calls):
        print("下一条: %s" % calls[hi].strip())
    # 单独跑这一条
    rc = run_lines([calls[hi - 1]])
    print("单独跑该条的 rc=%s" % rc)
    return 0


if __name__ == "__main__":
    sys.exit(main())

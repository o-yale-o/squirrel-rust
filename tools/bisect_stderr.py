# -*- coding: utf-8 -*-
"""定位差分语料里「在 Rust 侧产生未捕获错误（stderr 非空）」的调用点。

用法：python tools/bisect_stderr.py tests-corpus/t_errmsg2.nut
"""
import os
import subprocess
import sys

RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
TMP = r"D:\OpenSource\squirrel-rust\tmp-c2\_bisect2.nut"


def has_stderr(lines, prelude):
    with open(TMP, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(prelude + lines) + "\n")
    p = subprocess.run([RUST, TMP], capture_output=True, timeout=60)
    return bool(p.stderr.strip()), p.stderr.decode("latin-1", "replace")


def main(path):
    src = open(path, encoding="utf-8").read().split("\n")
    calls = [l for l in src if l.startswith('t("')]
    prelude = [l for l in src if not l.startswith('t("') and not l.startswith("print(")]
    print("调用点 %d 个；先整份跑一次：" % len(calls))
    bad, err = has_stderr(calls, prelude)
    print("  有 stderr? %s" % bad)
    if not bad:
        return 0
    # 逐个增加前缀，找第一个出现 stderr 的点
    lo, hi = 0, len(calls)
    while hi - lo > 1:
        mid = (lo + hi) // 2
        bad, _ = has_stderr(calls[:mid], prelude)
        if bad:
            hi = mid
        else:
            lo = mid
    print("第一个产生未捕获错误的调用点（第 %d 条）: %s" % (hi, calls[hi - 1].strip()))
    bad, err = has_stderr([calls[hi - 1]], prelude)
    print("单独跑该条 stderr: %r" % err[:300])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))

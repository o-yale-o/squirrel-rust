# -*- coding: utf-8 -*-
"""性能对照：同一批微基准，Rust 引擎 vs C++ 参考实现。

做法：
- 两个引擎跑**同一个脚本**（`tools/bench/bench.nut <case>`），保证代码一致；
- 脚本内用 `clock()`（进程 CPU 秒，两边语义一致）自报耗时 → 排除进程启动、
  stdout 缓冲差异；外层再用墙钟跑 N 次取**最小值**复核；
- 稳定的取最小（机器噪声只会让某次更慢）。

用法：
    python tools/bench/run.py            # 全部用例，各 5 次
    python tools/bench/run.py fib loop   # 指定用例
"""
import os
import subprocess
import sys
import time

try:
    import os as _os, sys as _sys
    _d = _os.path.dirname(_os.path.abspath(__file__))
    for _p in (_d, _os.path.join(_d, ".."), _os.path.join(_d, "..", "..", "tools")):
        if _os.path.isdir(_p):
            _sys.path.insert(0, _p)
    from safe_run import suppress_crash_dialogs as _scd
    _scd()
except Exception as e:  # pragma: no cover
    print("warning: no suppress: %r" % (e,), file=sys.stderr)

ROOT = r"D:\OpenSource\squirrel-rust"
RUST = os.path.join(ROOT, "target", "release", "sq.exe")
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")
SCRIPT = os.path.join(ROOT, "tools", "bench", "bench.nut")

CASES = ["fib", "loop", "table", "array", "method", "closure", "meta", "str", "str_small", "stringfind", "generator"]
REPEAT = 5


def run(exe, case):
    """返回 (内部 clock() 秒, 墙钟秒, 输出行)"""
    best_wall = None
    inner = None
    out_line = ""
    for _ in range(REPEAT):
        t0 = time.perf_counter()
        r = subprocess.run([exe, SCRIPT, case], capture_output=True, cwd=ROOT, timeout=600)
        wall = time.perf_counter() - t0
        best_wall = wall if best_wall is None else min(best_wall, wall)
        text = r.stdout.decode("utf8", "replace").replace("\r", "").strip()
        parts = text.split("\t")
        if len(parts) >= 2:
            try:
                v = float(parts[1])
                inner = v if inner is None else min(inner, v)
            except ValueError:
                pass
        out_line = text.replace("\t", " ")
    return inner, best_wall, out_line


def main():
    cases = sys.argv[1:] or CASES
    print("%-11s %12s %12s %8s   %s" % ("case", "rust(clk)", "ref(clk)", "rust/ref", "wall rust/ref"))
    print("-" * 66)
    for case in cases:
        ri, rw, rout = run(RUST, case)
        ci, cw, cout = run(REF, case)
        if ri is None or ci is None:
            print("%-11s  失败：rust=%r ref=%r" % (case, rout[:40], cout[:40]))
            continue
        # clock() 在 Windows 上粒度约 15ms；太小的工作量不可比，标记一下
        tiny = " (<50ms，粒度噪声大)" if min(ri, ci) < 0.05 else ""
        ratio = (ri / ci) if ci > 0 else float("inf")
        print("%-11s %11.3fs %11.3fs %7.2fx   %.3fs/%.3fs%s"
              % (case, ri, ci, ratio, rw, cw, tiny))


if __name__ == "__main__":
    main()

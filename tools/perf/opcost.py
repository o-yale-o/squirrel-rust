# -*- coding: utf-8 -*-
"""精确的"每条字节码指令成本"对照（Rust 引擎 vs C++ 参考实现）。

为什么需要它：`tools/bench/run.py` 给的是**整体倍率**，把"紧循环"和"字符串/表操作"
混在一起，无法判断差距是"每条指令的固定开销"还是"某个原语慢"。

做法：
1. 用 `--max-ops` 二分出该脚本执行的**确切**指令条数 N。
   机制：主循环每条指令先把预算减 4096，减完若 <0 才报错，
   所以"能跑完的最小预算" = 4096 × N（`--max-ops 0` 表示不限制）。
2. 两个引擎跑同一个脚本，各 5 次取最小墙钟（脚本体量 >100ms，进程启动可忽略）。
3. 打印 ns/指令 与差额 —— 差额在"最简指令组合"和"含算术指令组合"上是否稳定，
   就能区分"固定开销"与"原语开销"。

用法：
    python tools/perf/opcost.py tools/perf/opmin.nut 8000000
    python tools/perf/opcost.py tools/perf/opcount.nut 8000000
（第二个参数是迭代次数，只用于打印"每迭代几条指令"。）
"""
import os
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
RUST = os.path.join(ROOT, "target", "release", "sq.exe")
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")
STEP = 4096
REPEAT = 5


def completes(exe, script, budget):
    # 注意：sq 对"脚本运行时错误"仍返回 0，必须连输出一起看
    p = subprocess.run([exe, "--max-ops", str(budget), script],
                       capture_output=True, timeout=600)
    text = (p.stdout + p.stderr).decode("utf8", "replace")
    return p.returncode == 0 and "budget exceeded" not in text


def count_instructions(script):
    if not completes(RUST, script, 1 << 62):
        raise SystemExit("脚本本身跑不完，先排查")
    lo, hi = 0, 1 << 62
    while hi - lo > STEP:
        mid = (lo + hi) // 2 // STEP * STEP
        if mid <= lo:
            mid = lo + STEP
        if completes(RUST, script, mid):
            hi = mid
        else:
            lo = mid
    return hi // STEP


def best_wall(exe, script):
    b = None
    for _ in range(REPEAT):
        t0 = time.perf_counter()
        subprocess.run([exe, script], capture_output=True, cwd=ROOT, timeout=600)
        w = time.perf_counter() - t0
        b = w if b is None else min(b, w)
    return b


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    script = sys.argv[1]
    per_iter = float(sys.argv[2]) if len(sys.argv) > 2 else None
    n = count_instructions(script)
    tr = best_wall(RUST, script)
    tc = best_wall(REF, script)
    print("script        : %s" % os.path.basename(script))
    print("instructions N: %d" % n)
    if per_iter:
        print("per iteration : %.3f 条指令" % (n / per_iter))
    print("rust          : %.3fs  ->  %.3f ns/指令" % (tr, tr * 1e9 / n))
    print("ref  (c++)    : %.3fs  ->  %.3f ns/指令" % (tc, tc * 1e9 / n))
    print("rust/ref      : %.2fx   (每条指令多 %.3f ns)"
          % (tr / tc, (tr - tc) * 1e9 / n))
    return 0


if __name__ == "__main__":
    sys.exit(main())

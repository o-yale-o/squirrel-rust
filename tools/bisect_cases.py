# -*- coding: utf-8 -*-
"""把探针文件里每个用例单独跑一遍（参考实现），找出会崩溃/差异的那一条。

用法：
    python tools/bisect_cases.py tools/probes/fmt_types.nut [--rust]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import run  # noqa: E402

ROOT = r"D:\OpenSource\squirrel-rust"
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")
RUST = os.path.join(ROOT, "target", "release", "sq.exe")
TMP = os.path.join(ROOT, "tmp-c2", "one")


def main(argv):
    both = "--rust" in argv
    paths = [a for a in argv if not a.startswith("--")]
    for path in paths:
        src = open(path, encoding="utf-8").read().split("\n")
        head = [l for l in src if not l.startswith(("s(", "print("))]
        cases = [l for l in src if l.startswith("s(")]
        os.makedirs(TMP, exist_ok=True)
        for i, case in enumerate(cases):
            p = os.path.join(TMP, "c%03d.nut" % i)
            with open(p, "w", encoding="utf-8", newline="\n") as f:
                f.write("\n".join(head) + "\n" + case + "\n")
            tag = case.split('"')[1] if '"' in case else case[:20]
            out_r, err_r, rc_r = run(REF, [p])
            line_r = out_r.replace(b"\r\n", b"\n").decode("latin-1").strip()
            msg = "OK" if rc_r == 0 else "REF-CRASH(%d)" % rc_r
            extra = ""
            if both:
                out_u, err_u, rc_u = run(RUST, [p])
                line_u = out_u.replace(b"\r\n", b"\n").decode("latin-1").strip()
                if (line_r, rc_r) != (line_u, rc_u):
                    msg = "DIFF"
                    extra = "\n      ref = %s\n      rst = %s" % (line_r[:150], line_u[:150])
            print("%-14s %-16s %s%s" % (msg, tag, line_r[:120], extra))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

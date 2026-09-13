# -*- coding: utf-8 -*-
"""具名参数块：找到合法写法（作为语句）。"""
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import run  # noqa: E402

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
TMP = r"D:\OpenSource\squirrel-rust\tmp-c2\_named2.nut"

PRE = 'function kw(a, b, c) { return "" + a + "-" + b + "-" + c; }\n'

CASES = [
    ("stmt_local", 'local r = kw(1) { b = 2, c = 3 };\nprint(r + "\\n");'),
    ("stmt_direct", 'kw(1) { b = 2, c = 3 };\nprint("ok\\n");'),
    ("stmt_then_use", 'local r;\nr = kw(1) { c = 3, b = 2 };\nprint(r + "\\n");'),
    ("stmt_partial", 'local r = kw(1) { b = 2 };\nprint(r + "\\n");'),
    ("stmt_extra", 'local r = kw(1) { b = 2, c = 3, d = 4 };\nprint(r + "\\n");'),
    ("stmt_missing", 'local r = kw(1) { };\nprint(r + "\\n");'),
    ("stmt_expr_val", 'local x = 9;\nlocal r = kw(1) { b = x, c = x * 2 };\nprint(r + "\\n");'),
    ("stmt_nested", 'function id(v) { return v; }\nlocal r = kw(id(1)) { b = 2, c = 3 };\nprint(r + "\\n");'),
    ("stmt_dynamic_name", 'local n = "b";\nlocal r = kw(1) { [n] = 2, c = 3 };\nprint(r + "\\n");'),
    ("stmt_dup", 'local r = kw(1) { b = 2, b = 3, c = 4 };\nprint(r + "\\n");'),
]


def main():
    bad = 0
    for name, body in CASES:
        with open(TMP, "w", encoding="utf-8", newline="\n") as f:
            f.write(PRE + body + "\n")
        o1, e1, r1 = run(REF, [TMP], timeout=15)
        o2, e2, r2 = run(RUST, [TMP], timeout=15)
        n1 = o1.replace(b"\r\n", b"\n")
        n2 = o2.replace(b"\r\n", b"\n")
        same = n1 == n2 and e1.replace(b"\r\n", b"\n") == e2.replace(b"\r\n", b"\n")
        if not same:
            bad += 1
        print("%-18s %-5s ref=%-30r rust=%r" % (name, "OK" if same else "DIFF", n1.strip()[:30], n2.strip()[:30]))
        if not same:
            print("     ref err: %r" % e1[:150])
            print("     rust err: %r" % e2[:150])
    print("\n差异 %d 个" % bad)
    return 0


if __name__ == "__main__":
    sys.exit(main())

# -*- coding: utf-8 -*-
"""数字字面量形态：哪些是 Squirrel 接受的。"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import run  # noqa: E402

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
TMP = r"D:\OpenSource\squirrel-rust\tmp-c2\_lit.nut"

FORMS = [
    "1.", ".5", "1e2", "1E2", "0x10", "0X1f", "0b1010", "010", "1_000",
    "9223372036854775807", "0x7FFFFFFFFFFFFFFF", "-0.0", "-0", "1.5e-3", ".0", "0.", "1.e2",
]


def main():
    bad = 0
    for form in FORMS:
        src = 'try { print("%s => " + (%s) + "\\n"); } catch (e) { print("%s ! " + e + "\\n"); }\n' % (form, form, form)
        with open(TMP, "w", encoding="utf-8", newline="\n") as f:
            f.write(src)
        o1, _, _ = run(REF, [TMP], timeout=15)
        o2, _, _ = run(RUST, [TMP], timeout=15)
        n1, n2 = o1.replace(b"\r\n", b"\n").strip(), o2.replace(b"\r\n", b"\n").strip()
        same = n1 == n2
        if not same:
            bad += 1
        print("%-22s %-5s ref=%-34r rust=%r" % (form, "OK" if same else "DIFF", n1[:34], n2[:34]))
    print("\n差异 %d 个" % bad)
    return 0


if __name__ == "__main__":
    sys.exit(main())

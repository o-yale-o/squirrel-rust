# -*- coding: utf-8 -*-
"""tointeger/tofloat 的进制参数语义 + 参考实现的崩溃边界（不弹崩溃框）。"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import run  # noqa: E402

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
TMP = r"D:\OpenSource\squirrel-rust\tmp-c2\_toint.nut"

CASES = [
    ("no_args", '"42".tointeger()'),
    ("base10", '"42".tointeger(10)'),
    ("base16", '"ff".tointeger(16)'),
    ("base16_prefix", '"0xff".tointeger(16)'),
    ("base2", '"1010".tointeger(2)'),
    ("base36", '"z".tointeger(36)'),
    ("base1", '"1".tointeger(1)'),
    ("base0", '"1".tointeger(0)'),
    ("base_neg", '"1".tointeger(-1)'),
    ("empty_base10", '"".tointeger(10)'),
    ("empty_1arg", '"".tointeger(1)'),
    ("two_args", '"1".tointeger(1, 2)'),
    ("bad_str", '"zz".tointeger()'),
    ("float_base", '"1".tointeger(1.5)'),
    ("float_arg", '"1.5".tointeger()'),
    ("tofloat_1arg", '"1.5".tofloat(1)'),
    ("tofloat_2arg", '"1.5".tofloat(1, 2)'),
]


def main():
    bad = 0
    for name, body in CASES:
        src = 'try { print("%s=" + (%s) + "\\n"); } catch (e) { print("%s!" + e + "\\n"); }\n' % (
            name, body, name)
        with open(TMP, "w", encoding="utf-8", newline="\n") as f:
            f.write(src)
        o1, _, r1 = run(REF, [TMP], timeout=15)
        o2, _, r2 = run(RUST, [TMP], timeout=15)
        crash = r1 not in (0, -2, 4294967294)
        same = o1.replace(b"\r\n", b"\n") == o2.replace(b"\r\n", b"\n")
        mark = "CRASH" if crash else ("OK" if same else "DIFF")
        if crash or not same:
            bad += 1
        print("%-16s %-6s %-38r %r" % (name, mark, o1.replace(b"\r\n", b"\n").strip()[:36],
                                       o2.replace(b"\r\n", b"\n").strip()[:36]))
    print("\n有问题 %d 个" % bad)
    return 0


if __name__ == "__main__":
    sys.exit(main())

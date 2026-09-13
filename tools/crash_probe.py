# -*- coding: utf-8 -*-
"""定位参考实现的崩溃点：逐条候选调用单独跑，报告退出码（不弹崩溃框）。"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import run  # noqa: E402

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
TMP = r"D:\OpenSource\squirrel-rust\tmp-c2\_crashprobe.nut"

CASES = [
    ("values_1", '{}.values(1);'),
    ("values_2", '{}.values(1, 2);'),
    ("keys_1", '{}.keys(1);'),
    ("keys_2", '{}.keys(1, 2);'),
    ("filter_1", '{}.filter(1);'),
    ("map_1", '{}.map(1);'),
    ("table_rawget_1", '{}.rawget(1);'),
    ("array_slice_1", '[].slice(1);'),
    ("string_slice_1", '"".slice(1);'),
    ("string_find_1", '"".find(1);'),
    ("string_strip_1", 'strip(1);'),
    ("string_escape_1", 'escape(1);'),
    ("format_bad", 'format(1);'),
    ("getstackinfos_1", 'getstackinfos(1);'),
    ("thread_wakeup_1", 'newthread(function(){}).wakeup(1);'),
    ("class_setattrs", '(class {}).setattributes(1);'),
    ("newmember_bad", '(class {}).newmember(1);'),
    ("weakref_ref_1", '({}).weakref().ref(1);'),
    ("gen_getstatus_1", '(function(){yield 1;}()).getstatus(1);'),
]


def main():
    bad = 0
    for name, body in CASES:
        src = "try { %s } catch (e) { print(\"caught: \" + e + \"\\n\"); }\n" % body
        with open(TMP, "w", encoding="utf-8", newline="\n") as f:
            f.write(src)
        try:
            o1, e1, r1 = run(REF, [TMP], timeout=15)
        except Exception as ex:
            o1, e1, r1 = b"", str(ex).encode(), -999
        try:
            o2, e2, r2 = run(RUST, [TMP], timeout=15)
        except Exception as ex:
            o2, e2, r2 = b"", str(ex).encode(), -999
        crash = r1 not in (0, -2, 4294967294) or r2 not in (0, -2, 4294967294)
        same = o1.replace(b"\r\n", b"\n") == o2.replace(b"\r\n", b"\n")
        mark = "CRASH" if crash else ("OK" if same else "DIFF")
        if crash or not same:
            bad += 1
        print("%-18s %-5s ref_rc=%-12s rust_rc=%-12s" % (name, mark, r1, r2))
        if not same or crash:
            print("     ref : %r" % o1.replace(b"\r\n", b"\n")[:120])
            print("     rust: %r" % o2.replace(b"\r\n", b"\n")[:120])
    print("\n有问题 %d 个" % bad)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

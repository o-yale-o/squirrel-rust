# -*- coding: utf-8 -*-
"""具名参数块（↔ FunctionCallArgs 里 _token=='{' 的分支）：值 SET 到调用结果上。"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import run  # noqa: E402

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
TMP = r"D:\OpenSource\squirrel-rust\tmp-c2\_named3.nut"

CASES = [
    ("table_result", 'function mk(a) { return { a = a }; }\nlocal r = mk(1) { b = 2 };\nprint(r.a + ":" + r.b + "\\n");'),
    ("class_ctor", 'class C { a = 0; b = 0; }\nlocal c = C() { a = 1, b = 2 };\nprint(c.a + ":" + c.b + "\\n");'),
    ("class_ctor_partial", 'class C { a = 0; b = 5; }\nlocal c = C() { a = 7 };\nprint(c.a + ":" + c.b + "\\n");'),
    ("instance_expr_key", 'class C { a = 0; }\nlocal k = "a";\nlocal c = C() { [k] = 3 };\nprint(c.a + "\\n");'),
    ("nested_values", 'class C { a = 0; b = 0; }\nlocal c = C() { a = 1 + 1, b = "x" + "y" };\nprint(c.a + ":" + c.b + "\\n");'),
    ("missing_target", 'function mk() { return "str"; }\nlocal r = mk() { a = 1 };\nprint(typeof r + "\\n");'),
    ("extra_ident", 'class C { a = 0; }\nlocal c = C() { zzz = 4 };\nprint("zzz" in c);'),
    ("no_block", 'class C { a = 1; b = 2; }\nlocal c = C();\nprint(c.a + ":" + c.b + "\\n");'),
]


def main():
    bad = 0
    for name, src in CASES:
        with open(TMP, "w", encoding="utf-8", newline="\n") as f:
            f.write(src + "\n")
        o1, e1, _ = run(REF, [TMP], timeout=15)
        o2, e2, _ = run(RUST, [TMP], timeout=15)
        n1, n2 = o1.replace(b"\r\n", b"\n"), o2.replace(b"\r\n", b"\n")
        same = n1 == n2 and e1.replace(b"\r\n", b"\n") == e2.replace(b"\r\n", b"\n")
        if not same:
            bad += 1
        print("%-20s %-5s ref=%-26r rust=%r" % (name, "OK" if same else "DIFF", n1.strip()[:26], n2.strip()[:26]))
        if not same:
            print("     ref err: %r" % e1[:160])
            print("     rust err: %r" % e2[:160])
    print("\n差异 %d 个" % bad)
    return 0


if __name__ == "__main__":
    sys.exit(main())

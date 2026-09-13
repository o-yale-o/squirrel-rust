# -*- coding: utf-8 -*-
"""具名参数块（named parameter block）与 .tochar 的数字字面量陷阱：找正确写法。"""
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import run  # noqa: E402

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
TMP = r"D:\OpenSource\squirrel-rust\tmp-c2\_named.nut"

CASES = [
    ("block_after", 'function kw(a,b){return ""+a+"-"+b;} print(kw(1) { b = 2 } + "\\n");'),
    ("block_semicolon", 'function kw(a,b){return ""+a+"-"+b;} print(kw(1) { b = 2 }; + "\\n");'),
    ("block_nl", 'function kw(a,b){return ""+a+"-"+b;}\nprint((kw(1) { b = 2 }) + "\\n");'),
    ("block_two", 'function kw(a,b,c){return ""+a+"-"+b+"-"+c;} print(kw(1) { b = 2, c = 3 } + "\\n");'),
    ("block_expr", 'function kw(a,b){return ""+a+"-"+b;} local x = 5; print(kw(1) { b = x + 1 } + "\\n");'),
    ("block_default", 'function kw(a,b=9){return ""+a+"-"+b;} print(kw(1) { } + "\\n");'),
    ("nested_call", 'function kw(a,b){return ""+a+"-"+b;} function g(v){return v;} print(kw(g(1)) { b = 2 } + "\\n");'),
    ("varargs", 'function kw(a,...){local s=""+a; foreach(v in vargv) s+="-"+v; return s;} print(kw(1) { } + "\\n");'),
    ("tochar_paren", 'print((65).tochar() + (66).tochar() + "\\n");'),
    ("tochar_floatlex", 'print(65.5.tochar() + "\\n");'),
]


def main():
    bad = 0
    for name, src in CASES:
        with open(TMP, "w", encoding="utf-8", newline="\n") as f:
            f.write(src + "\n")
        try:
            o1, e1, r1 = run(REF, [TMP], timeout=15)
        except Exception as ex:
            o1, e1, r1 = b"", str(ex).encode(), -999
        try:
            o2, e2, r2 = run(RUST, [TMP], timeout=15)
        except Exception as ex:
            o2, e2, r2 = b"", str(ex).encode(), -999
        same = o1.replace(b"\r\n", b"\n") == o2.replace(b"\r\n", b"\n")
        if not same:
            bad += 1
        print("%-16s %-5s ref=%-40r rust=%r" % (
            name, "OK" if same else "DIFF",
            o1.replace(b"\r\n", b"\n").strip()[:40], o2.replace(b"\r\n", b"\n").strip()[:40]))
        if not same:
            print("     ref err: %r" % e1[:160])
            print("     rust err: %r" % e2[:160])
    print("\n差异 %d 个" % bad)
    return 0


if __name__ == "__main__":
    sys.exit(main())

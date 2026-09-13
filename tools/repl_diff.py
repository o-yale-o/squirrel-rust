# -*- coding: utf-8 -*-
"""REPL 差分测试：把同一段交互输入喂给原版 sq_static.exe 与 Rust sq.exe，
比对 stdout + stderr + 退出码（自动做 CRT 文本模式换行与指针地址归一化）。

用法：
    python tools/repl_diff.py                    # 跑 tests-repl/*.txt
    python tools/repl_diff.py tests-repl/x.txt   # 跑指定会话
"""

try:  # 关掉系统错误对话框：参考实现在 UB 形态上会真崩溃并弹模态框
    import os as _os, sys as _sys
    _d = _os.path.dirname(_os.path.abspath(__file__))
    for _p in (_d, _os.path.join(_d, "..", "tools"), _os.path.join(_d, "tools")):
        if _os.path.isdir(_p):
            _sys.path.insert(0, _p)
    from safe_run import suppress_crash_dialogs as _scd
    _scd()
    del _d, _p, _os, _sys, _scd
except Exception as _e:  # pragma: no cover
    import sys as _sys2
    print("warning: 无法关闭崩溃对话框: %r" % (_e,), file=_sys2.stderr)
import os
import re
import subprocess
import sys

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
ROOT = r"D:\OpenSource\squirrel-rust"


def run_one(exe, data, timeout=10):
    try:
        p = subprocess.run([exe], input=data, capture_output=True, timeout=timeout)
        return p.stdout, p.stderr, p.returncode
    except subprocess.TimeoutExpired:
        return b"<TIMEOUT>", b"<TIMEOUT>", -999


def norm(b):
    b = b.replace(b"\r\n", b"\n")  # CRT 文本模式翻译
    return re.sub(rb"0x[0-9a-fA-F]+", b"0xPTR", b)


def main(paths):
    npass = nfail = 0
    for path in paths:
        data = open(path, "rb").read()
        orf, erf, rcr = run_one(REF, data)
        oru, eru, rcu = run_one(RUST, data)
        ok = norm(orf) == norm(oru) and norm(erf) == norm(eru) and rcr == rcu
        name = os.path.basename(path)
        if ok:
            npass += 1
            print("PASS %s" % name)
        else:
            nfail += 1
            print("FAIL %s (rc %d vs %d)" % (name, rcr, rcu))
            if norm(orf) != norm(oru):
                print("  ref.out : %r" % norm(orf)[:400])
                print("  rust.out: %r" % norm(oru)[:400])
            if norm(erf) != norm(eru):
                print("  ref.err : %r" % norm(erf)[:400])
                print("  rust.err: %r" % norm(eru)[:400])
    print("\n%d passed, %d failed" % (npass, nfail))
    return 0 if nfail == 0 else 1


if __name__ == "__main__":
    args = []
    for a in sys.argv[1:]:
        if os.path.isdir(a):
            args += [os.path.join(a, f) for f in sorted(os.listdir(a)) if f.endswith(".txt")]
        else:
            args.append(a)
    if not args:
        d = os.path.join(ROOT, "tests-repl")
        args = [os.path.join(d, f) for f in sorted(os.listdir(d)) if f.endswith(".txt")]
    sys.exit(main(args))

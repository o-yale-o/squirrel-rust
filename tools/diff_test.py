# -*- coding: utf-8 -*-
"""差分测试：同一 .nut 脚本分别在原版 sq_static.exe 与 Rust sq.exe 上执行，
比对 stdout、stderr 与退出码。

stderr 也要比——错误钩子（AN ERROR HAS OCCURRED / CALLSTACK / LOCALS）全部写 stderr，
只比 stdout 会让整条错误路径失去覆盖。加 --no-stderr 可退回旧行为。
"""
import re
import subprocess
import sys
import os

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
SAMPLES = r"D:\OpenSource\squirrel\samples"


def run_one(exe, path, timeout=10):
    try:
        p = subprocess.run([exe, path], capture_output=True, timeout=timeout)
        return p.stdout, p.stderr, p.returncode
    except subprocess.TimeoutExpired:
        return b"<TIMEOUT>", b"<TIMEOUT>", -999


def norm(b):
    b = b.replace(b"\r\n", b"\n")  # CRT stdout/stderr 文本模式翻译，非语言语义
    return re.sub(rb"0x[0-9a-fA-F]+", b"0xPTR", b)


def main(paths, check_stderr=True):
    npass = nfail = 0
    for path in paths:
        out_r, err_r, rc_r = run_one(REF, path)
        out_u, err_u, rc_u = run_one(RUST, path)
        same = norm(out_r) == norm(out_u) and rc_r == rc_u
        if check_stderr:
            same = same and norm(err_r) == norm(err_u)
        if same:
            npass += 1
            print("PASS %s" % os.path.basename(path))
        else:
            nfail += 1
            print("FAIL %s (rc %d vs %d)" % (os.path.basename(path), rc_r, rc_u))
            if norm(out_r) != norm(out_u):
                print("  stdout ref : %r" % norm(out_r)[:300])
                print("  stdout rust: %r" % norm(out_u)[:300])
            if check_stderr and norm(err_r) != norm(err_u):
                print("  stderr ref : %r" % norm(err_r)[:300])
                print("  stderr rust: %r" % norm(err_u)[:300])
    print("\n%d passed, %d failed" % (npass, nfail))
    return 0 if nfail == 0 else 1


if __name__ == "__main__":
    argv = [a for a in sys.argv[1:] if not a.startswith("--")]
    check_stderr = "--no-stderr" not in sys.argv[1:]
    args = []
    for a in argv:
        if os.path.isdir(a):
            args += [os.path.join(a, f) for f in sorted(os.listdir(a)) if f.endswith(".nut")]
        else:
            args.append(a)
    if not args:
        args = [os.path.join(SAMPLES, f) for f in sorted(os.listdir(SAMPLES)) if f.endswith(".nut")]
    sys.exit(main(args, check_stderr))

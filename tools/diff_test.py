# -*- coding: utf-8 -*-
"""差分测试：同一 .nut 脚本分别在原版 sq_static.exe 与 Rust sq.exe 上执行，
比对 stdout 与退出码。"""
import subprocess
import sys
import os

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"


def run_one(exe, path, timeout=10):
    try:
        p = subprocess.run([exe, path], capture_output=True, timeout=timeout)
        return p.stdout, p.returncode
    except subprocess.TimeoutExpired:
        return b"<TIMEOUT>", -999


def main(paths):
    npass = nfail = 0
    for path in paths:
        out_r, rc_r = run_one(REF, path)
        out_u, rc_u = run_one(RUST, path)
        # 指针地址会不同，规范化 0x... 形式
        def norm(b):
            import re
            b = b.replace(bytes([13, 10]), bytes([10]))  # CRT stdout 文本模式翻译，非语言语义
            return re.sub(rb"0x[0-9a-fA-F]+", b"0xPTR", b)
        same = norm(out_r) == norm(out_u) and rc_r == rc_u
        if same:
            npass += 1
            print("PASS %s" % os.path.basename(path))
        else:
            nfail += 1
            print("FAIL %s (rc %d vs %d)" % (os.path.basename(path), rc_r, rc_u))
            print("  ref: %r" % out_r[:200])
            print("  rust:%r" % out_u[:200])
    print("\n%d passed, %d failed" % (npass, nfail))
    return 0 if nfail == 0 else 1


if __name__ == "__main__":
    args = []
    for a in sys.argv[1:]:
        if os.path.isdir(a):
            args += [os.path.join(a, f) for f in sorted(os.listdir(a)) if f.endswith(".nut")]
        else:
            args.append(a)
    if not args:
        args = [os.path.join(r"D:\OpenSource\squirrel\samples", f)
                for f in sorted(os.listdir(r"D:\OpenSource\squirrel\samples")) if f.endswith(".nut")]
    sys.exit(main(args))

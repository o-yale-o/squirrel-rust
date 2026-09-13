# -*- coding: utf-8 -*-
"""`.cnut` 序列化矩阵：字节一致 + 双向交叉执行。

对每个 .nut 脚本：
1. 两个引擎各编译一次（`sq -o out.cnut -c script.nut`），比对 **SHA256**；
2. 交叉执行：C++ 编译出的字节码交给 Rust 跑、Rust 编译出的交给 C++ 跑，
   输出（归一化后）必须与「各自直接跑源码」一致。

用法：
    python tools/cnut_matrix.py                 # 官方 samples
    python tools/cnut_matrix.py tests-corpus    # 语料
"""
import hashlib
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import suppress_crash_dialogs  # noqa: E402

ROOT = r"D:\OpenSource\squirrel-rust"
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")
RUST = os.path.join(ROOT, "target", "release", "sq.exe")
TMP = os.path.join(ROOT, "tmp-c2", "cnut")
SAMPLES = r"D:\OpenSource\squirrel\samples"


def norm(b):
    b = b.replace(b"\r\n", b"\n")
    return re.sub(rb"0x[0-9a-fA-F]{8,}", b"0xPTR", b)


def run(exe, args, timeout=30):
    try:
        p = subprocess.run([exe] + list(args), capture_output=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return b"<TIMEOUT>", b"<TIMEOUT>", -999
    return norm(p.stdout), norm(p.stderr), p.returncode


def compile_to(exe, src, out):
    # 参考实现的怪癖：-o 必须写在脚本路径之前
    p = subprocess.run([exe, "-o", out, "-c", src], capture_output=True, timeout=30)
    if not os.path.exists(out) or p.returncode != 0:
        return None
    return hashlib.sha256(open(out, "rb").read()).hexdigest()


def main(argv):
    suppress_crash_dialogs()
    paths = []
    for a in argv:
        if os.path.isdir(a):
            paths += [os.path.join(a, f) for f in sorted(os.listdir(a)) if f.endswith(".nut")]
        else:
            paths.append(a)
    if not paths:
        paths = [os.path.join(SAMPLES, f) for f in sorted(os.listdir(SAMPLES)) if f.endswith(".nut")]
    os.makedirs(TMP, exist_ok=True)
    nbyte = ncross = nfail = 0
    for src in paths:
        name = os.path.basename(src)
        a = os.path.join(TMP, "ref_" + name + ".cnut")
        b = os.path.join(TMP, "rust_" + name + ".cnut")
        for f in (a, b):
            if os.path.exists(f):
                os.remove(f)
        ha = compile_to(REF, src, a)
        hb = compile_to(RUST, src, b)
        if ha is None or hb is None:
            # 编译失败（语法错误语料）不算序列化用例，交给 diff_test
            print("SKIP %s（其中一个引擎编译失败）" % name)
            continue
        if ha == hb:
            nbyte += 1
        else:
            nfail += 1
            print("BYTE-DIFF %s：%s vs %s" % (name, ha[:16], hb[:16]))
        # 交叉执行
        base_r, base_e, base_c = run(REF, [src])
        base_u, base_eu, base_cu = run(RUST, [src])
        x1 = run(RUST, [a])       # C++ 字节码 → Rust VM
        x2 = run(REF, [b])        # Rust 字节码 → C++ VM
        ok = x1[0] == base_u and x1[2] == base_cu and x2[0] == base_r and x2[2] == base_c
        if ok:
            ncross += 1
        else:
            nfail += 1
            print("CROSS-FAIL %s" % name)
            if x1[0] != base_u:
                print("  ref.cnut on rust: %r != %r" % (x1[0][:160], base_u[:160]))
            if x2[0] != base_r:
                print("  rust.cnut on ref : %r != %r" % (x2[0][:160], base_r[:160]))
    print("\n字节一致 %d/%d，交叉执行一致 %d/%d，失败 %d" % (nbyte, nbyte + nfail, ncross, nbyte + nfail, nfail))
    return 0 if nfail == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

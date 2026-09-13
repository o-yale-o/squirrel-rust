# -*- coding: utf-8 -*-
"""按「行」比较两个引擎的输出，只打印不一致的行（带上下文）。

diff_test.py 用整块字节比较、只打印前 300 字节，对大语料（几千行）看不出
到底哪一条用例不同。这个脚本用来定位。

用法：
    python tools/line_diff.py tests-corpus/t_opmx.nut            # 前 40 条差异
    python tools/line_diff.py tests-corpus/t_opmx.nut -n 200     # 前 200 条
    python tools/line_diff.py tests-corpus/t_opmx.nut --all      # 全部差异
    python tools/line_diff.py tests-corpus/t_opmx.nut --stderr   # 也比 stderr
"""
import difflib
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import run  # noqa: E402

ROOT = r"D:\OpenSource\squirrel-rust"
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")
RUST = os.path.join(ROOT, "target", "release", "sq.exe")


def norm(b):
    b = b.replace(b"\r\n", b"\n")
    # 同 diff_test.py：只归一化至少 8 位的十六进制（真正的指针），
    # 否则 format("%#x", ...) 的正常输出会被吞掉
    return re.sub(rb"0x[0-9a-fA-F]{8,}", b"0xPTR", b)


def lines(b):
    return norm(b).decode("latin-1").split("\n")


def main(argv):
    show = 40
    check_stderr = False
    args = []
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == "-n":
            i += 1
            show = int(argv[i])
        elif a == "--all":
            show = 10 ** 9
        elif a == "--stderr":
            check_stderr = True
        else:
            args.append(a)
        i += 1
    if not args:
        print(__doc__)
        return 2
    rc = 0
    for path in args:
        try:
            out_r, err_r, rc_r = run(REF, [path])
        except Exception as e:  # 超时/启动失败：参考实现自身的 UB 输入会这样
            out_r, err_r, rc_r = ("<REF-FAILED: %s>" % e).encode(), b"", -999
        try:
            out_u, err_u, rc_u = run(RUST, [path])
        except Exception as e:
            out_u, err_u, rc_u = ("<RUST-FAILED: %s>" % e).encode(), b"", -999
        lr, lu = lines(out_r), lines(out_u)
        print("== %s  rc %d vs %d  stdout %d vs %d 行" % (os.path.basename(path), rc_r, rc_u, len(lr), len(lu)))
        if check_stderr:
            er, eu = lines(err_r), lines(err_u)
            if er != eu:
                print("-- stderr 不同（%d vs %d 行）" % (len(er), len(eu)))
                d = [l for l in difflib.unified_diff(er, eu, "ref", "rust", lineterm="", n=1)][2:]
                print("\n".join(d[:20]))
        if lr == lu:
            print("stdout 完全一致")
            continue
        rc = 1
        shown = 0
        sm = difflib.SequenceMatcher(None, lr, lu, autojunk=False)
        for tag, i1, i2, j1, j2 in sm.get_opcodes():
            if tag == "equal":
                continue
            if shown >= show:
                print("... （还有更多差异，用 -n 调大）")
                break
            print("  ref[%d:%d] = %s" % (i1, i2, " | ".join(lr[i1:i2])[:400]))
            print("  rst[%d:%d] = %s" % (j1, j2, " | ".join(lu[j1:j2])[:400]))
            shown += 1
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

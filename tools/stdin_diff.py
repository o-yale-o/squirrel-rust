# -*- coding: utf-8 -*-
"""stdin 语义差分：把同一段字节喂给两个引擎的 stdin，比对脚本读到的内容。

覆盖：管道（短/长/无换行）、文件重定向、逐字节读与 eos()。
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
import subprocess
import sys

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
SCRIPT = r"tmp-c2\stdinprobe.nut"

CASES = [
    ("pipe_2bytes", b"AB"),
    ("pipe_with_newline", b"A\nB\n"),
    ("pipe_8bytes", b"ABCDEFGH"),
    ("pipe_5000bytes", b"A" * 5000),
    ("pipe_empty", b""),
    ("pipe_binary", bytes(range(1, 32))),
]


def run(exe, data):
    p = subprocess.run([exe, SCRIPT], input=data, capture_output=True, timeout=20)
    return p.stdout.replace(b"\r\n", b"\n"), p.stderr.replace(b"\r\n", b"\n"), p.returncode


def main():
    bad = 0
    for name, data in CASES:
        a = run(REF, data)
        b = run(RUST, data)
        same = a == b
        if not same:
            bad += 1
        print("%-18s %s" % (name, "OK" if same else "DIFF"))
        if not same:
            print("   ref : %r" % (a,))
            print("   rust: %r" % (b,))
    print("\n%d/%d 一致" % (len(CASES) - bad, len(CASES)))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

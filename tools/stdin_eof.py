# -*- coding: utf-8 -*-
"""逐个原语测量 stdin 在**管道**与**文件重定向**下的行为（参考实现 vs 本实现）。

每个用例单独起一个进程，避免 SDS 的 `eos()`（内部会做 fseek(SEEK_END)）污染流状态后
影响后面的用例。

用法：
    python tools/stdin_eof.py
"""
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import suppress_crash_dialogs  # noqa: E402

ROOT = r"D:\OpenSource\squirrel-rust"
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")
RUST = os.path.join(ROOT, "target", "release", "sq.exe")
PROBE = os.path.join(ROOT, "tools", "probes", "stdin_eof.nut")
TMP = os.path.join(ROOT, "tmp-c2")

CASES = [
    # 名字, 喂给 stdin 的字节
    ("empty", b""),
    ("1byte", b"A"),
    ("2bytes", b"AB"),
    ("4bytes", b"ABCD"),
    ("5000bytes", b"A" * 5000),
]


def run_pipe(exe, data, probe):
    p = subprocess.run([exe, probe], input=data, capture_output=True, timeout=30)
    return p.stdout.replace(b"\r\n", b"\n"), p.stderr.replace(b"\r\n", b"\n"), p.returncode


def run_file(exe, data, probe):
    path = os.path.join(TMP, "stdin_data.bin")
    with open(path, "wb") as f:
        f.write(data)
    with open(path, "rb") as f:
        p = subprocess.run([exe, probe], stdin=f, capture_output=True, timeout=30)
    return p.stdout.replace(b"\r\n", b"\n"), p.stderr.replace(b"\r\n", b"\n"), p.returncode


def main():
    suppress_crash_dialogs()
    bad = 0
    total = 0
    for label, runner in (("管道", run_pipe), ("文件重定向", run_file)):
        print("=== %s ===" % label)
        for name, data in CASES:
            a = runner(REF, data, PROBE)
            b = runner(RUST, data, PROBE)
            total += 1
            same = a == b
            if not same:
                bad += 1
            print("  %-11s %s" % (name, "OK" if same else "DIFF"))
            if not same:
                print("     ref : %r" % (a[0][:200],))
                print("     rust: %r" % (b[0][:200],))
                if a[1] or b[1]:
                    print("     ref-err : %r" % (a[1][:160],))
                    print("     rust-err: %r" % (b[1][:160],))
    print("\n%d/%d 一致" % (total - bad, total))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

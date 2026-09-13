# -*- coding: utf-8 -*-
"""eos()/tell()/len() 在不可 seek 句柄上的行为：区分 stdin/stdout 与吞掉多少字节。"""

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

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"

SCRIPT = r"""function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
{READS}
t("eos", function() { return stdin.eos(); });
t("tell", function() { return typeof stdin.tell(); });
t("len", function() { return typeof stdin.len(); });
t("read_after", function() { return typeof stdin.readn('b'); });
t("eos2", function() { return stdin.eos(); });
"""

CASES = [
    ("empty", b"", 0),
    ("2bytes_read0", b"AB", 0),
    ("2bytes_read1", b"AB", 1),
    ("2bytes_read2", b"AB", 2),
    ("8bytes_read2", b"ABCDEFGH", 2),
    ("8bytes_read8", b"ABCDEFGH", 8),
]


def run(exe, data, reads):
    body = "".join("t(\"r%d\", function() { return stdin.readn('b'); });\n" % i for i in range(reads))
    src = SCRIPT.replace("{READS}", body)
    open("tmp-c2/eosprobe.nut", "w", encoding="utf-8").write(src)
    p = subprocess.run([exe, "tmp-c2/eosprobe.nut"], input=data, capture_output=True, timeout=20)
    return p.stdout.replace(b"\r\n", b"\n").decode("latin-1").strip().replace("\n", " | ")


def main():
    bad = 0
    for name, data, reads in CASES:
        a = run(REF, data, reads)
        b = run(RUST, data, reads)
        same = a == b
        if not same:
            bad += 1
        print("%-16s %s" % (name, "OK" if same else "DIFF"))
        if not same:
            print("   ref : %s" % a)
            print("   rust: %s" % b)
    print("\n%d/%d 一致" % (len(CASES) - bad, len(CASES)))
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())

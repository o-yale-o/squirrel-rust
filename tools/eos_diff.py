# -*- coding: utf-8 -*-
"""eos()/tell()/len() 在不可 seek 句柄上的行为：区分 stdin/stdout 与吞掉多少字节。"""
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

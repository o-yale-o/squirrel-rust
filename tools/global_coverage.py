"""扫"哪些已注册的全局函数在语料里完全没被调用过"。

用途：找**结构性未覆盖面** —— 按委托表/类成员生成的矩阵碰不到的东西。
已知的两族都是这样发现的：
  - string 库全局函数（startswith/strip/split/...）：语料 0 次；
  - math 库定义域边界：有基础用例但尾位/边界没测。

用法：
    python tools/global_coverage.py            # 全部 62 个全局
    python tools/global_coverage.py 3          # 只列覆盖 <=3 次的
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CORPUS = os.path.join(ROOT, "tests-corpus")

sys.path.insert(0, os.path.join(ROOT, "tools"))
import audit_globals as ag  # noqa: E402


def corpus_text():
    parts = []
    for name in sorted(os.listdir(CORPUS)):
        if name.endswith(".nut"):
            with open(os.path.join(CORPUS, name), "r", encoding="utf-8", errors="replace") as f:
                parts.append(f.read())
    return "\n".join(parts)


def main():
    threshold = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    text = corpus_text()
    names = sorted(ag.cpp_globals().keys())
    rows = []
    for n in names:
        # 调用形态：`name(` ，且前面不是标识符字符（避免 `myprint(` 之类）
        cnt = len(re.findall(r"(?<![A-Za-z0-9_.])" + re.escape(n) + r"\s*\(", text))
        rows.append((cnt, n))
    rows.sort()
    zero = [n for c, n in rows if c == 0]
    low = [(c, n) for c, n in rows if 0 < c <= threshold]
    print("已注册全局函数 %d 个" % len(rows))
    print("\n[完全没出现在语料里] %d 个：" % len(zero))
    for i in range(0, len(zero), 6):
        print("   " + "  ".join("%-22s" % n for n in zero[i:i + 6]))
    if threshold:
        print("\n[覆盖 <=%d 次] %d 个：" % (threshold, len(low)))
        for c, n in low:
            print("   %-22s %d" % (n, c))
    print("\n[覆盖最好的 10 个]")
    for c, n in rows[-10:][::-1]:
        print("   %-22s %d" % (n, c))
    return 0


if __name__ == "__main__":
    sys.exit(main())

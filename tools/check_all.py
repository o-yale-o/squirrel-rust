# -*- coding: utf-8 -*-
"""一条命令跑完全部验证（可复现的验收入口）。

包含八项差分/审计 + 语料体检 + Rust 测试。任何一项失败都会在最后汇总成 FAIL，
并且退出码非 0 —— 所以可以直接当作 CI 入口用。

    python tools/check_all.py
    python tools/check_all.py --quick     # 跳过 cargo test（只跑差分与审计，约 1 分钟）

各项的含义：
  corpus        tests-corpus/ 下全部语料，两侧比 stdout+stderr+退出码（归一化 CRLF 与指针）
  samples       官方 samples/ 17 个脚本
  compile-err   非法源码的编译错误文本对拍（60 条）
  cnut          .cnut 字节一致 + 双向交叉执行（C++ 编译的给 Rust 跑，反之亦然）
  lint          语料自检（文件能编译、不残留副作用文件等）
  health        语料"假绿"体检：用例是否真的执行到了（详见 tools/corpus_health.py）
  audit-*       全局函数表 / 委托表 / 根表插入顺序与参考实现逐项对照
  cargo test    单元 + 集成 + 健壮性 + 字节码互通 + 各语义矩阵
"""
import argparse
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PY = sys.executable

STEPS = [
    ("corpus", [PY, "tools/diff_test.py", "tests-corpus"]),
    ("samples", [PY, "tools/diff_test.py"]),
    ("compile-err", [PY, "tools/errmsg_matrix.py"]),
    ("cnut", [PY, "tools/cnut_matrix.py", "tests-corpus"]),
    ("lint", [PY, "tools/lint_corpus.py"]),
    ("health", [PY, "tools/corpus_health.py", "--terse"]),
    ("audit-globals", [PY, "tools/audit_globals.py"]),
    ("audit-delegates", [PY, "tools/audit_delegates.py"]),
    ("audit-regorder", [PY, "tools/audit_regorder.py"]),
]
CARGO_TEST = ("cargo test", ["cargo", "test", "--release"])


def ascii_summary(text, width=64):
    """摘要行只保留 ASCII 片段（控制台可能是 GBK，中文会乱码）。"""
    for line in reversed([l.strip() for l in text.splitlines() if l.strip()]):
        a = "".join(ch if 32 <= ord(ch) < 127 else " " for ch in line).strip()
        if any(ch.isdigit() for ch in a):
            return " ".join(a.split())[:width]
    return ""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--quick", action="store_true", help="跳过 cargo test")
    args = ap.parse_args()

    steps = list(STEPS) + ([] if args.quick else [CARGO_TEST])
    failed = []
    print("%-18s %-5s | %s" % ("step", "state", "summary"))
    print("-" * 96)
    for name, cmd in steps:
        r = subprocess.run(cmd, capture_output=True, cwd=ROOT)
        text = (r.stdout + r.stderr).decode("utf8", "replace")
        ok = r.returncode == 0
        if not ok:
            failed.append(name)
        print("%-18s %-5s | %s" % (name, "OK" if ok else "FAIL", ascii_summary(text)))
    print("-" * 96)
    if failed:
        print("失败项：%s" % ", ".join(failed))
        return 1
    print("全部通过（%d 项%s）" % (len(steps), "，已跳过 cargo test" if args.quick else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())

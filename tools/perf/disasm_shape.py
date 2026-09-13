# -*- coding: utf-8 -*-
"""从 dumpbin 反汇编里抽取某个符号的指令区间，统计**分派形态**。

要回答的问题：解释器的 `match opcode` 到底被编成了跳转表，还是比较链？
比较链意味着每条指令要过一串预测分支 —— 那会表现为"与操作码无关的固定开销"。

用法：
    tools\\perf\\disasm_dump.bat                       # 先生成 tmp-c2\\disasm2.txt
    python tools/perf/disasm_shape.py run_instructions

输出：指令数 / 助记符 Top / 分支统计 / 间接 jmp 分类。
判据：出现 `lea rXX,[表]` + `movsxd rXX,[rXX+rXX*4]` + `add` + `jmp rXX` 即跳转表。
"""
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
DEFAULT_DUMP = ROOT / "tmp-c2" / "disasm2.txt"

HEAD = re.compile(r"^(\S+):$")
BODY = re.compile(r"^\s+[0-9A-Fa-f]{16}:\s+((?:[0-9A-Fa-f]{2} )+)\s*([a-z][a-z0-9]*)\s*(.*?)\s*$")


def parse(path):
    """返回 [(symbol, [(mnemonic, operand)])]"""
    syms = []
    cur_name = None
    cur = []
    with open(path, "r", errors="replace") as f:
        for line in f:
            line = line.rstrip("\n")
            m = HEAD.match(line)
            if m:
                if cur_name is not None:
                    syms.append((cur_name, cur))
                cur_name = m.group(1)
                cur = []
                continue
            m = BODY.match(line)
            if m and cur_name is not None:
                cur.append((m.group(2), m.group(3)))
    if cur_name is not None:
        syms.append((cur_name, cur))
    return syms


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    needle = sys.argv[1]
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 3
    dump = Path(sys.argv[3]) if len(sys.argv) > 3 else DEFAULT_DUMP
    if not dump.exists():
        print("找不到 %s —— 先跑 tools\\perf\\disasm_dump.bat" % dump)
        return 1
    syms = parse(dump)
    hits = [(n, b) for n, b in syms if needle in n]
    print("dump 里符号总数 %d，匹配 %r 的符号 %d 个" % (len(syms), needle, len(hits)))
    for name, body in hits[:limit]:
        mn = Counter(m for m, _ in body)
        total = sum(mn.values())
        branches = sum(v for k, v in mn.items() if k.startswith("j") or k in ("call", "ret"))
        cmps = mn.get("cmp", 0) + mn.get("test", 0)
        indirect = []
        for m, o in body:
            if m != "jmp":
                continue
            if re.match(r"^[0-9A-Fa-f]{8,16}$", o.strip()):  # 直接跳转
                continue
            indirect.append(o)
        reg_jmp = [o for o in indirect if re.match(r"^r[a-z0-9]+$", o.strip())]
        mem_jmp = [o for o in indirect if "*" in o or "qword" in o]
        moves = sum(mn.get(k, 0) for k in ("mov", "push", "pop", "lea", "movaps", "movdqa", "movups"))
        print("\n== %s ==" % name)
        print("  指令数 %d, 分支/调用 %d, cmp/test %d" % (total, branches, cmps))
        print("  间接 jmp %d (寄存器 %d / 内存 %d)  <- 寄存器跳转通常就是跳转表"
              % (len(indirect), len(reg_jmp), len(mem_jmp)))
        print("  数据搬运(mov/push/pop/lea/movaps/movdqa/movups) %d = %.0f%%  <- 占比高说明寄存器分配差"
              % (moves, 100.0 * moves / max(total, 1)))
        print("  Top: %s" % ", ".join("%s=%d" % kv for kv in mn.most_common(12)))
        if indirect:
            print("  间接 jmp 样例: %s" % indirect[:4])
    return 0


if __name__ == "__main__":
    sys.exit(main())

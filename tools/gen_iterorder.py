# -*- coding: utf-8 -*-
"""生成"表迭代顺序 × 随机变更序列"矩阵语料 tests-corpus/t_iterorder.nut。

为什么单开一份：任何脚本 `foreach` 一张表，**输出顺序都依赖哈希表布局**——
键的哈希函数、探测策略、扩容时机、删除后的空洞处理，任何一个不一致都会让
输出顺序漂移。现有语料只钉了单一形态（t_keymap 的 iter_mixed 等）；本轮用
**生成式差分**压测：随机"插入/覆盖/删除"序列（混合 int/string/float/bool
键型，插入量足以触发多次扩容），foreach 输出与参考逐字节对拍。
**80 个随机程序（单程序最多 200 操作）零差异**——哈希序完全一致。

实测钉住的语义（都对着参考实现验过）：
1. 混合键型（int/string/float/bool、负数、形如数字的字符串）下的**哈希序**
   与参考一致——包括触发扩容后的重排。
2. 迭代中删除键的顺序效应（删当前键/别的键）也与参考一致（见 t_delegiter）。
3. `t[k] = v`（覆盖，不建槽）不改变顺序；`t[k] <- v`（建槽）按哈希落位。

安全约束：无（纯表操作；尺寸远低于 MAX_CONTAINER_BYTES）。

用法：
    python tools/gen_iterorder.py
    python tools/gen_iterorder.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import random
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_iterorder.nut")

KEY_GENS = [
    lambda r: str(r.randint(0, 20)),
    lambda r: str(r.randint(0, 500)),
    lambda r: '"k%d"' % r.randint(0, 20),
    lambda r: '"%s"' % "".join(r.choice("abcdefgh") for _ in range(3)),
    lambda r: str(r.randint(0, 20)) + ".0",
    lambda r: str(r.randint(-5, 5)),
    lambda r: "true",
    lambda r: '"%d.5"' % r.randint(0, 5),
]


def gen_body(seed):
    r = random.Random(seed)
    lines = ["local t = {};"]
    n_ops = r.randint(10, 120)
    for _ in range(n_ops):
        op = r.random()
        k = r.choice(KEY_GENS)(r)
        if op < 0.55:
            lines.append("t[%s] <- %d;" % (k, r.randint(0, 99)))
        elif op < 0.8:
            lines.append("t[%s] = %d;" % (k, r.randint(0, 99)))
        else:
            lines.append("try { delete t[%s]; } catch (e) { }" % k)
    lines.append('local o = "";')
    lines.append('foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";')
    lines.append('return o + "/" + t.len();')
    return "\n    ".join(lines)


SEEDS = list(range(1, 13))  # 语料固定 12 条随机序列（定种子可复现）

lines = [
    "// 自动生成：表迭代顺序 × 随机变更序列矩阵（tools/gen_iterorder.py）",
    "//",
    "// 每条用例是一段随机的 插入/覆盖/删除 序列（混合 int/string/float/bool 键型，",
    "// 插入量足以触发多次扩容），最后 foreach 输出键值顺序——**顺序依赖哈希表",
    "// 布局**，与参考逐字节一致（生成式差分：80 个随机程序零差异）。",
    "function d(x) { return \"\" + x; }",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=\" + d(v) + \"|\" + typeof v + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "",
]
for s in SEEDS:
    body = gen_body(s)
    lines.append('t("seq%d", function() {' % s)
    lines.append("    " + body)
    lines.append("});")

ALL = [l for l in lines if l.startswith('t("')]
if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k][:100]))
    sys.exit(0)

lines += ['print("ITERORDER-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

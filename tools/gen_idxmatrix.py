# -*- coding: utf-8 -*-
"""生成"索引值边界"矩阵语料 tests-corpus/t_idxmap.nut。

为什么单开一份：`t_delmatrix.nut` 覆盖的是方法 × 接收者 × **实参类型**
（`("").slice(-1)`、`([]).insert(-1)` 这种缺参形态），**值**的形状
—— 双参数组合、INT64 极值、浮点索引、find 的起始偏移 —— 一个都没有。
这一轮是**验证型**：118 条探针两侧逐字节一致（唯一的"差异"是指针文本长度），
语料把这块空白钉住。

语义要点（对着参考实现写，均已实测）：
1. `slice(start[, end])`：负数从尾部数；`end` 缺省=尾部；**start > end → 空串/空数组**；
   越界被夹进 [0, len]；浮点先 `tointeger` 截断。
2. `find(sub[, start])`：`start` 越界/负数都被夹；找不到返回 `null`。
3. `insert(idx, val)` / `remove(idx)`：idx 必须在 [0, len]（insert）/[0, len)
   （remove）内，否则 `index out of range`；浮点截断。
4. `resize(n[, fill])`：负数报 `resizing to negative length`；缺省 fill 是 `null`。

安全约束（**参考实现会死的输入，一律不进语料**，实测见 tmp-c2/rz2_*.nut）：
- `array(N)` / `resize(N)` / `blob(N)` 的**超大 N**（≥2^31）：
  参考实现 `resize(9223372036854775807)` → 0xC0000005 崩溃、
  `resize(2147483647)`/`resize(4294967296)` → 分配几十 GB，Windows 错误报告
  挂死整台机器。本实现给可捕获的 `container too big`（`MAX_CONTAINER_BYTES`
  = 2 GiB，见 config.rs）——**有意偏离**，已记入 MIGRATION.md。
- `blob(4).readblob(-1)`：参考实现把负尺寸转无符号后巨型分配，直接挂死；
  本实现夹成 0 → `no data left to read`。

用法：
    python tools/gen_idxmatrix.py
    python tools/gen_idxmatrix.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_idxmap.nut")

IMIN = "-9223372036854775808"
IMAX = "9223372036854775807"

# slice 双参数组合（值都安全：纯计算，无分配）
PAIRS = [
    ("nn5", "-5", "-5"), ("nm1m5", "-5", "-1"), ("rev", "-1", "-5"),
    ("z10", "1", "0"), ("z21", "2", "1"), ("bb", "-100", "-99"),
    ("oe", "3", "100"), ("eo", "100", "3"), ("min0", "0", IMIN),
    ("0min", IMIN, "0"), ("minmax", IMIN, IMAX), ("maxmin", IMAX, IMIN),
    ("f", "1.9", "3.9"), ("fn", "-2.5", "-1.5"), ("s0e", "0", "5"),
    ("s0x", "0", "6"), ("s5", "5", "5"), ("s6", "6", "6"),
]
# 单参数极值
SINGLES = [
    ("i", "0"), ("m1", "-1"), ("m6", "-6"), ("m100", "-100"),
    ("mn", IMIN), ("mx", IMAX), ("fl", "1.5"), ("fln", "-1.5"),
]
# find 起始偏移
FINDS = [
    ("fn100", "-100"), ("fm1", "-1"), ("f0", "0"), ("f1", "1"), ("f2", "2"),
    ("f4", "4"), ("f5", "5"), ("f6", "6"), ("f100", "100"),
    ("fmin", IMIN), ("fmax", IMAX), ("ffl", "2.5"), ("ffln", "-2.5"),
]
# insert/remove 索引
INSRM = [
    ("m6", "-6"), ("m1", "-1"), ("z0", "0"), ("p5", "5"), ("p6", "6"),
    ("p100", "100"), ("min", IMIN), ("max", IMAX), ("fl", "1.5"), ("fln", "-1.5"),
]
# resize 安全值（语料里只用这些；超大值会弄死参考实现，见模块 docstring）
RESIZES = [
    ("r0", "0"), ("r1", "1"), ("r3", "3"), ("r5", "5"), ("r8", "8"),
    ("rfl", "2.9"), ("rfln", "0.5"),
]

lines = [
    "// 自动生成：索引值边界矩阵（tools/gen_idxmatrix.py）",
    "//",
    "// 观测口径：数组用 dump()、字符串用 len:escape()，**绝不直接 print 容器**",
    "//（`(table : 0x…)` 的指针文本两侧本来就不同，别给差分添噪声）。",
    "// **不含**超大的 resize/array/blob 尺寸：参考实现在那些输入上直接崩溃或",
    "// 把系统拖死（本实现给可捕获的 container too big，见 MIGRATION.md）。",
    "function dump(a) {",
    "    local s = \"[\";",
    "    for (local i = 0; i < a.len(); i++) {",
    "        if (i > 0) s += \",\";",
    "        local v = a[i];",
    "        s += (typeof v == \"string\" ? \"\\\"\" + v + \"\\\"\" : \"\" + v);",
    "    }",
    "    return s + \"]\";",
    "}",
    "function esc(s) { return s.len() + \":\" + escape(s); }",
    "function t(tag, f) {",
    "    try { local v = f(); local o = typeof v; print(tag + \"=\" + (o == \"array\" ? dump(v) : (o == \"string\" ? esc(v) : \"\" + v)) + \"|\" + o + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "local S = \"hello\";",
    "local A = [1, 2, 3, 4, 5];",
    "",
]
n = 0
for nm, a, b in PAIRS:
    lines.append('t("ss_%s", function() { return S.slice(%s, %s); });' % (nm, a, b))
    lines.append('t("as_%s", function() { return A.slice(%s, %s); });' % (nm, a, b))
    n += 2
for nm, a in SINGLES:
    lines.append('t("s1_%s", function() { return S.slice(%s); });' % (nm, a))
    lines.append('t("a1_%s", function() { return A.slice(%s); });' % (nm, a))
    n += 2
for nm, a in FINDS:
    lines.append('t("fd_%s", function() { return S.find("l", %s); });' % (nm, a))
    n += 1
lines += [
    't("fd_empty", function() { return S.find(""); });',
    't("fd_empty2", function() { return S.find("", 2); });',
    't("fd_empty6", function() { return S.find("", 6); });',
    't("fd_miss", function() { return S.find("z"); });',
    't("fd_self", function() { return S.find("hello"); });',
    't("fd_self3", function() { return S.find("hello", 3); });',
]
n += 6
for nm, a in INSRM:
    lines.append('t("ins_%s", function() { local c = [1, 2, 3, 4, 5]; c.insert(%s, 99); return dump(c); });' % (nm, a))
    lines.append('t("rm_%s", function() { local c = [1, 2, 3, 4, 5]; return "" + c.remove(%s) + ":" + dump(c); });' % (nm, a))
    n += 2
for nm, a in RESIZES:
    lines.append('t("rz_%s", function() { local c = [1, 2, 3, 4, 5]; c.resize(%s); return dump(c); });' % (nm, a))
    lines.append('t("rzf_%s", function() { local c = [1, 2]; c.resize(%s, "x"); return dump(c); });' % (nm, a))
    n += 2
lines += [
    't("rz_null_fill", function() { local c = [1]; c.resize(3); return dump(c); });',
    't("ix_max", function() { return S[%s]; });' % IMAX,
    't("ix_min", function() { return S[%s]; });' % IMIN,
    't("ix_fl", function() { return S[1.9]; });',
    't("ixa_max", function() { return A[%s]; });' % IMAX,
    't("ixa_min", function() { return A[%s]; });' % IMIN,
    't("ixa_fl", function() { return A[2.9]; });',
    't("s3", function() { return S.slice(1, 2, 3); });',
    't("s_str", function() { return S.slice("1"); });',
    't("s_null", function() { return S.slice(null); });',
    't("a3", function() { return A.slice(1, 2, 3); });',
    # blob seek 越界与增长（守卫不得改变可达范围内的语义）
    't("bsk_past", function() { local b = blob(4); return b.seek(8, \'b\'); });',
    't("bsk_neg", function() { local b = blob(4); return b.seek(-8, \'b\'); });',
    't("bsk_max", function() { local b = blob(4); return b.seek(%s, \'b\'); });' % IMAX,
    't("bsk_end_past", function() { local b = blob(4); return b.seek(4, \'e\'); });',
    't("bsk_end_neg", function() { local b = blob(4); return b.seek(-8, \'e\'); });',
    't("bgrow_write", function() { local b = blob(4); b.seek(4, \'b\'); b.writen(7, \'b\'); return b.len() + ":" + b.tell(); });',
    't("bgrow_far", function() { local b = blob(4); b.seek(4, \'b\'); b.writen(7, \'i\'); return b.len() + ":" + b.tell(); });',
    't("bwdef", function() { local b = blob(); b.writen(1, \'b\'); return b.len() + ":" + b.tell(); });',
    't("breadback", function() { local b = blob(4); b.seek(0, \'b\'); b.writen(0x41, \'b\'); b.seek(0, \'b\'); return b.readn(\'b\'); });',
    't("brb_zero", function() { local b = blob(4); return b.readblob(0); });',
    't("brb_two", function() { local b = blob(4); local r = b.readblob(2); return r.len() + ":" + b.tell(); });',
    't("brb_all", function() { local b = blob(4); local r = b.readblob(99); return r.len() + ":" + b.tell(); });',
]
n += 22

ALL = [l for l in lines if l.startswith('t("')]
if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k][:100]))
    sys.exit(0)

lines += ['print("IDXMAP-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

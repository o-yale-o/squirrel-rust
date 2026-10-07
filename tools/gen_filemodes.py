# -*- coding: utf-8 -*-
"""生成"file 模式串 × 文本/二进制"矩阵语料 tests-corpus/t_filemodes.nut。

为什么单开一份：语料里 file 的模式串只出现过 5 种（wb+/rb/wb/w/ab），
**文本模式**（无 `b` 的所有模式）完全没测过——参考实现走 CRT 文本流：
**写入的 `\n` 落盘为 `\r\n`**、读取时 `\r\n`→`\n` 且**遇 Ctrl+Z 截断**。
本实现原来永远二进制——`file(p, "w")` 写 `\n` 的脚本（写文本文件的真实用法）
落盘字节与参考不同。P32 已修（`stream.rs` 的 FileBackend 文本翻译），
本矩阵（16 种模式 × 写入/读回/len/tell）钉住。

实测钉住的语义（都对着参考实现验过）：
1. **翻译规则**：`"w"`/`"wt"`/`"w+"`/`"w+t"`/`"a"`/`"a+t"` 等无 `b` 模式写入时
   `\n`（0x0A）落盘 `\r\n`（0x0D 0x0A）；二进制模式（`"wb"`/`"rb"`/`"wb+"`）
   原样。读回（`"r"` 文本）时 `\r\n`→`\n`。
2. **tell/len 报物理字节数**：文本模式写 3 字符（含 \n）→ tell/len = 4。
3. **Ctrl+Z（0x1A）截断**：文本读遇 0x1A 即 EOF（读短 → readn 报 `io error`）；
   二进制读不受影响。
4. **seek 后读裸字节**：文本模式 seek 后读出的是物理字节（绕过翻译）。
5. **`b` 与 `t` 同用**（`"wbt"`）：参考实现 fopen 判无效参数 → `0xC0000409`
   **崩溃**；本实现给**可捕获**的 `cannot open file`（有意偏离，
   与偏离表"非法模式串"条目同类）。

安全约束：**`wbt`（b 与 t 同用）不进语料**——参考实现 fopen 判无效参数直接
崩溃（`0xC0000409`，进程 rc=3221226505，diff_test 无法对拍）；本实现的
`cannot open file` 确定性行为钉在 `stream_semantics.rs` 的守门测试里。

用法：
    python tools/gen_filemodes.py
    python tools/gen_filemodes.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_filemodes.nut")

MODES = ["r", "rb", "w", "wb", "a", "ab", "r+", "rb+", "w+", "wb+",
         "wt", "rt", "w+t", "a+t", "r+t"]

lines = [
    "// 自动生成：file 模式串 × 文本/二进制矩阵（tools/gen_filemodes.py）",
    "//",
    "// **文本模式**（无 `b`）写入 LF→CRLF、读取 CRLF→LF 且遇 Ctrl+Z 截断、",
    "// seek 后读裸字节、tell/len 报物理字节数——全部与参考逐字节一致。",
    "// `wbt`（b 与 t 同用）：参考实现 fopen 判无效参数 → 0xC0000409 崩溃；",
    "// 本实现给可捕获的 `cannot open file`（**有意偏离**，入语料钉住本侧行为）。",
    "// 每条用例开头先 remove 探针文件，避免上一次运行的残留影响追加模式。",
    "function d(x) { return \"\" + x; }",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=\" + d(v) + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "",
]
for m in MODES:
    tag = "".join(ch for ch in m if ch.isalnum())
    lines.append('t("%s", function() {' % tag)
    lines.append('    try { remove("fm_probe_%s.bin"); } catch (e) { }' % tag)
    lines.append('    local f = file("fm_probe_%s.bin", "%s");' % (tag, m))
    lines.append("    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');")
    lines.append("    local tell = f.tell();")
    lines.append("    f.close();")
    lines.append('    local g = file("fm_probe_%s.bin", "rb");' % tag)
    lines.append("    local n = g.len();")
    lines.append('    local o = "";')
    lines.append("    while (!g.eos()) o += g.readn('b') + \",\";")
    lines.append("    g.close();")
    lines.append('    remove("fm_probe_%s.bin");' % tag)
    lines.append('    return "tell=" + tell + " len=" + n + " bytes=" + o;')
    lines.append("});")

ALL = [l for l in lines if l.startswith('t("')]
if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k][:110]))
    sys.exit(0)

lines += ['print("FILEMODES-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

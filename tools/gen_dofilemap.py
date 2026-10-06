# -*- coding: utf-8 -*-
"""生成"dofile/loadfile 边界"矩阵语料 tests-corpus/t_dofilemap.nut。

为什么单开一份：`t_loadfile_bom` 测的是**编码**（BOM/UTF-16）；本轮测的是
**路径与错误形态**——不存在、空串、目录作为路径、`./` 与 `\\` 形态、带空格
路径、文件中途抛错、语法错文件、空文件/纯注释文件、第 2 参（printerror）语义、
返回值与全局副作用。这些形态现有生成器碰不到（需要"脚本自建自删伴生文件"）。
本轮为**验证型**：31 行探针两侧逐字节一致。

实测钉住的语义（都对着参考实现验过）：
1. **错误文本统一是 `cannot open the file`**：不存在/空串/**目录**作为路径
   全都一样（目录也不特殊报错）；`loadfile` 同样报错（而不是返回 null——
   **注意**：`loadfile` 打不开时是**抛错**，不是返回 null）。
2. **第 2 参（printerror）只控制 stderr 打印，不影响抛错**：`false` 也照样抛
   `cannot open the file`。`loadfile` 的第 2 参同理。
3. dofile **返回文件的 return 值**；文件里 `<-` 建的槽进**根表**（全局副作用）。
4. 文件中途抛错 → 错误**穿透** dofile/loadfile 调用方（`before` 打印、`after`
   不打印）；loadfile 返回的闭包可以**重复调用**（每次都完整重跑）。
5. 空文件/纯注释文件 → 返回 null；`./x.nut` 与 `.\\x.nut` 与 `x.nut` 等价；
   带空格路径正常。
6. **语法错文件**：dofile/loadfile 都抛 `expression expected`（编译错误穿透）。

用法：
    python tools/gen_dofilemap.py
    python tools/gen_dofilemap.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_dofilemap.nut")

lines = [
    "// 自动生成：dofile/loadfile 边界矩阵（tools/gen_dofilemap.py）",
    "//",
    "// 脚本**自建自删伴生文件**（不依赖仓库内静态文件）。错误文本统一是",
    "// `cannot open the file`（目录作为路径也不特殊报错）；**loadfile 打不开时",
    "// 是抛错而非返回 null**；第 2 参（printerror）只控制 stderr 打印不影响抛错；",
    "// dofile 返回文件的 return 值、文件里的 <- 槽进根表；文件中途抛错会**穿透**",
    "// 到调用方。两侧逐字节一致（含 stderr）。",
    "function mkText(path, content) {",
    "    local f = file(path, \"wb\");",
    "    foreach (c in content) f.writen(c, 'b');",
    "    f.close();",
    "}",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=\" + (typeof v == \"string\" ? v : \"\" + v) + \"|\" + typeof v + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "",
]
CASES = [
    ("df_missing", 'return dofile("no_such_file_xy.nut");'),
    ("df_empty", 'return dofile("");'),
    ("lf_missing", 'local f = loadfile("no_such_file_xy.nut"); return f == null ? "null" : "closure";'),
    ("lf_empty", 'local f = loadfile(""); return f == null ? "null" : "closure";'),
    ("df_dir", 'return dofile("tmp-c2");'),
    ("lf_dir", 'local f = loadfile("tmp-c2"); return f == null ? "null" : "closure";'),
    ("df_raise_false", 'local r = dofile("no_such_xy2.nut", false); return "ret=" + r;'),
    ("df_raise_true", 'local r = dofile("no_such_xy3.nut", true); return "ret=" + r;'),
    ("lf_raise_false", 'local f = loadfile("no_such_xy4.nut", false); return f == null ? "null" : "closure";'),
    ("lf_raise_true", 'local f = loadfile("no_such_xy5.nut", true); return f == null ? "null" : "closure";'),
    ("df_throw", 'mkText("tmp_dm_throw.nut", "print(\\"before\\\\n\\"); throw \\"mid\\"; print(\\"after\\\\n\\");"); local r = dofile("tmp_dm_throw.nut"); return "ret=" + r;'),
    ("df_throw_caught", 'mkText("tmp_dm_throw2.nut", "print(\\"before\\\\n\\"); throw \\"mid\\";"); try { dofile("tmp_dm_throw2.nut"); } catch (e) { return "E:" + e; } return "no";'),
    ("lf_throw_call", 'mkText("tmp_dm_throw3.nut", "print(\\"before\\\\n\\"); throw \\"mid\\";"); local f = loadfile("tmp_dm_throw3.nut"); try { f(); } catch (e) { return "E:" + e; } return "no";'),
    ("df_syntax", 'mkText("tmp_dm_syn.nut", "local x = ;"); local r = dofile("tmp_dm_syn.nut"); return "ret=" + r;'),
    ("lf_syntax", 'mkText("tmp_dm_syn2.nut", "local x = ;"); local f = loadfile("tmp_dm_syn2.nut"); return f == null ? "null" : "callable";'),
    ("df_emptyfile", 'mkText("tmp_dm_empty.nut", ""); local r = dofile("tmp_dm_empty.nut"); return "ret=" + r;'),
    ("df_comment", 'mkText("tmp_dm_com.nut", "// only comment\\n"); local r = dofile("tmp_dm_com.nut"); return "ret=" + r;'),
    ("df_dotslash", 'mkText("tmp_dm_rel.nut", "return \\"rel-ok\\";"); local r = dofile("./tmp_dm_rel.nut"); return "ret=" + r;'),
    ("lf_dotslash", 'mkText("tmp_dm_rel2.nut", "return 1;"); local f = loadfile(".\\\\tmp_dm_rel2.nut"); return typeof f;'),
    ("df_spaces", 'mkText("tmp with space.nut", "return \\"sp-ok\\";"); local r = dofile("tmp with space.nut"); remove("tmp with space.nut"); return "ret=" + r;'),
    ("df_side", 'mkText("tmp_dm_side.nut", "g_side <- \\"side-done\\"; return 7;"); local r = dofile("tmp_dm_side.nut"); return "ret=" + r + " side=" + ("g_side" in getroottable());'),
    ("df_val", 'mkText("tmp_dm_val.nut", "return 41 + 1;"); local r = dofile("tmp_dm_val.nut"); return "ret=" + r;'),
    ("lf_twice", 'mkText("tmp_dm_twice.nut", "return 42;"); local f = loadfile("tmp_dm_twice.nut"); local r1 = f(); local r2 = f(); return "r1=" + r1 + " r2=" + r2;'),
]

CLEANUP = 'foreach (p in ["tmp_dm_throw.nut", "tmp_dm_throw2.nut", "tmp_dm_throw3.nut", "tmp_dm_syn.nut", "tmp_dm_syn2.nut", "tmp_dm_empty.nut", "tmp_dm_com.nut", "tmp_dm_rel.nut", "tmp_dm_rel2.nut", "tmp_dm_side.nut", "tmp_dm_val.nut", "tmp_dm_twice.nut"]) { try { remove(p); } catch (e) { } }'

for name, body in CASES:
    lines.append('t("%s", function() { %s });' % (name, body))
lines.append(CLEANUP)

ALL = [l for l in lines if l.startswith('t("')]
if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k][:110]))
    sys.exit(0)

lines += ['print("DOFILEMAP-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

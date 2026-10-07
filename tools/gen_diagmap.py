# -*- coding: utf-8 -*-
"""生成"compilestring/dofile 诊断一致性"矩阵语料 tests-corpus/t_diagmap.nut。

为什么单开一份：`errmsg_matrix` 测的是**文件编译**的错误文本、`t_errdumpframes`
测的是**线程/原生边界**的转储——但 **compilestring/dofile 的源名与行号归属**
（多行源的运行期错误行号、`getstackinfos(1).line` 在编译代码内的取值、嵌套
dofile 的转储源名链）没有用例。本轮为**验证型**：12 条 stdout + 完整 stderr
（CALLSTACK/LOCALS 转储穿过 dofile 嵌套）两侧逐字节一致。

实测钉住的语义（都对着参考实现验过）：
1. compilestring 的**编译错误**经 raiseerror 抛出，catch 到的文本**不含行号**
   （就是 `expression expected` 这类消息本身）；多行源与单行源一致。
2. 编译代码内的**运行期错误**同样抛出（catch 文本为 throw 的值）。
3. **`getstackinfos(1).line`** 在编译代码内返回**实际行号**（第 3 行返回 3）、
   `.src` 返回给定的源名。
4. **嵌套 dofile**：错误转储的 CALLSTACK 逐层显示 `d4.nut`→`dofile()`→
   `d3.nut`→`dofile()`→主脚本——源名链与行号逐字节一致。

安全约束：无。

用法：
    python tools/gen_diagmap.py
    python tools/gen_diagmap.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_diagmap.nut")

CASES = [
    ("cs_syn_l3", 'local f = compilestring("local a = 1\\nlocal b = 2\\nlocal c = (", "S3"); return "compiled";'),
    ("cs_syn_deep", 'local f = compilestring("\\n\\n\\n\\n\\n\\n\\n\\n\\n\\nbad(", "SDEEP"); return "compiled";'),
    ("rt_named_l4", 'local f = compilestring("local a = 1\\nlocal b = 2\\nlocal c = 3\\nthrow \\"L4\\"", "RN4"); f(); return "no";'),
    ("rt_named_nested", 'local f = compilestring("local g = function() { throw \\"inner\\"; }\\ng()", "RN5"); f(); return "no";'),
    ("df_comp_err", 'local r = dofile("d1.nut"); return "ret=" + r;'),
    ("df_rt_err", 'local r = dofile("d2.nut"); return "ret=" + r;'),
    ("df_nested", 'local r = dofile("d3.nut"); return "ret=" + r;'),
    ("ce_line10", 'local src = ""; for (local i = 0; i < 9; i++) src += "local v" + i + " = " + i + ";\\n"; src += "bad("; local f = compilestring(src, "CE10"); return "compiled";'),
    ("si_line", 'local f = compilestring("local a = 1\\nlocal b = 2\\nreturn getstackinfos(1).line", "SI"); return f();'),
    ("si_src_line", 'local f = compilestring("return getstackinfos(1).src + \\"/\\" + getstackinfos(1).line", "SI2"); return f();'),
]

lines = [
    "// 自动生成：compilestring/dofile 诊断一致性矩阵（tools/gen_diagmap.py）",
    "//",
    "// 钉住：编译错误 catch 文本（不含行号，即消息本身）、编译代码内运行期错误、",
    "// getstackinfos(1).line/src 在编译代码内的取值（.line=实际行号、.src=给定名）、",
    "// 嵌套 dofile 的错误转储源名链（d4.nut→dofile()→d3.nut→dofile()→主脚本）。",
    "// stderr 的 CALLSTACK/LOCALS 转储由 diff_test 一并对拍。",
    "function d1() { }  // 占位（避免与伴生文件名混淆）",
    "function mkText(path, content) {",
    "    local f = file(path, \"wb\");",
    "    foreach (c in content) f.writen(c, 'b');",
    "    f.close();",
    "}",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=ok:\" + (typeof v == \"string\" ? v : \"\" + v) + \"\\n\"); }",
    "    catch (e) { print(tag + \"=caught:\" + e + \"\\n\"); }",
    "}",
    "",
    "// 伴生文件（脚本自建、末尾自删）",
    'mkText("d1.nut", "local a = 1\\nlocal b = (\\n");',
    'mkText("d2.nut", "local a = 1\\nlocal b = 2\\nlocal c = 3\\nthrow \\"D4\\"");',
    'mkText("d3.nut", "return dofile(\\"d4.nut\\");");',
    'mkText("d4.nut", "throw \\"deep-nested\\"");',
    "",
]
for name, body in CASES:
    lines.append('t("%s", function() { %s });' % (name, body))

ALL = [l for l in lines if l.startswith('t("')]
if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k][:110]))
    sys.exit(0)

lines += [
    '// 伴生文件清理',
    'foreach (p in ["d1.nut", "d2.nut", "d3.nut", "d4.nut"]) { try { remove(p); } catch (e) { } }',
    'print("DIAGMAP-DONE\\n");',
]
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

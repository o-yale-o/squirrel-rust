# -*- coding: utf-8 -*-
"""生成"bindenv × 编译闭包"矩阵语料 tests-corpus/t_bindenv.nut。

**只收两侧一致的子集**。本族有一个**已记录的引擎分歧**（见 MIGRATION.md 的
"bindenv × 编译闭包"行）：`compilestring` 产出的闭包经 `bindenv(env)` 后，
**裸自由标识符**（`return marker`）在参考实现里**不读** env（读的是 root——
即使 `this.marker` 明明能命中也不行，`setdelegate` 有无无关）；本实现读 env。
4 种形态（有无委托 × 裸标识符/this.marker）复现文件见 `tmp-c2/bind_iso.nut`。
修复需要动自由标识符解析与 `_env` 调用路径，未做（TODO P28）。

一致子集钉住的语义：
1. 普通（文件编译）闭包 bindenv 后**能**读 env 自己的槽（`F_norm_bound`）。
2. 编译闭包**不 bindenv** 直接调用：this=root，裸标识符走 root（D，缺槽报错）。
3. `bindenv` 返回**新**闭包（不是原对象）。
4. 编译闭包读全局（`typeof print`）不 bindenv 也行（走 root）。
5. bindenv(root) 后 this=root：root 没有 marker → `?`（与参考一致）。

用法：
    python tools/gen_bindenv.py
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_bindenv.nut")

CASES = [
    ("norm_bound", 'local env = { marker = "env-ok" }; local probe = function() { return marker; }; local g = probe.bindenv(env); return g();'),
    ("plain_unbound", 'local env = { marker = "env-ok" }; local f = compilestring("return marker", "D"); return f();'),
    ("bind_returns_new", 'local f = compilestring("return 1", "CC7"); local g = f.bindenv(env0); return (f == g) ? "same" : "new";'),
    ("cs_root_read", 'local f = compilestring("return typeof print", "CC4"); return f();'),
    ("cs_bind_root", 'local f = compilestring("return \\"marker=\\" + ((\\"marker\\" in this) ? this.marker : \\\"?\\")", "CC6"); local g = f.bindenv(getroottable()); return g();'),
    ("this_is_env", 'local env = { marker = "env-ok" }; local probe = function() { return (this == env) + "/" + (this == getroottable()); }; local g = probe.bindenv(env); return g();'),
]

lines = [
    "// 自动生成：bindenv × 编译闭包矩阵（tools/gen_bindenv.py）",
    "//",
    "// **只收两侧一致的子集**：compilestring 闭包 bindenv 后的**裸自由标识符**",
    "// 在参考实现里不读 env（读 root），本实现读 env——该分歧 4 种形态见",
    "// MIGRATION.md 的 bindenv 行（未修，TODO P28）。一致子集钉住：",
    "// 普通闭包 bindenv 正常、编译闭包不 bindenv 时 this=root、",
    "// bindenv 返回新闭包、bindenv(root) 后读不到 env 的槽。",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=\" + d(v) + \"|\" + typeof v + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "function d(x) { return \"\" + x; }",
    "env0 <- { };",
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

lines += ['print("BINDENV-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

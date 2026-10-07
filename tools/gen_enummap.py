# -*- coding: utf-8 -*-
"""生成"enum/const 语义"矩阵语料 tests-corpus/t_enummap.nut。

为什么单开一份：语料里 enum/const 只有零星主干（t_const/t_surface4/5 共 ~8 条）
——**编译期内联语义**完全没钉：enum/const 声明后**连根表槽都没有**（纯编译期
替换，`("E_AUTO" in getroottable())` 为 false）、函数体内的 `enum` 合法
（与 const 不同）、enum 成员自引用报 `scalar expected`、重复项先到先得、
`delete` 报 `can't delete an expression` 等。本轮为**验证型**：27 条探针
两侧逐字节一致。

实测钉住的语义（都对着参考实现验过；注意 decl() 经 compilestring 执行，
返回值是被编译代码的返回——enum/const 的编译期内联让部分返回值为 null，
两侧一致）：
1. **enum 自动递增**：`{A, B, C}` → 0/1/2；显式值后继续递增（`{A=5,B,C}` →
   B=6, C=7）；负数起点同样递增（`{A=-1,B}` → B=0）。
2. **enum 成员值必须是字面量**：`A = 1 + 2` 报 `expected 'IDENTIFIER'`；
   自引用 `B = A + 10` 报 `scalar expected : integer, float, or string`；
   字符串值合法（`A = "x"`）；INT64_MAX 字面量合法。
3. **重复项先到先得**：`{A = 1, A = 2}` → A=1。
4. **函数体内 enum 合法**（与 const 不同！）：`function() { enum Q { A } }`
   函数体内 enum 合法（Q.A=0）。
5. **const**：字面量四种类型合法；`const KE = 1 + 2` 报
   `end of statement expected`；重定义**不报错**（后值覆盖）；
   `delete const名` 报 `can't delete an expression`；函数体内 const 合法。
6. **enum/const 都是纯编译期内联**：声明后连根表槽都没有
   （`("名字" in getroottable())` 为 false）。

安全约束：无。

用法：
    python tools/gen_enummap.py
    python tools/gen_enummap.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_enummap.nut")

CASES = [
    # ---------- enum ----------
    ("e_auto", 'enum E_AUTO { A, B, C } return E_AUTO.A + "/" + E_AUTO.B + "/" + E_AUTO.C;'),
    ("e_neg_after", 'enum E_NEG { A = 5, B, C } return E_NEG.B + "/" + E_NEG.C;'),
    ("e_negauto", 'enum E_N { A = -1, B } return E_N.B;'),
    ("e_dup", 'enum E_DUP { A = 1, A = 2 } return E_DUP.A;'),
    ("e_float", 'enum E_F { A = 1.5, B } return E_F.A + "/" + E_F.B;'),
    ("e_str", 'enum E_S { A = "x" } return E_S.A;'),
    # e_selfref 的 B = A + 10 是编译错（枚举成员值不接受表达式），必须走 compilestring
    # 才能让错误可 catch 而不是杀死整个语料文件
    ("e_selfref", 'local f = compilestring("enum E_R { A, B = A + 10 } return E_R.B;", "ER"); return f();'),
    ("e_big", 'local f = compilestring("enum E_G { A = 0x7FFFFFFFFFFFFFFF } return E_G.A;", "EG"); return f();'),
    # e_expr 的 A = 1 + 2 是编译错（枚举成员值不接受表达式），必须走 compilestring
    ("e_expr", 'local f = compilestring("enum E_X { A = 1 + 2 } return E_X.A;", "EX"); return f();'),
    ("e_typeof", 'local f = compilestring("enum E_T2 { A } return typeof E_T2 + \\"/\\" + typeof E_T2.A;", "ET"); return f();'),
    ("e_len", 'local f = compilestring("enum E_L { A, B, C } return E_L.len();", "EL"); return f();'),
    ("e_iter", 'local f = compilestring("enum E_I { A, B } local n = 0; foreach (k, v in E_I) n = n + 1; return n;", "EI"); return f();'),
    ("e_delete", 'local f = compilestring("enum E_DEL { A, B } delete E_DEL.A; return E_DEL.len();", "ED"); return f();'),
    ("e_in_fn", 'local f = compilestring("function f() { enum Q { A } return Q.A; } return f();", "EF"); return f();'),
    ("e_not_in_root", 'local f = compilestring("enum E_NR { A } return (\\"E_NR\\" in getroottable()) ? \\"in-root\\" : \\"inline-only\\";", "EN"); return f();'),
    # ---------- const ----------
    ("c_lit", 'local f = compilestring("const KC = 42; return KC;", "CL"); return f();'),
    ("c_types", 'local f = compilestring("const KS = \\"s\\"; const KF = 1.5; const KB = true; const KN = -3; return KS + \\"/\\" + KF + \\"/\\" + KB + \\"/\\" + KN;", "CT"); return f();'),
    ("c_expr", 'local f = compilestring("const KE = 1 + 2; return KE;", "CE"); return f();'),
    ("c_redef", 'local f = compilestring("const KR = 1; const KR = 2; return KR;", "CR"); return f();'),
    ("c_delete", 'local f = compilestring("const KD = 1; return delete KD;", "CD"); return f();'),
    ("c_typeof", 'local f = compilestring("const KT = \\"v\\"; return typeof KT;", "CT2"); return f();'),
    ("c_in_fn", 'local f = compilestring("function f() { const KF2 = 1; return KF2; } return f();", "CF"); return f();'),
    ("c_from_enum", 'local f = compilestring("enum E_SRC { V = 9 } const KC2 = E_SRC.V; return KC2;", "CS"); return f();'),
    ("c_not_in_root", 'local f = compilestring("const K_NR = 5; return (\\"K_NR\\" in getroottable()) ? \\"in-root\\" : \\"inline-only\\";", "CN"); return f();'),
    ("c_shadow_local", 'local f = compilestring("const KS2 = 1; return (function() { local KS2 = 9; return KS2; })();", "CX"); return f();'),
]

lines = [
    "// 自动生成：enum/const 语义矩阵（tools/gen_enummap.py）",
    "//",
    "// **enum/const 是纯编译期内联**：声明后连根表槽都没有（`(\"名字\" in getroottable())`",
    "// 为 false），值直接替换进字节码。钉住：enum 自动递增/显式值/负数/重复项先到先得/",
    "// 函数体内 enum 合法（与 const 不同！）/自引用报 scalar expected/表达式值报",
    "// expected 'IDENTIFIER'；const 字面量四类型/表达式值报 end of statement/",
    "// 重定义不报错（后值覆盖）/delete 报 can't delete an expression。",
    "// 每条声明经 compilestring 执行（隔离 + 可 catch），decl() 的返回值是",
    "// 编译代码的 return——enum/const 内联让部分返回值为 null（两侧一致）。",
    "function decl(src) {",
    "    local f = compilestring(src, \"DECL\");",
    "    f();",
    "    return \"ok\";",
    "}",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=\" + v + \"|\" + typeof v + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
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

lines += ['print("ENUMMAP-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

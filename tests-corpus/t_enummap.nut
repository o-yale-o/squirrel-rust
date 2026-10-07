// 自动生成：enum/const 语义矩阵（tools/gen_enummap.py）
//
// **enum/const 是纯编译期内联**：声明后连根表槽都没有（`("名字" in getroottable())`
// 为 false），值直接替换进字节码。钉住：enum 自动递增/显式值/负数/重复项先到先得/
// 函数体内 enum 合法（与 const 不同！）/自引用报 scalar expected/表达式值报
// expected 'IDENTIFIER'；const 字面量四类型/表达式值报 end of statement/
// 重定义不报错（后值覆盖）/delete 报 can't delete an expression。
// 每条声明经 compilestring 执行（隔离 + 可 catch），decl() 的返回值是
// 编译代码的 return——enum/const 内联让部分返回值为 null（两侧一致）。
function decl(src) {
    local f = compilestring(src, "DECL");
    f();
    return "ok";
}
function t(tag, f) {
    try { local v = f(); print(tag + "=" + v + "|" + typeof v + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

t("e_auto", function() { enum E_AUTO { A, B, C } return E_AUTO.A + "/" + E_AUTO.B + "/" + E_AUTO.C; });
t("e_neg_after", function() { enum E_NEG { A = 5, B, C } return E_NEG.B + "/" + E_NEG.C; });
t("e_negauto", function() { enum E_N { A = -1, B } return E_N.B; });
t("e_dup", function() { enum E_DUP { A = 1, A = 2 } return E_DUP.A; });
t("e_float", function() { enum E_F { A = 1.5, B } return E_F.A + "/" + E_F.B; });
t("e_str", function() { enum E_S { A = "x" } return E_S.A; });
t("e_selfref", function() { local f = compilestring("enum E_R { A, B = A + 10 } return E_R.B;", "ER"); return f(); });
t("e_big", function() { local f = compilestring("enum E_G { A = 0x7FFFFFFFFFFFFFFF } return E_G.A;", "EG"); return f(); });
t("e_expr", function() { local f = compilestring("enum E_X { A = 1 + 2 } return E_X.A;", "EX"); return f(); });
t("e_typeof", function() { local f = compilestring("enum E_T2 { A } return typeof E_T2 + \"/\" + typeof E_T2.A;", "ET"); return f(); });
t("e_len", function() { local f = compilestring("enum E_L { A, B, C } return E_L.len();", "EL"); return f(); });
t("e_iter", function() { local f = compilestring("enum E_I { A, B } local n = 0; foreach (k, v in E_I) n = n + 1; return n;", "EI"); return f(); });
t("e_delete", function() { local f = compilestring("enum E_DEL { A, B } delete E_DEL.A; return E_DEL.len();", "ED"); return f(); });
t("e_in_fn", function() { local f = compilestring("function f() { enum Q { A } return Q.A; } return f();", "EF"); return f(); });
t("e_not_in_root", function() { local f = compilestring("enum E_NR { A } return (\"E_NR\" in getroottable()) ? \"in-root\" : \"inline-only\";", "EN"); return f(); });
t("c_lit", function() { local f = compilestring("const KC = 42; return KC;", "CL"); return f(); });
t("c_types", function() { local f = compilestring("const KS = \"s\"; const KF = 1.5; const KB = true; const KN = -3; return KS + \"/\" + KF + \"/\" + KB + \"/\" + KN;", "CT"); return f(); });
t("c_expr", function() { local f = compilestring("const KE = 1 + 2; return KE;", "CE"); return f(); });
t("c_redef", function() { local f = compilestring("const KR = 1; const KR = 2; return KR;", "CR"); return f(); });
t("c_delete", function() { local f = compilestring("const KD = 1; return delete KD;", "CD"); return f(); });
t("c_typeof", function() { local f = compilestring("const KT = \"v\"; return typeof KT;", "CT2"); return f(); });
t("c_in_fn", function() { local f = compilestring("function f() { const KF2 = 1; return KF2; } return f();", "CF"); return f(); });
t("c_from_enum", function() { local f = compilestring("enum E_SRC { V = 9 } const KC2 = E_SRC.V; return KC2;", "CS"); return f(); });
t("c_not_in_root", function() { local f = compilestring("const K_NR = 5; return (\"K_NR\" in getroottable()) ? \"in-root\" : \"inline-only\";", "CN"); return f(); });
t("c_shadow_local", function() { local f = compilestring("const KS2 = 1; return (function() { local KS2 = 9; return KS2; })();", "CX"); return f(); });
print("ENUMMAP-DONE\n");

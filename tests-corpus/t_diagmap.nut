// 自动生成：compilestring/dofile 诊断一致性矩阵（tools/gen_diagmap.py）
//
// 钉住：编译错误 catch 文本（不含行号，即消息本身）、编译代码内运行期错误、
// getstackinfos(1).line/src 在编译代码内的取值（.line=实际行号、.src=给定名）、
// 嵌套 dofile 的错误转储源名链（d4.nut→dofile()→d3.nut→dofile()→主脚本）。
// stderr 的 CALLSTACK/LOCALS 转储由 diff_test 一并对拍。
function d1() { }  // 占位（避免与伴生文件名混淆）
function mkText(path, content) {
    local f = file(path, "wb");
    foreach (c in content) f.writen(c, 'b');
    f.close();
}
function t(tag, f) {
    try { local v = f(); print(tag + "=ok:" + (typeof v == "string" ? v : "" + v) + "\n"); }
    catch (e) { print(tag + "=caught:" + e + "\n"); }
}

// 伴生文件（脚本自建、末尾自删）
mkText("d1.nut", "local a = 1\nlocal b = (\n");
mkText("d2.nut", "local a = 1\nlocal b = 2\nlocal c = 3\nthrow \"D4\"");
mkText("d3.nut", "return dofile(\"d4.nut\");");
mkText("d4.nut", "throw \"deep-nested\"");

t("cs_syn_l3", function() { local f = compilestring("local a = 1\nlocal b = 2\nlocal c = (", "S3"); return "compiled"; });
t("cs_syn_deep", function() { local f = compilestring("\n\n\n\n\n\n\n\n\n\nbad(", "SDEEP"); return "compiled"; });
t("rt_named_l4", function() { local f = compilestring("local a = 1\nlocal b = 2\nlocal c = 3\nthrow \"L4\"", "RN4"); f(); return "no"; });
t("rt_named_nested", function() { local f = compilestring("local g = function() { throw \"inner\"; }\ng()", "RN5"); f(); return "no"; });
t("df_comp_err", function() { local r = dofile("d1.nut"); return "ret=" + r; });
t("df_rt_err", function() { local r = dofile("d2.nut"); return "ret=" + r; });
t("df_nested", function() { local r = dofile("d3.nut"); return "ret=" + r; });
t("ce_line10", function() { local src = ""; for (local i = 0; i < 9; i++) src += "local v" + i + " = " + i + ";\n"; src += "bad("; local f = compilestring(src, "CE10"); return "compiled"; });
t("si_line", function() { local f = compilestring("local a = 1\nlocal b = 2\nreturn getstackinfos(1).line", "SI"); return f(); });
t("si_src_line", function() { local f = compilestring("return getstackinfos(1).src + \"/\" + getstackinfos(1).line", "SI2"); return f(); });
// 伴生文件清理
foreach (p in ["d1.nut", "d2.nut", "d3.nut", "d4.nut"]) { try { remove(p); } catch (e) { } }
print("DIAGMAP-DONE\n");

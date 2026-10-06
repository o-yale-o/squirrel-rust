// 自动生成：dofile/loadfile 边界矩阵（tools/gen_dofilemap.py）
//
// 脚本**自建自删伴生文件**（不依赖仓库内静态文件）。错误文本统一是
// `cannot open the file`（目录作为路径也不特殊报错）；**loadfile 打不开时
// 是抛错而非返回 null**；第 2 参（printerror）只控制 stderr 打印不影响抛错；
// dofile 返回文件的 return 值、文件里的 <- 槽进根表；文件中途抛错会**穿透**
// 到调用方。两侧逐字节一致（含 stderr）。
function mkText(path, content) {
    local f = file(path, "wb");
    foreach (c in content) f.writen(c, 'b');
    f.close();
}
function t(tag, f) {
    try { local v = f(); print(tag + "=" + (typeof v == "string" ? v : "" + v) + "|" + typeof v + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

t("df_missing", function() { return dofile("no_such_file_xy.nut"); });
t("df_empty", function() { return dofile(""); });
t("lf_missing", function() { local f = loadfile("no_such_file_xy.nut"); return f == null ? "null" : "closure"; });
t("lf_empty", function() { local f = loadfile(""); return f == null ? "null" : "closure"; });
t("df_dir", function() { return dofile("tmp-c2"); });
t("lf_dir", function() { local f = loadfile("tmp-c2"); return f == null ? "null" : "closure"; });
t("df_raise_false", function() { local r = dofile("no_such_xy2.nut", false); return "ret=" + r; });
t("df_raise_true", function() { local r = dofile("no_such_xy3.nut", true); return "ret=" + r; });
t("lf_raise_false", function() { local f = loadfile("no_such_xy4.nut", false); return f == null ? "null" : "closure"; });
t("lf_raise_true", function() { local f = loadfile("no_such_xy5.nut", true); return f == null ? "null" : "closure"; });
t("df_throw", function() { mkText("tmp_dm_throw.nut", "print(\"before\\n\"); throw \"mid\"; print(\"after\\n\");"); local r = dofile("tmp_dm_throw.nut"); return "ret=" + r; });
t("df_throw_caught", function() { mkText("tmp_dm_throw2.nut", "print(\"before\\n\"); throw \"mid\";"); try { dofile("tmp_dm_throw2.nut"); } catch (e) { return "E:" + e; } return "no"; });
t("lf_throw_call", function() { mkText("tmp_dm_throw3.nut", "print(\"before\\n\"); throw \"mid\";"); local f = loadfile("tmp_dm_throw3.nut"); try { f(); } catch (e) { return "E:" + e; } return "no"; });
t("df_syntax", function() { mkText("tmp_dm_syn.nut", "local x = ;"); local r = dofile("tmp_dm_syn.nut"); return "ret=" + r; });
t("lf_syntax", function() { mkText("tmp_dm_syn2.nut", "local x = ;"); local f = loadfile("tmp_dm_syn2.nut"); return f == null ? "null" : "callable"; });
t("df_emptyfile", function() { mkText("tmp_dm_empty.nut", ""); local r = dofile("tmp_dm_empty.nut"); return "ret=" + r; });
t("df_comment", function() { mkText("tmp_dm_com.nut", "// only comment\n"); local r = dofile("tmp_dm_com.nut"); return "ret=" + r; });
t("df_dotslash", function() { mkText("tmp_dm_rel.nut", "return \"rel-ok\";"); local r = dofile("./tmp_dm_rel.nut"); return "ret=" + r; });
t("lf_dotslash", function() { mkText("tmp_dm_rel2.nut", "return 1;"); local f = loadfile(".\\tmp_dm_rel2.nut"); return typeof f; });
t("df_spaces", function() { mkText("tmp with space.nut", "return \"sp-ok\";"); local r = dofile("tmp with space.nut"); remove("tmp with space.nut"); return "ret=" + r; });
t("df_side", function() { mkText("tmp_dm_side.nut", "g_side <- \"side-done\"; return 7;"); local r = dofile("tmp_dm_side.nut"); return "ret=" + r + " side=" + ("g_side" in getroottable()); });
t("df_val", function() { mkText("tmp_dm_val.nut", "return 41 + 1;"); local r = dofile("tmp_dm_val.nut"); return "ret=" + r; });
t("lf_twice", function() { mkText("tmp_dm_twice.nut", "return 42;"); local f = loadfile("tmp_dm_twice.nut"); local r1 = f(); local r2 = f(); return "r1=" + r1 + " r2=" + r2; });
foreach (p in ["tmp_dm_throw.nut", "tmp_dm_throw2.nut", "tmp_dm_throw3.nut", "tmp_dm_syn.nut", "tmp_dm_syn2.nut", "tmp_dm_empty.nut", "tmp_dm_com.nut", "tmp_dm_rel.nut", "tmp_dm_rel2.nut", "tmp_dm_side.nut", "tmp_dm_val.nut", "tmp_dm_twice.nut"]) { try { remove(p); } catch (e) { } }
print("DOFILEMAP-DONE\n");

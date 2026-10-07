// 自动生成：file 模式串 × 文本/二进制矩阵（tools/gen_filemodes.py）
//
// **文本模式**（无 `b`）写入 LF→CRLF、读取 CRLF→LF 且遇 Ctrl+Z 截断、
// seek 后读裸字节、tell/len 报物理字节数——全部与参考逐字节一致。
// `wbt`（b 与 t 同用）：参考实现 fopen 判无效参数 → 0xC0000409 崩溃；
// 本实现给可捕获的 `cannot open file`（**有意偏离**，入语料钉住本侧行为）。
// 每条用例开头先 remove 探针文件，避免上一次运行的残留影响追加模式。
function d(x) { return "" + x; }
function t(tag, f) {
    try { local v = f(); print(tag + "=" + d(v) + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

t("r", function() {
    try { remove("fm_probe_r.bin"); } catch (e) { }
    local f = file("fm_probe_r.bin", "r");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_r.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_r.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("rb", function() {
    try { remove("fm_probe_rb.bin"); } catch (e) { }
    local f = file("fm_probe_rb.bin", "rb");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_rb.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_rb.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("w", function() {
    try { remove("fm_probe_w.bin"); } catch (e) { }
    local f = file("fm_probe_w.bin", "w");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_w.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_w.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("wb", function() {
    try { remove("fm_probe_wb.bin"); } catch (e) { }
    local f = file("fm_probe_wb.bin", "wb");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_wb.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_wb.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("a", function() {
    try { remove("fm_probe_a.bin"); } catch (e) { }
    local f = file("fm_probe_a.bin", "a");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_a.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_a.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("ab", function() {
    try { remove("fm_probe_ab.bin"); } catch (e) { }
    local f = file("fm_probe_ab.bin", "ab");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_ab.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_ab.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("r", function() {
    try { remove("fm_probe_r.bin"); } catch (e) { }
    local f = file("fm_probe_r.bin", "r+");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_r.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_r.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("rb", function() {
    try { remove("fm_probe_rb.bin"); } catch (e) { }
    local f = file("fm_probe_rb.bin", "rb+");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_rb.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_rb.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("w", function() {
    try { remove("fm_probe_w.bin"); } catch (e) { }
    local f = file("fm_probe_w.bin", "w+");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_w.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_w.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("wb", function() {
    try { remove("fm_probe_wb.bin"); } catch (e) { }
    local f = file("fm_probe_wb.bin", "wb+");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_wb.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_wb.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("wt", function() {
    try { remove("fm_probe_wt.bin"); } catch (e) { }
    local f = file("fm_probe_wt.bin", "wt");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_wt.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_wt.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("rt", function() {
    try { remove("fm_probe_rt.bin"); } catch (e) { }
    local f = file("fm_probe_rt.bin", "rt");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_rt.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_rt.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("wt", function() {
    try { remove("fm_probe_wt.bin"); } catch (e) { }
    local f = file("fm_probe_wt.bin", "w+t");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_wt.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_wt.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("at", function() {
    try { remove("fm_probe_at.bin"); } catch (e) { }
    local f = file("fm_probe_at.bin", "a+t");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_at.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_at.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
t("rt", function() {
    try { remove("fm_probe_rt.bin"); } catch (e) { }
    local f = file("fm_probe_rt.bin", "r+t");
    f.writen(97, 'b'); f.writen(10, 'b'); f.writen(98, 'b');
    local tell = f.tell();
    f.close();
    local g = file("fm_probe_rt.bin", "rb");
    local n = g.len();
    local o = "";
    while (!g.eos()) o += g.readn('b') + ",";
    g.close();
    remove("fm_probe_rt.bin");
    return "tell=" + tell + " len=" + n + " bytes=" + o;
});
print("FILEMODES-DONE\n");

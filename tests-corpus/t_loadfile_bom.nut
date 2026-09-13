// 源码嗅探：UTF-8 BOM / UTF-16LE / UTF-16BE（↔ sqstd_loadfile）
function makeFile(path, bytes) {
    local f = file(path, "wb");
    foreach (b in bytes) f.writen(b, 'b');
    f.close();
}
function leBytes(s) {
    local out = [0xFF, 0xFE];
    for (local i = 0; i < s.len(); i++) {
        local c = s[i];
        out.append(c & 0xFF);
        out.append((c >> 8) & 0xFF);
    }
    return out;
}
function beBytes(s) {
    local out = [0xFE, 0xFF];
    for (local i = 0; i < s.len(); i++) {
        local c = s[i];
        out.append((c >> 8) & 0xFF);
        out.append(c & 0xFF);
    }
    return out;
}
function utf8BomBytes(s) {
    local out = [0xEF, 0xBB, 0xBF];
    for (local i = 0; i < s.len(); i++) out.append(s[i] & 0xFF);
    return out;
}

local src = "print(\"payload\\n\");";

makeFile("tmp_bom_le.nut", leBytes(src));
loadfile("tmp_bom_le.nut")();
remove("tmp_bom_le.nut");

makeFile("tmp_bom_be.nut", beBytes(src));
loadfile("tmp_bom_be.nut")();
remove("tmp_bom_be.nut");

makeFile("tmp_bom_u8.nut", utf8BomBytes(src));
loadfile("tmp_bom_u8.nut")();
remove("tmp_bom_u8.nut");

// 无 BOM 普通源码
makeFile("tmp_plain.nut", utf8BomBytes(src).slice(3));
loadfile("tmp_plain.nut")();
remove("tmp_plain.nut");

// 错误路径
try { loadfile("no_such_file_xyz.nut"); } catch (e) { print("miss: " + e + "\n"); }
makeFile("tmp_badenc.nut", [0xEF, 0xBB, 0x41, 0x70]);
try { loadfile("tmp_badenc.nut"); } catch (e) { print("enc: " + e + "\n"); }
remove("tmp_badenc.nut");
makeFile("tmp_ioerr.nut", [0xEF, 0xBB]);
try { loadfile("tmp_ioerr.nut"); } catch (e) { print("io: " + e + "\n"); }
remove("tmp_ioerr.nut");
makeFile("tmp_syn.nut", utf8BomBytes("print(1 +"));
try { loadfile("tmp_syn.nut"); } catch (e) { print("syn: " + e + "\n"); }
print("printed-err: ");
try { loadfile("tmp_syn.nut", true); } catch (e) { print(e + "\n"); }
remove("tmp_syn.nut");
print("done\n");

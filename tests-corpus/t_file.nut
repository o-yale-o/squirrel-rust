local f = file("diff_test_data.bin", "wb+");
f.writen(0x4142, 'w');
f.writen(-100000, 'i');
f.writen(3.5, 'f');
f.writen(2.25, 'd');
f.writen(7, 'b');
f.writen(-3, 'c');
f.writen(123456789, 'l');
f.writen(-2, 's');
local blob = blob(4);
blob[0] = 0x11; blob[1] = 0x22; blob[2] = 0x33; blob[3] = 0x44;
f.writeblob(blob);
print("tell=" + f.tell() + "\n");
f.seek(0, 'b');
print("len=" + f.len() + "\n");
print("w=" + f.readn('w') + "\n");
print("i=" + f.readn('i') + "\n");
print("f=" + f.readn('f') + "\n");
print("d=" + f.readn('d') + "\n");
print("b=" + f.readn('b') + "\n");
print("c=" + f.readn('c') + "\n");
print("l=" + f.readn('l') + "\n");
print("s=" + f.readn('s') + "\n");
local rb = f.readblob(4);
print("blob.len=" + rb.len() + "\n");
print("blob[0]=" + rb[0] + " blob[3]=" + rb[3] + "\n");
print("eos=" + (f.eos() != null) + "\n");
f.close();
remove("diff_test_data.bin");

// ---- 文件句柄共享模式（↔ CRT fopen 用 _SH_DENYNO，不含 FILE_SHARE_DELETE）----
// 句柄开着时删不掉文件；Rust 的 std::fs::File 默认三种共享齐全（开着也能删），
// 已显式去掉 share-delete（前提：后端随实例死亡释放，见 stream.rs 的 bind_backend_inst）。
// 注意这里刻意**不用尾调用**：`return try_remove(p);` 在我们这边会把当前帧的局部提前释放
// （参考实现保留到帧弹出），那是 TODO.md P14 子问题 2 的残留差异，别混进这条语料。
class SubFile extends file { constructor(p, m) { base.constructor(p, m); } }
function try_remove(p) { try { remove(p); return "ok"; } catch (e) { return "E:" + e; } }

local h1 = file("diff_test_share.bin", "wb+");
local h2 = file("diff_test_share.bin", "wb+");
print("share_remove_while_open=" + try_remove("diff_test_share.bin") + "\n");
h1.close();
print("share_remove_after_one_close=" + try_remove("diff_test_share.bin") + "\n");
h2.close();
print("share_remove_after_all_close=" + try_remove("diff_test_share.bin") + "\n");

// 同名 local 重复声明：旧值仍被旧槽持有（帧弹出才释放），所以同样删不掉。
// 放进函数里、返回时帧弹出、句柄释放，顶层才删得掉（写在顶层的话顶层帧一直活着）。
function shadow_case() {
    local sh = SubFile("diff_test_share2.bin", "wb+");
    local sh = SubFile("diff_test_share2.bin", "wb+");
    local r = try_remove("diff_test_share2.bin");
    return r;
}
print("shadow_remove_while_open=" + shadow_case() + "\n");

// 用例结束（帧弹出、句柄释放）后清理，保证跑完不留文件
function cleanup_share() {
    local a = try_remove("diff_test_share.bin");
    local b = try_remove("diff_test_share2.bin");
    return "" + a + b;
}
print("share_cleanup=" + cleanup_share() + "\n");

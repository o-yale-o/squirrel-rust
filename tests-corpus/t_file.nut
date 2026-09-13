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

// ---- 文件句柄共享模式：暂时**不对齐**（见 TODO.md P14）----
// 参考实现的 fopen 用 _SH_DENYNO（不含 FILE_SHARE_DELETE）：句柄开着时 remove() 失败。
// 我们这边 Rust 的 std::fs::File 默认三种共享齐全（开着也能删）。要忠实对齐，得先修
// 后端注册表只增不减的问题（未显式 close 的后端会一直活到进程结束）—— 否则语料里
// "建很多实例但从不 GC"的用例连收尾清理都做不了。这条偏离已记进 MIGRATION.md。
print("share_note=deferred\n");

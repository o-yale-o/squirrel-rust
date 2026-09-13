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

// ---- 文件句柄共享模式：暂不对齐（见 TODO.md P14 的第二个子问题）----
// 参考实现的 fopen 不含 FILE_SHARE_DELETE（句柄开着时 remove() 失败），我们这边
// Rust 默认三种共享齐全（开着也能删）。P14 的注册表滞留已修，但还发现"实例可能在
// 作用域结束前就被释放"（函数里 `local a = file(p,"wb+"); return try_remove(p);`
// 我们给 ok、参考给 failed；中间加一次 a.len() 就两边一致），查清之前不打开 share-delete。
print("share_note=deferred\n");

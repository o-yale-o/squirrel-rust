// 流语义探针：文件流/blob 的 tell/len/eos/flush（与 C++ 参考实现逐行对拍）
// 注意 writen 的参数顺序是 (值, 格式)，与 readn(格式) 相反（sqstdstream.cpp 读的是 arg3）
local path = "tmp-c2\\stream4.bin";
local src = file(path, "wb");
src.writen(65, 'b');
src.writen(66, 'b');
src.writen(67, 'b');
src.writen(68, 'b');
src.close();
local f = file(path, "rb");
print(f.tell() + "," + f.len() + "\n");
print("" + f.eos() + "\n");
local a = f.readn('b'); local b = f.readn('b'); local c = f.readn('b');
print(a + "" + b + "" + c + "\n");
print(f.tell() + "," + f.len() + "\n");
print("" + f.eos() + "\n");
local d = f.readn('b');
print(f.tell() + "," + f.len() + "\n");
print("" + f.eos() + "\n");
print("" + f.flush() + "\n");
f.close();
local bl = blob(4);
print("blob " + bl.tell() + "," + bl.len() + "," + bl.eos() + "\n");
bl.seek(0, 'e');
print("blob " + bl.tell() + "," + bl.len() + "," + bl.eos() + "\n");
local bl2 = blob(0);
print("blob0 " + bl2.tell() + "," + bl2.len() + "," + bl2.eos() + "\n");

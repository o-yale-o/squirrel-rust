// 带 UTF-8 BOM 的源文件（CLI 直接加载路径 ↔ sqstd_loadfile 嗅探）
print("bom-cli-ok\n");
local t = { a = 1 };
foreach (k, v in t) print(k + "=" + v + "\n");

// 根表的插入/迭代顺序守门：注册顺序一旦不同，这里的输出就会与参考实现分叉。
//
// 为什么值得单独守：SQTable 的迭代顺序由**插入顺序**决定（哈希布局 + rehash 时机），
// 而根表是引擎按 sq_base_register → bloblib → iolib → systemlib → mathlib → stringlib
// 逐条插入的。任何一处注册顺序变化（例如把 regexp 放到 format 之后、stdin/stderr 对调）
// 都会改变 `foreach (k, v in getroottable())` 的结果——这是脚本可观察的。
local rt = getroottable();
local names = [];
foreach (k, v in rt) names.append("" + k + ":" + typeof v);
print("names=" + names.len() + "\n");
foreach (n in names) print(n + "\n");
local sorted = clone names;
sorted.sort();
foreach (n in sorted) print("s:" + n + "\n");
print("has_regexp=" + ("regexp" in rt) + "\n");
print("has_dummy=" + ("dummy" in rt) + "\n");
print("has_version=" + ("_version_" in rt) + "\n");
print("types=" + typeof rt.stream + "," + typeof rt.file + "," + typeof rt.blob + "," + typeof rt.regexp + "\n");
print("ROOTTABLE-DONE\n");

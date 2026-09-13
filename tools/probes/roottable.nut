// 根表内容与迭代顺序对比（getroot() 那条差异的定位）
local rt = getroottable();
local names = [];
foreach (k, v in rt) names.append("" + k + ":" + typeof v);
print("有序键数=" + names.len() + "\n");
foreach (n in names) print(n + "\n");
local sorted = clone names;
sorted.sort();
print("排序后:\n");
foreach (n in sorted) print(n + "\n");
print("有没有 regexp: " + (("regexp" in rt) ? "yes" : "no") + "\n");
print("有没有 dummy: " + (("dummy" in rt) ? "yes" : "no") + "\n");
print("有没有 lstrip: " + (("lstrip" in rt) ? "yes" : "no") + "\n");
print("有没有 _floatsize_: " + (("_floatsize_" in rt) ? "yes" : "no") + "\n");
print("regexp typeof=" + typeof rt.regexp + "\n");

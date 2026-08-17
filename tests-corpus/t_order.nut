local t = {};
for(local i = 0; i < 50; i++) t["k" + i] <- i;
foreach(k, v in t) print(k + "=" + v + " ");
print("\n");
print(t.len() + "\n");
local keys = t.keys();
foreach(k in keys) print(k + ",");
print("\n");

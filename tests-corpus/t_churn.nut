// 对象/字符串 churn：语义必须逐字节一致，**内存必须有界**。
// 内存侧守门在 crates/squirrel-stdlib/tests/regex_semantics.rs
// （string_table_stays_bounded_under_churn）与 tools/mem_probe.py，用法见 STATUS 三·补8。
local s = "";
for (local i = 0; i < 5000; i++) s += "x";
print("len=" + s.len() + "\n");

local acc = 0;
for (local i = 0; i < 20000; i++) { local t = { a = i }; acc += t.a; }
for (local i = 0; i < 20000; i++) { local a = [i, i + 1]; acc += a[1]; }
for (local i = 0; i < 20000; i++) { local f = function() { return 1; }; acc += f(); }
for (local i = 0; i < 20000; i++) { local v = "v" + i; acc += v.len(); }
print("acc=" + acc + "\n");
print("CHURN-DONE\n");

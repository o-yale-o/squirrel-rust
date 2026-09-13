// 与 tools/bench/bench.nut 的 loop 用例完全相同的循环，跑一次，用于精确计数指令条数
local s = 0;
for (local i = 0; i < 8000000; i++) s = s + i % 7 - 3;
print(s + "\n");

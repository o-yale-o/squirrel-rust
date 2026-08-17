for(local i = 0; i < 10; i++) {
    local t = {};
    t.self <- t;
    t.arr <- [t, {x = t}];
}
local n = collectgarbage();
print(n >= 0 ? "gc-ok" : "gc-fail");
print("\n");

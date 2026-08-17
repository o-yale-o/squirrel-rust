local t = { v = 1 };
local w = t.weakref();
print(typeof w + " ");
print(w.ref().v + "\n");
t = null;
collectgarbage();
print(w.ref() == null ? "dead" : "alive");
print("\n");

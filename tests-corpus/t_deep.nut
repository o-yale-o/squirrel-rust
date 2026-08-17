function rec(n) { return rec(n + 1); }
try { rec(0); } catch(e) { print(e + "\n"); }
print("after\n");

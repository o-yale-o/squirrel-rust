local t = {};
try { local x = t.missing; } catch(e) { print(e + "\n"); }
local a = [1,2,3];
try { local x = a[10]; } catch(e) { print(e + "\n"); }
try { local x = a["s"]; } catch(e) { print(e + "\n"); }
local s = "abc";
try { local x = s[5]; } catch(e) { print(e + "\n"); }

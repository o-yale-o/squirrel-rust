try { local x = 1; x(); } catch(e) { print(e + "\n"); }
try { local x = "s"; x(); } catch(e) { print(e + "\n"); }
function f(a, b) { return a+b; }
try { f(1); } catch(e) { print(e + "\n"); }
try { f(1,2,3); } catch(e) { print(e + "\n"); }
function g(a, ...) {}
try { g(); } catch(e) { print(e + "\n"); }

local b = blob(8);
for(local i = 0; i < 8; i++) b[i] = (i + 1) * 17;
print(b.len() + "\n");
print(b[0] + " " + b[7] + "\n");
b.swap2();
print(b[0] + " " + b[1] + " " + b[2] + " " + b[3] + "\n");
b.swap2();
b.swap4();
print(b[0] + " " + b[1] + " " + b[2] + " " + b[3] + "\n");
b.swap4();
local c = clone(b);
c[0] = 99;
print(b[0] + " " + c[0] + "\n");
b.resize(4);
print(b.len() + "\n");
b.resize(6);
print(b.len() + " " + b[4] + "\n");
foreach(i, v in b) print(i + ":" + v + " ");
print("\n");
print(typeof b + "\n");
b.seek(2, 'b');
print(b.tell() + "\n");
print(b.readn('b') + "\n");
local rb = b.readblob(10);
print("rb=" + rb.len() + "\n");
print("eos=" + (b.eos() != null) + "\n");
try { b[100] = 1; } catch(e) { print(e + "\n"); }
try { local x = b[-1]; } catch(e) { print(e + "\n"); }
local neg = blob(-5);

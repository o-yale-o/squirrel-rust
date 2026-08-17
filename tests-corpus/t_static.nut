class C {
    </ a = "attr" /> static version = 1;
    </ doc = "field" /> x = 5;
    </ m = "m1" /> function foo() { return version; }
}
print(C.version + "\n");
local attrs = C.getattributes("x");
print(attrs.doc + "\n");
local ma = C.getattributes("foo");
print(ma.m + "\n");
local i = C();
print(i.foo() + "\n");

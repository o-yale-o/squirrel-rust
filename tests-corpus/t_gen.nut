function g() {
    yield 1;
    try {
        yield 2;
    } catch(e) {
        yield "caught:" + e;
    }
    return "end";
}
local gen = g();
print(resume gen); print(" ");
print(resume gen); print(" ");
print(resume gen); print("\n");
print(gen.getstatus() + "\n");
try { resume gen; } catch(e) { print(e + "\n"); }

function inner(a, b) {
    local c = a + b;
    local arr = [1, 2, 3];
    return nonexistent_global_call(c, arr);
}
function outer() {
    local t = { x = 1, y = "s" };
    local s = "local-string";
    return inner(1, 2);
}
print("ERRTAIL-DONE\n");
outer();

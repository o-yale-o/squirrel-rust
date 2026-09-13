function inner(a, b) {
    local x = a + b;
    local s = "str";
    local t = { k = 1 };
    local infos = getstackinfos(1);
    foreach(k, v in infos.locals) {
        print(k + "=");
        if (typeof v == "table") print("{table}");
        else if (typeof v == "string") print("\"" + v + "\"");
        else print(v);
        print(" ");
    }
    print("\n");
    print("func=" + infos.func + " line=" + infos.line + "\n");
    return x;
}
function outer() {
    local o = 42;
    return inner(o, o + 1);
}
print(outer() + "\n");
try { throw "err"; } catch(e) { print("caught " + e + "\n"); }

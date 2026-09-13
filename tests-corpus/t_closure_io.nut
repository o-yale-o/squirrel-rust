// 脚本级闭包序列化（↔ sqstdio.cpp _g_io_writeclosuretofile：掩码 .sc、返回闭包本身）
function show(name, f) {
    try {
        local v = f();
        print(name + "=" + typeof v + ":" + v + "\n");
    } catch (e) {
        print(name + "!" + e + "\n");
    }
}
local f = function(a, b) { return a * b + 1; };
show("write", function() { return typeof writeclosuretofile("tmp_closure_io.cnut", f); });
show("call_written", function() {
    local g = loadfile("tmp_closure_io.cnut");
    return g(6, 7);
});
show("roundtrip", function() {
    local h = function(s) { return "[" + s + "]"; };
    local ret = writeclosuretofile("tmp_closure_io2.cnut", h);
    return ret("x") + loadfile("tmp_closure_io2.cnut")("y");
});
show("noargs", function() { return writeclosuretofile(); });
show("badarg2", function() { return writeclosuretofile("tmp_x.cnut", 1); });
show("badpath", function() { return writeclosuretofile("no_such_dir/x.cnut", f); });
remove("tmp_closure_io.cnut");
remove("tmp_closure_io2.cnut");
print("done\n");

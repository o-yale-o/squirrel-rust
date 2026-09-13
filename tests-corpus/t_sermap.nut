// 自动生成：python tools/gen_ser.py —— 请勿手改
// 序列化往返：writeclosuretofile 写 .cnut → loadfile 读回并调用
//（写文件名里带 0x… 的用例靠语料的指针归一化对齐）

// 序列化往返：writeclosuretofile + loadfile（脚本侧 .cnut 路径）
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
local P = "tmp-c2/ser_tmp.cnut";

// 1) 基本往返：写闭包 → loadfile 读回 → 调用
s("roundtrip_plain", function() {
    local f = function() { return 42; };
    // 文件可能不存在（第一次跑）——remove 失败会抛错，所以包起来
    try { remove(P); } catch (e) { }
    local r = writeclosuretofile(P, f);
    local g = loadfile(P);
    return typeof r + "/" + typeof g + "/" + g();
});
s("roundtrip_print", function() {
    local f = function() { print("inner-print\n"); return 1; };
    writeclosuretofile(P, f);
    local g = loadfile(P);
    return "r" + g();
});
s("roundtrip_args", function() {
    local f = function(a, b) { return a + b; };
    writeclosuretofile(P, f);
    return loadfile(P)(2, 3);
});
s("roundtrip_default", function() {
    local f = function(a, b = 10) { return a + b; };
    writeclosuretofile(P, f);
    local g = loadfile(P);
    return g(1) + "/" + g(1, 2);
});
s("roundtrip_varargs", function() {
    local f = function(...) { return vargv.len(); };
    writeclosuretofile(P, f);
    return loadfile(P)(1, 2, 3);
});
s("roundtrip_strings", function() {
    local f = function() { return "abc\u00e9\xff"; };
    writeclosuretofile(P, f);
    local g = loadfile(P);
    local v = g();
    return v.len() + "/" + (v == "abc\u00e9\xff");
});
s("roundtrip_floats", function() {
    local f = function() { return 1.5 + 0.25; };
    writeclosuretofile(P, f);
    return loadfile(P)();
});
s("roundtrip_table_literal", function() {
    local f = function() { local t = { a = 1, b = "x" }; return t.a + t.b; };
    writeclosuretofile(P, f);
    return loadfile(P)();
});
s("roundtrip_class", function() {
    local f = function() { local C = class { v = 7; function get() { return v; } }; return C().get(); };
    writeclosuretofile(P, f);
    return loadfile(P)();
});
s("roundtrip_nested_fn", function() {
    local f = function() { local inner = function(q) { return q * 2; }; return inner(21); };
    writeclosuretofile(P, f);
    return loadfile(P)();
});
s("roundtrip_control_flow", function() {
    local f = function(n) { local s = 0; for (local i = 0; i < n; i++) { if (i % 2 == 0) s += i; } return s; };
    writeclosuretofile(P, f);
    return loadfile(P)(10);
});
s("roundtrip_recurse", function() {
    local f = function(n) { return n <= 1 ? 1 : n * f(n - 1); };
    writeclosuretofile(P, f);
    return loadfile(P)(5);
});
s("roundtrip_generator", function() {
    local f = function() { local g = (function() { yield 1; yield 2; })(); return resume g + resume g; };
    writeclosuretofile(P, f);
    return loadfile(P)();
});

// 2) 不能序列化的形态
s("ser_with_outer", function() {
    local captured = 5;
    local f = function() { return captured; };
    return "" + writeclosuretofile(P, f);
});
s("ser_with_this_outer", function() {
    local f = function() { return this; };
    return "" + writeclosuretofile(P, f);
});
s("ser_nonclosure", function() { return "" + writeclosuretofile(P, 5); });
s("ser_native", function() { return "" + writeclosuretofile(P, print); });
s("ser_table", function() { return "" + writeclosuretofile(P, {}); });
s("ser_missing_dir", function() { local f = function() { return 1; }; return "" + writeclosuretofile("no_such_dir_xyz/out.cnut", f); });
s("ser_one_arg", function() { try { return "" + writeclosuretofile(P); } catch (e) { return "E:" + e; } });
s("ser_class_closure_method", function() {
    local C = class { function m() { return 9; } };
    local o = C();
    local f = o.m;   // 绑定了 this 的闭包
    return "" + writeclosuretofile(P, f);
});

// 3) 读回字节码的边界
s("load_missing", function() { return "" + loadfile("tmp-c2/no_such_file.cnut"); });
s("load_text_as_cnut", function() {
    local p = "tmp-c2/ser_text.nut";
    local f = file(p, "wb"); f.writen(114, 'b'); f.writen(101, 'b'); f.writen(116, 'b'); f.close();
    return "" + loadfile(p);
});
s("load_empty_file", function() {
    local p = "tmp-c2/ser_empty.cnut";
    local f = file(p, "wb"); f.close();
    return "" + loadfile(p);
});

// 4) 往返后再往返（写回来的闭包能不能再写）
s("double_roundtrip", function() {
    local f = function(a) { return a * 3; };
    writeclosuretofile(P, f);
    local g = loadfile(P);
    writeclosuretofile(P, g);
    return loadfile(P)(4);
});

print("SERMAP-DONE\n");

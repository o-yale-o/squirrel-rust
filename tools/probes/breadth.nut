// 标准库广度探针（不依赖 getdelegate）：逐项调用并打印，便于双引擎差分
local function show(tag, f) {
    try { print(tag + " => " + f() + "\n"); }
    catch (e) { print(tag + " !! " + e + "\n"); }
}

// ---- string ----
show("len", function() { return "abc".len(); });
show("slice", function() { return "hello".slice(1, 3); });
show("find", function() { return "hello".find("l"); });
show("tolower", function() { return "ABC".tolower(); });
show("toupper", function() { return "abc".toupper(); });
show("tointeger", function() { return "42abc".tointeger(); });
show("tofloat", function() { return "3.5x".tofloat(); });
show("tostring", function() { return (12).tostring(); });
show("format", function() { return format("%d|%s|%5.2f|%x", 42, "s", 3.14159, 255); });
show("format2", function() { return format("%c%o%e%g%%", 65, 8, 1234.5, 0.0001); });
show("split", function() { local a = split("a,b,,c", ","); return a.len() + ":" + a[2].len(); });
show("strip", function() { return "[" + strip("  x  ") + "]"; });
show("lstrip", function() { return "[" + lstrip("  x  ") + "]"; });
show("rstrip", function() { return "[" + rstrip("  x  ") + "]"; });
show("escape", function() { return escape("a\nb\t\"c\""); });
show("startswith", function() { return startswith("hello", "he"); });
show("endswith", function() { return endswith("hello", "lo"); });
show("printf", function() { printf("%s-%d\n", "pf", 7); return "ok"; });
show("str_index", function() { return "abc"[1]; });
show("tochar", function() { return (66).tochar(); });

// ---- math ----
show("sqrt", function() { return sqrt(2.0); });
show("pow", function() { return pow(2, 10); });
show("abs_int", function() { return abs(-7); });
show("abs_float", function() { return abs(-7.5); });
show("floor", function() { return floor(2.7); });
show("ceil", function() { return ceil(2.1); });
show("sin", function() { return sin(0.5); });
show("log", function() { return log(2.718281828); });
show("exp", function() { return exp(1.0); });
show("atan2", function() { return atan2(1.0, 2.0); });
show("fabs", function() { return fabs(-3.25); });
show("RAND_MAX", function() { return RAND_MAX; });
show("srand/rand", function() { srand(1); local a = rand(); srand(1); local b = rand(); return (a == b) + ":" + (a >= 0); });
show("casti2f", function() { return casti2f(1065353216); });
show("castf2i", function() { return castf2i(1.0); });
show("swapfloat", function() { return swapfloat(1.0); });

// ---- system / io ----
show("clock", function() { return typeof clock(); });
show("time", function() { return typeof time(); });
show("date", function() { local d = date(); return d.year + "-" + (d.month + 1) + "-" + d.day + " " + typeof d.hour; });
show("date_fmt", function() { local d = date(0, 'u'); return typeof d.year; });
show("getenv", function() { return typeof getenv("PATH"); });
show("system", function() { return system("cmd /c exit 3"); });
show("_version_", function() { return _version_ + "/" + _versionnumber_ + "/" + _intsize_ + "/" + _floatsize_ + "/" + _charsize_; });

// ---- blob ----
show("blob_len", function() { local b = blob(8); return b.len(); });
show("blob_resize", function() { local b = blob(4); b.resize(10); return b.len(); });
show("blob_swap2", function() { local b = blob(2); b[0] = 1; b[1] = 2; b.swap2(); return b[0] + ":" + b[1]; });
show("blob_swap4", function() { local b = blob(4); b[0] = 1; b[1] = 2; b[2] = 3; b[3] = 4; b.swap4(); return b[0] + ":" + b[3]; });
show("blob_write/read", function() {
    local b = blob(16); local s = stream(); s.writeblob(b); s.seek(0); return s.readblob(b ? 4 : 4).len();
});
show("blob_tostring", function() { local b = blob(3); b[0] = 65; b[1] = 66; b[2] = 67; return b.tostring(); });

// ---- stream / file ----
show("stream_writen", function() { local s = stream(); s.writen(0x41, 'b'); s.writen(1234, 'i'); s.seek(0); return s.readn('b') + ":" + s.readn('i'); });
show("stream_len_tell_eos", function() { local s = stream(); s.writen(1, 'b'); return s.len() + ":" + s.tell() + ":" + s.eos(); });
show("file_write_read", function() {
    local f = file("tmp-c2\\probe_io.txt", "wb+");
    f.writen(7, 'i'); f.seek(0); local v = f.readn('i'); f.close();
    return v;
});
show("stdout_type", function() { return typeof stdout + ":" + typeof stderr + ":" + typeof stdin; });

// ---- regex ----
show("re_match", function() { return regexp(@"\d+").match("abc123"); });
show("re_capture", function() { return regexp(@"(\w)(\d)").capture("a1"); });
show("re_search", function() { local r = regexp(@"(\d+)"); local s = r.search("a12b34"); return s[0] + "/" + s[1]; });
show("re_balanced", function() { return regexp(@"\m<>\d+").match("x<42>"); });
show("re_icase", function() { return regexp(@"abc", 'i').match("ABC"); });
show("re_nongreedy", function() { return regexp(@"a.*?b").match("axxbxxb"); });
show("re_wordbound", function() { return regexp(@"\bfoo\b").match("a foo b"); });
show("re_class", function() { return regexp(@"[a-c]+").match("zzabc"); });
show("re_repeat", function() { return regexp(@"a{2,3}").match("aaa"); });

// ---- base ----
show("type/typeof", function() { return type(1) + ":" + typeof 1.5; });
show("assert", function() { assert(1 == 1); return "assert-ok"; });
show("compilestring", function() { local f = compilestring("return 40+2", "cs"); return f(); });
show("callee", function() { return typeof callee(); });
show("getroottable", function() { return typeof getroottable(); });
show("setroottable_consts", function() { local c = getconsttable(); setconsttable(c); return typeof c; });
show("array_methods", function() { local a = [3,1,2]; a.sort(); a.reverse(); a.append(9); return a.tostring() + ":" + a.top() + ":" + a.pop(); });
show("array_map_filter", function() { local a = [1,2,3,4]; local m = a.map(function(v){return v*2}); local f = a.filter(function(v){return v%2==0}); return m.tostring() + ":" + f.tostring(); });
show("array_reduce", function() { return [1,2,3,4].reduce(function(a,b){return a+b}, 0); });
show("table_methods", function() { local t = {a=1}; t.rawset("b", 2); return t.len() + ":" + ("a" in t) + ":" + t.rawget("b"); });
show("table_setdelegate", function() { local p = {x=1}; local c = {}; c.setdelegate(p); return c.x; });
show("weakref", function() { local o = {}; local w = o.weakref(); return typeof w + ":" + (w.ref() != null); });
show("newthread", function() { local t = newthread(function(){ return 5; }); t.call(); return t.getstatus(); });
show("suspend/resume", function() { local t = newthread(function(){ suspend("s"); return "done"; }); t.call(); return t.wakeup(); });
show("collectgarbage", function() { collectgarbage(); return "gc-ok"; });

// ---- class / metadata ----
class Base { v = 10; function get() { return v; } }
class Derived extends Base { constructor() { base.constructor(); } function get() { return base.get() + 1; } }
show("class_basic", function() { local d = Derived(); return d.get() + ":" + (d instanceof Base) + ":" + (d instanceof Derived); });
show("class_attributes", function() { local c = Derived; return typeof c.getattributes(); });
show("getstackinfos0", function() { local i = getstackinfos(0); return typeof i.func + ":" + typeof i.src + ":" + typeof i.line; });

// ---- metamethods ----
class MM {
    x = 0;
    constructor(v) { x = v; }
    function _add(o) { return MM(x + o.x); }
    function _cmp(o) { return x <=> o.x; }
    function _tostring() { return "MM(" + x + ")"; }
    function _get(k) { return "dyn:" + k; }
}
show("mm_add", function() { return (MM(1) + MM(2)).x; });
show("mm_cmp", function() { return (MM(1) < MM(2)); });
show("mm_tostring", function() { return "" + MM(5); });
show("mm_get", function() { return MM(1).whatever; });

// ---- serialization ----
show("cnut_roundtrip", function() {
    local f = function(a, b) { return a * b + 1; };
    writeclosuretofile("tmp-c2\\probe_c.cnut", f);
    local g = loadfile("tmp-c2\\probe_c.cnut");
    return g(6, 7);
});

// ---- generator ----
show("gen", function() {
    local g = function() { for (local i = 0; i < 3; i++) yield i; return "end"; }();
    local acc = "";
    local r = resume g;
    while (g.getstatus() == "suspended") { acc += r; r = resume g; }
    return acc + ":" + g.getstatus();
});
print("PROBE DONE\n");

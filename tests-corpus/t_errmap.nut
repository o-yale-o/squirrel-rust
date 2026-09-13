// 自动生成：python tools/gen_errmap.py —— 请勿手改
// 异常机制矩阵：throw 各类型、VM 错误消息、嵌套 trap、错误处理器、getstackinfos
// 注：getstackinfos(-1) 是参考实现的越界读（UB），语料里已去掉，见 exception_semantics.rs

// 异常机制探测：错误对象、消息、嵌套 trap、错误处理器、getstackinfos
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
function th(x) { throw x; }
function wrap(tag, f) {
    try { return f(); } catch (e) { return "CAUGHT:" + typeof e + ":" + e; }
}

// 1) throw 各种类型的值与错误对象
s("throw_str", function() { return wrap("t", function() { throw "boom"; }); });
s("throw_int", function() { return wrap("t", function() { throw 42; }); });
s("throw_float", function() { return wrap("t", function() { throw 1.5; }); });
s("throw_null", function() { return wrap("t", function() { throw null; }); });
s("throw_true", function() { return wrap("t", function() { throw true; }); });
s("throw_table", function() { return wrap("t", function() { throw { code = 7 }; }); });
s("throw_array", function() { return wrap("t", function() { throw [1, 2]; }); });
s("throw_func", function() { return wrap("t", function() { throw function() { return 1; }; }); });
s("throw_class", function() { return wrap("t", function() { throw class { }; }); });
s("throw_instance", function() { return wrap("t", function() { local C = class {}; throw C(); }); });
s("throw_blob", function() { return wrap("t", function() { throw blob(2); }); });
s("throw_regexp", function() { return wrap("t", function() { throw regexp("a"); }); });
s("throw_thread", function() { return wrap("t", function() { throw newthread(function() { return 1; }); }); });
s("throw_generator", function() { return wrap("t", function() { throw (function() { yield 1; })(); }); });
s("throw_weakref", function() { return wrap("t", function() { local t = {}; throw t.weakref(); }); });
s("throw_in_native", function() { return wrap("t", function() { throw "x"; }); });
s("throw_obj_tostring", function() { return wrap("t", function() { local t = {}; t._tostring <- function() { return "OBJ"; }; throw t; }); });

// 2) VM 自己抛的错误：消息与可捕获性
s("err_index_missing", function() { return wrap("t", function() { local t = {}; return t.nope; }); });
s("err_index_on_int", function() { return wrap("t", function() { local x = 5; return x.foo; }); });
s("err_call_int", function() { return wrap("t", function() { local x = 5; return x(); }); });
s("err_call_str", function() { return wrap("t", function() { local x = "s"; return x(); }); });
s("err_call_table", function() { return wrap("t", function() { local x = {}; return x(); }); });
s("err_call_null", function() { return wrap("t", function() { local x = null; return x(); }); });
s("err_call_array", function() { return wrap("t", function() { local x = []; return x(); }); });
s("err_call_bool", function() { return wrap("t", function() { local x = true; return x(); }); });
s("err_arith", function() { return wrap("t", function() { local x = {}; return x + 1; }); });
s("err_arith2", function() { return wrap("t", function() { return "a" - 1; }); });
s("err_unm", function() { return wrap("t", function() { local x = {}; return -x; }); });
s("err_bitw", function() { return wrap("t", function() { local x = {}; return x & 1; }); });
s("err_div0", function() { return wrap("t", function() { return 1 / 0; }); });
s("err_local_uninit", function() { return wrap("t", function() { local x; return x + 1; }); });
s("err_newslot_int", function() { return wrap("t", function() { local x = 5; x.k <- 1; return x.k; }); });
s("err_delete_int", function() { return wrap("t", function() { local x = 5; return delete x.k; }); });
s("err_array_oob", function() { return wrap("t", function() { local a = [1]; return a[5]; }); });
s("err_neg_array", function() { return wrap("t", function() { local a = [1]; return a[-5]; }); });
s("err_table_no_key", function() { return wrap("t", function() { local t = {}; return t["k"]; }); });
s("err_instance_missing", function() { return wrap("t", function() { local C = class { v = 1 }; local o = C(); return o.nope; }); });
s("err_class_missing", function() { return wrap("t", function() { local C = class { }; return C.nope; }); });
s("err_native_params", function() { return wrap("t", function() { return "abc".slice(); }); });
s("err_native_type", function() { return wrap("t", function() { return abs("x"); }); });
s("err_closure_params", function() { return wrap("t", function() { local f = function(a) { return a; }; return f(); }); });
s("err_too_many", function() { return wrap("t", function() { local f = function(a) { return a; }; return f(1, 2); }); });
s("err_ctor_params", function() { return wrap("t", function() { local C = class { constructor(a) { } }; return C(); }); });

// 3) 嵌套 trap / catch 里再抛 / 重抛 / 作用域
s("nested_inner", function() { try { try { throw "inner"; } catch (e) { return "got:" + e; } } catch (e) { return "outer:" + e; } });
s("nested_rethrow", function() { try { try { throw "a"; } catch (e) { throw "b:" + e; } } catch (e) { return "outer:" + e; } });
s("throw_in_catch", function() { try { try { throw "x"; } catch (e) { throw e; } } catch (e2) { return "again:" + e2; } });
s("catch_scope", function() { local e = "outer"; try { throw "inner"; } catch (e) { } return e; });
s("catch_after_body", function() { local out = ""; try { out += "t"; throw "e"; out += "unreached"; } catch (ex) { out += "c"; } return out; });
s("no_catch_match", function() { try { throw "e"; } catch (x) { return "always"; } return "never"; });
s("try_in_loop", function() { local out = ""; for (local i = 0; i < 3; i++) { try { if (i == 1) throw i; out += i; } catch (e) { out += "c" + e; } } return out; });
s("try_in_func", function() { local f = function() { try { throw "deep"; } catch (e) { return "in:" + e; } }; return f(); });
s("throw_through_frames", function() { local a = function() { local b = function() { throw "up"; }; b(); }; try { a(); } catch (e) { return "caught:" + e; } });
s("return_in_try", function() { try { return "r"; } catch (e) { return "c"; } });
s("return_in_catch", function() { try { throw "x"; } catch (e) { return "c"; } });
s("empty_catch_body", function() { try { throw "x"; } catch (e) { } return "after"; });
s("semantics_after_error", function() { local x = 1; try { throw "e"; } catch (e) { x = 2; } return x; });

// 4) 错误处理器（seterrorhandler）
s("errorhandler_set", function() {
    local seen = "";
    seterrorhandler(function(err) { seen = "H:" + err; });
    try { throw "handled?"; } catch (e) { }
    return seen == "" ? "not-called-for-caught" : seen;
});
s("errorhandler_uncaught", function() {
    local eh = getroottable().getdelegate();
    return typeof eh;
});
s("errorhandler_native", function() {
    local calls = 0;
    seterrorhandler(function(err) { calls++; });
    try { local t = {}; t.nope; } catch (e) { }
    seterrorhandler(null);
    return calls;
});

// 5) getstackinfos 层级
function errprobe_named() { return getstackinfos(0).func; }
s("si_level0", function() { local si = getstackinfos(0); return typeof si + "/" + typeof si.func; });
s("si_named", function() { return errprobe_named(); });
s("si_src", function() { return typeof getstackinfos(0).src; });
s("si_line_type", function() { return typeof getstackinfos(0).line; });
s("si_locals_type", function() { return typeof getstackinfos(0).locals; });
s("si_level1", function() { local inner = function() { return typeof getstackinfos(1); }; return inner(); });
s("si_level_deep", function() { local a = function() { local b = function() { return typeof getstackinfos(2); }; return b(); }; return a(); });
s("si_level_oob", function() { return typeof getstackinfos(50); });
s("si_native_frame", function() { return typeof getstackinfos(1); });
s("si_args", function() { local f = function(p, q) { local si = getstackinfos(0); local n = 0; foreach (k, v in si.locals) n++; return typeof si.locals + "/" + (n >= 2); }; return f(1, 2); });
s("si_this", function() { local C = class { function f() { return typeof getstackinfos(0).locals; } }; return C().f(); });

// 6) try/catch 与生成器/线程
s("gen_try_catch", function() { local g = (function() { try { yield 1; throw "in-gen"; } catch (e) { yield "caught:" + e; } })(); local a = resume g; local b = resume g; return a + "/" + b; });
s("gen_err_to_caller", function() { local g = (function() { yield 1; throw "gen-err"; })(); resume g; try { resume g; return "no"; } catch (e) { return "caller:" + e; } });
s("thread_err_to_caller", function() { local t = newthread(function() { throw "thr-err"; }); try { t.call(); return "no"; } catch (e) { return "caller:" + e + "/" + t.getstatus(); } });
s("try_in_thread", function() { local t = newthread(function() { try { throw "in-thread"; } catch (e) { return "caught:" + e; } }); return t.call(); });
s("catch_in_generator", function() { local g = (function() { local r = ""; try { r = "t"; throw "g"; } catch (e) { r += "c"; } yield r; })(); return resume g; });

// 7) 元方法里抛错
s("meta_get_throws", function() { local C = class { function _get(k) { throw "from-_get"; } }; local o = C(); return wrap("t", function() { return o.anything; }); });
s("meta_call_throws", function() { local C = class { function _call(a) { throw "from-_call"; } }; local o = C(); return wrap("t", function() { return o(); }); });
s("meta_tostring_throws", function() { local C = class { function _tostring() { throw "from-_tostring"; } }; local o = C(); return wrap("t", function() { return "" + o; }); });
s("meta_add_throws", function() { local C = class { function _add(o) { throw "from-_add"; } }; local o = C(); return wrap("t", function() { return o + 1; }); });

// 8) 构造器里抛错
s("ctor_throws", function() { local C = class { constructor() { throw "ctor-fail"; } }; return wrap("t", function() { return C(); }); });
s("ctor_throws_caught_inside", function() { local C = class { constructor() { try { throw "x"; } catch (e) { } } }; local o = C(); return "made:" + typeof o; });

print("ERRMAP-DONE\n");

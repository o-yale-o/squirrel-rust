# -*- coding: utf-8 -*-
"""生成 metamethod 全矩阵语料 tests-corpus/t_meta3.nut。

参考实现的 18 个 metamethod（sqstate.cpp:123-140 的注册顺序）：
    _add _sub _mul _div _unm _modulo _set _get _typeof _nexti _cmp
    _call _cloned _newslot _delslot _tostring _newmember _inherited

每个用例都覆盖：正常路径 / 返回值类型不对 / 抛异常 / 参数个数不对 / 值不是函数，
以及「没有 metamethod 时的回退与报错文本」。全部包在 try/catch 里，
脚本必须正常跑到最后（否则 diff_test.py 判 SUSPECT）。

用法：
    python tools/gen_metaprobe.py           # 写出 tests-corpus/t_meta3.nut
"""
import os

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_meta3.nut")

CASES = []
A = CASES.append

# ---------------------------------------------------------------- _get
A(("get_tbl_hit", 'local t = {}; t.setdelegate({ _get = function(k) { return "G:" + k; } }); return t.foo;'))
A(("get_tbl_hit_int", 'local t = {}; t.setdelegate({ _get = function(k) { return "G:" + k; } }); return t[3];'))
A(("get_tbl_hit_bool", 'local t = {}; t.setdelegate({ _get = function(k) { return "G:" + k; } }); return t[true];'))
A(("get_tbl_hit_nullkey", 'local t = {}; t.setdelegate({ _get = function(k) { return "G:" + k; } }); return t[null];'))
A(("get_tbl_hit_tablekey", 'local t = {}; t.setdelegate({ _get = function(k) { return "G"; } }); return t[{}];'))
A(("get_tbl_own_slot", 'local t = { foo = 1 }; t.setdelegate({ _get = function(k) { print("MT;"); return "G"; } }); return t.foo;'))
A(("get_tbl_deleg_slot", 'local t = {}; t.setdelegate({ foo = 5, _get = function(k) { print("MT;"); return "G"; } }); return t.foo;'))
A(("get_tbl_missing", 'local t = {}; return t.foo;'))
A(("get_tbl_missing_deleg", 'local t = {}; t.setdelegate({ bar = 1 }); return t.foo;'))
A(("get_tbl_mt_null", 'local t = {}; t.setdelegate({ _get = function(k) { return null; } }); return t.foo;'))
A(("get_tbl_mt_throw", 'local t = {}; t.setdelegate({ _get = function(k) { throw "boom"; } }); return t.foo;'))
A(("get_tbl_mt_throw_null", 'local t = {}; t.setdelegate({ _get = function(k) { throw null; } }); try { return t.foo; } catch (e) { return "caught:" + e; }'))
A(("get_tbl_mt_arity0", 'local t = {}; t.setdelegate({ _get = function() { return 1; } }); return t.foo;'))
A(("get_tbl_mt_arity2", 'local t = {}; t.setdelegate({ _get = function(k, x) { return x; } }); return t.foo;'))
A(("get_tbl_mt_notfunc", 'local t = {}; t.setdelegate({ _get = 5 }); return t.foo;'))
A(("get_tbl_chain", 'local a = {}; a.setdelegate({ _get = function(k) { return "A"; } }); local b = {}; b.setdelegate(a); return b.foo;'))
A(("get_inst_hit", 'class C { function _get(k) { return "I:" + k; } } local c = C(); return c.foo;'))
A(("get_inst_own", 'class C { foo = 1; function _get(k) { print("MT;"); return "I"; } } local c = C(); return c.foo;'))
A(("get_inst_missing", 'class C { } local c = C(); return c.foo;'))
A(("get_inst_base", 'class B { x = 1; } class D extends B { } return D().x;'))
A(("get_inst_mt_null", 'class C { function _get(k) { return null; } } return C().foo;'))
A(("get_cls_missing", 'class C { } return C.foo;'))
A(("get_cls_static", 'class C { static s = 2; } return C.s;'))
A(("get_arr_oob", 'return [1][5];'))
A(("get_arr_neg", 'local a = [1, 2]; return a[-1];'))
A(("get_arr_float", 'local a = [1, 2]; return a[0.5];'))
A(("get_arr_strkey", 'local a = [1, 2]; return a["x"];'))
A(("get_arr_attr", 'return [1].foo;'))
A(("get_str_char", 'return "abc"[1];'))
A(("get_str_neg", 'return "abc"[-1];'))
A(("get_str_oob", 'return "abc"[9];'))
A(("get_str_float", 'return "abc"[1.7];'))
A(("get_str_attr", 'return "abc".foo;'))
A(("get_int_attr", 'return (1).foo;'))
A(("get_flt_attr", 'return (1.5).foo;'))
A(("get_bool_attr", 'return true.foo;'))
A(("get_null_idx", 'return null[0];'))
A(("get_null_attr", 'return null.foo;'))
A(("get_tbl_ddel_len", 'local t = { a = 1 }; return t.len();'))
A(("get_arr_ddel_len", 'return [1, 2].len();'))

# ---------------------------------------------------------------- _set / _newslot
A(("set_tbl_missing", 'local t = {}; t.foo = 1; return "ok";'))
A(("set_tbl_mt", 'local t = {}; t.setdelegate({ _set = function(k, v) { print("S:" + k + "=" + v + ";"); } }); t.foo = 1; return "ok";'))
A(("set_tbl_mt_return", 'local t = {}; t.setdelegate({ _set = function(k, v) { return 99; } }); t.foo = 1; return t.rawin("foo");'))
A(("set_tbl_mt_existing", 'local t = { foo = 0 }; t.setdelegate({ _set = function(k, v) { print("S;"); } }); t.foo = 1; return t.foo;'))
A(("set_tbl_mt_throw", 'local t = {}; t.setdelegate({ _set = function(k, v) { throw "nope"; } }); t.foo = 1; return "ok";'))
A(("set_tbl_mt_arity1", 'local t = {}; t.setdelegate({ _set = function(k) { return 1; } }); t.foo = 1; return "ok";'))
A(("set_tbl_mt_arity3", 'local t = {}; t.setdelegate({ _set = function(k, v, x) { return 1; } }); t.foo = 1; return "ok";'))
A(("set_tbl_mt_notfunc", 'local t = {}; t.setdelegate({ _set = 5 }); t.foo = 1; return "ok";'))
A(("set_tbl_deleg_write", 'local d = { a = 5 }; local t = {}; t.setdelegate(d); t.a = 1; return d.a + "," + t.a;'))
A(("set_tbl_deleg_write_mt", 'local d = { a = 5, _set = function(k, v) { print("S;"); } }; local t = {}; t.setdelegate(d); t.a = 1; return "ok";'))
A(("set_tbl_newslot_mt", 'local t = {}; t.setdelegate({ _newslot = function(k, v) { print("N:" + k + "=" + v + ";"); } }); t.foo <- 1; return t.rawin("foo");'))
A(("set_tbl_newslot_mt_existing", 'local t = { foo = 1 }; t.setdelegate({ _newslot = function(k, v) { print("N;"); } }); t.foo <- 2; return t.foo;'))
A(("set_tbl_newslot_mt_deleg", 'local t = {}; t.setdelegate({ bar = 9, _newslot = function(k, v) { print("N;"); } }); t.bar <- 1; return t.rawin("bar");'))
A(("set_tbl_newslot_mt_throw", 'local t = {}; t.setdelegate({ _newslot = function(k, v) { throw "ns"; } }); t.foo <- 1; return "ok";'))
A(("set_tbl_newslot_no_mt", 'local t = {}; t.foo <- 1; return t.foo;'))
A(("set_tbl_newslot_nullkey", 'local t = {}; t[null] <- 1; return "ok";'))
A(("set_tbl_set_nullkey", 'local t = {}; t[null] = 1; return "ok";'))
A(("set_tbl_both_mt", 'local t = {}; t.setdelegate({ _set = function(k, v) { print("S;"); }, _newslot = function(k, v) { print("N;"); } }); t.a = 1; t.b <- 2; return "ok";'))
A(("set_inst_missing", 'class C { function _set(k, v) { print("S:" + k + "=" + v + ";"); } } local c = C(); c.foo = 1; return "ok";'))
A(("set_inst_existing", 'class C { foo = 0; function _set(k, v) { print("S;"); } } local c = C(); c.foo = 1; return c.foo;'))
A(("set_inst_missing_no_mt", 'class C { } local c = C(); c.foo = 1; return "ok";'))
A(("set_inst_newslot", 'class C { } local c = C(); c.foo <- 1; return "ok";'))
A(("set_inst_newslot_mt", 'class C { function _newslot(k, v) { print("N;"); } } local c = C(); c.foo <- 1; return "ok";'))
A(("set_cls_newslot", 'class C { } C.foo <- 1; return C.foo;'))
A(("set_cls_existing", 'class C { foo = 1 } C.foo = 2; return C.foo;'))
A(("set_cls_dup", 'class C { foo = 1 } C.foo <- 2; return "ok";'))
A(("set_cls_locked", 'class C { foo = 1 } local i = C(); C.bar <- 1; return "ok";'))
A(("set_arr_idx", 'local a = [1]; a[0] = 5; return a[0];'))
A(("set_arr_oob", 'local a = [1]; a[3] = 5; return "ok";'))
A(("set_arr_float", 'local a = [1, 2]; a[1.2] = 5; return a[1];'))
A(("set_arr_attr", 'local a = [1]; a.foo = 5; return "ok";'))
A(("set_str_idx", 'local s = "a"; s[0] = "b"; return "ok";'))
A(("set_int_attr", 'local i = 1; i.foo = 5; return "ok";'))
A(("set_null_attr", 'local n = null; n.foo = 5; return "ok";'))
A(("set_bool_attr", 'local b = true; b.foo = 5; return "ok";'))
A(("set_gen_attr", 'local g = (function() { yield 1; }()); g.foo = 5; return "ok";'))

# ---------------------------------------------------------------- _delslot
A(("del_tbl_mt", 'local t = {}; t.setdelegate({ _delslot = function(k) { print("D:" + k + ";"); return 7; } }); return delete t.foo;'))
A(("del_tbl_mt_existing", 'local t = { foo = 1 }; t.setdelegate({ _delslot = function(k) { print("D;"); return 7; } }); return delete t.foo;'))
A(("del_tbl_mt_null", 'local t = {}; t.setdelegate({ _delslot = function(k) { return null; } }); return delete t.foo;'))
A(("del_tbl_mt_throw", 'local t = {}; t.setdelegate({ _delslot = function(k) { throw "d"; } }); return delete t.foo;'))
A(("del_tbl_mt_arity0", 'local t = {}; t.setdelegate({ _delslot = function() { return 1; } }); return delete t.foo;'))
A(("del_tbl_mt_notfunc", 'local t = {}; t.setdelegate({ _delslot = 3 }); return delete t.foo;'))
A(("del_tbl", 'local t = { foo = 1 }; return delete t.foo;'))
A(("del_tbl_missing", 'local t = {}; return delete t.foo;'))
A(("del_tbl_deleg", 'local t = {}; t.setdelegate({ foo = 1 }); return delete t.foo;'))
A(("del_arr", 'local a = [1, 2]; return delete a[0];'))
A(("del_arr_oob", 'local a = [1, 2]; return delete a[5];'))
A(("del_arr_attr", 'local a = [1]; return delete a.foo;'))
A(("del_inst_mt", 'class C { function _delslot(k) { print("D;"); return 1; } } return delete C().x;'))
A(("del_inst_field", 'class C { x = 1 } return delete C().x;'))
A(("del_cls", 'class C { x = 1 } return delete C.x;'))
A(("del_str", 'local s = "abc"; return delete s[0];'))
A(("del_int", 'local i = 1; return delete i[0];'))
A(("del_null", 'local n = null; return delete n[0];'))
A(("del_gen", 'local g = (function() { yield 1; }()); return delete g.x;'))

# ---------------------------------------------------------------- 算术 metamethod
for op_txt, op_name in (("+", "add"), ("-", "sub"), ("*", "mul"), ("/", "div"), ("%", "modulo")):
    A(("arith_%s_tbl" % op_name,
       'local t = {}; t.setdelegate({ _%s = function(o) { return "M" + o; } }); return t %s 1;' % (op_name, op_txt)))
    A(("arith_%s_tbl_no_mt" % op_name, 'return {} %s 1;' % op_txt))
    A(("arith_%s_tbl_mt_null" % op_name,
       'local t = {}; t.setdelegate({ _%s = function(o) { return null; } }); return t %s 1;' % (op_name, op_txt)))
    A(("arith_%s_tbl_mt_throw" % op_name,
       'local t = {}; t.setdelegate({ _%s = function(o) { throw "am"; } }); return t %s 1;' % (op_name, op_txt)))
    A(("arith_%s_tbl_mt_arity0" % op_name,
       'local t = {}; t.setdelegate({ _%s = function() { return 1; } }); return t %s 1;' % (op_name, op_txt)))
    A(("arith_%s_tbl_mt_arity2" % op_name,
       'local t = {}; t.setdelegate({ _%s = function(a, b) { return 1; } }); return t %s 1;' % (op_name, op_txt)))
    A(("arith_%s_inst" % op_name,
       'class C { function _%s(o) { return "I" + o; } } return C() %s 1;' % (op_name, op_txt)))
    A(("arith_%s_rhs_only" % op_name,
       'class C { function _%s(o) { return "I"; } } return 1 %s C();' % (op_name, op_txt)))
    A(("arith_%s_arr" % op_name, 'return [1] %s [2];' % op_txt))

A(("arith_add_str_int", 'return "ab" + 1;'))
A(("arith_add_int_str", 'return 1 + "ab";'))
A(("arith_add_null_str", 'return null + "ab";'))
A(("arith_add_tbl_str", 'return {} + "ab";'))
A(("arith_add_str_tbl", 'return "ab" + {};'))
A(("arith_mul_str_int", 'return "ab" * 3;'))
A(("arith_mul_str_float", 'return "ab" * 2.5;'))
A(("arith_mul_str_neg", 'return "ab" * -1;'))
A(("arith_mul_int_str", 'return 3 * "ab";'))
A(("arith_add_bool_int", 'return true + 1;'))
A(("arith_add_int_bool", 'return 1 + true;'))
A(("arith_div_int_zero", 'return 1 / 0;'))
A(("arith_mod_int_zero", 'return 1 % 0;'))
A(("arith_div_float_zero", 'return 1.0 / 0.0;'))
A(("arith_mod_float_zero", 'return 1.0 % 0.0;'))
A(("arith_div_neg_int_zero", 'return -1 / 0;'))
A(("arith_mod_neg_int_zero", 'return -1 % 0;'))
A(("arith_div_int_float_zero", 'return 1 / 0.0;'))
A(("arith_div_floor", 'return -7 / 2;'))
A(("arith_mod_neg", 'return -7 % 3;'))
A(("arith_div_int_min", 'return -9223372036854775807 - 1;'))
A(("arith_int_overflow", 'return 9223372036854775807 + 1;'))
A(("arith_int_underflow", 'return -9223372036854775807 - 2;'))
A(("arith_int_mul_overflow", 'return 9223372036854775807 * 2;'))
A(("arith_float_nan", 'return 0.0 / 0.0;'))
A(("arith_unm_tbl_mt", 'local t = {}; t.setdelegate({ _unm = function() { return "U"; } }); return -t;'))
A(("arith_unm_tbl_mt_null", 'local t = {}; t.setdelegate({ _unm = function() { return null; } }); return -t;'))
A(("arith_unm_tbl_mt_throw", 'local t = {}; t.setdelegate({ _unm = function() { throw "u"; } }); return -t;'))
A(("arith_unm_tbl_mt_arity1", 'local t = {}; t.setdelegate({ _unm = function(a) { return 1; } }); return -t;'))
A(("arith_unm_inst", 'class C { function _unm() { return "U"; } } return -C();'))
A(("arith_unm_null", 'return -null;'))
A(("arith_unm_str", 'return -"a";'))
A(("arith_unm_bool", 'return -true;'))
A(("arith_unm_arr", 'return -[1];'))
A(("arith_bnot_int", 'return ~7;'))
A(("arith_bnot_float", 'return ~1.5;'))
A(("arith_bnot_str", 'return ~"a";'))
A(("arith_bnot_bool", 'return ~true;'))
A(("arith_bnot_null", 'return ~null;'))
A(("arith_bnot_tbl", 'return ~{};'))
A(("arith_bnot_inst_mt", 'class C { function _unm() { return 1; } } return ~C();'))

# ---------------------------------------------------------------- 比较 metamethod
A(("cmp_tbl_mt", 'local t = {}; t.setdelegate({ _cmp = function(o) { return -1; } }); return t <=> {};'))
A(("cmp_tbl_mt_lt", 'local t = {}; t.setdelegate({ _cmp = function(o) { return -1; } }); return t < {};'))
A(("cmp_tbl_mt_ge", 'local t = {}; t.setdelegate({ _cmp = function(o) { return -1; } }); return t >= {};'))
A(("cmp_tbl_mt_float", 'local t = {}; t.setdelegate({ _cmp = function(o) { return 1.5; } }); return t <=> {};'))
A(("cmp_tbl_mt_str", 'local t = {}; t.setdelegate({ _cmp = function(o) { return "x"; } }); return t <=> {};'))
A(("cmp_tbl_mt_null", 'local t = {}; t.setdelegate({ _cmp = function(o) { return null; } }); return t <=> {};'))
A(("cmp_tbl_mt_throw", 'local t = {}; t.setdelegate({ _cmp = function(o) { throw "c"; } }); return t <=> {};'))
A(("cmp_tbl_mt_arity0", 'local t = {}; t.setdelegate({ _cmp = function() { return 0; } }); return t <=> {};'))
A(("cmp_tbl_self", 'local t = {}; return t <=> t;'))
A(("cmp_tbl_self_eq", 'local t = {}; return t == t;'))
A(("cmp_tbl_other_eq", 'local t = {}; return t == {};'))
A(("cmp_inst_mt", 'class C { function _cmp(o) { return 1; } } return C() <=> C();'))
A(("cmp_inst_mt_self", 'class C { } local c = C(); return c <=> c;'))
A(("cmp_arr_self", 'local a = [1]; return a <=> a;'))
A(("cmp_str_str", 'return "a" <=> "b";'))
A(("cmp_str_same", 'return "a" <=> "a";'))
A(("cmp_str_empty", 'return "" <=> "a";'))
A(("cmp_int_str", 'return 1 <=> "a";'))
A(("cmp_int_tbl", 'return 1 <=> {};'))
A(("cmp_int_null", 'return 1 <=> null;'))
A(("cmp_null_int", 'return null <=> 1;'))
A(("cmp_null_null", 'return null <=> null;'))
A(("cmp_null_str", 'return null <=> "a";'))
A(("cmp_null_tbl", 'return null <=> {};'))
A(("cmp_tbl_arr", 'return {} <=> [1];'))
A(("cmp_bool_bool", 'return true <=> false;'))
A(("cmp_bool_int", 'return true <=> 1;'))
A(("cmp_bool_null", 'return true <=> null;'))
A(("cmp_func_self", 'local f = function() {}; return f <=> f;'))
A(("cmp_cls_self", 'local c = class {}; return c <=> c;'))
A(("cmp_gen_self", 'local g = (function() { yield 1; }()); return g <=> g;'))
A(("cmp_wk_self", 'local w = ({}).weakref(); return w <=> w;'))
A(("cmp_thr_self", 'local t = newthread(function() {}); return t <=> t;'))

# ---------------------------------------------------------------- _typeof
A(("typeof_tbl_mt", 'local t = {}; t.setdelegate({ _typeof = function() { return "ZZ"; } }); return typeof t;'))
A(("typeof_tbl_mt_int", 'local t = {}; t.setdelegate({ _typeof = function() { return 5; } }); return typeof t;'))
A(("typeof_tbl_mt_null", 'local t = {}; t.setdelegate({ _typeof = function() { return null; } }); return typeof t;'))
A(("typeof_tbl_mt_throw", 'local t = {}; t.setdelegate({ _typeof = function() { throw "t"; } }); return typeof t;'))
A(("typeof_tbl_mt_arity2", 'local t = {}; t.setdelegate({ _typeof = function(a, b) { return "Z"; } }); return typeof t;'))
A(("typeof_tbl_mt_notfunc", 'local t = {}; t.setdelegate({ _typeof = 1 }); return typeof t;'))
A(("typeof_inst_mt", 'class C { function _typeof() { return "ZZ"; } } return typeof C();'))
for tag, v in [("null", "null"), ("int", "1"), ("float", "1.5"), ("bool", "true"), ("str", '"a"'),
               ("tbl", "{}"), ("arr", "[1]"), ("func", "(function(){})"), ("cls", "(class{})"),
               ("gen", "(function(){yield 1;}())"), ("wk", "(({}).weakref())"), ("thr", "(newthread(function(){}))")]:
    A(("typeof_lit_%s" % tag, "return typeof %s;" % v))

# ---------------------------------------------------------------- _tostring
A(("tostr_mt_tbl", 'local t = {}; t.setdelegate({ _tostring = function() { return "T"; } }); return "" + t;'))
A(("tostr_mt_tbl_int", 'local t = {}; t.setdelegate({ _tostring = function() { return 5; } }); return "" + t;'))
A(("tostr_mt_tbl_null", 'local t = {}; t.setdelegate({ _tostring = function() { return null; } }); return "" + t;'))
A(("tostr_mt_tbl_throw", 'local t = {}; t.setdelegate({ _tostring = function() { throw "ts"; } }); return "" + t;'))
A(("tostr_mt_tbl_arity1", 'local t = {}; t.setdelegate({ _tostring = function(a) { return "T"; } }); return "" + t;'))
A(("tostr_mt_inst", 'class C { function _tostring() { return "TT"; } } return "" + C();'))
A(("tostr_mt_inst_builtin", 'class C { function _tostring() { return "TT"; } } return C().tostring();'))
A(("tostr_mt_tbl_builtin", 'local t = {}; t.setdelegate({ _tostring = function() { return "T"; } }); return t.tostring();'))
A(("tostr_plain_tbl", 'return "" + {};'))
A(("tostr_plain_arr", 'return "" + [1];'))
A(("tostr_plain_cls", 'return "" + class {};'))
A(("tostr_plain_inst", 'return "" + class {}();'))
A(("tostr_plain_clo", 'return "" + function() {};'))
A(("tostr_plain_native", 'return "" + print;'))
A(("tostr_plain_gen", 'return "" + (function() { yield 1; }());'))
A(("tostr_plain_thr", 'return "" + newthread(function() {});'))
A(("tostr_plain_wk", 'return "" + ({}).weakref();'))
A(("tostr_mt_in_format", 'local t = {}; t.setdelegate({ _tostring = function() { return "T"; } }); return format("%s|%s", t, 1);'))
A(("tostr_mt_in_print", 'local t = {}; t.setdelegate({ _tostring = function() { return "T"; } }); print(t); print("|"); return "ok";'))
A(("tostr_mt_in_printf", 'local t = {}; t.setdelegate({ _tostring = function() { return "T"; } }); printf("%s", t); return "ok";'))

# ---------------------------------------------------------------- _nexti / foreach
NEXTI_PROLOG = 'local t = {}; t.setdelegate({ _nexti = function(i) { %s }, _get = function(k) { return "v" + k; } }); local out = ""; foreach (k, v in t) out += k + "=" + v + ";"; return out;'
A(("foreach_tbl", 'local t = { a = 1, b = 2 }; local out = ""; foreach (k, v in t) out += k + "=" + v + ";"; return out;'))
A(("foreach_arr", 'local a = [1, 2, 3]; local out = ""; foreach (i, v in a) out += i + "=" + v + ";"; return out;'))
A(("foreach_str", 'local out = ""; foreach (i, c in "abc") out += i + "=" + c + ";"; return out;'))
A(("foreach_cls", 'class C { a = 1; static s = 2; function f() {} } local out = ""; foreach (k, v in C) out += k + ";"; return out;'))
A(("foreach_gen", 'local g = (function() { yield 1; yield 2; }()); local out = ""; foreach (i, v in g) out += i + "=" + v + ";"; return out;'))
A(("foreach_int", 'foreach (i, v in 1) return "never"; return "ok";'))
A(("foreach_null", 'foreach (i, v in null) return "never"; return "ok";'))
A(("foreach_bool", 'foreach (i, v in true) return "never"; return "ok";'))
A(("foreach_func", 'foreach (i, v in function() {}) return "never"; return "ok";'))
A(("foreach_wk", 'foreach (i, v in ({}).weakref()) return "never"; return "ok";'))
A(("foreach_inst_no_mt", 'class C { } foreach (i, v in C()) return "never"; return "ok";'))
A(("foreach_inst_mt", 'class C { function _nexti(i) { if (i == null) return 0; return null; } function _get(k) { return "iv" + k; } } local out = ""; foreach (k, v in C()) out += k + "=" + v + ";"; return out;'))
A(("foreach_inst_mt_no_get", 'class C { function _nexti(i) { if (i == null) return 0; return null; } } foreach (k, v in C()) return "never"; return "ok";'))
A(("foreach_inst_mt_throw", 'class C { function _nexti(i) { throw "in"; } } foreach (k, v in C()) return "never"; return "ok";'))
A(("foreach_mt", NEXTI_PROLOG % 'if (i == null) return 0; if (i < 3) return i + 1; return null;'))
A(("foreach_mt_del", NEXTI_PROLOG % 'if (i == null) return 0; return null;'))
A(("foreach_mt_true", NEXTI_PROLOG % 'if (i == null) return true; return null;'))
A(("foreach_mt_float", NEXTI_PROLOG % 'if (i == null) return 0.5; return null;'))
A(("foreach_mt_throw", NEXTI_PROLOG % 'throw "ni";'))
A(("foreach_mt_no_get", 'local t = {}; t.setdelegate({ _nexti = function(i) { if (i == null) return 5; return null; } }); foreach (k, v in t) return "never"; return "ok";'))
A(("foreach_mt_invalid", 'local t = {}; t.setdelegate({ _nexti = function(i) { if (i == null) return 0; return null; }, _get = function(k) { throw "clean"; } }); foreach (k, v in t) return "never"; return "ok";'))
A(("foreach_mt_returns_self", 'local t = {}; t.setdelegate({ _nexti = function(i) { return t; }, _get = function(k) { return "v"; } }); local n = 0; foreach (k, v in t) { n++; if (n > 3) break; } return n;'))

# ---------------------------------------------------------------- _call
A(("call_tbl_mt", 'local t = {}; t.setdelegate({ _call = function(a, b) { return "C:" + a + "," + b; } }); return t(1, 2);'))
A(("call_tbl_mt_noargs", 'local t = {}; t.setdelegate({ _call = function() { return "C"; } }); return t();'))
A(("call_tbl_mt_this", 'local t = {}; t.setdelegate({ _call = function() { return this == t; } }); return t();'))
A(("call_tbl_mt_null", 'local t = {}; t.setdelegate({ _call = function() { return null; } }); return t();'))
A(("call_tbl_mt_throw", 'local t = {}; t.setdelegate({ _call = function() { throw "cl"; } }); return t();'))
A(("call_tbl_mt_notfunc", 'local t = {}; t.setdelegate({ _call = 1 }); return t();'))
A(("call_tbl_no_mt", 'return ({})(1);'))
A(("call_tbl_ddel_has", 'local t = {}; t.setdelegate({ len = 1 }); return t(1);'))
A(("call_inst_mt", 'class C { function _call(a) { return "I:" + a; } } return C()(9);'))
A(("call_inst_no_mt", 'class C { } return C()();'))
A(("call_arr", 'return [1](2);'))
A(("call_str", 'return "a"(2);'))
A(("call_int", 'return (1)(2);'))
A(("call_null", 'return null();'))
A(("call_bool", 'return true();'))
A(("call_gen", 'local g = (function() { yield 1; }()); return g();'))
A(("call_wk", 'local w = ({}).weakref(); return w();'))
A(("call_thr", 'local t = newthread(function() {}); return t();'))

# ---------------------------------------------------------------- _cloned
A(("clone_tbl_mt", 'local t = { a = 1 }; t.setdelegate({ _cloned = function(o) { print("CL:" + o.a + ";"); } }); local c = clone t; return c.a;'))
A(("clone_tbl_mt_ret", 'local t = { a = 1 }; t.setdelegate({ _cloned = function(o) { return { b = 2 }; } }); local c = clone t; return c.a + "," + c.b;'))
A(("clone_tbl_mt_throw", 'local t = { a = 1 }; t.setdelegate({ _cloned = function(o) { throw "cl"; } }); return clone t;'))
A(("clone_tbl_mt_arity0", 'local t = { a = 1 }; t.setdelegate({ _cloned = function() { return 1; } }); return clone t;'))
A(("clone_tbl_mt_notfunc", 'local t = { a = 1 }; t.setdelegate({ _cloned = 1 }); return clone t;'))
A(("clone_tbl", 'local t = { a = 1, b = [2] }; local c = clone t; c.a = 9; return t.a + "," + c.a + "," + (t.b == c.b);'))
A(("clone_arr", 'local a = [1, 2]; local c = clone a; c[0] = 9; return a[0] + "," + c[0];'))
A(("clone_tbl_delegate_kept", 'local d = { z = 1 }; local t = {}; t.setdelegate(d); local c = clone t; return c.getdelegate() == d;'))
A(("clone_inst_mt", 'class C { a = 1; function _cloned(o) { print("CL;"); } } local c = clone C(); return c.a;'))
A(("clone_inst", 'class C { a = 1; } local c = clone C(); c.a = 5; return (clone C()).a + "," + c.a;'))
A(("clone_str", 'return clone "a";'))
A(("clone_int", 'return clone 1;'))
A(("clone_float", 'return clone 1.5;'))
A(("clone_null", 'return clone null;'))
A(("clone_bool", 'return clone true;'))
A(("clone_cls", 'return clone class {};'))
A(("clone_func", 'return clone function() {};'))
A(("clone_gen", 'return clone (function() { yield 1; }());'))
A(("clone_wk", 'return clone ({}).weakref();'))
A(("clone_thr", 'return clone newthread(function() {});'))

# ---------------------------------------------------------------- _newmember / _inherited
# 参考实现 NewSlotA 里 _newmember 被调用时压入 5 个值（this=类, key, val, attrs, bstatic），
# 即声明 4 个形参
NM = 'class B { function _newmember(k, v, attrs, bstatic) { print("NM:" + k + ":" + bstatic + ";"); } } '
A(("newmember_field", NM + 'class D extends B { x = 1; } return D().x;'))
A(("newmember_func", NM + 'class D extends B { function f() { return 1; } } return D().f();'))
A(("newmember_static", NM + 'class D extends B { static s = 1; } return D.s;'))
A(("newmember_ctor", NM + 'class D extends B { constructor() { } } return typeof D();'))
A(("newmember_ret", 'class B { function _newmember(k, v, a, b) { return 9; } } class D extends B { x = 1; } return D().x;'))
A(("newmember_throw", 'class B { function _newmember(k, v, a, b) { throw "nm"; } } class D extends B { x = 1; } return "ok";'))
A(("newmember_arity1", 'class B { function _newmember(k) { print("NM;"); } } class D extends B { x = 1; } return "ok";'))
A(("newmember_notfunc", 'class B { _newmember = 1 } class D extends B { x = 1; } return "ok";'))
A(("newmember_rawnewmember", NM + 'class D extends B { } D.rawnewmember("z", function() { return 3; }); return D().z();'))
A(("newmember_newmember_api", NM + 'class D extends B { } D.newmember("z", function() { return 3; }); return D().z();'))
A(("newmember_base", NM + 'class D extends B { } return typeof D;'))
INH = 'class B { function _inherited(a) { print("INH:" + a + ";"); } } '
A(("inherited_plain", INH + 'class D extends B { } return typeof D;'))
A(("inherited_method", INH + 'class D extends B { function f() { return 1; } } return D().f();'))
A(("inherited_ret", 'class B { function _inherited(a) { return 5; } } class D extends B { } return typeof D;'))
A(("inherited_throw", 'class B { function _inherited(a) { throw "inh"; } } class D extends B { } return "ok";'))
A(("inherited_arity0", 'class B { function _inherited() { print("INH;"); } } class D extends B { } return typeof D;'))
A(("inherited_notfunc", 'class B { _inherited = 1 } class D extends B { } return typeof D;'))
A(("inherited_second_level", INH + 'class D extends B { } class E extends D { } return typeof E;'))
A(("extends_nonclass", 'class B { } return typeof B;'))
A(("extends_instance", 'class B { } local b = B(); return typeof b;'))

# ---------------------------------------------------------------- 其它委托交互
A(("deleg_mt_chain", 'local a = { _typeof = function() { return "A"; } }; local b = {}; b.setdelegate(a); return typeof b;'))
A(("deleg_mt_own_wins", 'local a = { _typeof = function() { return "A"; } }; local b = { _typeof = function() { return "B"; } }; b.setdelegate(a); return typeof b;'))
A(("deleg_instance_mt", 'class C { function _typeof() { return "C"; } } local c = C(); local t = {}; t.setdelegate(c); return typeof t;'))
A(("deleg_setdelegate_cycle", 'local a = {}; a.setdelegate(a); return "ok";'))
A(("deleg_setdelegate_cycle2", 'local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a); return "ok";'))
A(("deleg_setdelegate_non_table", 'local a = {}; a.setdelegate(1); return "ok";'))
A(("deleg_setdelegate_instance", 'class C { } local a = {}; a.setdelegate(C()); return a.getdelegate() == null;'))
A(("deleg_class_mt_inherit", 'class B { function _typeof() { return "B"; } } class D extends B { } return typeof D();'))
A(("deleg_class_mt_override", 'class B { function _typeof() { return "B"; } } class D extends B { function _typeof() { return "D"; } } return typeof D();'))
A(("deleg_class_mt_base_call", 'class B { function _typeof() { return "B"; } } class D extends B { function _typeof() { return "D" + base._typeof(); } } return typeof D();'))

lines = [
    "// 自动生成：metamethod 全矩阵（tools/gen_metaprobe.py）",
    "function s(tag, f) {",
    '    try { print(tag + "=" + f() + "\\n"); }',
    '    catch (e) { print(tag + "!" + e + "\\n"); }',
    "}",
    "",
]
for tag, body in CASES:
    lines.append('s("%s", function() { %s });' % (tag, body))
lines.append('print("METAMATRIX-DONE\\n");')

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")

tags = [t for t, _ in CASES]
assert len(tags) == len(set(tags)), "tag 重复：%s" % [t for t in tags if tags.count(t) > 1]
print("已写出 %s：%d 个用例" % (OUT, len(CASES)))

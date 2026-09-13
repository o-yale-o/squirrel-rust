// 错误对象类型与 CALLSTACK/LOCALS 打印规范
function fail_string() { throw "str-err"; }
function fail_int() { throw 42; }
function fail_float() { throw 1.5; }
function fail_table() { throw { code = 7 }; }
function fail_class() { class E {} throw E; }
function fail_instance() { class E {} throw E(); }
function fail_null() { throw null; }
function fail_bool() { throw false; }

local which = vargv.len() > 0 ? vargv[0] : "string";
if (which == "string") fail_string();
else if (which == "int") fail_int();
else if (which == "float") fail_float();
else if (which == "table") fail_table();
else if (which == "class") fail_class();
else if (which == "instance") fail_instance();
else if (which == "null") fail_null();
else if (which == "bool") fail_bool();

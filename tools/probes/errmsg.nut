function t(name, f) { try { f(); print(name + " OK\n"); } catch (e) { print(name + " FAIL " + e + "\n"); } }
t("format_noargs", function() { format(); });
t("split_badtype", function() { split(1, ","); });
t("string_slice_bad", function() { "abc".slice("x"); });
t("array_insert_bad", function() { local a=[1]; a.insert("x", 1); });
t("array_remove_oob", function() { local a=[1]; a.remove(5); });
t("table_rawget_missing", function() { local t={}; t.rawget("nope"); });
t("closure_call_bad", function() { local c = function(a){}; c.call(1,2); });
t("blob_resize_bad", function() { local b = blob(1); b.resize("x"); });
t("file_open_missing", function() { file("no_such_file_xyz.txt", "r"); });
t("loadfile_missing", function() { loadfile("no_such_file_xyz.nut"); });
t("compilestring_bad", function() { compilestring("1+", "x"); });
t("throw_string", function() { throw "x"; });
t("newthread_bad", function() { newthread(1); });
t("num_tochar_range", function() { (70000).tochar(); });
t("str_index_oob", function() { "abc"[9]; });

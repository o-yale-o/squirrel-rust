// 自动生成：python tools/gen_sysmap.py —— 请勿手改
// io / system / date 语义矩阵（固定时间戳；同一台机器上两侧时区相同）

// io / system / date 语义探测
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
function dt(t) {
    local d = date(t);
    return d.year + "-" + d.month + "-" + d.day + " " + d.hour + ":" + d.min + ":" + d.sec
        + " w" + d.wday + " y" + d.yday;
}
function dtu(t) {
    local d = date(t, 117);   // 'u' = 117，走 gmtime
    return d.year + "-" + d.month + "-" + d.day + " " + d.hour + ":" + d.min + ":" + d.sec
        + " w" + d.wday + " y" + d.yday;
}

// 1) date：固定时间戳，字段全打（本地/UTC 各一套；同一台机器上两侧时区相同）
s("date_epoch", function() { return dt(0); });
s("date_epoch_u", function() { return dtu(0); });
s("date_1", function() { return dt(1); });
s("date_neg", function() { return dt(-1); });
s("date_1970mid", function() { return dt(1234567890); });
s("date_1970mid_u", function() { return dtu(1234567890); });
s("date_y2038", function() { return dt(2147483647) + "/" + dtu(2147483647); });
s("date_2100", function() { return dt(4102444800) + "/" + dtu(4102444800); });
s("date_1900", function() { return dtu(-2208988800); });
s("date_leap", function() { return dtu(951782400); });
s("date_default_shape", function() { local d = date(); return typeof d + "/" + (d.year > 2000); });
s("date_badfmt_str", function() { return "" + date(0, "u"); });
s("date_badfmt_num", function() { return typeof date(0, 999); });
s("date_nonnum", function() { return "" + date("x"); });
s("date_extra_args", function() { return "" + date(0, 117, 5); });
s("date_fields_count", function() { local n = 0; foreach (k, v in date(0)) n++; return n; });

// 2) time / clock（只比类型与单调性）
s("time_type", function() { return typeof time(); });
s("time_reasonable", function() { local t = time(); return (t > 1600000000) + "/" + (t < 4102444800); });
s("time_extra", function() { try { return "" + time(1); } catch (e) { return "E:" + e; } });
s("clock_type", function() { return typeof clock(); });
s("clock_nonneg", function() { return clock() >= 0; });
s("clock_extra", function() { try { return typeof clock(1); } catch (e) { return "E:" + e; } });  // 值本身是计时噪声，只比类型

// 3) getenv
s("getenv_path", function() { local p = getenv("PATH"); return typeof p + "/" + (p.len() > 0); });
s("getenv_missing", function() { return typeof getenv("__SQ_RUST_NOPE__"); });
s("getenv_empty", function() { return typeof getenv(""); });
s("getenv_nonstr", function() { try { return "" + getenv(5); } catch (e) { return "E:" + e; } });
s("getenv_noargs", function() { try { return "" + getenv(); } catch (e) { return "E:" + e; } });

// 4) system（只用无害命令）
// 注：不要把子进程的输出和脚本自己的 print 混在同一个 stdout 上比较 ——
// 参考实现的 stdout 是**块缓冲**（退出时才 flush），子进程直接写句柄，
// 两者的**行序**一定不同（不是语义差异）。这里重定向到文件再读回。
s("system_true", function() { return system("exit 0"); });
s("system_code3", function() { return system("exit 3"); });
s("system_echo_file", function() {
    system("echo SQTEST> tmp-c2/sys_echo_out.txt");
    local f = file("tmp-c2/sys_echo_out.txt", "rb");
    local s = f.readblob(64);
    f.close();
    return s.len() + ":" + s;
});
s("system_bad_cmd", function() { return typeof system("this_command_does_not_exist_xyz"); });
s("system_nonstr", function() { try { return "" + system(5); } catch (e) { return "E:" + e; } });

// 5) remove / rename 错误路径
s("remove_missing", function() { return "" + remove("tmp-c2/definitely_missing_xyz.txt"); });
s("rename_missing", function() { return "" + rename("tmp-c2/definitely_missing_xyz.txt", "tmp-c2/other.txt"); });
s("remove_nonstr", function() { try { return "" + remove(5); } catch (e) { return "E:" + e; } });
s("rename_one_arg", function() { try { return "" + rename("a"); } catch (e) { return "E:" + e; } });
s("file_remove_roundtrip", function() {
    local p = "tmp-c2/sys_tmp_file.txt";
    local f = file(p, "wb"); f.writeblob(blob("hello")); f.close();
    local exists = true;
    local rc = remove(p);
    local g = null;
    try { g = file(p, "rb"); } catch (e) { g = null; }
    return rc + "/" + (g == null);
});
s("file_rename_roundtrip", function() {
    local a = "tmp-c2/sys_tmp_a.txt", b = "tmp-c2/sys_tmp_b.txt";
    local f = file(a, "wb"); f.writen(7, 'b'); f.close();
    local rc = rename(a, b);
    local g = file(b, "rb"); local v = g.readn('b'); g.close();
    remove(b);
    return rc + "/" + v;
});

// 6) loadfile / dofile
s("loadfile_shape", function() {
    local p = "tmp-c2/sys_tmp_script.nut";
    local f = file(p, "wb"); f.writeblob(blob("return 40 + 2;")); f.close();
    local fn = loadfile(p);
    return typeof fn + "/" + fn();
});
s("dofile_ret", function() {
    local p = "tmp-c2/sys_tmp_script2.nut";
    local f = file(p, "wb"); f.writeblob(blob("return \"from-dofile\";")); f.close();
    return "" + dofile(p);
});
s("dofile_args", function() {
    local p = "tmp-c2/sys_tmp_script3.nut";
    local f = file(p, "wb"); f.writeblob(blob("return vargv.len() + \":\" + vargv[0];")); f.close();
    return "" + dofile(p, "arg1", "arg2");
});
s("loadfile_missing", function() { try { return "" + loadfile("tmp-c2/no_such_script.nut"); } catch (e) { return "E:" + e; } });
s("loadfile_broken", function() {
    local p = "tmp-c2/sys_tmp_broken.nut";
    local f = file(p, "wb"); f.writeblob(blob("local = ;")); f.close();
    try { return "" + loadfile(p); } catch (e) { return "E:" + e; }
});
s("dofile_broken", function() {
    local p = "tmp-c2/sys_tmp_broken2.nut";
    local f = file(p, "wb"); f.writeblob(blob("local = ;")); f.close();
    try { return "" + dofile(p); } catch (e) { return "E:" + e; }
});
s("dofile_prints", function() {
    local p = "tmp-c2/sys_tmp_script4.nut";
    local f = file(p, "wb"); f.writeblob(blob("print(\"inner\\n\"); return 1;")); f.close();
    local r = dofile(p);
    return "r" + r;
});

print("SYSMAP-DONE\n");

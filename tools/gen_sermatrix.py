# -*- coding: utf-8 -*-
"""生成「闭包序列化 × 值形状」矩阵语料 tests-corpus/t_sershape.nut。

背景：`t_sermap.nut` 只有 25 条，覆盖的是"能不能写/读文件"的错误路径；
**值形状本身的往返**（写出去再读回来还是不是同一个值）几乎没有系统覆盖。

`writeclosuretofile` 序列化的是**闭包**（原型 + 字面量表 + outer 值）。所以要让
**值**真正参与往返，值必须是闭包里的**字面量**（数组/表/字符串/嵌套函数/类…），
而不是外部 local —— 用外部 local 会先撞"自由变量闭包不能序列化"那条，
根本走不到值序列化（第一版探针就是这么写错的）。

每条用例：写出闭包 → 读回 → 调用 → 用 show() 渲染返回值 + typeof。
安全约束（参考实现自己的 UB/崩溃，不进语料）：
  - 自引用结构（表/数组互相指）：探针实测两侧都报同样的错 → 可以进语料；
  - 深嵌套值：**参考实现在 ~10000 层时栈溢出崩溃**（`0xC00000FD`，属已记录的
    "深嵌套值释放"族），语料只到几层；
  - 负数/非法 blob 操作、非法文件模式等一律不用。

用法：
    python tools/gen_sermatrix.py
"""
import os
import subprocess
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_sershape.nut")
TMP = "tmp-c2/sershape_tmp.cnut"

PREAMBLE = r"""
// 自动生成：闭包序列化 × 值形状矩阵（tools/gen_sermatrix.py）
// 每条：写出去 -> 读回来 -> 调用 -> show(返回值) + typeof
// 临时文件路径用 root 表的全局（local 声明在函数之后时，函数里解析不到 → 会报
// "the index 'PATH' does not exist"，生成器自检抓到过）
PATH <- "TMPPATH";
function show(r, d = 0) {
    if (d > 4) return "...";
    local t = typeof r;
    if (t == "array") { local s = "["; for (local i = 0; i < r.len(); i++) { if (i > 0) s += ","; s += show(r[i], d + 1); } return s + "]"; }
    if (t == "table") { local s = "{"; foreach (k, v in r) s += show(k, d + 1) + ":" + show(v, d + 1) + ","; return s + "}"; }
    if (t == "string") return "\"" + r + "\"";
    if (t == "float") return format("%.17g", r);
    if (t == "class" || t == "instance") { try { return r.tostring(); } catch (e) { return "(" + t + ")"; } }
    return "" + r;
}
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!outer:" + e + "\n"); }
}
// 往返：写闭包、读回、调用（可带实参）、渲染
function rt(f) {
    writeclosuretofile(PATH, f);
    local g = loadfile(PATH);
    local r = null; local err = null;
    try { r = g(); } catch (e) { err = "" + e; }
    return show(r) + "|" + typeof r + "|err=" + show(err);
}
function rt_args(f, a1) {
    writeclosuretofile(PATH, f);
    local g = loadfile(PATH);
    local r = null; local err = null;
    try { r = g(a1); } catch (e) { err = "" + e; }
    return show(r) + "|" + typeof r + "|err=" + show(err);
}
function rt_two(f, a1, a2) {
    writeclosuretofile(PATH, f);
    local g = loadfile(PATH);
    local r = null; local err = null;
    try { r = g(a1, a2); } catch (e) { err = "" + e; }
    return show(r) + "|" + typeof r + "|err=" + show(err);
}
// 错误路径也要自己吃掉错误再打印（否则会变成 t() 外层的 "!outer:"，
// 既让生成器自检失去意义，也让"用例是否真的执行到了"的信号变弱）
function terr(f) {
    local r = null; local err = null;
    try { r = f(); } catch (e) { err = "" + e; }
    return show(r) + "|err=" + show(err);
}
local PATH_UNUSED = 0;
"""

# (标签, 闭包体 —— 返回值必须是闭包内的字面量/由字面量构造)
SHAPES = [
    ("null", "return null;"),
    ("true", "return true;"),
    ("false", "return false;"),
    ("int0", "return 0;"),
    ("int_neg", "return -12345;"),
    ("int_max", "return 9223372036854775807;"),
    ("int_min", "return -9223372036854775807 - 1;"),
    ("float0", "return 0.0;"),
    ("float_neg0", "return -0.0;"),
    ("float_frac", "return -1.5;"),
    ("float_big", "return 1e30;"),
    ("float_tiny", "return 1.1754944e-38;"),
    ("str_empty", 'return "";'),
    ("str_ascii", 'return "hello";'),
    ("str_escapes", 'return "a\\tb\\nc\\\\d\\"e";'),
    ("str_high", 'return "\\xff\\xfe";'),
    ("str_long", 'return "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";'),
    ("arr_empty", "return [];"),
    ("arr_ints", "return [1, 2, 3];"),
    ("arr_mixed", 'return [1, "a", null, true, 1.5];'),
    ("arr_nested", "return [1, [2, [3, [4]]]];"),
    ("arr_of_arr", "return [[1], [2], [3]];"),
    ("tbl_empty", "return {};"),
    ("tbl_str_keys", 'return { a = 1, b = "two" };'),
    ("tbl_int_key", 'return { [1] = "intkey" };'),
    ("tbl_mixed_keys", 'return { a = 1, [2] = "b", [1.5] = true };'),
    ("tbl_nested", "return { a = { b = { c = 3 } } };"),
    ("tbl_in_arr", 'return [{ a = 1 }, { b = [2] }];'),
    ("closure_inner", "local f = function(x) { return x * 2; }; return f(21);"),
    ("closure_nested", "local f = function(x) { local g = function(y) { return y + 1; }; return g(x); }; return f(41);"),
    ("closure_returned", "return function(x) { return x + 1; };"),
    ("closure_varargs", "local f = function(...) { return vargv.len(); }; return f(1, 2, 3);"),
    ("closure_default", "local f = function(a, b = 7) { return a + b; }; return f(1);"),
    ("closure_recursive", "local f = null; f = function(n) { return n <= 1 ? 1 : n * f(n - 1); }; return f(5);"),
    ("class_basic", "class C { x = 1; function getx() { return x; } } return C().getx();"),
    ("class_static", "class C { static s = 3; function get() { return s; } } return C().get();"),
    ("class_inherit", 'class B { function who() { return "B"; } } class D extends B { function who() { return "D" + base.who(); } } return D().who();'),
    ("class_attrs", 'class C { </ doc = "d" /> x = 1; } return C.getattributes("x").doc;'),
    ("class_newmember", "class C { } C.newmember(\"f\", function() { return 9; }); return C().f();"),
    ("class_tostring", 'class C { function _tostring() { return "C!"; } } return "" + C();'),
    ("class_instanceof", "class C { } local c = C(); return \"\" + (c instanceof C);"),
    ("gen_created", "local g = (function() { yield 1; yield 2; }()); local a = resume g; return a;"),
    ("gen_status", "local g = (function() { yield 1; }()); return g.getstatus();"),
    ("blob_created", "local b = blob(3); b.writen(65, 'b'); return b.readn('b');"),
    ("blob_method", "local b = blob(2); b.seek(0, 'b'); b.writen(0x0102, 'w'); b.seek(0, 'b'); return b.readn('w');"),
    ("weakref_created", "local tb = {}; local w = tb.weakref(); return \"\" + (w.ref() == tb);"),
    ("re_created", 'local r = regexp("a(b)c"); local m = r.match("xabcx"); return m[0] + "," + m[1];'),
    ("re_search", 'local r = regexp("[0-9]+"); return r.search("ab123");'),
    ("string_methods", 'return "Hello".tolower() + "/" + "Hello".toupper();'),
    ("min_max", "return [1, 2, 3].reduce(@(a, b) a > b ? a : b);"),
    ("sort_in_closure", "local a = [3, 1, 2]; a.sort(); return a;"),
    ("try_inside", 'try { throw "inner"; } catch (e) { return "caught:" + e; }'),
    ("loop_inside", "local s = 0; for (local i = 0; i < 5; i++) s += i; return s;"),
    # 生成器闭包往返：序列化后的闭包仍要能被识别成生成器（实测两侧一致：
    # typeof g() == "generator"、getstatus() == "suspended"、resume 依次给 1/2）。
    # 注意别写成"只含 suspend 不含量 yield 的闭包"—— 那种不是生成器，直接调用会把
    # 主 VM 挂起（生成器自检就是这么发现第一版写法把整个脚本卡住的）。
    ("gen_closure_roundtrip", "GEN_SPECIAL"),
    ("env_current", "return getroottable() == getroottable();"),
    ("native_ref", "return typeof print;"),
]

# 传参往返（默认参数、varargs、参数个数错误）
WITH_ARGS = [
    ("args_two", "local f = function(a, b) { return a + b; };"),
    ("args_default_used", "local f = function(a, b = 100) { return b; };"),
    ("args_default_missing", "local f = function(a, b = 100) { return b; };"),
    ("args_varargs", "local f = function(a, ...) { return vargv.len(); };"),
]

# 错误路径（不可序列化的东西 / 坏文件）
ERRORS = [
    ("native_direct", 'writeclosuretofile(PATH, print); return "no-error";'),
    ("freevar", 'local n = 5; local f = function() { return n; }; writeclosuretofile(PATH, f); return "no-error";'),
    ("badarg", 'writeclosuretofile(PATH, 1); return "no-error";'),
    ("noargs", 'writeclosuretofile(); return "no-error";'),
    ("badpath", 'writeclosuretofile("no_such_dir/x.cnut", function() { return 1; }); return "no-error";'),
    ("load_missing", 'return loadfile("tmp-c2/definitely_missing_xyz.cnut");'),
    ("load_garbage", 'local f = file("tmp-c2/sershape_garbage.cnut", "wb"); f.writen(1, \'b\'); f.close(); '
                     'try { local g = loadfile("tmp-c2/sershape_garbage.cnut"); return "loaded:" + typeof g; } '
                     'catch (e) { return "E:" + e; }'),
    # writeclosuretofile 的**检查顺序**（模糊测试在私有工作目录里暴露的差异，已修）：
    #   掩码 .sc 先查（第 3 参是整数 → 参数类型错，**不建文件**）；
    #   过了掩码先 fopen("wb+")：不可写 → "cannot open the file"（哪怕第 3 参是 native
    #   closure、后面还会类型不符）；可写则文件立刻建出来，之后无论序列化失败还是
    #   native closure 类型不符，**0 字节文件都会留下**。
    ("wcf_native_badpath", 'writeclosuretofile("no_such_dir_zz/x.cnut", print); return "no-error";'),
    ("wcf_int_badpath", 'writeclosuretofile("no_such_dir_zz/x.cnut", 1); return "no-error";'),
    ("wcf_native_leftover", 'writeclosuretofile("tmp-c2/sershape_leftover.cnut", print); return "no-error";'),
    ("wcf_native_leftover_check", 'local n = null; '
                                  'try { local fh = file("tmp-c2/sershape_leftover.cnut", "rb"); n = fh.len(); fh.close(); } '
                                  'catch (e) { n = "E:" + e; } '
                                  'try { remove("tmp-c2/sershape_leftover.cnut"); } catch (e) {} '
                                  'return "leftover=" + show(n);'),
    ("wcf_freevar_leftover", 'local fv = 5; local bad = function() { return fv; }; '
                             'writeclosuretofile("tmp-c2/sershape_leftover2.cnut", bad); return "no-error";'),
    ("wcf_freevar_leftover_check", 'local n = null; '
                                   'try { local fh = file("tmp-c2/sershape_leftover2.cnut", "rb"); n = fh.len(); fh.close(); } '
                                   'catch (e) { n = "E:" + e; } '
                                   'try { remove("tmp-c2/sershape_leftover2.cnut"); } catch (e) {} '
                                   'return "leftover=" + show(n);'),
]


def main():
    lines = [PREAMBLE.replace("TMPPATH", TMP)]
    n = 0
    for tag, body in SHAPES:
        if body == "GEN_SPECIAL":
            lines.append(
                't("v_%s", function() {\n'
                '    writeclosuretofile(PATH, function() { yield 1; yield 2; });\n'
                '    local g = loadfile(PATH);\n'
                '    local made = g();\n'
                '    local a = null; local b = null; local err = null;\n'
                '    try { a = resume made; b = resume made; } catch (e) { err = "" + e; }\n'
                '    return typeof g + "/" + typeof made + "/" + made.getstatus()\n'
                '         + "/" + show(a) + "," + show(b) + "/err=" + show(err);\n'
                '});' % tag)
        else:
            lines.append('t("v_%s", function() { return rt(function() { %s }); });' % (tag, body))
        n += 1
    for tag, decl in WITH_ARGS:
        if tag == "args_two":
            lines.append('t("a_%s", function() { return rt_two(function(a, b) { return a + b; }, 3, 4); });' % tag)
        elif tag == "args_default_used":
            lines.append('t("a_%s", function() { return rt_two(function(a, b = 100) { return b; }, 1, 2); });' % tag)
        elif tag == "args_default_missing":
            # 已知偏离：默认参数值往返会丢 → 读回后 b 是 null 而不是 100
            lines.append('t("a_%s", function() { return rt_args(function(a, b = 100) { return "b=" + b; }, 1); });' % tag)
        else:
            lines.append('t("a_%s", function() { return rt_two(function(a, ...) { return vargv.len(); }, 1, 2); });' % tag)
        n += 1
    for tag, body in ERRORS:
        # 用 terr 包住：错误是本用例的**观测结果**，不是用例体自己坏了
        lines.append('t("e_%s", function() { return terr(function() { %s }); });' % (tag, body))
        n += 1
    # 收尾清理（写文件的用例自己删；这条保证跑完仓库干净 —— check_all 有动态守卫）
    lines.append('try { remove(PATH); } catch (e) {}')
    lines.append('try { remove("tmp-c2/sershape_garbage.cnut"); } catch (e) {}')
    lines.append('try { remove("tmp-c2/sershape_leftover.cnut"); } catch (e) {}')
    lines.append('try { remove("tmp-c2/sershape_leftover2.cnut"); } catch (e) {}')
    lines.append('print("SERMAP-DONE\\n");')
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")
    print("已写出 %s：%d 条用例（值形状 %d + 传参 %d + 错误路径 %d）"
          % (OUT, n, len(SHAPES), len(WITH_ARGS), len(ERRORS)))

    # 生成器自检：用例体必须自己吃掉错误（!outer: 说明用例体坏了 → 假绿，差分测不出来）
    exe = os.path.join(ROOT, "target", "release", "sq.exe")
    if os.path.exists(exe):
        p = subprocess.run([exe, OUT], capture_output=True, cwd=ROOT, timeout=180)
        text = p.stdout.decode("utf8", "replace")
        bad = [l for l in text.splitlines() if "!outer:" in l]
        done = "SERMAP-DONE" in text
        print("自检：跑到 DONE=%s，!outer 行数=%d" % (done, len(bad)))
        for l in bad[:5]:
            print("   " + l[:140])
        if bad or not done:
            print("自检失败：用例体自己抛错了，矩阵没有真正测到东西")
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

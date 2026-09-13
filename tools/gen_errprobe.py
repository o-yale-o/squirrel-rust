# -*- coding: utf-8 -*-
"""生成错误消息对拍语料：全部全局函数 + 各类默认委托成员 × 多种错误参数形态。

- 全局函数名单**从参考实现的 root table 运行时取**（避免把委托成员误当全局）；
- 委托成员名单来自 tools/audit_delegates.py 的解析（按表分组）；
- 有副作用/阻塞/改全局环境的名称一律排除。

用法：
    python tools/gen_errprobe.py            # 写出 tests-corpus/t_errmsg2.nut
"""

try:  # 关掉系统错误对话框：参考实现在 UB 形态上会真崩溃并弹模态框
    import os as _os, sys as _sys
    _d = _os.path.dirname(_os.path.abspath(__file__))
    for _p in (_d, _os.path.join(_d, "..", "tools"), _os.path.join(_d, "tools")):
        if _os.path.isdir(_p):
            _sys.path.insert(0, _p)
    from safe_run import suppress_crash_dialogs as _scd
    _scd()
    del _d, _p, _os, _sys, _scd
except Exception as _e:  # pragma: no cover
    import sys as _sys2
    print("warning: 无法关闭崩溃对话框: %r" % (_e,), file=_sys2.stderr)
import os
import re
import subprocess
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")
CPP_BASELIB = r"D:\OpenSource\squirrel\squirrel\sqbaselib.cpp"
OUT = os.path.join(ROOT, "tests-corpus", "t_errmsg2.nut")

EXCLUDE = {
    "print", "printf", "error", "assert", "system", "getenv", "remove", "rename",
    "dofile", "loadfile", "writeclosuretofile", "suspend", "setroottable", "setconsttable",
    "seterrorhandler", "setdebughook", "collectgarbage", "resurrectunreachable",
    "compilestring", "enabledebuginfo", "array", "blob", "file", "regexp", "stream",
    "newthread", "getstackinfos", "getroottable", "getconsttable", "time", "date", "clock",
    "srand", "rand", "type", "callee", "dummy", "dump", "swap2", "swap4", "swapfloat",
    "casti2f", "castf2i",
    # 参考实现在 split(<非串>, <非串>) 上会崩：sq_getstring 的返回值没检查，
    # str 保持未初始化后被 while(*end) 解引用（Windows 崩溃框）。本实现报类型错误。
    "split",
}

RECEIVERS = [
    ("table", "{}", "t_len tbl_rawget rawset rawdelete rawin clear setdelegate getdelegate filter map keys values weakref tostring"),
    ("array", "[]", "len append extend push pop top insert remove resize reverse sort slice map apply reduce filter find clear weakref tostring"),
    ("string", '""', "len tointeger tofloat slice find tolower toupper tochar weakref tostring"),
    ("number", "(1)", "tointeger tofloat tochar weakref tostring"),
    ("closure", "(function(){})", "call pcall acall pacall bindenv getinfos getroot setroot weakref tostring"),
    ("generator", "(function(){ yield 1; }())", "getstatus weakref tostring"),
    ("thread", "(newthread(function(){}))", "call wakeup wakeupthrow getstatus getstackinfos weakref tostring"),
    ("class", "(class {})", "getattributes setattributes rawget rawset rawin weakref tostring instance getbase newmember rawnewmember"),
    ("instance", "(class {}())", "getclass rawget rawset rawin weakref tostring"),
    ("weakref", "(({}).weakref())", "ref weakref tostring"),
]

ARG_SHAPES = ["", "1", "1, 2"]

# 参考实现在这些（接收者, 成员, 形态序号）上会崩：CRT strtol 收到非法进制
# （base=1 / 负数）时未定义行为，实测 STATUS_STACK_BUFFER_OVERRUN。本实现报类型/转换错误。
BLOCK = {
    ("string", "tointeger", 1),
    ("string", "tointeger", 2),
}

# 委托成员名单：直接复用审计脚本的 C++ 表解析
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import audit_delegates as ad  # noqa: E402

TABLE_FOR_TYPE = {
    "table": "_table_default_delegate_funcz",
    "array": "_array_default_delegate_funcz",
    "string": "_string_default_delegate_funcz",
    "number": "_number_default_delegate_funcz",
    "closure": "_closure_default_delegate_funcz",
    "generator": "_generator_default_delegate_funcz",
    "thread": "_thread_default_delegate_funcz",
    "class": "_class_default_delegate_funcz",
    "instance": "_instance_default_delegate_funcz",
    "weakref": "_weakref_default_delegate_funcz",
}


def root_functions():
    """从参考实现的 root table 运行时取函数名（只读，无副作用）"""
    src = (
        "local names = [];\n"
        "foreach (k, v in getroottable()) if (typeof v == \"function\") names.append(k);\n"
        "names.sort();\n"
        "foreach (n in names) print(n + \"\\n\");\n"
    )
    probe = os.path.join(ROOT, "tmp-c2", "_rootfn.nut")
    os.makedirs(os.path.dirname(probe), exist_ok=True)
    with open(probe, "w", encoding="utf-8", newline="\n") as f:
        f.write(src)
    p = subprocess.run([REF, probe], capture_output=True, timeout=30)
    names = [l.strip() for l in p.stdout.decode("latin-1").split("\n") if l.strip()]
    return [n for n in names if n not in EXCLUDE]


def main():
    cpp_tables = ad.cpp_tables()
    lines = [
        "// 自动生成：错误消息对拍（tools/gen_errprobe.py）——每个调用都在 try/catch 里取消息文本",
        "// 全局函数名单取自参考实现 root table；委托成员取自 C++ 的 *_default_delegate_funcz 表",
        "function t(tag, f) {",
        "    try { print(tag + \"=\" + f() + \"\\n\"); }",
        "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
        "}",
        "local done = 0;",
        "",
    ]
    for name in root_functions():
        for i, args in enumerate(ARG_SHAPES):
            lines.append('t("g_%s_%d", function() { return %s(%s); });' % (name, i, name, args))
    for tyname, recv, _ in RECEIVERS:
        table = cpp_tables.get(TABLE_FOR_TYPE[tyname]) or {}
        for mem in sorted(table):
            for i, args in enumerate(ARG_SHAPES):
                if (tyname, mem, i) in BLOCK:
                    continue
                lines.append(
                    't("%s_%s_%d", function() { return %s.%s(%s); });'
                    % (tyname, mem, i, recv, mem, args)
                )
    lines.append('print("SWEEP-DONE\\n");')
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")
    print("已写出 %s：%d 行" % (OUT, len(lines)))
    return 0


if __name__ == "__main__":
    sys.exit(main())

# -*- coding: utf-8 -*-
"""审计「根表插入顺序」：把 C++ 各注册函数的顺序与我们的注册顺序逐项对比。

为什么需要它：SQTable 的迭代顺序由**插入顺序**决定，而根表是按
`sq_base_register → bloblib → iolib → systemlib → mathlib → stringlib` 逐条插入的。
注册顺序一旦不同，`foreach (k, v in getroottable())` 的结果就不同——脚本可观察，
`tests-corpus/t_roottable.nut` 守门。本脚本做**静态**对比，改注册代码时立刻能发现。

做法：对每个库，取出 C++ 注册表里的名字顺序；再在我们的注册函数体里按这些名字
**首次出现的字符偏移**排序——不依赖我们是 `reg_global!` 还是 `simple!`/`bfn!` 宏。

用法：python tools/audit_regorder.py
"""
import os
import re
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
SQ = r"D:\OpenSource\squirrel"
RS_LIB = os.path.join(ROOT, "crates", "squirrel-stdlib", "src", "lib.rs")
RS_BASE = os.path.join(ROOT, "crates", "squirrel", "src", "baselib.rs")


def read(path):
    return open(path, encoding="utf-8", errors="replace").read()


def cpp_rows(path, table):
    """C++ 注册表顺序：既支持展开写法 {_SC("x"),f,n,mask}，也支持宏写法 _DECL_XXX(name,n,mask)"""
    s = read(path)
    i = s.find(table)
    if i < 0:
        return []
    j = s.find("\n};", i)
    body = s[i:j]
    rows = re.findall(
        r'\{\s*_SC\("([^"]+)"\)\s*,\s*\w+\s*,\s*-?\d+\s*,\s*(?:NULL|_SC\("[^"]*"\))\s*\}', body
    )
    if not rows:
        rows = re.findall(r'_DECL_\w+\(\s*(\w+)\s*,', body)
    return rows


def rs_body(func, base=False):
    s = read(RS_BASE if base else RS_LIB)
    i = s.find(func)
    if i < 0:
        return ""
    j = s.find("\n}\n", i)
    return s[i:j]


def order_by_offset(body, names):
    pos = []
    for n in names:
        p = body.find('"%s"' % n)
        if p >= 0:
            pos.append((p, n))
    pos.sort()
    return [n for _, n in pos]


LIBS = [
    # 库名, C++ 文件, 注册表, 我们这边的注册函数, 静态看不到的额外项（按 C++ 插入位置）
    ("base", os.path.join(SQ, "squirrel", "sqbaselib.cpp"), "base_funcs[]", "pub fn base_register",
     ["_versionnumber_", "_version_", "_charsize_", "_intsize_", "_floatsize_"], True),
    ("blob", os.path.join(SQ, "sqstdlib", "sqstdblob.cpp"), "bloblib_funcs[]", "pub fn register_bloblib",
     [], False),
    ("io", os.path.join(SQ, "sqstdlib", "sqstdio.cpp"), "iolib_funcs[]", "pub fn register_iolib",
     [], False),
    ("system", os.path.join(SQ, "sqstdlib", "sqstdsystem.cpp"), "systemlib_funcs[]", "pub fn register_systemlib",
     [], False),
    ("math", os.path.join(SQ, "sqstdlib", "sqstdmath.cpp"), "mathlib_funcs[]", "pub fn register_mathlib",
     [], False),
    ("string", os.path.join(SQ, "sqstdlib", "sqstdstring.cpp"), "stringlib_funcs[]", "pub fn register_stringlib",
     [], False),
]

# 这三个类由 declare_stream 插入（C++ 静态提取不到），我们这边显式注册：位置必须一致
EXTRA_ANCHORS = {
    "blob": [("stream", 'register_class(vm, "stream"'), ("blob", 'register_class(vm, "blob"')],
    "io": [("file", 'register_class(vm, "file"'), ("stdout", 'reg|"stdout"'), ("stdin", '"stdin"'), ("stderr", '"stderr"')],
    "string": [("regexp", 'make_regexp_class')],
}


def main():
    bad = 0
    for name, path, table, func, tail, base in LIBS:
        cpp = cpp_rows(path, table) + tail
        body = rs_body(func, base)
        # 加上 declare_stream 之类插入的名字（用锚点字符串定位）
        anchors = list(EXTRA_ANCHORS.get(name, []))
        ordered_rs = order_by_offset(body, [n for n in cpp if n not in [a for a, _ in anchors]])
        for aname, astr in anchors:
            p = body.find(astr if "|" not in astr else astr.split("|")[0])
            if p >= 0 and aname in cpp:
                ordered_rs.append(aname)
                ordered_rs.sort(key=lambda n: body.find('"%s"' % n) if body.find('"%s"' % n) >= 0
                                else body.find(astr.split("|")[0]))
        cpp_common = [x for x in cpp if x in ordered_rs]
        rs_common = [x for x in ordered_rs if x in cpp]
        ok = cpp_common == rs_common
        print("== %-7s C++ %2d 项 / Rust %2d 项 : %s" % (name, len(cpp), len(ordered_rs), "顺序一致" if ok else "**顺序不一致**"))
        if not ok:
            bad += 1
            for i, (x, y) in enumerate(zip(cpp_common, rs_common)):
                if x != y:
                    print("   首次分歧 @%d: C++=%s Rust=%s" % (i, x, y))
                    print("     C++  : %s" % cpp_common[i:i + 6])
                    print("     Rust : %s" % rs_common[i:i + 6])
                    break
        missing = [x for x in cpp if x not in rs_common]
        if missing:
            print("   （未比对：%s）" % missing)
    print("\n%d 个库顺序不一致" % bad)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

# -*- coding: utf-8 -*-
"""全局函数注册审计：把 C++（sqbaselib.cpp 的 base_funcs + sqstdlib 各库的 *_funcs 表）
与 Rust 端（baselib.rs + squirrel-stdlib/src/lib.rs 的 reg_global 调用）逐项对比
（函数名 + nparamscheck + 类型掩码）。

这一类比对能一次性抓出「nparamscheck 写错」「掩码漏了 this 占位 `.`」这类问题
（math 库 18 个函数全废、blob 五个全局、split 的 -3 都是这么来的）。
"""
import os
import re
import sys

SQ = r"D:\OpenSource\squirrel"
RS_ROOT = r"D:\OpenSource\squirrel-rust"

CPP_TABLES = [
    (os.path.join(SQ, "squirrel", "sqbaselib.cpp"), r"base_funcs\[\]\s*=\s*\{(.*?)\n\};"),
    (os.path.join(SQ, "sqstdlib", "sqstdmath.cpp"), r"mathlib_funcs\[\]\s*=\s*\{(.*?)\n\};"),
    (os.path.join(SQ, "sqstdlib", "sqstdstring.cpp"), r"stringlib_funcs\[\]\s*=\s*\{(.*?)\n\};"),
    (os.path.join(SQ, "sqstdlib", "sqstdsystem.cpp"), r"systemlib_funcs\[\]\s*=\s*\{(.*?)\n\};"),
    (os.path.join(SQ, "sqstdlib", "sqstdio.cpp"), r"iolib_funcs\[\]\s*=\s*\{(.*?)\n\};"),
    (os.path.join(SQ, "sqstdlib", "sqstdblob.cpp"), r"bloblib_funcs\[\]\s*=\s*\{(.*?)\n\};"),
]

# 表里的宏写法不同，统一抓「名字 + nparams + 掩码」
CPP_ROW = re.compile(
    r'\{\s*(?:_SC\("([^"]+)"\)|#name|\w+)\s*,\s*\w+\s*,\s*(-?\d+)\s*,\s*(NULL|_SC\("([^"]*)"\))\s*\}'
)
CPP_ROW_NAMED = re.compile(
    r'_DECL(?:_FUNC|_GLOBALIO_FUNC|_GLOBALBLOB_FUNC|_BLOB_FUNC)\(\s*(\w+)\s*,\s*(-?\d+)\s*,\s*(NULL|_SC\("([^"]*)"\))\s*\)'
)

RS_FILES = [
    os.path.join(RS_ROOT, "crates", "squirrel", "src", "baselib.rs"),
    os.path.join(RS_ROOT, "crates", "squirrel-stdlib", "src", "lib.rs"),
]
RS_ROW = re.compile(
    r'reg_global\(\s*&ss\s*,\s*&\w+\s*,\s*"([^"]+)"\s*,.*?,\s*(-?\d+)\s*,\s*(None|Some\("([^"]*)"\))\s*\)',
    re.S,
)
RS_MACROS = [
    # m1!/m2!/bfn! 宏：掩码会被拼上 '.' 前缀
    (re.compile(r'(m1|m2|bfn)!\(\s*"([^"]+)"\s*,\s*"([^"]*)"', re.S), {b"m1": 2, b"m2": 3, b"bfn": 2}),
    # simple!(name, nparams, mask, f)
    (re.compile(r'simple!\(\s*"([^"]+)"\s*,\s*(-?\d+)\s*,\s*(None|Some\("([^"]*)"\))', re.S), None),
]


def cpp_globals():
    out = {}
    for path, table_re in CPP_TABLES:
        if not os.path.exists(path):
            continue
        text = open(path, encoding="utf-8", errors="replace").read()
        m = re.search(table_re, text, re.S)
        if not m:
            print("  ! 未找到表: %s" % os.path.basename(path))
            continue
        body = m.group(1)
        for row in CPP_ROW_NAMED.finditer(body):
            mask = row.group(4) if row.group(3) != "NULL" else None
            out[row.group(1)] = (int(row.group(2)), mask)
        for row in CPP_ROW.finditer(body):
            if row.group(1):
                mask = row.group(4) if row.group(3) != "NULL" else None
                out[row.group(1)] = (int(row.group(2)), mask)
    return out


def rust_globals():
    out = {}
    for path in RS_FILES:
        text = open(path, encoding="utf-8", errors="replace").read()
        for row in RS_ROW.finditer(text):
            mask = row.group(4) if row.group(3) != "None" else None
            out[row.group(1)] = (int(row.group(2)), mask)
        # m1!/m2!/bfn! 宏调用
        for row in re.finditer(r'\b(m1|m2|bfn)!\(\s*"([^"]+)"\s*,\s*"([^"]*)"', text):
            nparams = {"m1": 2, "m2": 3, "bfn": 2}[row.group(1)]
            out[row.group(2)] = (nparams, "." + row.group(3))
        # simple!
        for row in re.finditer(
            r'simple!\(\s*"([^"]+)"\s*,\s*(-?\d+)\s*,\s*(None|Some\("([^"]*)"\))', text
        ):
            mask = row.group(4) if row.group(3) != "None" else None
            out[row.group(1)] = (int(row.group(2)), mask)
    return out


def main():
    cpp = cpp_globals()
    rs = rust_globals()
    print("C++ 注册 %d 个；Rust 注册 %d 个" % (len(cpp), len(rs)))
    only_cpp = sorted(set(cpp) - set(rs))
    only_rs = sorted(set(rs) - set(cpp))
    meta = []
    for k in sorted(set(cpp) & set(rs)):
        if cpp[k] != rs[k]:
            meta.append((k, cpp[k], rs[k]))
    if only_cpp:
        print("\n仅 C++ 有（Rust 缺）: %s" % ", ".join(only_cpp))
    if only_rs:
        print("\n仅 Rust 有（多出来）: %s" % ", ".join(only_rs))
    if meta:
        print("\n元数据不一致:")
        for k, c, r in meta:
            print("   %-18s C++(nparams=%s mask=%s)  Rust(nparams=%s mask=%s)" % (k, c[0], c[1], r[0], r[1]))
    if not (only_cpp or only_rs or meta):
        print("全部一致")
    return 1 if (only_cpp or only_rs or meta) else 0


if __name__ == "__main__":
    sys.exit(main())

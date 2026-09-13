# -*- coding: utf-8 -*-
"""默认委托表成员审计：把 C++ sqbaselib.cpp 的 10 张 _*_default_delegate_funcz 表
与 Rust 端 baselib.rs 的 fill_*_ddel 注册逐项对比（成员名 + nparamscheck + 掩码）。

之前只比对过 root 表，委托表里多一个/少一个成员都不会被 root 对比发现。
"""
import os
import re
import sys

CPP = r"D:\OpenSource\squirrel\squirrel\sqbaselib.cpp"
RS = r"D:\OpenSource\squirrel-rust\crates\squirrel\src\baselib.rs"

# C++ 表名 → Rust fill 函数名
PAIRS = [
    ("_table_default_delegate_funcz", "fill_table_ddel"),
    ("_array_default_delegate_funcz", "fill_array_ddel"),
    ("_string_default_delegate_funcz", "fill_string_ddel"),
    ("_number_default_delegate_funcz", "fill_number_ddel"),
    ("_closure_default_delegate_funcz", "fill_closure_ddel"),
    ("_generator_default_delegate_funcz", "fill_generator_ddel"),
    ("_thread_default_delegate_funcz", "fill_thread_ddel"),
    ("_class_default_delegate_funcz", "fill_class_ddel"),
    ("_instance_default_delegate_funcz", "fill_instance_ddel"),
    ("_weakref_default_delegate_funcz", "fill_weakref_ddel"),
]


def cpp_tables():
    text = open(CPP, encoding="utf-8", errors="replace").read()
    out = {}
    for name, _ in PAIRS:
        m = re.search(re.escape(name) + r"\[\]\s*=\s*\{(.*?)\n\};", text, re.S)
        if not m:
            out[name] = None
            continue
        body = m.group(1)
        rows = {}
        # 形如 {_SC("len"),_len,1, NULL } 或 {_SC("rawget"),container_rawget,2, NULL}
        for row in re.finditer(
            r'\{\s*_SC\("([^"]+)"\)\s*,\s*(\w+)\s*,\s*(-?\d+)\s*,\s*(NULL|_SC\("([^"]*)"\))\s*\}',
            body,
        ):
            mask = row.group(5) if row.group(4) != "NULL" else None
            rows[row.group(1)] = (int(row.group(3)), mask)
        out[name] = rows
    return out


def rust_delegates():
    text = open(RS, encoding="utf-8").read()
    out = {}
    for _, fn in PAIRS:
        m = re.search(r"fn " + fn + r"\(.*?\n\}", text, re.S)
        if not m:
            out[fn] = None
            continue
        body = m.group(0)
        rows = {}
        for row in re.finditer(
            r'reg\(ss,\s*&\w+,\s*"([^"]+)"\s*,.*?,\s*(-?\d+)\s*,\s*(None|Some\("([^"]*)"\))\s*\)',
            body,
            re.S,
        ):
            mask = row.group(4) if row.group(3) != "None" else None
            rows[row.group(1)] = (int(row.group(2)), mask)
        out[fn] = rows
    return out


def main():
    bad = 0
    cpp = cpp_tables()
    rust = rust_delegates()
    for cname, rname in PAIRS:
        c = cpp.get(cname)
        r = rust.get(rname)
        label = cname.replace("_default_delegate_funcz", "")
        if c is None or r is None:
            print("== %-10s 解析失败: cpp=%s rust=%s" % (label, c is not None, r is not None))
            bad += 1
            continue
        only_cpp = sorted(set(c) - set(r))
        only_rust = sorted(set(r) - set(c))
        meta = []
        for k in sorted(set(c) & set(r)):
            if c[k] != r[k]:
                meta.append((k, c[k], r[k]))
        if only_cpp or only_rust or meta:
            bad += 1
            print("== %s ==" % label)
            if only_cpp:
                print("   仅 C++ 有: %s" % ", ".join(only_cpp))
            if only_rust:
                print("   仅 Rust 有: %s" % ", ".join(only_rust))
            for k, cv, rv in meta:
                print("   元数据不同 %-12s C++(nparams=%s mask=%s) Rust(nparams=%s mask=%s)"
                      % (k, cv[0], cv[1], rv[0], rv[1]))
        else:
            print("== %-10s 一致（%d 个成员）" % (label, len(c)))
    print("\n%d 张表有问题" % bad)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

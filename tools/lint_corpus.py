# -*- coding: utf-8 -*-
"""扫描语料里的常见 Squirrel 语法陷阱（这些文件曾因「两边报同样的编译错误」而假绿）：
1) 表构造器内用 `<-`（应为 `=`）
2) 用关键字当标识符（base/class/this/constructor/...）
3) `class X ... {}` 之后同一行紧跟语句（缺分隔）
另附：逐个文件用 Rust CLI 试编译，报告结果。
"""
import re
import subprocess
import sys
import os

KEYWORDS = [
    "base", "class", "this", "constructor", "function", "local", "return", "if", "else",
    "while", "for", "foreach", "switch", "case", "default", "break", "continue", "const",
    "enum", "try", "catch", "throw", "typeof", "clone", "yield", "resume", "delete",
    "instanceof", "static", "extends", "rawcall", "newthread", "null", "true", "false",
]
CLI = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"


def scan(path):
    issues = []
    with open(path, "r", encoding="utf-8") as f:
        text = f.read()
    lines = text.split("\n")
    for i, line in enumerate(lines, 1):
        code = re.sub(r"//.*$", "", line)          # 去行注释
        code = re.sub(r'"(?:\\.|[^"\\])*"', '""', code)  # 去字符串
        # 1) 构造器内 <-
        for m in re.finditer(r"\{([^{}]*)\}", code):
            if "<-" in m.group(1):
                issues.append((i, "构造器内出现 <-：" + m.group(0).strip()))
        # 2) 关键字当标识符
        for m in re.finditer(r"\blocal\s+(\w+)", code):
            if m.group(1) in KEYWORDS:
                issues.append((i, "用关键字当局部变量名：" + m.group(1)))
        # 3) class ... {} 之后同行还有语句
        for m in re.finditer(r"\bclass\b[^{]*\{\}\s*(\S)", code):
            issues.append((i, "class {} 之后同行紧跟语句：" + m.group(1)))
    return issues


def try_compile(path):
    out = subprocess.run([CLI, "-o", "tmp-c2/_scan.cnut", "-c", path],
                         capture_output=True, timeout=30)
    txt = (out.stdout + out.stderr).decode("latin-1", "replace")
    bad = "Error [" in txt or "error " in txt
    return (not bad), txt.strip().replace("\n", " ")[:120]


def main(paths):
    bad = 0
    for p in paths:
        ok, msg = try_compile(p)
        issues = scan(p)
        if not ok or issues:
            bad += 1
            print("== %s ==" % os.path.basename(p))
            if not ok:
                print("   编译失败: %s" % msg)
            for ln, why in issues:
                print("   行 %d: %s" % (ln, why))
    print("\n检查 %d 个文件，%d 个有问题" % (len(paths), bad))
    return 1 if bad else 0


if __name__ == "__main__":
    args = sys.argv[1:]
    if not args:
        d = r"D:\OpenSource\squirrel-rust\tests-corpus"
        args = [os.path.join(d, f) for f in sorted(os.listdir(d)) if f.endswith(".nut")]
    elif len(args) == 1 and os.path.isdir(args[0]):
        args = [os.path.join(args[0], f) for f in sorted(os.listdir(args[0])) if f.endswith(".nut")]
    sys.exit(main(args))

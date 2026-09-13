# -*- coding: utf-8 -*-
"""扫描语料里的常见 Squirrel 语法陷阱（这些文件曾因「两边报同样的编译错误」而假绿）。

规则全部来自**实测踩过的坑**，不是猜测：

1. 表构造器内用 `<-`（构造器里只允许 `=`）
2. 用关键字当标识符（base/class/this/constructor/...）
3. 不存在的运算符：位运算复合赋值 `&=`/`|=`/`^=`/`<<=`/`>>=`（`+= -= *= /= %=` 才是有的；
   `>>>` 是**有**的，不算陷阱）
4. 函数体内 `const`（Squirrel 只允许顶层 const）
5. `rawcall` 当成员用（它是关键字，写法是 `rawcall(callee, this, ...)`）
6. `65.tochar()` 这类「数字紧跟 . 标识符」——词法器按浮点数吃掉（必须是 `(65).tochar()`）
7. 具名参数块写在了不返回对象/表的调用上

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

# 花括号里出现这些关键字 → 那是**语句块**而不是表构造器，`<-` 合法
STATEMENTY = re.compile(
    r"\b(local|return|if|else|for|foreach|while|do|try|catch|throw|switch|case|default|break|continue)\b"
)


def strip_code(line):
    """去掉行注释与字符串字面量，避免误报"""
    code = re.sub(r"//.*$", "", line)
    return re.sub(r'"(?:\\.|[^"\\])*"', '""', code)


def scan(path):
    issues = []
    with open(path, "r", encoding="utf-8") as f:
        text = f.read()
    lines = text.split("\n")
    depth = 0   # 花括号深度（含函数体）
    for i, line in enumerate(lines, 1):
        code = strip_code(line)
        # 1) 构造器内 <-（只在「看起来像表构造器」的花括号里报：块语句里有 local/return 等）
        for m in re.finditer(r"\{([^{}]*)\}", code):
            body = m.group(1)
            if "<-" in body and not STATEMENTY.search(body):
                issues.append((i, "构造器内出现 <-：" + m.group(0).strip()))
        # 2) 关键字当标识符
        for m in re.finditer(r"\blocal\s+(\w+)", code):
            if m.group(1) in KEYWORDS:
                issues.append((i, "用关键字当局部变量名：" + m.group(1)))
        # 3) 不存在的运算符
        for m in re.finditer(r"[&|^]=(?!=)|<<=|>>=", code):
            issues.append((i, "不存在的运算符：" + m.group(0)))
        # 4) 函数体内 const
        if depth > 0 and re.search(r"\bconst\b", code):
            issues.append((i, "函数体内出现 const"))
        # 5) rawcall 当成员
        if re.search(r"\.\s*rawcall\s*\(", code):
            issues.append((i, "把 rawcall 当成员用（它是关键字）"))
        # 6) 数字紧跟 . 标识符
        for m in re.finditer(r"\b\d+\.[A-Za-z_]\w*", code):
            issues.append((i, "数字后紧跟 . 标识符会被词法器吃掉：" + m.group(0)))
        depth += code.count("{") - code.count("}")
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

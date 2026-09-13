# -*- coding: utf-8 -*-
"""生成「运算符 × 类型」矩阵语料 tests-corpus/t_opmx.nut。

设计要点：

- 每个类型**只建一个对象**（`vtbl` 全局唯一）。同类型两两比较时就是同一个对象，
  `<=>` 直接命中 `_rawval` 相等分支返回 0；若用两个不同对象，参考实现按
  裸指针序比较（`_userpointer(o1) < _userpointer(o2)`），跨实现不可复现，
  这种差异不是缺陷，不该混进语料。
- 每个用例都在 try/catch 里，结果或错误文本都会打印，脚本正常结束
  （否则 diff_test.py 会判 SUSPECT「提前结束」）。
- 位运算的右操作数单独取小集合：`7 << -3` 在 C 里是 UB，参考实现会直接崩，
  不复刻。

用法：
    python tools/gen_opmatrix.py            # 写出 tests-corpus/t_opmx.nut
"""
import os

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_opmx.nut")

# 名字, 字面量
CORE = [
    ("null", "null"),
    ("int", "7"),
    ("neg", "-3"),
    ("zero", "0"),
    ("flt", "2.5"),
    ("fneg", "-0.5"),
    ("tru", "true"),
    ("fls", "false"),
    ("str", '"a"'),
    ("estr", '""'),
    ("tbl", "{}"),
    ("arr", "[1]"),
]

# 另外单独建变量的类型（构造有副作用/需要多字符，不适合塞进 CORE 的重复使用矩阵）
EXTRA = [
    ("clo", "(function(){})"),
    ("cls", "(class{})"),
    ("inst", "(class{}())"),
    ("gen", "((function(){yield 1;})())"),
    ("thr", "(newthread(function(){}))"),
    ("wk", "(({}).weakref())"),
]

ARITH = [("add", "+"), ("sub", "-"), ("mul", "*"), ("div", "/"), ("mod", "%")]
BIT = [("band", "&"), ("bor", "|"), ("bxor", "^"), ("shl", "<<"), ("shr", ">>")]
CMP = [("eq", "=="), ("ne", "!="), ("lt", "<"), ("le", "<="), ("gt", ">"), ("ge", ">="), ("cmp3", "<=>")]

# 位运算的右操作数子集（避开负数移位这种 C 语言 UB）
BITRHS = ["int", "zero", "flt", "str", "null", "tbl"]

UNARY = [("neg", "-"), ("bnot", "~"), ("not", "!")]

lines = [
    "// 自动生成：运算符 × 类型矩阵（tools/gen_opmatrix.py）",
    "// 每个类型只建一个对象：同类型比较即同一对象，避免指针序这种不可复现的差异",
    "function s(tag, f) {",
    '    try { print(tag + "=" + f() + "\\n"); }',
    '    catch (e) { print(tag + "!" + e + "\\n"); }',
    "}",
    "",
]
for name, lit in CORE + EXTRA:
    lines.append("local v%s = %s;" % (name, lit))
lines.append("")

names = [n for n, _ in CORE]
allnames = [n for n, _ in CORE + EXTRA]


def emit(opname, op, a, b):
    lines.append('s("op_%s_%s_%s", function() { return v%s %s v%s; });' % (opname, a, b, a, op, b))


# 1) 算术：核心集合两两组合（含 null/bool/table/array 的错误路径）
for opname, op in ARITH:
    for a in names:
        for b in names:
            emit(opname, op, a, b)

# 2) 比较：核心集合两两组合
for opname, op in CMP:
    for a in names:
        for b in names:
            emit(opname, op, a, b)

# 3) 位运算：左操作数全类型，右操作数取安全子集
for opname, op in BIT:
    for a in names:
        for b in BITRHS:
            emit(opname, op, a, b)

# 4) in：核心集合两两组合
for a in names:
    for b in names:
        emit("in", "in", a, b)

# 5) 非核心类型（闭包/类/实例/生成器/线程/弱引用）：只与代表值组合，两侧都测
for name in [n for n, _ in EXTRA]:
    for other in ["null", "int", "str", "tbl"]:
        for opname, op in ARITH + CMP:
            emit(opname, op, name, other)
            emit(opname, op, other, name)
        emit("in", "in", other, name)
        emit("in", "in", name, other)

# 6) 一元运算：全类型
for opname, op in UNARY:
    for a in names + [n for n, _ in EXTRA]:
        lines.append('s("un_%s_%s", function() { return %sv%s; });' % (opname, a, op, a))

# 7) 取索引 / 取成员 / 默认委托（每类型一条）
for a in names + [n for n, _ in EXTRA]:
    lines.append('s("idx_int_%s", function() { return v%s[0]; });' % (a, a))
    lines.append('s("idx_str_%s", function() { return v%s["k"]; });' % (a, a))
    lines.append('s("idx_null_%s", function() { return v%s[null]; });' % (a, a))
    lines.append('s("attr_%s", function() { return v%s.foo; });' % (a, a))
    lines.append('s("tostr_%s", function() { return "" + v%s; });' % (a, a))
    lines.append('s("typeof_%s", function() { return typeof v%s; });' % (a, a))
    lines.append('s("dstr_%s", function() { return v%s.tostring(); });' % (a, a))
    lines.append('s("len_%s", function() { return v%s.len(); });' % (a, a))
    lines.append('s("weakref_%s", function() { return typeof v%s.weakref(); });' % (a, a))
    lines.append('s("delnull_%s", function() { return delete v%s[null]; });' % (a, a))

# 8) 赋值/删除：必须用**新建**的对象，否则会污染上面共享的矩阵值
FRESH = [
    ("tbl", "{}"),
    ("arr", "[1, 2]"),
    ("inst", "(class{}())"),
    ("cls", "(class{})"),
    ("str", '"a"'),
    ("int", "1"),
    ("null", "null"),
    ("gen", "((function(){yield 1;})())"),
    ("wk", "(({}).weakref())"),
]
for a, lit in FRESH:
    lines.append('s("set_attr_%s", function() { local o = %s; o.foo = 1; return "ok"; });' % (a, lit))
    lines.append('s("set_idx_%s", function() { local o = %s; o[0] = 1; return "ok"; });' % (a, lit))
    lines.append('s("newslot_%s", function() { local o = %s; o.foo <- 1; return "ok"; });' % (a, lit))
    lines.append('s("newslot_idx_%s", function() { local o = %s; o[0] <- 1; return "ok"; });' % (a, lit))
    lines.append('s("del_attr_%s", function() { local o = %s; return delete o.foo; });' % (a, lit))
    lines.append('s("del_idx_%s", function() { local o = %s; return delete o[0]; });' % (a, lit))

# 9) 复合赋值（只有 += -= *= /= %= 存在；位运算复合赋值不是 Squirrel 语法）
for opname, op in ARITH:
    lines.append('s("cassign_%s_tbl", function() { local o = { n = 4 }; o.n %s= 2; return o.n; });' % (opname, op))
    lines.append('s("cassign_%s_arr", function() { local o = [4]; o[0] %s= 2; return o[0]; });' % (opname, op))
    lines.append('s("cassign_%s_cls", function() { class C { n = 4; } C.n %s= 2; return C.n; });' % (opname, op))
    lines.append('s("cassign_%s_inst", function() { class C { n = 4; } local o = C(); o.n %s= 2; return o.n; });' % (opname, op))

# 自增自减（前缀/后缀）打在实例字段、类静态字段、数组元素、表槽上
for tail, desc in (("++", "inc"), ("--", "dec")):
    lines.append('s("post_%s_inst", function() { class C { n = 4; } local o = C(); local r = o.n%s; return r + "," + o.n; });' % (desc, tail))
    lines.append('s("pre_%s_inst", function() { class C { n = 4; } local o = C(); local r = %so.n; return r + "," + o.n; });' % (desc, tail))
    lines.append('s("post_%s_clsstatic", function() { class C { static n = 4; } local r = C.n%s; return r + "," + C.n; });' % (desc, tail))
    lines.append('s("post_%s_arr", function() { local a = [4]; local r = a[0]%s; return r + "," + a[0]; });' % (desc, tail))
    lines.append('s("post_%s_str", function() { local s = "a"; local r = s[0]%s; return r + "," + s; });' % (desc, tail))
    lines.append('s("post_%s_int", function() { local i = 4; local r = i%s; return r + "," + i; });' % (desc, tail))
    lines.append('s("post_%s_flt", function() { local f = 4.5; local r = f%s; return r + "," + f; });' % (desc, tail))
    lines.append('s("post_%s_bool", function() { local b = true; local r = b%s; return r + "," + b; });' % (desc, tail))

lines.append('print("OPMATRIX-DONE\\n");')

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 行" % (OUT, len(lines)))

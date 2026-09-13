# -*- coding: utf-8 -*-
"""随机差分模糊测试：生成随机**确定性合法**脚本，两侧逐字节对比。

为什么不是普通的 fuzzer：我们要找的是"人工想不到的组合"造成的**语义**分叉，所以

1. **只生成合法且确定的程序**：给定 seed，程序固定；不生成 `time()`/`rand()` 的值、
   不打印裸指针（容器统一用递归 `show()` 渲染，带深度上限防环）、不读写文件；
2. **每个用例一行** `t("cNNN", function(){...})`，报错也是一种被比较的观测
   （`catch` 里打印错误文本）—— 于是"运行时差异"不会让整个脚本失败；
3. **自动收缩**：发生差异时按行删除，保留仍然复现差异的最小子集；
4. **避开参考实现的已知 UB**（约 26 类，见 STATUS.md/MIGRATION.md）：
   负数 blob 尺寸、非法文件模式、`tointeger` 坏进制、深嵌套、`format` 超长、
   `split(非串)`、`regexp("")`、≥0x80 字节的 `escape`、无限循环/递归等一律不生成；
5. 参考实现**崩溃**（rc≠0 或没有 DONE 标记）单独归类为 "ref-ub" 跳过，
   不算我们的差异 —— 但会打印出来，方便人工判断是不是新形态。

用法：
    python tools/fuzz_diff.py                 # 默认 100 个种子
    python tools/fuzz_diff.py --seeds 500
    python tools/fuzz_diff.py --seed 12345 --keep   # 保留生成的脚本
"""
import argparse
import os
import random
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RUST = os.path.join(ROOT, "target", "release", "sq.exe")
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")
WORK = os.path.join(ROOT, "tmp-c2", "fuzz")
CORPUS = os.path.join(ROOT, "tests-corpus")

# 参考实现会在若干输入上崩溃（已知 UB）：必须抑制模态崩溃框，否则批量跑会卡住
sys.path.insert(0, os.path.join(ROOT, "tools"))
try:
    from safe_run import suppress_crash_dialogs
    suppress_crash_dialogs()
except Exception as e:  # pragma: no cover
    print("warning: 未能抑制崩溃框：%r" % (e,), file=sys.stderr)

PREAMBLE = r"""
// fuzz_diff.py 生成（seed=%SEED%）
function show(r, d) {
    if (d == null) d = 0;
    if (d > 3) return "...";
    local t = typeof r;
    if (t == "array") {
        local s = "[";
        for (local i = 0; i < r.len(); i++) { if (i > 0) s += ","; s += show(r[i], d + 1); }
        return s + "]";
    }
    if (t == "table") {
        local s = "{";
        foreach (k, v in r) s += show(k, d + 1) + ":" + show(v, d + 1) + ",";
        return s + "}";
    }
    if (t == "string") return "\"" + r + "\"";
    if (t == "float") return format("%.17g", r);
    return "" + r;
}
function t(tag, f) {
    try { local v = f(); print(tag + "=" + show(v) + "|" + typeof v + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
class FzClass { x = 1; function getx() { return x; } }
class FzBase { function who() { return "base"; } }
class FzDerived extends FzBase { function who() { return "derived/" + base.who(); } }
local fz_counter = 0;
function fz_inc() { fz_counter = fz_counter + 1; return fz_counter; }
"""

# ---------------------------------------------------------------- 取值池
INTS = ["0", "1", "-1", "2", "7", "-7", "3", "255", "256", "65535", "-65536",
        "2147483647", "-2147483648", "4294967296", "-4294967297",
        "9223372036854775807", "-9223372036854775808", "1000000007", "-3"]
FLOATS = ["0.0", "-0.0", "0.5", "-0.5", "1.5", "-1.5", "2.0", "3.1415927",
          "1e10", "-1e10", "1e-10", "3.4028235e38", "-3.4028235e38",
          "1.1754944e-38", "(1.0 / 0.0)", "(-1.0 / 0.0)", "(0.0 / 0.0)"]
STRINGS = ['""', '"a"', '"abc"', '"hello world"', '"\\t\\n"', '"0x1f"', '"42abc"',
           '"  pad  "', '"a,b,,c"', '"AbC"', '"\\\\"', '"\\""', '"x"', '"0123"']
CONTAINERS = ["[]", "[1]", "[1, 2, 3]", '["a", "b"]', '[1, "a", null, true]',
              "[[1], [2]]", "[]", "{}", "{ a = 1 }", "{ a = 1, b = 2 }",
              '{ x = "s" }', '{ [1] = "intkey" }', "array(3)", "array(2, 7)"]

UNARY = ["-", "!", "~", "typeof ", "clone "]
BINOPS = ["+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">=", "<=>",
          "&&", "||", "&", "|", "^", "<<", ">>", ">>>", "in"]

# 安全的全局函数：(名字, 参数个数, 参数生成器)
SAFE_GLOBALS = [
    ("sqrt", 1), ("fabs", 1), ("sin", 1), ("cos", 1), ("tan", 1),
    ("asin", 1), ("acos", 1), ("atan", 1), ("log", 1), ("log10", 1),
    ("exp", 1), ("floor", 1), ("ceil", 1), ("abs", 1), ("pow", 2), ("atan2", 2),
    ("type", 1), ("array", 1), ("dummy", 1),
    ("strip", 1), ("lstrip", 1), ("rstrip", 1), ("escape", 1),
    ("startswith", 2), ("endswith", 2),
]

ARRAY_METHODS = [
    ("len", 0), ("append", 1), ("push", 1), ("pop", 0), ("top", 0),
    ("insert", 2), ("remove", 1), ("resize", 1), ("reverse", 0), ("slice", 1),
    ("slice", 2), ("map", 1), ("apply", 1), ("filter", 1), ("find", 1),
    ("extend", 1), ("clear", 0), ("tostring", 0), ("weakref", 0),
]
TABLE_METHODS = [
    ("len", 0), ("rawget", 1), ("rawset", 2), ("rawdelete", 1), ("rawin", 1),
    ("keys", 0), ("values", 0), ("clear", 0), ("tostring", 0), ("weakref", 0),
]
STRING_METHODS = [
    ("len", 0), ("tointeger", 0), ("tointeger", 1), ("tofloat", 0),
    ("tostring", 0), ("slice", 1), ("slice", 2), ("find", 1), ("find", 2),
    ("tolower", 0), ("toupper", 0), ("tochar", 0), ("weakref", 0),
]
NUMBER_METHODS = [("tointeger", 0), ("tofloat", 0), ("tochar", 0),
                  ("tostring", 0), ("len", 0), ("weakref", 0)]


class Gen:
    def __init__(self, seed):
        self.rnd = random.Random(seed)

    def pick(self, xs):
        return self.rnd.choice(xs)

    def scalar(self):
        k = self.rnd.randrange(10)
        if k < 4:
            return self.pick(INTS)
        if k < 6:
            return self.pick(FLOATS)
        if k < 8:
            return self.pick(STRINGS)
        if k == 8:
            return self.pick(["true", "false", "null"])
        return self.pick(INTS + FLOATS)

    def value(self, depth=0):
        """产生一个"可安全求值"的表达式（不 throw 或者 throw 也被 catch）"""
        k = self.rnd.randrange(12)
        if depth >= 2:
            return self.scalar()
        if k < 6:
            return self.scalar()
        if k < 8:
            return self.pick(CONTAINERS)
        if k == 8:
            return "%s(%s)" % (self.pick(["sqrt", "fabs", "sin", "floor", "exp"]),
                               self.pick(FLOATS + INTS))
        if k == 9:
            return "(%s %s %s)" % (self.value(depth + 1),
                                   self.pick(["+", "-", "*", "==", "!=", "<", "<=>"]),
                                   self.value(depth + 1))
        if k == 10:
            # 必须加括号：`-` 直接粘到 `-0.5` 上会变成自减 `--0.5`
            return "%s(%s)" % (self.pick(["-", "!", "~", "typeof ", "clone "]),
                               self.value(depth + 1))
        return "(%s ? %s : %s)" % (self.value(depth + 1), self.value(depth + 1),
                                   self.value(depth + 1))

    def method_call(self, depth=0):
        """随机挑一个 (接收者, 方法, 参数) 组合"""
        which = self.rnd.randrange(4)
        if which == 0:
            recv = self.pick(CONTAINERS)
            if recv.startswith("array("):
                recv = "array(3)"
            name, n = self.pick(ARRAY_METHODS)
            if recv.startswith("{") and name not in [m for m, _ in TABLE_METHODS]:
                recv = "[1, 2, 3]"
            if recv.startswith("["):
                recv = recv if name in [m for m, _ in ARRAY_METHODS] else "[1, 2, 3]"
            if recv.startswith("{"):
                return "%s.%s(%s)" % (recv, self.pick(TABLE_METHODS)[0],
                                      self.args(self.pick([m for m, _ in TABLE_METHODS]), depth))
        elif which == 1:
            name, n = self.pick(STRING_METHODS)
            recv = self.pick(STRINGS)
            if name == "tointeger" and n == 1:
                return "%s.tointeger(%s)" % (recv, self.pick(["0", "10", "16", "8", "2"]))
            if name == "find" and n == 2:
                return "%s.find(%s, %s)" % (recv, self.pick(STRINGS), self.pick(["0", "1", "-1", "99"]))
            if name == "slice":
                if n == 1:
                    return "%s.slice(%s)" % (recv, self.pick(["0", "1", "-1", "-3", "99", "1.7"]))
                return "%s.slice(%s, %s)" % (recv, self.pick(["0", "1", "-2", "2"]),
                                             self.pick(["1", "3", "-1", "99"]))
            if name == "tochar":
                return "%s.tochar()" % self.pick(["65", "0", "255", "127", "'a'", '"A"'])
            return "%s.%s()" % (recv, name)
        elif which == 2:
            name, n = self.pick(NUMBER_METHODS)
            recv = self.pick(INTS + FLOATS)
            if name == "tochar":
                return "(%s).tochar()" % self.pick(["65", "0", "255", "127"])
            return "(%s).%s()" % (recv, name)
        else:
            recv = self.pick(CONTAINERS)
            name, n = self.pick(ARRAY_METHODS if recv.startswith("[") or recv.startswith("array") else TABLE_METHODS)
            return "%s.%s(%s)" % (recv, name, self.args(name, depth))

    def args(self, name, depth=0):
        if name in ("append", "push", "top", "pop", "reverse", "clear", "len",
                    "keys", "values", "tostring", "weakref"):
            return ""
        if name in ("insert", "rawset"):
            return "%s, %s" % (self.pick(["0", "1", "2", "-1", "99"]), self.value(depth + 1))
        if name in ("remove", "rawget", "rawdelete", "rawin", "find"):
            return self.pick(["0", "1", "-1", "99", self.pick(STRINGS)])
        if name == "resize":
            return self.pick(["0", "1", "3", "2.9", "5"])
        if name in ("slice",):
            return self.pick(["0", "1", "-2", "2"])
        if name in ("map", "apply", "filter"):
            return self.pick(["@(x) x", "@(x) x + 1", "@(x) \"s\"", "@(x) null"])
        if name == "extend":
            return self.pick(["[]", "[1, 2]", '["a"]', "array(2, 1)"])
        return ""

    def stmt_block(self, depth=0):
        """随机语句块（局部变量 + 运算 + 控制流），最后 return 一个值"""
        r = self.rnd.randrange(16)
        if r < 3:
            v = self.value(depth)
            return ["return %s;" % v]
        if r < 5:
            op = self.pick(BINOPS)
            a = self.pick(INTS + FLOATS + STRINGS + CONTAINERS)
            b = self.pick(INTS + FLOATS + STRINGS + CONTAINERS)
            return ["local a = %s;" % a, "local b = %s;" % b,
                    "local r = a %s b;" % op,
                    "return show(r) + \"|\" + typeof r;"]
        if r < 7:
            n = self.pick(["1", "3", "5", "8"])
            body = self.pick(["s = s + i;", "s = s + i % 3;", "s += i * 2;",
                              "if (i % 2 == 0) s = s + 1; else s = s - 1;",
                              "s = s ^ i;", "s = s + (i << 1);"])
            return ["local s = %s;" % self.pick(["0", "1", "-1", "0.5"]),
                    "for (local i = 0; i < %s; i++) { %s }" % (n, body),
                    "return s;"]
        if r < 8:
            c = self.pick(CONTAINERS)
            return ["local c = %s;" % c,
                    "local n = 0; local acc = \"\";",
                    "foreach (k, v in c) { n++; acc += show(k) + \"->\" + show(v) + \";\"; }",
                    "return n + \"/\" + acc;"]
        if r < 9:
            inner = self.pick([
                "throw \"boom\"", "throw 42", "throw null", "local z = 1 / 0",
                "local z = null; z.x", "local z = [1]; z[9]",
                "local z = \"a\".tointeger(\"dec\")", "local z = \"x\" + {}",
                "local z = sqrt(\"a\")", "return 1",
            ])
            return ["try { %s; } catch (e) { return \"caught:\" + e; }" % inner,
                    "return \"no-throw\";"]
        recv = self.pick(CONTAINERS)
        name = self.pick(["append", "resize", "insert", "remove", "len", "pop", "reverse"])
        if recv.startswith("{"):
            return ["local c = %s;" % recv, "return c.len();"]
        if r == 10:
            # 闭包 + outer 捕获（最容易在栈槽/共享单元上分叉）
            return ["local n = %s;" % self.pick(["0", "1", "-1", "2.5"]),
                    "local f = function(a) { n = n + a; return n; };",
                    "local x = f(%s); local y = f(%s);" % (self.pick(INTS), self.pick(INTS)),
                    "return show(x) + \"/\" + show(y) + \"/\" + show(n);"]
        if r == 11:
            # 闭包数组：每个闭包捕获不同的 i
            return ["local fs = [];",
                    "for (local i = 0; i < 3; i++) { fs.append(function() { return i; }); }",
                    "local out = \"\"; foreach (f in fs) out += show(f()) + \",\";",
                    "return out;"]
        if r == 12:
            # 类 / 实例 / 继承 / 属性表
            return [self.pick([
                "local o = FzClass(); o.x = %s; return show(o.x) + \"/\" + show(o.getx());"
                % self.pick(INTS + FLOATS + STRINGS),
                "local a = FzClass(); local b = FzClass(); a.x = 9; return show(a.x) + \"/\" + show(b.x);",
                "return FzDerived().who();",
                "return typeof FzClass + \"/\" + typeof FzClass() + \"/\" + (FzClass() instanceof FzClass);",
                "local o = FzClass(); return show(o.getclass()) + \"/\" + (o.getclass() == FzClass);",
                "return show(FzClass.getattributes(\"x\")) ;",
                "local tb = {}; tb.setdelegate({ _get = function(k) { return \"M:\" + k; } }); return show(tb.anything);",
                "local tb = {}; tb.setdelegate({ _set = function(k, v) { return \"ignored\"; } }); tb.k = 1; return show(tb.len());",
                "local tb = { _get = function(k) { return this.len(); } }; return show(tb[\"whatever\"]);",
            ])]
        if r == 13:
            # switch / 嵌套作用域
            return ["local v = %s;" % self.pick(INTS + STRINGS),
                    "switch (v) { case 0: return \"zero\"; case 1: return \"one\"; case \"a\": return \"stra\"; default: return \"other\"; }"]
        if r == 14:
            # 生成器（有界，避免死循环）
            return ["local g = (function() { for (local i = 0; i < 3; i++) yield i * %s; }());"
                    % self.pick(["1", "-1", "2"]),
                    "local out = \"\"; local st = 0;",
                    "while (st != \"dead\") { local r = resume g; if (st == \"dead\") break; out += show(r) + \",\"; st = g.getstatus(); }",
                    "return out + \"/\" + g.getstatus();"]
        if r == 15:
            # 方法链 / 就地修改
            return ["local c = %s;" % self.pick(["[3, 1, 2]", "[1, 2, 3]", '["b", "a"]', "[]"]),
                    "local r1 = c.%s(%s);" % (self.pick([("sort", ""), ("reverse", ""), ("append", "9"),
                                                         ("extend", "[5]"), ("clear", ""), ("len", "")])),
                    "return show(c) + \" / \" + show(r1);"]
        return ["local c = %s;" % recv,
                "local r = c.%s(%s);" % (name, self.args(name)),
                "return show(c) + \" / \" + show(r);"]

    def case(self, i):
        body = " ".join(self.stmt_block())
        return 't("c%03d", function() { %s });' % (i, body)


def gen_program(seed, ncases=None):
    g = Gen(seed)
    rnd = random.Random(seed ^ 0x5F5F)
    n = ncases or rnd.randrange(12, 26)
    lines = [PREAMBLE.replace("%SEED%", str(seed))]
    for i in range(n):
        lines.append(g.case(i))
    lines.append('print("FUZZ-DONE\\n");')
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------- 变异模式
# 结构化随机只能碰到"浅层组合"。语料里的程序是人工/矩阵打磨过的，对它们做**小变异**
# 更容易走到深路径（正则、数值矩阵、序列化…）。变异保持"逐行局部"，不改结构。
MUT_SKIP = ("rand(", "srand(", "clock(", "time(", "date(", "getenv(", "system(",
            "loadfile", "dofile", "compilestring", "writeclosuretofile", "file(",
            "remove(", "rename(", "printf", "print(", "regexp(", "newthread",
            "suspend", "collectgarbage")
MUT_INTS = ["0", "1", "-1", "2", "7", "255", "256", "2147483647", "-2147483648",
            "2147483648", "4294967296", "9223372036854775807",
            "-9223372036854775808", "63", "64", "65"]
MUT_FLOATS = ["0.0", "-0.0", "0.5", "-0.5", "1.5", "1e30", "1e-30", "3.4028235e38",
              "1.1754944e-38", "0.99999994", "(1.0 / 0.0)", "(0.0 / 0.0)"]
MUT_STRS = ['"a"', '"abc"', '""', '"A"', '"abcabc"', '"\\t"', '"0x1f"', '"9"']
MUT_OPS = [(" + ", " - "), (" - ", " * "), (" * ", " + "), (" / ", " % "),
           (" < ", " <= "), (" > ", " >= "), (" == ", " != "), (" <= ", " > "),
           ("<<", ">>"), (">>>", "<<"), ("&", "|"), ("|", "^"), ("&&", "||")]
LIT_INT_RE = re.compile(r"(?<![\w.])(\d+)(?![\w.])")
LIT_FLOAT_RE = re.compile(r"(?<![\w.])(\d+\.\d+(?:[eE][-+]?\d+)?)(?![\w.])")
LIT_STR_RE = re.compile(r'"(?:[^"\\]|\\.)*"')


def mutate_program(seed):
    """随机取一个语料文件，随机做 1~3 处小变异"""
    rnd = random.Random(seed)
    names = sorted(n for n in os.listdir(CORPUS) if n.endswith(".nut"))
    name = rnd.choice(names)
    with open(os.path.join(CORPUS, name), "r", encoding="utf-8", errors="replace") as f:
        lines = f.read().splitlines()
    idxs = [i for i, l in enumerate(lines)
            if l.strip() and not any(s in l for s in MUT_SKIP)]
    if not idxs:
        return None, name
    for _ in range(rnd.randrange(1, 4)):
        i = rnd.choice(idxs)
        kind = rnd.randrange(6)
        s = lines[i]
        if kind == 0:
            s2 = LIT_INT_RE.sub(lambda m: rnd.choice(MUT_INTS), s, count=1)
        elif kind == 1:
            s2 = LIT_FLOAT_RE.sub(lambda m: rnd.choice(MUT_FLOATS), s, count=1)
        elif kind == 2:
            s2 = LIT_STR_RE.sub(lambda m: rnd.choice(MUT_STRS), s, count=1)
        elif kind == 3:
            a, b = rnd.choice(MUT_OPS)
            s2 = s.replace(a, b, 1)
        elif kind == 4:
            s2 = s + " " + s           # 复制一行
        else:
            s2 = s.replace("==", "!=", 1) if "==" in s else s
        if s2 != s:
            lines[i] = s2
    return "\n".join(lines) + "\n", name


def run(exe, path):
    p = subprocess.run([exe, path], capture_output=True, cwd=ROOT, timeout=60)
    out = p.stdout.decode("utf8", "replace").replace("\r", "")
    err = p.stderr.decode("utf8", "replace").replace("\r", "")
    return p.returncode, out, err


def norm(text):
    return re.sub(r"0x[0-9a-fA-F]{8,}", "0xPTR", text)


def compare(src, tag, require_done=True):
    """返回 (kind, detail)：kind ∈ ok / diff / ref-ub / rust-fail / both-fail

    require_done：只有生成器脚本才打印 FUZZ-DONE；变异模式下语料文件没有这个标记，
    完成判据退化为"rc == 0"（编译失败是 -2，崩溃是 0xC……，都能区分开）。
    """
    path = os.path.join(WORK, "case_%s.nut" % tag)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(src)
    rc_r, out_r, err_r = run(RUST, path)
    rc_f, out_f, err_f = run(REF, path)
    if require_done:
        ok_r = rc_r == 0 and "FUZZ-DONE" in out_r
        ok_f = rc_f == 0 and "FUZZ-DONE" in out_f
    else:
        ok_r = rc_r == 0
        ok_f = rc_f == 0
    if not ok_r and not ok_f:
        # 生成器自检：两侧都跑不完通常是生成器产出了非法程序，把引擎错误带出来
        first = (err_r or out_r).strip().splitlines()
        return "both-fail", "rust_rc=%d ref_rc=%d %s" % (
            rc_r, rc_f, first[0][:80] if first else "")
    if not ok_f:
        first = (err_f or out_f).strip().splitlines()
        return "ref-ub", "ref_rc=%d %s" % (rc_f, first[0][:80] if first else "")
    if not ok_r:
        first = (err_r or out_r).strip().splitlines()
        return "rust-fail", "rust_rc=%d %s" % (rc_r, first[0][:80] if first else "")
    a, b = norm(out_r).splitlines(), norm(out_f).splitlines()
    if a != b:
        d = [(x, y) for x, y in zip(a, b) if x != y]
        # 已知的有意偏离：对已挂起/已死线程再次 call() 时，参考实现是从**残留栈槽**
        # 读出类型名（实测同一个程序形态里给过 'string'，复制一行 call 之后变成
        # 'integer'），我们关帧后取到环境表 → 'table'。两者都是可捕获错误，
        # 只有类型名不同，见 MIGRATION.md 第 3 节。这类单独归类，不当成新差异。
        if d and all(("attempt to call '" in x and "attempt to call '" in y)
                     for x, y in d):
            return "known-dev", "%d 行（线程二次 call 的已知偏离）" % len(d)
        return "diff", "%d 行不同（共 %d/%d）" % (len(d), len(a), len(b))
    return "ok", ""


def crash_case(src, tag):
    """参考实现崩了：找出让它可以崩的最小前缀，返回那条用例的源码行。

    注意 stdout 是带缓冲的，所以"零输出"既可能是编译期崩溃也可能是运行期崩溃，
    必须用前缀二分来定位（见 P5/P6 轮的经验）。
    """
    lines = src.splitlines()
    head = [l for l in lines if not l.startswith('t("c') and 'FUZZ-DONE' not in l]
    cases = [l for l in lines if l.startswith('t("c')]
    lo, hi = 0, len(cases)
    # hi 一定崩（整体就崩）；二分最小的崩前缀
    while hi - lo > 1:
        mid = (lo + hi) // 2
        kind, _ = compare("\n".join(head + cases[:mid]) + "\n", tag + "_bis")
        if kind == "ref-ub":
            hi = mid
        else:
            lo = mid
    return cases[hi - 1] if hi >= 1 else "(preamble)"


def shrink(src, tag, require_done=True):
    """按行删除，保留仍然复现差异的最小子集

    require_done 必须与 compare 的口径一致：变异模式下语料文件没有 FUZZ-DONE，
    传错会让每次候选都判成 both-fail，于是"永远收缩不动"（这个 bug 实际发生过）。
    """
    lines = src.splitlines()
    i = 0
    while i < len(lines):
        cand = lines[:i] + lines[i + 1:]
        if len(cand) < 4:
            break
        kind, _ = compare("\n".join(cand) + "\n", tag + "_shrink",
                          require_done=require_done)
        if kind == "diff":
            lines = cand
        else:
            i += 1
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seeds", type=int, default=100)
    ap.add_argument("--seed", type=int, default=None)
    ap.add_argument("--keep", action="store_true")
    ap.add_argument("--mutate", action="store_true",
                    help="对 tests-corpus/ 里的程序做小变异（走深路径，比结构化随机更容易出货）")
    args = ap.parse_args()

    for exe in (RUST, REF):
        if not os.path.exists(exe):
            print("缺少可执行文件：%s" % exe)
            return 2
    if not os.path.isdir(WORK):
        os.makedirs(WORK)

    seeds = [args.seed] if args.seed is not None else list(range(1, args.seeds + 1))
    stat = {"ok": 0, "diff": 0, "ref-ub": 0, "rust-fail": 0, "both-fail": 0,
            "known-dev": 0}
    fails = []
    badgen = []
    for s in seeds:
        if args.mutate:
            src, origin = mutate_program(s)
            if src is None:
                stat["both-fail"] += 1
                continue
            tag = "m%d" % s
        else:
            src, origin = gen_program(s), "gen"
            tag = str(s)
        kind, detail = compare(src, tag, require_done=not args.mutate)
        stat[kind] += 1
        if kind == "diff":
            small = shrink(src, tag, require_done=not args.mutate)
            path = os.path.join(WORK, "fail_%d.nut" % s)
            with open(path, "w", encoding="utf-8", newline="\n") as f:
                f.write(small)
            fails.append((s, path, "%s (%s)" % (detail, origin)))
            print("DIFF seed=%d %s from=%s -> %s" % (s, detail, origin, path))
        elif kind in ("ref-ub",):
            if args.mutate:
                path = os.path.join(WORK, "refub_%d.nut" % s)
                with open(path, "w", encoding="utf-8", newline="\n") as f:
                    f.write(src)
                print("ref-ub seed=%d %s from=%s -> %s" % (s, detail, origin, path))
            else:
                line = crash_case(src, tag)
                print("ref-ub seed=%d %s\n        崩在: %s" % (s, detail, line[:150]))
        elif kind == "known-dev":
            print("known-dev seed=%d %s from=%s" % (s, detail, origin))
        elif kind == "rust-fail":
            path = os.path.join(WORK, "rustfail_%d.nut" % s)
            shutil.copy(os.path.join(WORK, "case_%s.nut" % tag), path)
            fails.append((s, path, "%s (%s)" % (detail, origin)))
            print("RUST-FAIL seed=%d %s -> %s" % (s, detail, path))
        elif kind == "both-fail":
            # 生成器/变异自检失败：两侧都跑不动（变异后语法坏了属正常，结构化生成则是 bug）
            path = os.path.join(WORK, "badgen_%d.nut" % s)
            shutil.copy(os.path.join(WORK, "case_%s.nut" % tag), path)
            badgen.append((s, path, "%s (%s)" % (detail, origin)))
            if not args.mutate:
                print("BAD-GEN seed=%d %s -> %s" % (s, detail, path))
        if not args.keep and kind == "ok":
            try:
                os.remove(os.path.join(WORK, "case_%s.nut" % tag))
            except OSError:
                pass

    print("\n== 结果 ==")
    for k in ("ok", "diff", "ref-ub", "rust-fail", "both-fail", "known-dev"):
        print("  %-10s %d" % (k, stat[k]))
    if fails:
        print("\n需要人工看的（真实差异/我方失败）：")
        for s, path, d in fails:
            print("  seed=%-6d %s  %s" % (s, d, path))
    if badgen and not args.mutate:
        print("\n生成器要修的（两侧都跑不动）：")
        for s, path, d in badgen:
            print("  seed=%-6d %s  %s" % (s, d, path))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())

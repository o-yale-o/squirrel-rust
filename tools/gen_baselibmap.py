# -*- coding: utf-8 -*-
"""生成 base 库"零覆盖全局函数"矩阵语料 tests-corpus/t_baselibmap.nut。

`tools/global_coverage.py` 扫 62 个已注册全局函数，只有 5 个在语料里出现 0 次：
`assert` / `error` / `array` / `type` / `dummy`。全是 sqbaselib.cpp 里的
错误与构造路径，也正是最容易藏分叉的地方。逐条对着参考源码写用例：

- `assert(cond[, msg])`（nparams=-2）：判假用 `SQVM::IsFalse` —— **null / false / 0 / 0.0 都为假**，
  而 `""` / `[]` / `{}` / `function(){}` / `blob(0)` 都是**真**（实测，不是想当然：
  `assert(0)` 会失败、`assert("")` 会通过）；消息参数先 `sq_tostring`，拿得到字符串
  就把它当错误抛出去（`assert(false, 123)` → 错误文本 `123`），否则退回 `assertion failed`。
  注意 `sq_tostring` 对 table/array 给的是 `(table : 0x…)`，**不调 `_tostring`**。
- `error(msg)`（nparams=2）：**不抛异常**，只调 errorfunc 打印，返回 `null`。
- `array(size[, fill])`（nparams=-2，掩码 `.n`）：`tointeger(size)` 后 `Create`/`Resize`；
  负数尺寸是参考实现的 UB（SQVector::resize 收到负值），**不进语料**。
- `type(o)`（nparams=2）：就是 `typeof` 的别名。
- `dummy(...)`（**nparams=0**）：不做参数个数检查，恒返回 `null`。

用法：
    python tools/gen_baselibmap.py
"""
import os

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_baselibmap.nut")

lines = [
    "// 自动生成：base 库零覆盖全局函数矩阵（tools/gen_baselibmap.py）",
    "// assert/error/array/type/dummy —— 62 个全局里仅有的 5 个零覆盖项",
    "function show(r) {",
    "    local t = typeof r;",
    "    if (t == \"array\") {",
    "        local s = \"[\";",
    "        for (local i = 0; i < r.len(); i++) { if (i > 0) s += \",\"; s += show(r[i]); }",
    "        return s + \"]\";",
    "    }",
    "    if (t == \"table\") { local s = \"{\"; foreach (k, v in r) s += \"\" + k + \":\" + show(v) + \",\"; return s + \"}\"; }",
    "    if (t == \"string\") return \"\\\"\" + r + \"\\\"\";",
    "    return \"\" + r;",
    "}",
    "function t(tag, f) {",
    "    try { print(tag + \"=\" + show(f()) + \"|\" + typeof f() + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "// class 声明只能是语句，不能当表达式（`type(class C {})` 是语法错），所以先声明",
    "class TypeProbeClass {}",
    "",
]

# ---- assert：IsFalse 只认 null / false
ASSERT_SHAPES = [
    ("true", "true"), ("false", "false"), ("null", "null"),
    ("i0", "0"), ("i1", "1"), ("f0", "0.0"), ("f1", "1.5"),
    ("es", "\"\""), ("s", "\"x\""), ("arr_e", "[]"), ("arr_1", "[0]"),
    ("tbl_e", "{}"), ("fn", "function() {}"), ("blob", "blob(0)"),
]
for name, expr in ASSERT_SHAPES:
    lines.append('t("a_%s", function() { assert(%s); return "passed"; });' % (name, expr))

ASSERT_MSG = [
    ("msg_s", 'false, "boom"'),
    ("msg_i", "false, 123"),
    ("msg_f", "false, 1.5"),
    ("msg_null", "false, null"),
    ("msg_true", "false, true"),
    ("msg_arr", "false, [1, 2]"),
    ("msg_tbl", 'false, { a = 1 }'),
    ("msg_fn", "false, function() {}"),
    ("msgs_i", "0, 123"),
    ("msg_undef", "false, null"),
]
for name, args in ASSERT_MSG:
    lines.append('t("a_%s", function() { assert(%s); return "passed"; });' % (name, args))

ASSERT_ERR = [
    ("noargs", ""),
    ("manyargs", 'true, "a", "b", "c"'),
]
for name, args in ASSERT_ERR:
    lines.append('t("a_%s", function() { assert(%s); return "passed"; });' % (name, args))

# 消息里的自定义 tostring
lines.append('t("a_msg_tostring", function() { local o = { _tostring = function() { return "custom"; } }; assert(false, o); return "passed"; });')
lines.append('t("a_ret", function() { return assert(true); });')
lines.append('t("a_ret_msg", function() { return assert(true, "unused"); });')

# ---- error：只打印，不抛
ERRORS = [
    ("s", '"boom"'),
    ("es", '""'),
    ("i", "123"),
    ("f", "1.5"),
    ("null", "null"),
    ("true", "true"),
    ("arr", "[1, 2]"),
    ("tbl", "{ a = 1 }"),
    ("blob", "blob(2)"),
    ("fn", "function() {}"),
]
for name, expr in ERRORS:
    lines.append('t("e_%s", function() { local r = error(%s); return "" + r; });' % (name, expr))
lines.append('t("e_none", function() { return error(); });')
lines.append('t("e_two", function() { return error("a", "b"); });')

# ---- array
ARRAY_CASES = [
    ("z0", "0"), ("z1", "1"), ("z3", "3"),
    ("fill_i", "3, 7"), ("fill_s", '2, "x"'), ("fill_null", "2, null"),
    ("fill_arr", "2, [1]"), ("fill_tbl", "1, { a = 1 }"),
    ("frac", "2.9"), ("frac_small", "0.5"),
    ("int_arg", "3"),
    ("extra", "2, 5, 6"),
    ("big", "1000"),
]
for name, args in ARRAY_CASES:
    lines.append('t("ar_%s", function() { return array(%s); });' % (name, args))
lines.append('t("ar_none", function() { return array(); });')
lines.append('t("ar_str", function() { return array("x"); });')
lines.append('t("ar_null", function() { return array(null); });')
lines.append('t("ar_bool", function() { return array(true); });')
lines.append('t("ar_len", function() { return array(3, 7).len(); });')
lines.append('t("ar_type", function() { return typeof array(1); });')
lines.append('t("ar_getclass", function() { return array(1).getclass(); });')

# ---- type（typeof 的别名）
TYPE_VALUES = [
    ("null", "null"), ("int", "1"), ("float", "1.5"), ("bool", "true"),
    ("str", '"x"'), ("arr", "[]"), ("tbl", "{}"), ("fn", "function() {}"),
    ("natfn", "print"), ("blob", "blob(1)"), ("cls", "TypeProbeClass"),
    ("inst", "TypeProbeClass()"), ("gen", "(function() { yield 1; }())"),
    ("th", "newthread(function() {})"), ("wref", "((1).weakref())"),
    ("re", 'regexp("a")'),
]
for name, expr in TYPE_VALUES:
    lines.append('t("ty_%s", function() { return type(%s); });' % (name, expr))
# 非引用类型上的 weakref() 返回**自身**（所以上面那个 ty_wref 是 integer）；真 weakref 要看这里
lines.append('t("ty_wref_real", function() { local tt = {}; return type(tt.weakref()); });')
lines.append('t("ty_wref_int", function() { local i = 5; return type(i.weakref()) + "/" + i.weakref(); });')
lines.append('t("ty_weakref_weakref", function() { local tt = {}; local w = tt.weakref(); return type(w.weakref()); });')
lines.append('t("ty_bare_fn", function() { return type(); });')
lines.append('t("ty_extra", function() { return type(1, 2); });')
lines.append('t("ty_vs_typeof", function() { local a = [1], b = {}; return type(a) + "/" + type(b) + "/" + typeof(a) + "/" + typeof(b); });')

# ---- dummy
lines.append('t("d_none", function() { return dummy(); });')
lines.append('t("d_one", function() { return dummy(1); });')
lines.append('t("d_many", function() { return dummy(1, 2, 3, "x", null); });')
lines.append('t("d_type", function() { return typeof dummy; });')
lines.append('t("d_ret_type", function() { return typeof dummy(1); });')
lines.append('t("d_in_expr", function() { return "" + (dummy(1) == null); });')

lines.append('print("BASELIBMAP-DONE\\n");')

with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")

print("已写出 %s：assert %d 条 + error %d 条 + array %d 条 + type %d 条 + dummy %d 条"
      % (OUT, len(ASSERT_SHAPES) + len(ASSERT_MSG) + len(ASSERT_ERR) + 3,
         len(ERRORS) + 2, len(ARRAY_CASES) + 6, len(TYPE_VALUES) + 3, 6))

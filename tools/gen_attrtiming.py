# -*- coding: utf-8 -*-
"""生成"类属性：时序/共享/继承边界"矩阵语料 tests-corpus/t_attrtiming.nut。

为什么单开一份：`t_attrmap.nut`（40 条）测的是**形态**（</ ... /> 写法、基本
get/set、可见性）；这里补的是**动力学**——实例化之后改属性、继承后改父类、
给继承来的成员 set/get、newmember 的 4 参形式、属性表的**共享引用**、clone 的
边界、参数个数。本轮为**验证型**：30 条探针两侧逐字节一致。

实测钉住的语义（都对着参考实现验过）：
1. **时序**：`setattributes` 在实例化**之后**改，实例透过 `getclass()` 立即可见
   （属性挂在类上，实例不快照）；**继承后改父类属性，子类不受影响**
   （extend 时成员/属性拷贝进子类）。
2. **继承来的成员**：`getattributes` 能读到父类侧的属性；`setattributes` 也能
   设置（返回 null）。三层继承、中间层覆盖 → 顶层看到中间层的。
3. **类级属性（null 键）不随 extends 继承**：子类的 `getattributes(null)` 是
   自己的（没设就是 null）。
4. **属性表是共享引用**：`getattributes("v").a.n = 42` 的改动对后续读取可见。
   （注意：**给属性表加新键** `x.same = "yes"` 会报
   `the index 'same' does not exist`——这是 Squirrel 核心语义"普通 `=` 不建新槽，
   要用 `<-`"，不是属性特有行为。）
5. `newmember` 的 **4 参形式** `(name, val, attrs, isstatic)`：属性表与 static
   标记都能生效；3 参形式 `(name, val, attrs)` 也收属性；不带 attrs 则为 null。
6. `clone` 一个 **class** 直接报 `cloning a class`（类不可 clone）。
7. 实例**没有** `setattributes/getattributes` 方法（在类上）。
8. `getattributes`/`setattributes` 的参数个数都是死规矩（0/2 参、1/3 参全报
   `wrong number of parameters`）。
9. `delete C.v` 报 `attempt to delete a slot from a class`；`rawset` 新成员的
   属性为 null（不影响既有成员的属性）。

用法：
    python tools/gen_attrtiming.py
    python tools/gen_attrtiming.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_attrtiming.nut")

CASES = [
    # --- 1) 时序 ---
    ("t_after_inst", 'local C = class { </ a = 1 /> v = 1; }; local o = C(); C.setattributes("v", { a = 99 }); return d(o.getclass().getattributes("v")) + "/" + d(C.getattributes("v"));'),
    # --- 2) 继承后的归属 ---
    ("t_parent_after_extend", 'local A = class { </ a = 1 /> v = 1; }; local B = class extends A { }; A.setattributes("v", { a = 77 }); return "A=" + d(A.getattributes("v")) + " B=" + d(B.getattributes("v"));'),
    ("set_inherited_member", 'local A = class { v = 1; }; local B = class extends A { }; return d(B.setattributes("v", { a = 5 })) + "/" + d(B.getattributes("v"));'),
    ("get_inherited_member", 'local A = class { </ a = 1 /> v = 1; }; local B = class extends A { }; return d(B.getattributes("v"));'),
    ("mid_override", 'local A = class { </ a = "A" /> v = 1; }; local B = class extends A { </ a = "B" /> v = 2; }; local C = class extends B { }; return "A=" + d(A.getattributes("v")) + " B=" + d(B.getattributes("v")) + " C=" + d(C.getattributes("v"));'),
    # --- 3) 类级（null 键）属性不继承 ---
    ("cl_inherit", 'local A = class { }; A.setattributes(null, { lvl = "A" }); local B = class extends A { }; return "A=" + d(A.getattributes(null)) + " B=" + d(B.getattributes(null));'),
    ("cl_child", 'local A = class { }; local B = class extends A { }; B.setattributes(null, { lvl = "B" }); return "A=" + d(A.getattributes(null)) + " B=" + d(B.getattributes(null));'),
    # --- 4) 属性表共享引用 ---
    ("shared_mutate", 'local C = class { </ a = { n = 1 } /> v = 1; }; C.getattributes("v").a.n = 42; return C.getattributes("v").a.n;'),
    ("shared_newkey", 'local C = class { </ a = { n = 1 } /> v = 1; }; local x = C.getattributes("v"); x.same = "yes"; return d(x);'),
    ("shared_same_view", 'local C = class { </ a = { n = 1 } /> v = 1; }; local x = C.getattributes("v"); local y = C.getattributes("v"); x.a.n = 5; return x.a.n + "/" + y.a.n;'),
    # --- 5) newmember 的形式 ---
    ("nm4", 'local C = class { v = 1; }; C.newmember("w", 2, { a = 8 }, false); return d(C.getattributes("w")) + "/" + C.w;'),
    ("nm4_static", 'local C = class { v = 1; }; C.newmember("sw", 3, { a = 6 }, true); return d(C.getattributes("sw")) + "/" + C.sw + "/" + typeof C.rawget("sw");'),
    ("nm3", 'local C = class { v = 1; }; C.newmember("w3", 4, { a = 7 }); return d(C.getattributes("w3"));'),
    ("nm_noattrs", 'local C = class { v = 1; }; C.newmember("w4", 5); return d(C.getattributes("w4"));'),
    # --- 6) clone / 实例边界 ---
    ("clone_class", 'local A = class { </ a = 1 /> v = 1; }; return clone A;'),
    ("inst_set", 'local C = class { </ a = 1 /> v = 1; }; local o = C(); return d(o.setattributes("v", { a = 3 }));'),
    ("inst_get_null", 'local C = class { </ a = 1 /> v = 1; }; local o = C(); return d(o.getattributes(null));'),
    # --- 7) 参数个数 ---
    ("ga_zero", 'local C = class { v = 1; }; return C.getattributes();'),
    ("ga_two", 'local C = class { v = 1; }; return C.getattributes("v", 2);'),
    ("sa_one", 'local C = class { v = 1; }; return C.setattributes("v");'),
    ("sa_three", 'local C = class { v = 1; }; return C.setattributes("v", {}, 3);'),
    # --- 8) 构造函数 / 元方法身上的属性 ---
    ("ctor_attr", 'local C = class { </ a = 1 /> constructor() { } }; return d(C.getattributes("constructor"));'),
    ("meta_attr", 'local C = class { </ a = 1 /> function _tostring() { return "C"; } }; return d(C.getattributes("_tostring")) + "/" + ("" + C());'),
    # --- 9) 属性不是成员：删成员/加成员后属性去哪 ---
    ("delete_member", 'local C = class { </ a = 1 /> v = 1; }; local r = delete C.v; return "" + r + " attr=" + d(C.getattributes("v")) + " member=" + d(C.rawget("v"));'),
    ("rawset_new", 'local C = class { </ a = 1 /> v = 1; }; C.rawset("w", 2); return "w=" + d(C.getattributes("w")) + " v=" + d(C.getattributes("v"));'),
    # --- 10) 属性值是闭包/类 ---
    ("closure_value", 'local C = class { </ a = function() { return 1; } /> v = 1; }; local a = C.getattributes("v").a; return typeof a + "/" + a();'),
    ("class_value", 'local Inner = class { q = 1 }; local C = class { v = 1; }; C.setattributes("v", Inner); return typeof C.getattributes("v");'),
    # --- 11) static 与属性、实例遮蔽 ---
    ("static_shadow", 'local C = class { </ a = 1 /> static v = 10; }; local o = C(); return "" + C.v + "/" + o.v + "/" + d(C.getattributes("v"));'),
]

lines = [
    "// 自动生成：类属性时序/共享/继承边界矩阵（tools/gen_attrtiming.py）",
    "//",
    "// 与 t_attrmap.nut（形态矩阵）互补：这里测**动力学**——实例化后改属性、",
    "// 继承后改父类、属性表共享引用、newmember 4 参、clone 边界、参数个数。",
    "// 全部用例已在参考实现上实测，两侧逐字节一致。",
    "function d(x) {",
    "    local t = typeof x;",
    "    if (t == \"table\") { local o = \"{\"; local f = true; foreach (k, v in x) { if (!f) o += \",\"; f = false; o += k + \"=\" + v; } return o + \"}\"; }",
    "    return \"\" + x;",
    "}",
    "function t(tag, f) {",
    "    try { local v = f(); print(tag + \"=\" + d(v) + \"|\" + typeof v + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    "",
]
for name, body in CASES:
    lines.append('t("%s", function() { %s });' % (name, body))

ALL = [l for l in lines if l.startswith('t("')]
if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k][:110]))
    sys.exit(0)

lines += ['print("ATTRTIMING-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

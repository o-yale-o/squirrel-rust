# -*- coding: utf-8 -*-
"""生成"委托链 + 迭代期变更"矩阵语料 tests-corpus/t_delegiter.nut。

为什么单开一份：`t_delmatrix.nut` 测的是**方法 × 实参类型**，`t_meta3.nut` 测的是
**单个元方法**的行为——但"**链的动力学**"（深度、循环、跨类型挂链、链上 raw*）
和"**迭代动力学**"（foreach 期间增/删/缩/清容器）这两族，现有生成器结构性地
碰不到。本轮是**验证型**：探针 40 + 20 条两侧逐字节一致，语料把空白钉住。

实测钉住的语义（都对着参考实现验过）：
1. **循环委托**在**每一种**操作上报同一个错：`delegate cycle`（get/set/newslot/
   delete/in/rawin/len/foreach/tostring/typeof/clone/getdelegate 全部如此）。
   `setdelegate(自身)` 也是。
2. `setdelegate` 只接受 `null|table`（类/实例/数组都报参数类型错）；实例和数组
   **没有** setdelegate 这个方法。
3. `rawget/rawin/rawset` **不穿链**；`<- newslot` 没有元方法时挂到**接收者自身**。
4. `foreach`/`len` 只看**自己的槽**，不看委托链上的（`foreach_chain`=1）。
5. 迭代期变更（参考实现居然全程不崩，两侧结果逐字节一致）：
   - 表：迭代中删当前键 → 计数 3、剩 1；删别的键 → 后面的键被跳过（`acb`）；
     边删边插 → 4/3；clear 之后停止（`a/0`）。
   - 数组：append 会**延长**迭代（12/12）；`remove(0)` 让迭代提前结束（2/2）；
     resize 缩短也让迭代提前停（`123/2`）；resize 变长按**旧长度**走（10/10）。
   - break/continue/return/throw 在 foreach 里语义正常。

安全约束：本矩阵的所有用例都已在参考实现上**实测跑通**（不崩不挂，见
`tmp-c2/foreach_mut.py` 的逐条隔离结果）。没有涉及超大分配。

用法：
    python tools/gen_delegiter.py
    python tools/gen_delegiter.py --list 3 8   # 查用例下标（差分失败时用）
"""
import os
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_delegiter.nut")

CASES = [
    # --- 委托链深度与遮蔽 ---
    ("d1", 'local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.x;'),
    ("d2", 'local a = { x = 1 }; local b = {}; local c = {}; b.setdelegate(a); c.setdelegate(b); return c.x;'),
    ("d6", 'local root = { x = "deep" }; local prev = root; for (local i = 0; i < 5; i++) { local n = {}; n.setdelegate(prev); prev = n; } return prev.x;'),
    ("d20", 'local root = { x = "d20" }; local prev = root; for (local i = 0; i < 19; i++) { local n = {}; n.setdelegate(prev); prev = n; } return prev.x;'),
    ("shadow", 'local a = { x = "a" }; local b = { x = "b" }; b.setdelegate(a); return b.x;'),
    ("mid_shadow", 'local a = { x = "a" }; local b = {}; local c = {}; b.setdelegate(a); c.setdelegate(b); b.x = "b"; return c.x;'),
    # --- 跨类型挂链 ---
    ("t2c", 'class K { y = 7 } local o = {}; o.setdelegate(K); return o.y;'),
    ("t2i", 'local o = {}; o.setdelegate(K_INST); return o.y;'),
    ("c2t", 'local tt = { z = 9 }; class K2 { }; local k = K2(); k.setdelegate(tt); return k.z;'),
    ("t2a", 'local o = {}; o.setdelegate([1, 2]); return "ok";'),
    ("a2t", 'local a = [1, 2]; a.setdelegate({ q = 1 }); return "ok";'),
    # --- 循环委托 × 各种操作 ---
    ("cyc2", 'CYC2; return a.zzz;'),
    ("cyc1", 'CYC1; return a.zzz;'),
    ("cyc3", 'CYC3; return a.zzz;'),
    ("cyc_get", 'local a = { existing = 1 }; local b = {}; a.setdelegate(b); b.setdelegate(a); return a.existing;'),
    ("cyc_set", 'CYC2; a.foo <- 1; return a.foo;'),
    ("cyc_del", 'CYC2; return delete a.foo;'),
    ("cyc_in", 'CYC2; return "foo" in a;'),
    ("cyc_rawin", 'CYC2; return a.rawin("foo");'),
    ("cyc_len", 'local a = { p = 1 }; local b = {}; a.setdelegate(b); b.setdelegate(a); return a.len();'),
    ("cyc_foreach", 'local a = { p = 1 }; local b = {}; a.setdelegate(b); b.setdelegate(a); local n = 0; foreach (k, v in a) n++; return n;'),
    ("cyc_tostr", 'CYC2; return "" + a;'),
    ("cyc_typeof", 'CYC2; return typeof a;'),
    ("cyc_clone", 'local a = { p = 1 }; local b = {}; a.setdelegate(b); b.setdelegate(a); return clone a;'),
    ("cyc_getdelegate", 'CYC2; return typeof a.getdelegate();'),
    # --- setdelegate 本身 ---
    ("sd_ret", 'local a = {}; local b = {}; return typeof a.setdelegate(b);'),
    ("sd_null", 'local a = {}; a.setdelegate({ q = 1 }); a.setdelegate(null); return a.q;'),
    ("sd_self", 'local a = {}; a.setdelegate({ q = 1 }); return a.setdelegate(a).q;'),
    ("sd_cyc_assign", 'local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a); return "no-cycle-error";'),
    # --- raw* 不穿链 / newslot 挂自身 ---
    ("raw_get", 'local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.rawget("x");'),
    ("raw_in", 'local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.rawin("x");'),
    ("raw_set", 'local a = { x = 1 }; local b = {}; b.setdelegate(a); b.rawset("x", 9); return a.x + "/" + b.x;'),
    ("mm_before_chain", 'local bt = { x = "chain" }; local d = { _get = function(k) { return "mm"; } }; local o = {}; o.setdelegate(d); d.setdelegate(bt); return o.x;'),
    ("mm_after_miss", 'local bt = { }; local d = { _get = function(k) { return "mm"; } }; local o = {}; o.setdelegate(d); d.setdelegate(bt); return o.zzz;'),
    ("ns_chain", 'local a = { }; local b = {}; b.setdelegate(a); b.q <- 5; return a.rawin("q") + "/" + b.rawin("q");'),
    ("del_chain", 'local a = { q = 1 }; local b = {}; b.setdelegate(a); return delete b.q;'),
    ("del_chain_mm", 'local a = { q = 1 }; local d = { _delslot = function(k) { return "mm"; } }; local b = {}; b.setdelegate(d); d.setdelegate(a); return delete b.q;'),
    ("foreach_chain", 'local a = { x = 1, y = 2 }; local b = { z = 3 }; b.setdelegate(a); local n = 0; foreach (k, v in b) n++; return n;'),
    ("keys_chain", 'local a = { x = 1 }; local b = {}; b.setdelegate(a); return b.len();'),
    # --- 迭代期变更（表）---
    ("tbl_ins_self", 'local tb = { a = 1, b = 2, c = 3 }; local n = 0; foreach (k, v in tb) { n++; if (n == 1) tb.newk <- 9; } return n + "/" + tb.len();'),
    ("tbl_ins_many", 'local tb = { a = 1, b = 2, c = 3 }; local n = 0; foreach (k, v in tb) { n++; tb["k" + n] <- n; if (n > 50) break; } return n + "/" + tb.len();'),
    ("tbl_del_cur", 'local tb = { a = 1, b = 2, c = 3, d = 4 }; local n = 0; foreach (k, v in tb) { n++; delete tb[k]; } return n + "/" + tb.len();'),
    ("tbl_del_other", 'local tb = { a = 1, b = 2, c = 3, d = 4 }; local seen = ""; foreach (k, v in tb) { seen += k; if (k == "a") delete tb.d; } return seen + "/" + tb.len();'),
    ("tbl_set_cur", 'local tb = { a = 1, b = 2, c = 3 }; local n = 0; foreach (k, v in tb) { tb[k] = v * 10; n++; } return n + "/" + tb.a + tb.b + tb.c;'),
    ("tbl_clear", 'local tb = { a = 1, b = 2, c = 3 }; local seen = ""; foreach (k, v in tb) { seen += k; tb.clear(); } return seen + "/" + tb.len();'),
    ("tbl_del_then_ins", 'local tb = { a = 1, b = 2, c = 3 }; local n = 0; foreach (k, v in tb) { n++; delete tb[k]; tb[k + "2"] <- v; if (n > 30) break; } return n + "/" + tb.len();'),
    # --- 迭代期变更（数组）---
    ("arr_append", 'local a = [1, 2, 3]; local n = 0; foreach (v in a) { n++; if (n < 10) a.append(n); } return n + "/" + a.len();'),
    ("arr_remove_cur", 'local a = [1, 2, 3, 4]; local n = 0; foreach (v in a) { n++; a.remove(0); } return n + "/" + a.len();'),
    ("arr_set_cur", 'local a = [1, 2, 3]; local n = 0; foreach (v in a) { a[0] = 9; n++; } return n + "/" + a[0] + a[1] + a[2];'),
    ("arr_resize", 'local a = [1, 2, 3]; local n = 0; foreach (v in a) { n++; a.resize(10); } return n + "/" + a.len();'),
    ("arr_shrink", 'local a = [1, 2, 3, 4, 5]; local seen = ""; foreach (v in a) { seen += v; a.resize(a.len() - 1); } return seen + "/" + a.len();'),
    ("nested_same_tbl", 'local tb = { a = 1, b = 2 }; local n = 0; foreach (k1, v1 in tb) foreach (k2, v2 in tb) n++; return "" + n;'),
    # --- 控制流与其它可迭代物 ---
    ("break_foreach", 'local s = ""; foreach (v in [1, 2, 3]) { if (v == 2) break; s += v; } return s;'),
    ("continue_foreach", 'local s = ""; foreach (v in [1, 2, 3]) { if (v == 2) continue; s += v; } return s;'),
    ("return_foreach", 'foreach (v in [1, 2, 3]) { if (v == 2) return v; } return 0;'),
    ("throw_foreach", 'foreach (v in [1, 2, 3]) { if (v == 2) throw "mid"; } return 0;'),
    ("str_foreach", 'local s = ""; foreach (c in "abc") s += c + ","; return s;'),
    ("blob_foreach", 'local b = blob(3); local n = 0; foreach (v in b) n++; return "" + n;'),
    ("gen_foreach", 'local g = function() { yield 1; yield 2; }; local n = 0; foreach (v in g()) n++; return "" + n;'),
]

# K_INST 是语料里的公共夹具（放进全局，避免每条用例重复定义）。
# **注意**：不能在顶层造循环委托——检测发生在 setdelegate **赋值时**（实测
# `b.setdelegate(a)` 在 a 已挂 b 时直接抛 `delegate cycle`，两侧一致），
# 所以所有环用例都在 t() 的 try 里自己造。
FIXTURES = r'''
class K_INST_H { y = 7 }
local K_INST = K_INST_H();
'''

REPL = {
    "CYC2;": "local a = {}; local b = {}; a.setdelegate(b); b.setdelegate(a);",
    "CYC1;": "local a = {}; a.setdelegate(a);",
    "CYC3;": "local a = {}; local b = {}; local c = {}; a.setdelegate(b); b.setdelegate(c); c.setdelegate(a);",
    "K_INST": "K_INST",
}

lines = [
    "// 自动生成：委托链 + 迭代期变更矩阵（tools/gen_delegiter.py）",
    "//",
    "// 两族都是现有生成器结构性地碰不到的：链的动力学（深度/循环/跨类型/raw*）",
    "// 与迭代的动力学（foreach 期间增/删/缩/清）。全部用例已在参考实现上实测",
    "// 跑通（tmp-c2/foreach_mut.py 逐条隔离），两侧逐字节一致。",
    "function t(tag, f) {",
    "    try { local v = f(); local o = typeof v; print(tag + \"=\" + (o == \"string\" ? v : \"\" + v) + \"|\" + o + \"\\n\"); }",
    "    catch (e) { print(tag + \"!\" + e + \"\\n\"); }",
    "}",
    FIXTURES,
    "",
]
for name, body in CASES:
    for k, v in REPL.items():
        if body.startswith(k):
            body = body.replace(k, v + " ", 1)
            break
    lines.append('t("%s", function() { %s });' % (name, body))

ALL = [l for l in lines if l.startswith('t("')]
if len(sys.argv) >= 2 and sys.argv[1] == "--list":
    lo = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    hi = int(sys.argv[3]) if len(sys.argv) > 3 else lo
    for k in range(lo, min(hi, len(ALL) - 1) + 1):
        sys.stdout.write("%4d  %s\n" % (k, ALL[k][:110]))
    sys.exit(0)

lines += ['print("DELEGITER-DONE\\n");']
with open(OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(lines) + "\n")
print("已写出 %s：%d 条用例" % (OUT, len(ALL)))

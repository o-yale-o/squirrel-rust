# -*- coding: utf-8 -*-
"""生成 stream/blob「**位置状态 × API**」矩阵语料 tests-corpus/t_streammap.nut。

动机：上一轮的随机模糊测试在 stream/blob 上抓到两个真实差异
（`blob.swap2/4` 多推进了读写位置、文件流负绝对偏移被夹成 0），
共同点是**差异藏在"调用之后的读写位置/长度"里**，而原有语料每次都 `seek(0)` 重置，
结构性地看不到状态维度的分叉。所以这里把"状态"当作矩阵的第二维：

- 状态：新建/空/1 字节/3 字节（奇数长）/已写入（pos=len）/pos=1/pos=len/
  seek 失败（pos 不变）/swap2 之后/resize 变大/resize 变小/写超长；
- 观测：**调用返回值 + 调用后的 tell/len/eos**（三者都要，否则状态差异看不见）；
- 出错也照样观测（catch 后仍然打印状态）——参考实现的错误路径同样会改状态。

覆盖的 API：`tell/len/eos/seek`（7 偏移 × 3 origin）、`readn`（8 种格式）、
`writen`（含 int→float 与 float→int 的隐式转换）、`readblob`（含越界）、
`writeblob`、`resize`、`swap2/swap4`、`flush`、`clone`。
file 侧同一套（去掉 blob 专有的 resize/swap）。

安全约束：参考实现的 `resize(<负数>)`、`readblob(<负数>)`、非法文件模式是它自己的 UB
（实测崩溃），这里一律只用非负参数与合法模式。

用法：
    python tools/gen_streammatrix.py
"""
import os

ROOT = r"D:\OpenSource\squirrel-rust"
OUT = os.path.join(ROOT, "tests-corpus", "t_streammap.nut")

# 状态构造器：blob / file 各一套（file 用同一个临时文件反复重建）
BLOB_STATES = [
    ("fresh4", 'blob(4)'),
    ("zero", 'blob(0)'),
    ("one", 'blob(1)'),
    ("three", 'blob(3)'),
    ("written", 'bz_written()'),
    ("pos1", 'bz_at(1)'),
    ("posend", 'bz_at(4)'),
    ("seekfail", 'bz_seekfail()'),
    ("swapped", 'bz_swapped()'),
    ("resized_up", 'bz_resized_up()'),
    ("resized_down", 'bz_resized_down()'),
    ("oversize", 'bz_oversize()'),
]
FILE_STATES = [
    ("fresh4", 'fz_fresh(4)'),
    ("zero", 'fz_fresh(0)'),
    ("one", 'fz_fresh(1)'),
    ("three", 'fz_fresh(3)'),
    ("written", 'fz_written()'),
    ("pos1", 'fz_at(1)'),
    ("posend", 'fz_at(4)'),
    ("seekfail", 'fz_seekfail()'),
    ("resized_up", 'fz_resized_up()'),
    ("resized_down", 'fz_resized_down()'),
]

# (标签, 调用表达式模板 {o} = 对象)
SEEKS = []
for off in (-2, -1, 0, 1, 4, 5, 99):
    SEEKS.append(("b_%s" % ("m" + str(-off) if off < 0 else "p" + str(off)), "seek(%d, 'b')" % off))
for off in (-2, -1, 0, 1, 2):
    SEEKS.append(("c_%s" % ("m" + str(-off) if off < 0 else "p" + str(off)), "seek(%d, 'c')" % off))
for off in (-5, -1, 0, 1, 2):
    SEEKS.append(("e_%s" % ("m" + str(-off) if off < 0 else "p" + str(off)), "seek(%d, 'e')" % off))

READN = [(c, "readn('%s')" % c) for c in "l i s w c b f d".split()]
WRITEN = [
    ("b65", "writen(65, 'b')"),
    ("w_1234", "writen(0x1234, 'w')"),
    ("i_neg", "writen(-1, 'i')"),
    ("l_big", "writen(0x0102030405060708, 'l')"),
    ("s_neg", "writen(-2, 's')"),
    ("c_65", "writen(65, 'c')"),
    ("f_15", "writen(1.5, 'f')"),
    ("d_15", "writen(1.5, 'd')"),
    ("int_to_f", "writen(3, 'f')"),
    ("float_to_i", "writen(2.9, 'i')"),
    ("bad_fmt", "writen(1, 'z')"),
]
READBLOB = [("r0", "readblob(0)"), ("r1", "readblob(1)"), ("r2", "readblob(2)"),
            ("r4", "readblob(4)"), ("r5", "readblob(5)"), ("r99", "readblob(99)")]
BLOB_ONLY = [
    ("resize0", "resize(0)"), ("resize1", "resize(1)"), ("resize2", "resize(2)"),
    ("resize5", "resize(5)"), ("resize9", "resize(9)"),
    ("swap2", "swap2()"), ("swap4", "swap4()"), ("flush", "flush()"),
    ("cloned", None),   # 特殊：clone 之后看副本状态
]
FILE_ONLY = [("flush", "flush()"), ("cloned", None), ("close", "close()")]
COMMON = [("tell", "tell()"), ("len", "len()"), ("eos", "eos()"),
          ("writeblob2", "writeblob(src)")]

PREAMBLE = r"""
// 自动生成：stream/blob 位置状态 × API 矩阵（tools/gen_streammatrix.py）
// 观测口径：调用返回值 + 调用后的 tell/len/eos（状态差异就藏在这里）
function show(r, d = 0) {
    if (d == null) d = 0;
    if (d > 2) return "...";
    local t = typeof r;
    if (t == "array") { local s = "["; for (local i = 0; i < r.len(); i++) { if (i > 0) s += ","; s += show(r[i], d + 1); } return s + "]"; }
    if (t == "table") { local s = "{"; foreach (k, v in r) s += show(k, d + 1) + ":" + show(v, d + 1) + ","; return s + "}"; }
    if (t == "string") return "\"" + r + "\"";
    if (t == "float") return format("%.17g", r);
    return "" + r;
}
function obs(o) {
    try { return "tell=" + o.tell() + "/len=" + o.len() + "/eos=" + show(o.eos()); }
    catch (e) { return "obsE:" + e; }
}
function t(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!outer:" + e + "\n"); }
}
// ---- blob 状态构造器
function bz_new(n) { return blob(n); }
function bz_written() { local o = blob(4); o.writen(0x01020304, 'i'); return o; }
function bz_at(p) { local o = bz_written(); o.seek(p, 'b'); return o; }
function bz_seekfail() { local o = bz_written(); o.seek(99, 'b'); return o; }
function bz_swapped() { local o = bz_written(); o.seek(0, 'b'); o.swap2(); return o; }
function bz_resized_up() { local o = blob(2); o.writen(0x1234, 'w'); o.resize(5); return o; }
function bz_resized_down() { local o = bz_written(); o.resize(1); return o; }
function bz_oversize() { local o = blob(2); o.writen(0x01020304, 'i'); return o; }
// ---- file 状态构造器（同一个临时文件反复重建）
local FZ_PATH = "streammap_tmp.bin";
function fz_make(n) {
    local o = file(FZ_PATH, "wb+");
    if (n > 0) { local b = blob(n); b.writen(0x11, 'b'); o.writeblob(b); }
    o.flush();
    o.seek(0, 'b');
    return o;
}
function fz_fresh(n) { return fz_make(n); }
// 注意：C 标准规定"更新流"（wb+）**写入之后不能直接读取**，中间必须有 fflush 或
// 文件定位调用（C11 7.21.5.3）。少了这一步就是 UB —— 实测参考实现会给出"读越界却把
// 文件撑大"的怪行为（4 字节文件 readn('l') 后 len 变 12）。所以凡是写完就停的状态，
// 都补一次 seek(当前位置)，把它变成有定义的序列（这也是手写语料总先 seek(0) 的原因）。
function fz_written() { local o = fz_make(4); o.writen(0x01020304, 'i'); o.seek(o.tell(), 'b'); return o; }
function fz_at(p) { local o = fz_written(); o.seek(p, 'b'); return o; }
function fz_seekfail() { local o = fz_written(); o.seek(99, 'b'); return o; }
function fz_resized_up() { local o = fz_make(2); o.writen(0x1234, 'w'); o.seek(o.tell(), 'b'); return o; }
function fz_resized_down() { local o = fz_make(1); return o; }
"""


def cases_for(prefix, states, apis):
    out = []
    for sname, sexpr in states:
        for aname, call in apis:
            tag = "%s_%s_%s" % (prefix, sname, aname)
            if call is None:
                body = ("local c = clone o; "
                        "return \"orig[\" + obs(o) + \"] copy[\" + obs(c) + \"]\";")
            else:
                body = ("local r = null; local err = null; "
                        "try { r = o.%s; } catch (e) { err = \"\" + e; } "
                        "return show(r) + \"|err=\" + show(err) + \"|\" + obs(o);" % call)
            # 注意：这里的引号是拼接出来的，别在 Python 字符串里写 ''b''（那会生成坏代码，
            # 而且两侧会"一致地"报同一个错误 —— 整个矩阵变成假绿，抽查才发现）
            line = ('t("{tag}", function() {{ local o = {sexpr}; '
                    'local src = blob(2); src.writen(0x22, {q}b{q}); {body} }});'
                    .format(tag=tag, sexpr=sexpr, q="'", body=body))
            out.append(line)
    return out


def main():
    lines = [PREAMBLE]
    blob_apis = COMMON + SEEKS + READN + WRITEN + READBLOB + [a for a in BLOB_ONLY if a[0] != "cloned"]
    file_apis = COMMON + SEEKS + READN + WRITEN + READBLOB + [a for a in FILE_ONLY if a[0] != "cloned"]
    lines += cases_for("bb", BLOB_STATES, blob_apis)
    lines += cases_for("ff", FILE_STATES, file_apis)
    # clone 单独一组（clone 后看副本与原件状态）；file 的 clone 会抛
    # "this object cannot be cloned"，所以 clone 本身也要放进 try 里当观测
    for prefix, states in (("bb", BLOB_STATES), ("ff", FILE_STATES)):
        for sname, sexpr in states:
            lines.append(
                't("%s_%s_clone", function() {{ local o = %s; local c = null; local err = null; '
                'try {{ c = clone o; }} catch (e) {{ err = "" + e; }} '
                'local extra = (err == null) ? (" copy[" + obs(c) + "]") : (" cloneErr=" + err); '
                'return "orig[" + obs(o) + "]" + extra; }});' % (prefix, sname, sexpr))
    # 收尾：矩阵会反复重建同一个临时文件，跑完必须删掉（曾经把它提交进仓库过一次）
    lines.append('remove("streammap_tmp.bin");')
    lines.append('print("STREAMMAP-DONE\\n");')
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")
    n = sum(1 for l in lines if l.startswith("t("))
    print("已写出 %s：%d 条用例（blob %d 状态 / file %d 状态）"
          % (OUT, n, len(BLOB_STATES), len(FILE_STATES)))

    # ---- 生成器自检：用例体必须自己把错误吃掉（t() 外层的 !outer: 说明用例体坏了）。
    # 只靠差分测试是发现不了的：那种情况下两侧会"一致地"报同一个错误 → 假绿。
    import subprocess
    exe = os.path.join(ROOT, "target", "release", "sq.exe")
    if os.path.exists(exe):
        p = subprocess.run([exe, OUT], capture_output=True, cwd=ROOT, timeout=180)
        text = p.stdout.decode("utf8", "replace")
        bad = [l for l in text.splitlines() if "!outer:" in l]
        done = "STREAMMAP-DONE" in text
        print("自检：跑到 DONE=%s，!outer 行数=%d" % (done, len(bad)))
        for l in bad[:5]:
            print("   " + l[:140])
        if bad or not done:
            print("自检失败：用例体自己抛错了，矩阵没有真正测到东西")
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

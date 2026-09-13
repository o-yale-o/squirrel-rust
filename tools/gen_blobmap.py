# -*- coding: utf-8 -*-
"""生成 tests-corpus/t_blobmap.nut：blob / stream 二进制语义矩阵。

= tmp-c2/blobprobe.nut 的全部用例（56 条，两侧已逐字节一致）
  + 生成式扩充：writen/readn 的**格式 × 值**往返矩阵、resize/seek 组合。

注意：非法文件模式（"", "z", "R", "rbb", "trash"…）**不进语料** ——
参考实现会落到 UCRT 的 invalid parameter handler 直接崩（0xC0000409，见 STATUS 第四节）；
本实现给可捕获的 `cannot open file`，由 stream_semantics.rs 守门。

用法：python tools/gen_blobmap.py
"""
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = os.path.join(ROOT, "tmp-c2", "blobprobe.nut")
OUT = os.path.join(ROOT, "tests-corpus", "t_blobmap.nut")

L = []
A = L.append
A("// 自动生成：python tools/gen_blobmap.py —— 请勿手改")
A("// blob / stream 二进制语义矩阵")
A("")

src = open(PROBE, encoding="utf-8").read()
body = src[: src.index('print("BLOBPROBE-DONE')].rstrip()

# ---------------------------------------------------------------- 生成式扩充
gen = []
G = gen.append
G("")
G("// ---- 扩充：writen/readn 格式 × 值 往返（写入长度 + 读回值 + 游标/长度）")
INT_VALUES = [("z", "0"), ("one", "1"), ("mone", "-1"), ("258", "258"),
              ("neg258", "-258"), ("big16", "70000"), ("b31", "2147483647"),
              ("nb31", "-2147483648"), ("max", "9223372036854775807"),
              ("min", "(-9223372036854775807 - 1)")]
for fmt in ["l", "i", "s", "w", "c", "b"]:
    for vn, v in INT_VALUES:
        G('s("rt_%s_%s", function() { local b = blob(0); b.writen(%s, \'%s\'); local n = b.len();'
          ' b.seek(0); return n + ":" + b.readn(\'%s\') + ":" + b.tell() + "/" + b.eos(); });'
          % (fmt, vn, v, fmt, fmt))
FLOAT_VALUES = [("f0", "0.0"), ("fnz", "-0.0"), ("f15", "1.5"), ("fm15", "-1.5"),
                ("fpi", "3.14159"), ("fbig", "1.0e30"), ("fsmall", "1.0e-30")]
for fmt in ["f", "d"]:
    for vn, v in FLOAT_VALUES:
        G('s("rt_%s_%s", function() { local b = blob(0); b.writen(%s, \'%s\'); local n = b.len();'
          ' b.seek(0); local r = b.readn(\'%s\'); return n + ":" + r; });' % (fmt, vn, v, fmt, fmt))
G("")
G("// ---- 扩充：多值混写 + 定位读（先写后按偏移读）")
G('s("mixed_offsets", function() {')
G("    local b = blob(0);")
G("    b.writen(1, 'b'); b.writen(258, 's'); b.writen(1.5, 'f'); b.writen(7, 'l'); b.writen(9, 'b');")
G("    local total = b.len();")
G("    local out = total + \":\";")
G("    b.seek(0); out += b.readn('b') + \",\";")
G("    b.seek(1); out += b.readn('s') + \",\";")
G("    b.seek(3); out += b.readn('f') + \",\";")
G("    b.seek(7); out += b.readn('l') + \",\";")
G("    b.seek(-1, 2); out += b.readn('b') + \",\";")
G("    return out + b.eos() + \"/\" + b.tell();")
G("});")
G('s("resize_keep", function() { local b = blob(0); b.writen(1, \'b\'); b.writen(2, \'b\'); b.resize(5); return b.len() + ":" + hexdump(b); });')
G('s("resize_shrink_seek", function() { local b = blob(0); b.writen(1, \'b\'); b.writen(2, \'b\'); b.writen(3, \'b\'); b.seek(3); b.resize(1); return b.len() + ":" + b.tell(); });')
G('s("seek_roundtrip_reads", function() {')
G("    local b = blob(0);")
G("    for (local i = 0; i < 8; i++) b.writen(i, 'b');")
G("    local out = \"\";")
G("    for (local i = 7; i >= 0; i--) { b.seek(i); out += b.readn('b'); }")
G("    return out;")
G("});")
G('s("writeblob_chain", function() { local b = blob(0); local s1 = blob(\"ab\"); local s2 = blob(\"cd\");'
  ' b.writeblob(s1); b.writeblob(s2); return hexdump(b) + "/" + b.tell(); });')
G('s("readblob_partial", function() { local b = blob("abcdef"); b.seek(4); local r = b.readblob(10); return hexdump(r) + "/" + b.tell() + "/" + b.eos(); });')
G('s("swap2_twice", function() { local b = blob(0); b.writen(258, \'i\'); b.swap2(); b.swap2(); return hexdump(b); });')
G('s("swap4_twice", function() { local b = blob(0); b.writen(258, \'i\'); b.swap4(); b.swap4(); return hexdump(b); });')
G('s("nested_blob_stream", function() { local b = blob(0); local inner = blob("xy"); b.writen(inner.len(), \'b\'); b.writeblob(inner); return hexdump(b); });')
G('s("blob_len_after_writen", function() { local b = blob(0); b.writen(0, \'l\'); return b.len(); });')


def hexdump_helper():
    return [
        "function hexdump(b) {",
        '    local out = b.len() + ":";',
        '    for (local i = 0; i < b.len(); i++) { out += (b[i] < 16 ? "0" : "") + format("%x", b[i]); }',
        "    return out;",
        "}",
    ]


out = ["\n".join(L) + "\n" + body + "\n" + "\n".join(hexdump_helper()) + "\n" + "\n".join(gen) + "\n",
       'print("BLOBMAP-DONE\\n");', ""]
open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(out))
n = sum(1 for l in open(OUT, encoding="utf-8") if l.startswith("s("))
print("wrote %s (%d 用例)" % (OUT, n))

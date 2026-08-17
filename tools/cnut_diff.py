# -*- coding: utf-8 -*-
"""对比两个 .cnut 文件的结构差异（基于 parse_cnut 的解析逻辑）。"""
import struct
import sys

OPS = ["LINE","LOAD","LOADINT","LOADFLOAT","DLOAD","TAILCALL","CALL","PREPCALL","PREPCALLK",
"GETK","MOVE","NEWSLOT","DELETE","SET","GET","EQ","NE","ADD","SUB","MUL","DIV","MOD","BITW",
"RETURN","LOADNULLS","LOADROOT","LOADBOOL","DMOVE","JMP","JCMP","JZ","SETOUTER","GETOUTER",
"NEWOBJ","APPENDARRAY","COMPARITH","INC","INCL","PINC","PINCL","CMP","EXISTS","INSTANCEOF",
"AND","OR","NEG","NOT","BWNOT","CLOSURE","YIELD","RESUME","FOREACH","POSTFOREACH","CLONE",
"TYPEOF","PUSHTRAP","POPTRAP","THROW","NEWSLOTA","GETBASE","CLOSE"]

class R:
    def __init__(self, d):
        self.d = d; self.o = 0
    def u16(self):
        v, = struct.unpack_from('<H', self.d, self.o); self.o += 2; return v
    def u32(self):
        v, = struct.unpack_from('<I', self.d, self.o); self.o += 4; return v
    def i64(self):
        v, = struct.unpack_from('<q', self.d, self.o); self.o += 8; return v
    def u8(self):
        v = self.d[self.o]; self.o += 1; return v
    def raw(self, n):
        v = self.d[self.o:self.o+n]; self.o += n; return v

def obj(r):
    t = r.u32()
    if t == 0x08000010:
        ln = r.i64(); return b'"' + r.raw(ln) + b'"'
    if t == 0x08000002: return str(r.i64()).encode()
    if t == 0x08000004:
        v, = struct.unpack_from('<f', r.d, r.o); r.o += 4
        return ('{%g}' % v).encode()
    if t == 0x01000008: return (b'true' if r.i64() else b'false')
    if t == 0x01000001: return b'null'
    return b'<type %08x>' % t

def proto(r, depth, lines, prefix=""):
    r.u32()  # PART
    sn = obj(r); nm = obj(r)
    r.u32()
    counts = [r.i64() for _ in range(8)]
    (nlit, npar, nouter, nlocal, nline, ndefp, ninstr, nfunc) = counts
    ind = '  ' * depth
    lines.append('%sFUNC %s%s counts=%s' % (prefix, ind, nm, counts))
    r.u32()
    lits = [obj(r) for _ in range(nlit)]
    r.u32()
    pars = [obj(r) for _ in range(npar)]
    lines.append('%s%s  PARAMS %s' % (prefix, ind, b','.join(pars)))
    r.u32()
    for _ in range(nouter):
        r.i64(); obj(r); obj(r)
    r.u32()
    for _ in range(nlocal):
        nm2 = obj(r); pos = r.i64(); so = r.i64(); eo = r.i64()
        lines.append('%s%s  LOCAL %s pos=%d [%d,%d]' % (prefix, ind, nm2, pos, so, eo))
    r.u32()
    for _ in range(nline):
        ln = r.i64(); op = r.i64()
    r.u32()
    for _ in range(ndefp):
        r.i64()
    r.u32()
    for i in range(ninstr):
        a1, op, a0, a2, a3 = struct.unpack_from('<iBBBB', r.d, r.o); r.o += 8
        lit = b''
        if op in (1, 4, 8, 9) and 0 <= a1 < len(lits): lit = b' ; ' + lits[a1]
        lines.append('%s%s  [%03d] %-12s %d %d %d %d%s' % (prefix, ind, i, OPS[op], a0, a1, a2, a3, lit))
    r.u32()
    for _ in range(nfunc):
        proto(r, depth + 1, lines, prefix)
    ss = r.i64(); bg = r.u8(); vp = r.i64()
    lines.append('%s%s  stacksize=%d bgen=%d varparams=%d' % (prefix, ind, ss, bg, vp))

def dump(path, prefix):
    data = open(path, 'rb').read()
    r = R(data)
    r.u16(); r.u32(); r.u32(); r.u32(); r.u32()
    lines = []
    proto(r, 0, lines, prefix)
    return lines

a = dump(sys.argv[1], "REF ")
b = dump(sys.argv[2], "RS ")
import difflib
n = 0
for line in difflib.unified_diff(a, b, lineterm='', n=1):
    print(line)
    n += 1
    if n > 60:
        print("...(truncated)")
        break
if n == 0:
    print("STRUCTURALLY IDENTICAL（结构一致，原始字节差异在未比对字段之外）")

# -*- coding: utf-8 -*-
"""解析 Squirrel .cnut 字节码（对照 SQFunctionProto::Save 的写入顺序）。"""
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
        self.d = d
        self.o = 0
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
    if t == 0x08000010:  # OT_STRING
        ln = r.i64()
        return b'"' + r.raw(ln) + b'"'
    if t == 0x08000002:  # OT_INTEGER
        return str(r.i64()).encode()
    if t == 0x08000004:  # OT_FLOAT (f32)
        v, = struct.unpack_from('<f', r.d, r.o); r.o += 4
        return ('{%g}' % v).encode()
    if t == 0x01000008:  # OT_BOOL
        return (b'true' if r.i64() else b'false')
    if t == 0x01000001:  # OT_NULL
        return b'null'
    return b'<type %08x>' % t

def proto(r, depth, name_hint=b'?'):
    assert r.u32() == 0x50415254  # PART
    sourcename = obj(r)
    name = obj(r)
    assert r.u32() == 0x50415254
    counts = [r.i64() for _ in range(8)]
    (nlit, npar, nouter, nlocal, nline, ndefp, ninstr, nfunc) = counts
    ind = '  ' * depth
    print('%sFUNCTION [%s] src=[%s]' % (ind, name, sourcename))
    print('%s  counts: lit=%d par=%d outer=%d local=%d line=%d defp=%d instr=%d func=%d'
          % (ind, nlit, npar, nouter, nlocal, nline, ndefp, ninstr, nfunc))
    assert r.u32() == 0x50415254
    lits = [obj(r) for _ in range(nlit)]
    for i, l in enumerate(lits):
        print('%s  LIT[%d] %s' % (ind, i, l.decode('latin-1')))
    assert r.u32() == 0x50415254
    pars = [obj(r) for _ in range(npar)]
    print('%s  PARAMS %s' % (ind, b','.join(pars).decode('latin-1')))
    assert r.u32() == 0x50415254
    for _ in range(nouter):
        t = r.i64()
        src = obj(r)
        nm = obj(r)
        print('%s  OUTER %s %s %d' % (ind, nm, src, t))
    assert r.u32() == 0x50415254
    for _ in range(nlocal):
        nm = obj(r)
        pos = r.i64(); sop = r.i64(); eop = r.i64()
        print('%s  LOCAL %s pos=%d [%d,%d]' % (ind, nm, pos, sop, eop))
    assert r.u32() == 0x50415254
    for _ in range(nline):
        ln = r.i64(); op = r.i64()
        print('%s  LINE op=%d line=%d' % (ind, op, ln))
    assert r.u32() == 0x50415254
    for _ in range(ndefp):
        print('%s  DEFP %d' % (ind, r.i64()))
    assert r.u32() == 0x50415254
    for i in range(ninstr):
        arg1, op, arg0, arg2, arg3 = struct.unpack_from('<iBBBB', r.d, r.o)
        r.o += 8
        lit = b''
        if op in (1, 4, 8, 9) and 0 <= arg1 < len(lits):
            lit = b' ; ' + lits[arg1]
        print('%s  [%03d] %-12s %d %d %d %d%s' % (ind, i, OPS[op], arg0, arg1, arg2, arg3, lit.decode('latin-1')))
    assert r.u32() == 0x50415254
    for _ in range(nfunc):
        proto(r, depth + 1)
    stacksize = r.i64()
    bgen = r.u8()
    varparams = r.i64()
    print('%s  stacksize=%d bgen=%d varparams=%d' % (ind, stacksize, bgen, varparams))

def main(path):
    data = open(path, 'rb').read()
    r = R(data)
    assert r.u16() == 0xFAFA
    assert r.u32() == 0x53514952  # SQ_CLOSURESTREAM_HEAD
    print('sizeof: SQChar=%d SQInteger=%d SQFloat=%d' % (r.u32(), r.u32(), r.u32()))
    proto(r, 0)
    assert r.u32() == 0x4C4C4154  # SQ_CLOSURESTREAM_TAIL 'TAIL' little endian? 'T','A','I','L' = 0x5441494C
    print('TAIL ok, remaining %d bytes' % (len(data) - r.o))

if __name__ == '__main__':
    main(sys.argv[1])

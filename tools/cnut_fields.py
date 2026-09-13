# -*- coding: utf-8 -*-
"""按 sqobject.cpp 的 Save 布局解析 .cnut，输出「字段 => 值」的扁平列表，
用来精确定位两个引擎产出的字节码到底差在哪个字段（tools/cnut_diff.py 只在
指令层比对，遇到结构差异会解析跑飞）。

用法：
    python tools/cnut_fields.py a.cnut b.cnut      # 只打印不一致的字段
    python tools/cnut_fields.py a.cnut             # 打印全部字段
"""
import struct
import sys

PART = 0x50415254  # "TRAP"
HEAD = 0x48535153  # "SQSH"：见 SQ_CLOSURESTREAM_HEAD


class R:
    def __init__(self, d):
        self.d = d
        self.o = 0

    def take(self, n):
        b = self.d[self.o:self.o + n]
        if len(b) != n:
            raise EOFError('截断：需要 %d 字节，只剩 %d' % (n, len(b)))
        self.o += n
        return b

    def u32(self):
        return struct.unpack('<I', self.take(4))[0]

    def usize(self):
        # _SQ64 构建下 SQUnsignedInteger 是 8 字节（Save 用 sizeof(SQUnsignedInteger)）
        return struct.unpack('<Q', self.take(8))[0]

    def i64(self):
        return struct.unpack('<q', self.take(8))[0]

    def obj(self, out, tag):
        t = self.u32()
        if t == 0x08000010:  # OT_STRING
            n = self.i64()
            s = self.take(n)
            out.append((tag, 'string', s.decode('latin-1')))
        elif t == 0x01000001:  # OT_NULL
            out.append((tag, 'null', None))
        elif t == 0x05000002:  # OT_INTEGER
            out.append((tag, 'int', self.i64()))
        elif t == 0x01000008:  # OT_BOOL
            out.append((tag, 'bool', self.i64()))
        elif t == 0x05000004:  # OT_FLOAT
            out.append((tag, 'float', struct.unpack('<f', self.take(4))[0]))
        else:
            raise ValueError('未知对象类型 0x%08x @%d' % (t, self.o - 4))


def proto(r, out, path):
    t = r.u32()
    if t != PART:
        raise ValueError('缺少 PART 标记 @%d（读到 0x%08x）' % (r.o - 4, t))
    r.obj(out, path + '.sourcename')
    r.obj(out, path + '.name')
    if r.u32() != PART:
        raise ValueError('缺少 PART 标记(2) @%d' % (r.o - 4))
    names = ['nliterals', 'nparameters', 'noutervalues', 'nlocalvarinfos',
             'nlineinfos', 'ndefaultparams', 'ninstructions', 'nfunctions']
    c = {}
    for nm in names:
        c[nm] = r.i64()
        out.append((path + '.' + nm, 'count', c[nm]))
    if r.u32() != PART:
        raise ValueError('缺少 PART 标记(3) @%d' % (r.o - 4))
    for i in range(c['nliterals']):
        r.obj(out, '%s.lit[%d]' % (path, i))
    if r.u32() != PART:
        raise ValueError('缺少 PART 标记(4) @%d' % (r.o - 4))
    for i in range(c['nparameters']):
        r.obj(out, '%s.param[%d]' % (path, i))
    if r.u32() != PART:
        raise ValueError('缺少 PART 标记(5) @%d' % (r.o - 4))
    for i in range(c['noutervalues']):
        out.append(('%s.outer[%d].type' % (path, i), 'usize', r.usize()))
        r.obj(out, '%s.outer[%d].src' % (path, i))
        r.obj(out, '%s.outer[%d].name' % (path, i))
    if r.u32() != PART:
        raise ValueError('缺少 PART 标记(6) @%d' % (r.o - 4))
    for i in range(c['nlocalvarinfos']):
        r.obj(out, '%s.local[%d].name' % (path, i))
        out.append(('%s.local[%d].pos' % (path, i), 'usize', r.usize()))
        out.append(('%s.local[%d].start_op' % (path, i), 'usize', r.usize()))
        out.append(('%s.local[%d].end_op' % (path, i), 'usize', r.usize()))
    if r.u32() != PART:
        raise ValueError('缺少 PART 标记(7) @%d' % (r.o - 4))
    lines = []
    for i in range(c['nlineinfos']):
        lines.append((r.i64(), r.i64()))
    out.append((path + '.lineinfos', 'lines', lines))
    if r.u32() != PART:
        raise ValueError('缺少 PART 标记(8) @%d' % (r.o - 4))
    dps = [r.i64() for _ in range(c['ndefaultparams'])]
    out.append((path + '.defaultparams', 'ints', dps))
    if r.u32() != PART:
        raise ValueError('缺少 PART 标记(9) @%d' % (r.o - 4))
    instr = []
    for i in range(c['ninstructions']):
        a1, op, a0, a2, a3 = struct.unpack('<iBBBB', r.take(8))
        instr.append((op, a0, a1, a2, a3))
    out.append((path + '.instructions', 'instr', instr))
    if r.u32() != PART:
        raise ValueError('缺少 PART 标记(10) @%d' % (r.o - 4))
    for i in range(c['nfunctions']):
        proto(r, out, '%s.fn[%d]' % (path, i))
    out.append((path + '.stacksize', 'i64', r.i64()))
    # ↔ sqfuncproto.h：_bgenerator 是 bool（1 字节），_stacksize/_varparams 是 SQInteger
    out.append((path + '.bgenerator', 'u8', r.take(1)[0]))
    out.append((path + '.varparams', 'i64', r.i64()))


def dump(path):
    d = open(path, 'rb').read()
    r = R(d)
    out = []
    # ↔ sq_writeclosure：u16 的 SQ_BYTECODE_STREAM_TAG(0xFAFA) + 4 个 u32 标记
    out.append(('header.streamtag', 'u16', struct.unpack('<H', r.take(2))[0]))
    out.append(('header.head', 'u32', r.u32()))
    out.append(('header.charsize', 'u32', r.u32()))
    out.append(('header.intsize', 'u32', r.u32()))
    out.append(('header.floatsize', 'u32', r.u32()))
    proto(r, out, 'main')
    out.append(('file.tail', 'u32', r.u32()))
    out.append(('file.size', 'len', len(d)))
    out.append(('file.parsed', 'len', r.o))
    return out


def main(argv):
    if len(argv) == 1:
        for k, t, v in dump(argv[0]):
            print('%-40s %-6s %s' % (k, t, v if t != 'instr' else '(%d 条)' % len(v)))
        return 0
    a, b = dump(argv[0]), dump(argv[1])
    n = 0
    for i in range(max(len(a), len(b))):
        x = a[i] if i < len(a) else ('<无>', '', '')
        y = b[i] if i < len(b) else ('<无>', '', '')
        if x != y:
            n += 1
            if n <= 40:
                print('REF  %-40s %s' % (x[0], x[2]))
                print('RUST %-40s %s' % (y[0], y[2]))
    print('\n共 %d 处字段差异（REF 字段数 %d，RUST 字段数 %d）' % (n, len(a), len(b)))
    return 0 if n == 0 else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))

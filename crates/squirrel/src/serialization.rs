//! ↔ sqobject.cpp 的 Save/Load（.cnut 字节码序列化）。
//!
//! 格式与 C++ 完全一致（Win64 默认构建：SQChar=1、SQInteger=8、SQFloat=4）：
//! `u16 0xFAFA` + `u32 HEAD` + 3×`u32 sizeof` + proto 树 + `u32 TAIL`。

use std::rc::Rc;

use crate::config::*;
use crate::funcproto::{SQFunctionProto, SQLineInfo, SQLocalVarInfo, SQOuterType, SQOuterVar};
use crate::object::{ClosureRef, FuncProtoRef, Value};
use crate::opcodes::SQInstruction;
use crate::vm::VmData;

pub const SQ_CLOSURESTREAM_HEAD: u32 = (b'S' as u32) << 24 | (b'Q' as u32) << 16 | (b'I' as u32) << 8 | b'R' as u32;
pub const SQ_CLOSURESTREAM_PART: u32 = (b'P' as u32) << 24 | (b'A' as u32) << 16 | (b'R' as u32) << 8 | b'T' as u32;
pub const SQ_CLOSURESTREAM_TAIL: u32 = (b'T' as u32) << 24 | (b'A' as u32) << 16 | (b'I' as u32) << 8 | b'L' as u32;

// ---------------------------------------------------------------------------
// 写入
// ---------------------------------------------------------------------------

struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn i64(&mut self, v: SQInteger) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: SQFloat) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn bytes(&mut self, v: &[u8]) {
        self.buf.extend_from_slice(v);
    }
}

/// ↔ WriteObject（仅支持 string/integer/float/bool/null）
fn write_object(w: &mut Writer, o: &Value) -> bool {
    w.u32(o.type_());
    match o {
        Value::String(s) => {
            w.i64(s.len());
            w.bytes(&s.val);
            true
        }
        Value::Bool(b) => {
            w.i64(if *b { 1 } else { 0 });
            true
        }
        Value::Integer(i) => {
            w.i64(*i);
            true
        }
        Value::Float(f) => {
            w.f32(*f);
            true
        }
        Value::Null => true,
        _ => false,
    }
}

/// ↔ SQFunctionProto::Save
fn proto_save(w: &mut Writer, f: &SQFunctionProto) -> bool {
    w.u32(SQ_CLOSURESTREAM_PART);
    if !write_object(w, &f.sourcename) || !write_object(w, &f.name) {
        return false;
    }
    w.u32(SQ_CLOSURESTREAM_PART);
    w.i64(f.literals.len() as SQInteger);
    w.i64(f.parameters.len() as SQInteger);
    w.i64(f.outervalues.len() as SQInteger);
    w.i64(f.localvarinfos.len() as SQInteger);
    w.i64(f.lineinfos.len() as SQInteger);
    w.i64(f.defaultparams.len() as SQInteger);
    w.i64(f.instructions.len() as SQInteger);
    w.i64(f.functions.len() as SQInteger);

    w.u32(SQ_CLOSURESTREAM_PART);
    for l in &f.literals {
        if !write_object(w, l) {
            return false;
        }
    }
    w.u32(SQ_CLOSURESTREAM_PART);
    for p in &f.parameters {
        if !write_object(w, p) {
            return false;
        }
    }
    w.u32(SQ_CLOSURESTREAM_PART);
    for o in &f.outervalues {
        w.i64(o.otype as SQInteger);
        if !write_object(w, &o.src) || !write_object(w, &o.name) {
            return false;
        }
    }
    w.u32(SQ_CLOSURESTREAM_PART);
    for lvi in &f.localvarinfos {
        if !write_object(w, &lvi.name) {
            return false;
        }
        w.i64(lvi.pos as SQInteger);
        w.i64(lvi.start_op as SQInteger);
        w.i64(lvi.end_op as SQInteger);
    }
    w.u32(SQ_CLOSURESTREAM_PART);
    for li in &f.lineinfos {
        w.i64(li.line);
        w.i64(li.op);
    }
    w.u32(SQ_CLOSURESTREAM_PART);
    for dp in &f.defaultparams {
        w.i64(*dp);
    }
    w.u32(SQ_CLOSURESTREAM_PART);
    for ins in &f.instructions {
        let raw: [u8; 8] = unsafe { std::mem::transmute::<SQInstruction, [u8; 8]>(*ins) };
        w.bytes(&raw);
    }
    w.u32(SQ_CLOSURESTREAM_PART);
    for sub in &f.functions {
        if let Value::FuncProto(fp) = sub {
            if !proto_save(w, fp) {
                return false;
            }
        } else {
            return false;
        }
    }
    w.i64(f.stacksize);
    w.u8(f.bgenerator as u8);
    w.i64(f.varparams);
    true
}

/// ↔ sq_writeclosure：返回完整 .cnut 字节流（含 0xFAFA 头）
pub fn write_closure(vm: &mut VmData, closure: &ClosureRef) -> Result<Vec<u8>, String> {
    let func = closure.borrow().function.clone();
    if !func.outervalues.is_empty() {
        vm.raise_error_str("a closure with free variables bound cannot be serialized");
        return Err("bound closure".into());
    }
    let mut w = Writer { buf: Vec::new() };
    w.u16(SQ_BYTECODE_STREAM_TAG);
    w.u32(SQ_CLOSURESTREAM_HEAD);
    w.u32(1); // sizeof(SQChar)
    w.u32(8); // sizeof(SQInteger)
    w.u32(4); // sizeof(SQFloat)
    if !proto_save(&mut w, &func) {
        vm.raise_error_str("cannot serialize a closure");
        return Err("serialize failed".into());
    }
    w.u32(SQ_CLOSURESTREAM_TAIL);
    Ok(w.buf)
}

// ---------------------------------------------------------------------------
// 读取
// ---------------------------------------------------------------------------

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn u8(&mut self) -> Option<u8> {
        let v = *self.buf.get(self.pos)?;
        self.pos += 1;
        Some(v)
    }
    fn u16(&mut self) -> Option<u16> {
        let b = self.buf.get(self.pos..self.pos + 2)?;
        self.pos += 2;
        Some(u16::from_le_bytes(b.try_into().unwrap()))
    }
    fn u32(&mut self) -> Option<u32> {
        let b = self.buf.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_le_bytes(b.try_into().unwrap()))
    }
    fn i64(&mut self) -> Option<SQInteger> {
        let b = self.buf.get(self.pos..self.pos + 8)?;
        self.pos += 8;
        Some(SQInteger::from_le_bytes(b.try_into().unwrap()))
    }
    fn f32(&mut self) -> Option<SQFloat> {
        let b = self.buf.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(f32::from_le_bytes(b.try_into().unwrap()))
    }
    fn raw(&mut self, n: usize) -> Option<&'a [u8]> {
        let b = self.buf.get(self.pos..self.pos + n)?;
        self.pos += n;
        Some(b)
    }
    fn tag(&mut self, expect: u32) -> bool {
        match self.u32() {
            Some(t) if t == expect => true,
            _ => false,
        }
    }
}

/// ↔ ReadObject
fn read_object(vm: &VmData, r: &mut Reader) -> Option<Value> {
    let t = r.u32()?;
    let v = match t {
        OT_STRING => {
            let len = r.i64()?;
            if len < 0 {
                return None;
            }
            let bytes = r.raw(len as usize)?.to_vec();
            Value::String(vm.ss.stringtable.add(&bytes))
        }
        OT_INTEGER => Value::Integer(r.i64()?),
        OT_BOOL => Value::Bool(r.i64()? != 0),
        OT_FLOAT => Value::Float(r.f32()?),
        OT_NULL => Value::Null,
        _ => return None,
    };
    Some(v)
}

/// ↔ SQFunctionProto::Load
fn proto_load(vm: &VmData, r: &mut Reader, ss: &Rc<crate::state::SharedState>) -> Option<FuncProtoRef> {
    if !r.tag(SQ_CLOSURESTREAM_PART) {
        return None;
    }
    let sourcename = read_object(vm, r)?;
    let name = read_object(vm, r)?;
    let mut f = SQFunctionProto::empty(sourcename, name);

    if !r.tag(SQ_CLOSURESTREAM_PART) {
        return None;
    }
    let nliterals = r.i64()?;
    let nparameters = r.i64()?;
    let noutervalues = r.i64()?;
    let nlocalvarinfos = r.i64()?;
    let nlineinfos = r.i64()?;
    let ndefaultparams = r.i64()?;
    let ninstructions = r.i64()?;
    let nfunctions = r.i64()?;
    if nliterals < 0 || nparameters < 0 || noutervalues < 0 || nlocalvarinfos < 0
        || nlineinfos < 0 || ndefaultparams < 0 || ninstructions < 0 || nfunctions < 0
    {
        return None;
    }

    if !r.tag(SQ_CLOSURESTREAM_PART) {
        return None;
    }
    for _ in 0..nliterals {
        f.literals.push(read_object(vm, r)?);
    }
    if !r.tag(SQ_CLOSURESTREAM_PART) {
        return None;
    }
    for _ in 0..nparameters {
        f.parameters.push(read_object(vm, r)?);
    }
    if !r.tag(SQ_CLOSURESTREAM_PART) {
        return None;
    }
    for _ in 0..noutervalues {
        let ty = r.i64()?;
        let src = read_object(vm, r)?;
        let nm = read_object(vm, r)?;
        f.outervalues.push(SQOuterVar {
            otype: if ty == 0 { SQOuterType::OtLocal } else { SQOuterType::OtOuter },
            name: nm,
            src,
        });
    }
    if !r.tag(SQ_CLOSURESTREAM_PART) {
        return None;
    }
    for _ in 0..nlocalvarinfos {
        let mut lvi = SQLocalVarInfo::new();
        lvi.name = read_object(vm, r)?;
        lvi.pos = r.i64()? as SQUnsignedInteger;
        lvi.start_op = r.i64()? as SQUnsignedInteger;
        lvi.end_op = r.i64()? as SQUnsignedInteger;
        f.localvarinfos.push(lvi);
    }
    if !r.tag(SQ_CLOSURESTREAM_PART) {
        return None;
    }
    for _ in 0..nlineinfos {
        let line = r.i64()?;
        let op = r.i64()?;
        f.lineinfos.push(SQLineInfo { line, op });
    }
    if !r.tag(SQ_CLOSURESTREAM_PART) {
        return None;
    }
    for _ in 0..ndefaultparams {
        f.defaultparams.push(r.i64()?);
    }
    if !r.tag(SQ_CLOSURESTREAM_PART) {
        return None;
    }
    let ins_bytes = r.raw(ninstructions as usize * 8)?;
    for chunk in ins_bytes.chunks_exact(8) {
        f.instructions.push(unsafe {
            std::mem::transmute::<[u8; 8], SQInstruction>(chunk.try_into().unwrap())
        });
    }
    if !r.tag(SQ_CLOSURESTREAM_PART) {
        return None;
    }
    for _ in 0..nfunctions {
        let sub = proto_load(vm, r, ss)?;
        f.functions.push(Value::FuncProto(sub));
    }
    f.stacksize = r.i64()?;
    f.bgenerator = r.u8()? != 0;
    f.varparams = r.i64()?;
    Some(SQFunctionProto::register(ss, f))
}

/// ↔ sq_readclosure：从 .cnut 字节流（含 0xFAFA 头）恢复闭包

pub fn read_closure(vm: &mut VmData, data: &[u8]) -> Result<Value, String> {
    let mut r = Reader { buf: data, pos: 0 };
    let tag = r.u16().ok_or("truncated")?;
    if tag != SQ_BYTECODE_STREAM_TAG {
        return Err("invalid or corrupted closure stream".into());
    }
    if !r.tag(SQ_CLOSURESTREAM_HEAD) {
        return Err("invalid or corrupted closure stream".into());
    }
    // sizeof 校验（必须与当前构建一致：1/8/4）
    if r.u32() != Some(1) || r.u32() != Some(8) || r.u32() != Some(4) {
        return Err("invalid closure stream (sizeof mismatch)".into());
    }
    let ss = vm.ss.clone();
    let proto = proto_load(vm, &mut r, &ss).ok_or("invalid or corrupted closure stream")?;
    if !r.tag(SQ_CLOSURESTREAM_TAIL) {
        return Err("invalid or corrupted closure stream".into());
    }
    let root_weak = match &vm.roottable {
        Value::Table(t) => Some(Value::Table(t.clone()).get_weakref_as_weakref()),
        _ => None,
    };
    let clo = crate::closure::SQClosure::create(&ss, proto, root_weak);
    Ok(Value::Closure(clo))
}

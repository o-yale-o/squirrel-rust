//! ↔ include/sqconfig.h
//!
//! 对齐官方 Windows x64 默认构建配置：`_SQ64` 开、`SQUNICODE` 关、`SQFloat = f32`。
//! 因此：SQChar=1 字节（字节串，Latin-1 语义）、SQInteger=i64、SQHash=u64。

pub type SQInteger = i64;
pub type SQUnsignedInteger = u64;
pub type SQInt32 = i32;
pub type SQUnsignedInteger32 = u32;
pub type SQHash = u64;
pub type SQFloat = f32;
pub type SQUserPointer = usize;
pub type SQObjectType = u32;
pub type SQRESULT = SQInteger;

pub const SQUIRREL_VERSION: &str = "Squirrel 3.2 stable";
pub const SQUIRREL_VERSION_NUMBER: SQInteger = 320;
pub const SQUIRREL_COPYRIGHT: &str = "Copyright (C) 2003-2022 Alberto Demichelis";

pub const MAX_CHAR: u32 = 0xFF;
pub const NUMBER_MAX_CHAR: usize = 50;

/// ↔ squirrel.h 中的 SQOBJECT_* 位标志
pub const SQOBJECT_REF_COUNTED: SQObjectType = 0x0800_0000;
pub const SQOBJECT_NUMERIC: SQObjectType = 0x0400_0000;
pub const SQOBJECT_DELEGABLE: SQObjectType = 0x0200_0000;
pub const SQOBJECT_CANBEFALSE: SQObjectType = 0x0100_0000;

pub const _RT_MASK: SQObjectType = 0x00FF_FFFF;

pub const fn raw_type(t: SQObjectType) -> SQObjectType {
    t & _RT_MASK
}

pub const _RT_NULL: SQObjectType = 0x0000_0001;
pub const _RT_INTEGER: SQObjectType = 0x0000_0002;
pub const _RT_FLOAT: SQObjectType = 0x0000_0004;
pub const _RT_BOOL: SQObjectType = 0x0000_0008;
pub const _RT_STRING: SQObjectType = 0x0000_0010;
pub const _RT_TABLE: SQObjectType = 0x0000_0020;
pub const _RT_ARRAY: SQObjectType = 0x0000_0040;
pub const _RT_USERDATA: SQObjectType = 0x0000_0080;
pub const _RT_CLOSURE: SQObjectType = 0x0000_0100;
pub const _RT_NATIVECLOSURE: SQObjectType = 0x0000_0200;
pub const _RT_GENERATOR: SQObjectType = 0x0000_0400;
pub const _RT_USERPOINTER: SQObjectType = 0x0000_0800;
pub const _RT_THREAD: SQObjectType = 0x0000_1000;
pub const _RT_FUNCPROTO: SQObjectType = 0x0000_2000;
pub const _RT_CLASS: SQObjectType = 0x0000_4000;
pub const _RT_INSTANCE: SQObjectType = 0x0000_8000;
pub const _RT_WEAKREF: SQObjectType = 0x0001_0000;
pub const _RT_OUTER: SQObjectType = 0x0002_0000;

pub const OT_NULL: SQObjectType = _RT_NULL | SQOBJECT_CANBEFALSE;
pub const OT_INTEGER: SQObjectType = _RT_INTEGER | SQOBJECT_NUMERIC | SQOBJECT_CANBEFALSE;
pub const OT_FLOAT: SQObjectType = _RT_FLOAT | SQOBJECT_NUMERIC | SQOBJECT_CANBEFALSE;
pub const OT_BOOL: SQObjectType = _RT_BOOL | SQOBJECT_CANBEFALSE;
pub const OT_STRING: SQObjectType = _RT_STRING | SQOBJECT_REF_COUNTED;
pub const OT_TABLE: SQObjectType = _RT_TABLE | SQOBJECT_REF_COUNTED | SQOBJECT_DELEGABLE;
pub const OT_ARRAY: SQObjectType = _RT_ARRAY | SQOBJECT_REF_COUNTED;
pub const OT_USERDATA: SQObjectType = _RT_USERDATA | SQOBJECT_REF_COUNTED | SQOBJECT_DELEGABLE;
pub const OT_CLOSURE: SQObjectType = _RT_CLOSURE | SQOBJECT_REF_COUNTED;
pub const OT_NATIVECLOSURE: SQObjectType = _RT_NATIVECLOSURE | SQOBJECT_REF_COUNTED;
pub const OT_GENERATOR: SQObjectType = _RT_GENERATOR | SQOBJECT_REF_COUNTED;
pub const OT_USERPOINTER: SQObjectType = _RT_USERPOINTER;
pub const OT_THREAD: SQObjectType = _RT_THREAD | SQOBJECT_REF_COUNTED;
pub const OT_FUNCPROTO: SQObjectType = _RT_FUNCPROTO | SQOBJECT_REF_COUNTED;
pub const OT_CLASS: SQObjectType = _RT_CLASS | SQOBJECT_REF_COUNTED;
pub const OT_INSTANCE: SQObjectType = _RT_INSTANCE | SQOBJECT_REF_COUNTED | SQOBJECT_DELEGABLE;
pub const OT_WEAKREF: SQObjectType = _RT_WEAKREF | SQOBJECT_REF_COUNTED;
pub const OT_OUTER: SQObjectType = _RT_OUTER | SQOBJECT_REF_COUNTED;

pub const SQ_VMSTATE_IDLE: SQInteger = 0;
pub const SQ_VMSTATE_RUNNING: SQInteger = 1;
pub const SQ_VMSTATE_SUSPENDED: SQInteger = 2;

pub const SQ_OK: SQRESULT = 0;
pub const SQ_ERROR: SQRESULT = -1;

/// ↔ sqvm.h
pub const MAX_NATIVE_CALLS: SQInteger = 100;
pub const MIN_STACK_OVERHEAD: SQInteger = 15;
pub const SQ_SUSPEND_FLAG: SQInteger = -666;
pub const SQ_TAILCALL_FLAG: SQInteger = -777;
pub const DONT_FALL_BACK: SQInteger = 666;

pub const SQ_BYTECODE_STREAM_TAG: u16 = 0xFAFA;

/// C++ `(SQInteger)someFloat` 在 MSVC 下的语义（cvttss2si：越界/NaN → i64 最小值）。
/// 原版大量依赖此转换（HashObj、tointeger 等），必须逐位对齐。
pub fn c_f32_to_i64(f: f32) -> SQInteger {
    if f.is_nan() || f >= 9_223_372_036_854_775_808.0f32 || f < -9_223_372_036_854_775_808.0f32 {
        SQInteger::MIN
    } else {
        f as SQInteger
    }
}

pub fn c_f64_to_i64(f: f64) -> SQInteger {
    if f.is_nan() || f >= 9_223_372_036_854_775_808.0f64 || f < -9_223_372_036_854_775_808.0f64 {
        SQInteger::MIN
    } else {
        f as SQInteger
    }
}

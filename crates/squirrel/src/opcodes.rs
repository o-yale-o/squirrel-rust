//! ↔ squirrel/sqopcodes.h

use crate::config::{SQInteger, SQInt32};

pub const MAX_FUNC_STACKSIZE: usize = 0xFF;
pub const MAX_LITERALS: SQInteger = 0x7FFFFFFF;

/// ↔ BitWiseOP
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BitWiseOP {
    BWAnd = 0,
    BWOr = 2,
    BWXor = 3,
    BWShiftL = 4,
    BWShiftR = 5,
    BWUShiftR = 6,
}

/// ↔ CmpOP
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CmpOP {
    CmpG = 0,
    CmpGe = 2,
    CmpL = 3,
    CmpLe = 4,
    Cmp3W = 5,
}

/// ↔ BitWiseOP / CmpOP 的数值常量（与 C++ 枚举值一致，作指令 arg3 用）
pub const BW_AND: SQInteger = 0;
pub const BW_OR: SQInteger = 2;
pub const BW_XOR: SQInteger = 3;
pub const BW_SHIFTL: SQInteger = 4;
pub const BW_SHIFTR: SQInteger = 5;
pub const BW_USHIFTR: SQInteger = 6;
pub const CMP_G: SQInteger = 0;
pub const CMP_GE: SQInteger = 2;
pub const CMP_L: SQInteger = 3;
pub const CMP_LE: SQInteger = 4;
pub const CMP_3W: SQInteger = 5;

/// ↔ NewObjectType
pub const NOT_TABLE: SQInteger = 0;
pub const NOT_ARRAY: SQInteger = 1;
pub const NOT_CLASS: SQInteger = 2;

/// ↔ AppendArrayType
pub const AAT_STACK: u8 = 0;
pub const AAT_LITERAL: u8 = 1;
pub const AAT_INT: u8 = 2;
pub const AAT_FLOAT: u8 = 3;
pub const AAT_BOOL: u8 = 4;

/// ↔ SQOpcode：数值与 C++ 枚举一一对应（.cnut 兼容依赖此编码）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Op {
    Line = 0x00,
    Load = 0x01,
    LoadInt = 0x02,
    LoadFloat = 0x03,
    DLoad = 0x04,
    TailCall = 0x05,
    Call = 0x06,
    PrepCall = 0x07,
    PrepCallK = 0x08,
    GetK = 0x09,
    Move = 0x0A,
    NewSlot = 0x0B,
    Delete = 0x0C,
    Set = 0x0D,
    Get = 0x0E,
    Eq = 0x0F,
    Ne = 0x10,
    Add = 0x11,
    Sub = 0x12,
    Mul = 0x13,
    Div = 0x14,
    Mod = 0x15,
    BitW = 0x16,
    Return = 0x17,
    LoadNulls = 0x18,
    LoadRoot = 0x19,
    LoadBool = 0x1A,
    DMove = 0x1B,
    Jmp = 0x1C,
    JCmp = 0x1D, // 原 _OP_JNZ(0x1D) 已废弃，编号被 JCmp 复用
    JZ = 0x1E,
    SetOuter = 0x1F,
    GetOuter = 0x20,
    NewObj = 0x21,
    AppendArray = 0x22,
    Comparith = 0x23,
    Inc = 0x24,
    IncL = 0x25,
    PInc = 0x26,
    PIncL = 0x27,
    Cmp = 0x28,
    Exists = 0x29,
    InstanceOf = 0x2A,
    And = 0x2B,
    Or = 0x2C,
    Neg = 0x2D,
    Not = 0x2E,
    BwNot = 0x2F,
    Closure = 0x30,
    Yield = 0x31,
    Resume = 0x32,
    Foreach = 0x33,
    PostForeach = 0x34,
    Clone = 0x35,
    TypeOf = 0x36,
    PushTrap = 0x37,
    PopTrap = 0x38,
    Throw = 0x39,
    NewSlotA = 0x3A,
    GetBase = 0x3B,
    Close = 0x3C,
}

pub const OP_COUNT: u8 = 0x3D;

impl Op {
    pub fn from_u8(v: u8) -> Option<Op> {
        if v < OP_COUNT {
            Some(unsafe { std::mem::transmute(v) })
        } else {
            None
        }
    }

    /// ↔ g_InstrDesc（sqfuncstate.cpp:14-76）
    pub fn name(self) -> &'static str {
        use Op::*;
        match self {
            Line => "LINE",
            Load => "LOAD",
            LoadInt => "LOADINT",
            LoadFloat => "LOADFLOAT",
            DLoad => "DLOAD",
            TailCall => "TAILCALL",
            Call => "CALL",
            PrepCall => "PREPCALL",
            PrepCallK => "PREPCALLK",
            GetK => "GETK",
            Move => "MOVE",
            NewSlot => "NEWSLOT",
            Delete => "DELETE",
            Set => "SET",
            Get => "GET",
            Eq => "EQ",
            Ne => "NE",
            Add => "ADD",
            Sub => "SUB",
            Mul => "MUL",
            Div => "DIV",
            Mod => "MOD",
            BitW => "BITW",
            Return => "RETURN",
            LoadNulls => "LOADNULLS",
            LoadRoot => "LOADROOT",
            LoadBool => "LOADBOOL",
            DMove => "DMOVE",
            Jmp => "JMP",
            JCmp => "JCMP",
            JZ => "JZ",
            SetOuter => "SETOUTER",
            GetOuter => "GETOUTER",
            NewObj => "NEWOBJ",
            AppendArray => "APPENDARRAY",
            Comparith => "COMPARITH",
            Inc => "INC",
            IncL => "INCL",
            PInc => "PINC",
            PIncL => "PINCL",
            Cmp => "CMP",
            Exists => "EXISTS",
            InstanceOf => "INSTANCEOF",
            And => "AND",
            Or => "OR",
            Neg => "NEG",
            Not => "NOT",
            BwNot => "BWNOT",
            Closure => "CLOSURE",
            Yield => "YIELD",
            Resume => "RESUME",
            Foreach => "FOREACH",
            PostForeach => "POSTFOREACH",
            Clone => "CLONE",
            TypeOf => "TYPEOF",
            PushTrap => "PUSHTRAP",
            PopTrap => "POPTRAP",
            Throw => "THROW",
            NewSlotA => "NEWSLOTA",
            GetBase => "GETBASE",
            Close => "CLOSE",
        }
    }
}

/// ↔ SQInstruction：8 字节定长，字段顺序/偏移与 C++ 完全一致（.cnut 直接读写）
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SQInstruction {
    pub arg1: SQInt32, // 偏移 0
    pub op: u8,        // 偏移 4
    pub arg0: u8,      // 偏移 5
    pub arg2: u8,      // 偏移 6
    pub arg3: u8,      // 偏移 7
}

impl SQInstruction {
    /// ↔ 构造函数（参数截断为 u8）
    pub fn new(op: Op, a0: SQInteger, a1: SQInteger, a2: SQInteger, a3: SQInteger) -> Self {
        SQInstruction {
            arg1: a1 as SQInt32,
            op: op as u8,
            arg0: a0 as u8,
            arg2: a2 as u8,
            arg3: a3 as u8,
        }
    }

    pub fn opcode(&self) -> Op {
        Op::from_u8(self.op).unwrap_or(Op::Line)
    }

    /// ↔ `*((const SQInt32*)&_arg1)`（有符号解释）
    #[inline]
    pub fn sarg1(&self) -> SQInteger {
        self.arg1 as SQInteger
    }

    /// ↔ `*((const SQFloat*)&arg1)`（位重解释）
    #[inline]
    pub fn farg1(&self) -> crate::config::SQFloat {
        f32::from_bits(self.arg1 as u32)
    }
}

impl std::fmt::Debug for SQInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<12} {} {} {} {}",
            self.opcode().name(),
            self.arg0,
            self.arg1,
            self.arg2,
            self.arg3
        )
    }
}

pub const NEW_SLOT_ATTRIBUTES_FLAG: u8 = 0x01;
pub const NEW_SLOT_STATIC_FLAG: u8 = 0x02;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_layout_is_8_bytes_c_compatible() {
        assert_eq!(std::mem::size_of::<SQInstruction>(), 8);
        assert_eq!(std::mem::offset_of!(SQInstruction, arg1), 0);
        assert_eq!(std::mem::offset_of!(SQInstruction, op), 4);
        assert_eq!(std::mem::offset_of!(SQInstruction, arg0), 5);
        assert_eq!(std::mem::offset_of!(SQInstruction, arg2), 6);
        assert_eq!(std::mem::offset_of!(SQInstruction, arg3), 7);
        let i = SQInstruction::new(Op::Add, 1, -2, 3, 4);
        let bytes = unsafe { std::mem::transmute::<SQInstruction, [u8; 8]>(i) };
        assert_eq!(bytes[4], Op::Add as u8);
        assert_eq!(i.sarg1(), -2);
    }
}

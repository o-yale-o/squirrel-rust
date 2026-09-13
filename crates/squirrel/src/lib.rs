//! Squirrel 3.2 脚本引擎的 Rust 复刻（仅 Windows）。
//!
//! 模块划分与 C++ 原版（squirrel/*.cpp）一一对应：
//! - `config`    ↔ include/sqconfig.h     类型与编译配置（Win64 默认：_SQ64、f32、非 UNICODE）
//! - `object`    ↔ sqobject.h/.cpp        对象模型（18 种类型、weakref、哈希）
//! - `table`     ↔ sqtable.h/.cpp         Lua4 风格开链哈希表（物理布局逐位复刻）
//! - `array`     ↔ sqarray.h              数组
//! - `userdata`  ↔ squserdata.h           用户数据
//! - `opcodes`   ↔ sqopcodes.h            61 条指令与 8 字节编码
//! - `funcproto` ↔ sqfuncproto.h/.cpp     函数原型（字节码容器）
//! - `closure`   ↔ sqclosure.h/.cpp       闭包/outer/generator/nativeclosure
//! - `class`     ↔ sqclass.h/.cpp         类/实例
//! - `state`     ↔ sqstate.h/.cpp         SharedState、字符串表、RefTable、GC
//! - `vm`        ↔ sqvm.h/.cpp           虚拟机（栈、帧、执行循环）
//! - `lexer`     ↔ sqlexer.h/.cpp         词法分析
//! - `funcstate` ↔ sqfuncstate.h/.cpp     寄存器分配与窥孔优化
//! - `compiler`  ↔ sqcompiler.h/.cpp      单遍递归下降编译器（M4）
//! - `baselib`   ↔ sqbaselib.cpp          内建 base 库（M5）
//! - `api`       ↔ sqapi.cpp              宿主 API（Rust 风格，M5）

pub mod api;
pub mod array;
pub mod class;
pub mod closure;
pub mod config;
pub mod debug_util;
pub mod funcproto;
pub mod funcstate;
pub mod lexer;
pub mod object;
pub mod opcodes;
pub mod release;
pub mod serialization;
pub mod state;
pub mod table;
pub mod userdata;
pub mod vm;

pub mod cformat;
pub mod compiler;
pub mod baselib;

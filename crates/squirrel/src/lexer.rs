//! ↔ squirrel/sqlexer.h + sqlexer.cpp + sqcompiler.h 的 TK_* 定义
//!
//! 8-bit 构建（SQChar=unsigned char）。C++ 用 longjmp 跳出错误；Rust 用 Result。
//! 输入用逐字符回调（↔ SQLEXREADFUNC），返回 0 表示 EOF。

use crate::config::*;

// ---------- Token（↔ sqcompiler.h） ----------
pub type Token = SQInteger;

pub const TK_IDENTIFIER: Token = 258;
pub const TK_STRING_LITERAL: Token = 259;
pub const TK_INTEGER: Token = 260;
pub const TK_FLOAT: Token = 261;
pub const TK_BASE: Token = 262;
pub const TK_DELETE: Token = 263;
pub const TK_EQ: Token = 264;
pub const TK_NE: Token = 265;
pub const TK_LE: Token = 266;
pub const TK_GE: Token = 267;
pub const TK_SWITCH: Token = 268;
pub const TK_ARROW: Token = 269;
pub const TK_AND: Token = 270;
pub const TK_OR: Token = 271;
pub const TK_IF: Token = 272;
pub const TK_ELSE: Token = 273;
pub const TK_WHILE: Token = 274;
pub const TK_BREAK: Token = 275;
pub const TK_FOR: Token = 276;
pub const TK_DO: Token = 277;
pub const TK_NULL: Token = 278;
pub const TK_FOREACH: Token = 279;
pub const TK_IN: Token = 280;
pub const TK_NEWSLOT: Token = 281;
pub const TK_MODULO: Token = 282;
pub const TK_LOCAL: Token = 283;
pub const TK_CLONE: Token = 284;
pub const TK_FUNCTION: Token = 285;
pub const TK_RETURN: Token = 286;
pub const TK_TYPEOF: Token = 287;
pub const TK_UMINUS: Token = 288;
pub const TK_PLUSEQ: Token = 289;
pub const TK_MINUSEQ: Token = 290;
pub const TK_CONTINUE: Token = 291;
pub const TK_YIELD: Token = 292;
pub const TK_TRY: Token = 293;
pub const TK_CATCH: Token = 294;
pub const TK_THROW: Token = 295;
pub const TK_SHIFTL: Token = 296;
pub const TK_SHIFTR: Token = 297;
pub const TK_RESUME: Token = 298;
pub const TK_DOUBLE_COLON: Token = 299;
pub const TK_CASE: Token = 300;
pub const TK_DEFAULT: Token = 301;
pub const TK_THIS: Token = 302;
pub const TK_PLUSPLUS: Token = 303;
pub const TK_MINUSMINUS: Token = 304;
pub const TK_3WAYSCMP: Token = 305;
pub const TK_USHIFTR: Token = 306;
pub const TK_CLASS: Token = 307;
pub const TK_EXTENDS: Token = 308;
pub const TK_CONSTRUCTOR: Token = 310;
pub const TK_INSTANCEOF: Token = 311;
pub const TK_VARPARAMS: Token = 312;
pub const TK___LINE__: Token = 313;
pub const TK___FILE__: Token = 314;
pub const TK_TRUE: Token = 315;
pub const TK_FALSE: Token = 316;
pub const TK_MULEQ: Token = 317;
pub const TK_DIVEQ: Token = 318;
pub const TK_MODEQ: Token = 319;
pub const TK_ATTR_OPEN: Token = 320;
pub const TK_ATTR_CLOSE: Token = 321;
pub const TK_STATIC: Token = 322;
pub const TK_ENUM: Token = 323;
pub const TK_CONST: Token = 324;
pub const TK_RAWCALL: Token = 325;

pub const SQUIRREL_EOB: u8 = 0;

/// ↔ ADD_KEYWORD 关键字表（注册顺序与 C++ 一致）
pub const KEYWORDS: &[(&str, Token)] = &[
    ("while", TK_WHILE),
    ("do", TK_DO),
    ("if", TK_IF),
    ("else", TK_ELSE),
    ("break", TK_BREAK),
    ("continue", TK_CONTINUE),
    ("return", TK_RETURN),
    ("null", TK_NULL),
    ("function", TK_FUNCTION),
    ("local", TK_LOCAL),
    ("for", TK_FOR),
    ("foreach", TK_FOREACH),
    ("in", TK_IN),
    ("typeof", TK_TYPEOF),
    ("base", TK_BASE),
    ("delete", TK_DELETE),
    ("try", TK_TRY),
    ("catch", TK_CATCH),
    ("throw", TK_THROW),
    ("clone", TK_CLONE),
    ("yield", TK_YIELD),
    ("resume", TK_RESUME),
    ("switch", TK_SWITCH),
    ("case", TK_CASE),
    ("default", TK_DEFAULT),
    ("this", TK_THIS),
    ("class", TK_CLASS),
    ("extends", TK_EXTENDS),
    ("constructor", TK_CONSTRUCTOR),
    ("instanceof", TK_INSTANCEOF),
    ("true", TK_TRUE),
    ("false", TK_FALSE),
    ("static", TK_STATIC),
    ("enum", TK_ENUM),
    ("const", TK_CONST),
    ("__LINE__", TK___LINE__),
    ("__FILE__", TK___FILE__),
    ("rawcall", TK_RAWCALL),
];

/// ↔ CompilerErrorFunc 的载体（C++ 经 errfunc longjmp；Rust 走 Result）
#[derive(Debug, Clone)]
pub struct LexError {
    pub msg: String,
}

fn err<T>(msg: &str) -> Result<T, LexError> {
    Err(LexError {
        msg: msg.to_string(),
    })
}

/// ↔ SQLexer。`readf` 每次返回一个字符（>0），0 = EOF。
pub struct SqlLexer {
    readf: Box<dyn FnMut() -> SQInteger>,
    curtoken: Token,
    pub prevtoken: Token,
    pub currentline: SQInteger,
    pub lasttokenline: SQInteger,
    pub currentcolumn: SQInteger,
    reached_eof: bool,
    currdata: u8,
    longstr: Vec<u8>,
    pub nvalue: SQInteger,
    pub fvalue: SQFloat,
}

impl SqlLexer {
    /// ↔ SQLexer::Init
    pub fn new(readf: Box<dyn FnMut() -> SQInteger>) -> SqlLexer {
        let mut lx = SqlLexer {
            readf,
            curtoken: 0,
            prevtoken: -1,
            currentline: 1,
            lasttokenline: 1,
            currentcolumn: 0,
            reached_eof: false,
            currdata: 0,
            longstr: Vec::new(),
            nvalue: 0,
            fvalue: 0.0,
        };
        lx.next_char();
        lx
    }

    /// ↔ SQLexer::Next（读一个字符；越界/EOF 处理）
    fn next_char(&mut self) {
        let t = (self.readf)();
        if t > MAX_CHAR as SQInteger {
            // Error("Invalid character")——C++ 经 errfunc 抛出；此处按 EOB 终止
            self.currdata = SQUIRREL_EOB;
            self.reached_eof = true;
            return;
        }
        if t != 0 {
            self.currdata = t as u8;
            return;
        }
        self.currdata = SQUIRREL_EOB;
        self.reached_eof = true;
    }

    /// ↔ NEXT 宏：{Next(); _currentcolumn++;}
    fn advance(&mut self) {
        self.next_char();
        self.currentcolumn += 1;
    }

    fn is_eob(&self) -> bool {
        self.currdata <= SQUIRREL_EOB
    }

    fn return_token(&mut self, t: Token) -> Result<Token, LexError> {
        self.prevtoken = self.curtoken;
        self.curtoken = t;
        Ok(t)
    }

    /// ↔ SQLexer::Tok2Str
    pub fn tok2str(tok: Token) -> Option<&'static str> {
        KEYWORDS.iter().find(|(_, id)| *id == tok).map(|(s, _)| *s)
    }

    fn lex_block_comment(&mut self) -> Result<(), LexError> {
        let mut done = false;
        while !done {
            match self.currdata {
                b'*' => {
                    self.advance();
                    if self.currdata == b'/' {
                        done = true;
                        self.advance();
                    }
                }
                b'\n' => {
                    self.currentline += 1;
                    self.advance();
                }
                SQUIRREL_EOB => return err("missing \"*/\" in comment"),
                _ => self.advance(),
            }
        }
        Ok(())
    }

    fn lex_line_comment(&mut self) {
        loop {
            self.advance();
            if self.currdata == b'\n' || self.is_eob() {
                break;
            }
        }
    }

    /// ↔ SQLexer::Lex（主入口）
    pub fn lex(&mut self) -> Result<Token, LexError> {
        self.lasttokenline = self.currentline;
        while self.currdata != SQUIRREL_EOB {
            let c = self.currdata;
            match c {
                b'\t' | b'\r' | b' ' => {
                    self.advance();
                    continue;
                }
                b'\n' => {
                    self.currentline += 1;
                    self.prevtoken = self.curtoken;
                    self.curtoken = b'\n' as Token;
                    self.advance();
                    self.currentcolumn = 1;
                    continue;
                }
                b'#' => {
                    self.lex_line_comment();
                    continue;
                }
                b'/' => {
                    self.advance();
                    match self.currdata {
                        b'*' => {
                            self.advance();
                            self.lex_block_comment()?;
                            continue;
                        }
                        b'/' => {
                            self.lex_line_comment();
                            continue;
                        }
                        b'=' => {
                            self.advance();
                            return self.return_token(TK_DIVEQ);
                        }
                        b'>' => {
                            self.advance();
                            return self.return_token(TK_ATTR_CLOSE);
                        }
                        _ => return self.return_token(b'/' as Token),
                    }
                }
                b'=' => {
                    self.advance();
                    if self.currdata != b'=' {
                        return self.return_token(b'=' as Token);
                    } else {
                        self.advance();
                        return self.return_token(TK_EQ);
                    }
                }
                b'<' => {
                    self.advance();
                    match self.currdata {
                        b'=' => {
                            self.advance();
                            if self.currdata == b'>' {
                                self.advance();
                                return self.return_token(TK_3WAYSCMP);
                            }
                            return self.return_token(TK_LE);
                        }
                        b'-' => {
                            self.advance();
                            return self.return_token(TK_NEWSLOT);
                        }
                        b'<' => {
                            self.advance();
                            return self.return_token(TK_SHIFTL);
                        }
                        b'/' => {
                            self.advance();
                            return self.return_token(TK_ATTR_OPEN);
                        }
                        _ => {}
                    }
                    return self.return_token(b'<' as Token);
                }
                b'>' => {
                    self.advance();
                    if self.currdata == b'=' {
                        self.advance();
                        return self.return_token(TK_GE);
                    } else if self.currdata == b'>' {
                        self.advance();
                        if self.currdata == b'>' {
                            self.advance();
                            return self.return_token(TK_USHIFTR);
                        }
                        return self.return_token(TK_SHIFTR);
                    } else {
                        return self.return_token(b'>' as Token);
                    }
                }
                b'!' => {
                    self.advance();
                    if self.currdata != b'=' {
                        return self.return_token(b'!' as Token);
                    } else {
                        self.advance();
                        return self.return_token(TK_NE);
                    }
                }
                b'@' => {
                    self.advance();
                    if self.currdata != b'"' {
                        return self.return_token(b'@' as Token);
                    }
                    let stype = self.read_string(b'"', true)?;
                    return self.return_token(stype);
                }
                b'"' | b'\'' => {
                    let stype = self.read_string(c, false)?;
                    return self.return_token(stype);
                }
                b'{' | b'}' | b'(' | b')' | b'[' | b']' | b';' | b',' | b'?' | b'^' | b'~' => {
                    let ret = c as Token;
                    self.advance();
                    return self.return_token(ret);
                }
                b'.' => {
                    self.advance();
                    if self.currdata != b'.' {
                        return self.return_token(b'.' as Token);
                    }
                    self.advance();
                    if self.currdata != b'.' {
                        return err("invalid token '..'");
                    }
                    self.advance();
                    return self.return_token(TK_VARPARAMS);
                }
                b'&' => {
                    self.advance();
                    if self.currdata != b'&' {
                        return self.return_token(b'&' as Token);
                    } else {
                        self.advance();
                        return self.return_token(TK_AND);
                    }
                }
                b'|' => {
                    self.advance();
                    if self.currdata != b'|' {
                        return self.return_token(b'|' as Token);
                    } else {
                        self.advance();
                        return self.return_token(TK_OR);
                    }
                }
                b':' => {
                    self.advance();
                    if self.currdata != b':' {
                        return self.return_token(b':' as Token);
                    } else {
                        self.advance();
                        return self.return_token(TK_DOUBLE_COLON);
                    }
                }
                b'*' => {
                    self.advance();
                    if self.currdata == b'=' {
                        self.advance();
                        return self.return_token(TK_MULEQ);
                    } else {
                        return self.return_token(b'*' as Token);
                    }
                }
                b'%' => {
                    self.advance();
                    if self.currdata == b'=' {
                        self.advance();
                        return self.return_token(TK_MODEQ);
                    } else {
                        return self.return_token(b'%' as Token);
                    }
                }
                b'-' => {
                    self.advance();
                    if self.currdata == b'=' {
                        self.advance();
                        return self.return_token(TK_MINUSEQ);
                    } else if self.currdata == b'-' {
                        self.advance();
                        return self.return_token(TK_MINUSMINUS);
                    } else {
                        return self.return_token(b'-' as Token);
                    }
                }
                b'+' => {
                    self.advance();
                    if self.currdata == b'=' {
                        self.advance();
                        return self.return_token(TK_PLUSEQ);
                    } else if self.currdata == b'+' {
                        self.advance();
                        return self.return_token(TK_PLUSPLUS);
                    } else {
                        return self.return_token(b'+' as Token);
                    }
                }
                SQUIRREL_EOB => return Ok(0),
                _ => {
                    if c.is_ascii_digit() {
                        let ret = self.read_number()?;
                        return self.return_token(ret);
                    } else if c.is_ascii_alphabetic() || c == b'_' {
                        let t = self.read_id();
                        return self.return_token(t);
                    } else {
                        if c.is_ascii_control() {
                            return err("unexpected character(control)");
                        }
                        self.advance();
                        return self.return_token(c as Token);
                    }
                }
            }
        }
        Ok(0)
    }

    /// ↔ GetIDType
    fn get_id_type(s: &[u8]) -> Token {
        if let Ok(name) = std::str::from_utf8(s) {
            if let Some((_, id)) = KEYWORDS.iter().find(|(k, _)| *k == name) {
                return *id;
            }
        }
        TK_IDENTIFIER
    }

    /// ↔ AddUTF8（\u/\U 转义按 UTF-8 编码追加）
    fn add_utf8(&mut self, ch: SQUnsignedInteger) -> SQInteger {
        if ch < 0x80 {
            self.longstr.push(ch as u8);
            return 1;
        }
        if ch < 0x800 {
            self.longstr.push(((ch >> 6) | 0xC0) as u8);
            self.longstr.push(((ch & 0x3F) | 0x80) as u8);
            return 2;
        }
        if ch < 0x10000 {
            self.longstr.push(((ch >> 12) | 0xE0) as u8);
            self.longstr.push((((ch >> 6) & 0x3F) | 0x80) as u8);
            self.longstr.push(((ch & 0x3F) | 0x80) as u8);
            return 3;
        }
        if ch < 0x110000 {
            self.longstr.push(((ch >> 18) | 0xF0) as u8);
            self.longstr.push((((ch >> 12) & 0x3F) | 0x80) as u8);
            self.longstr.push((((ch >> 6) & 0x3F) | 0x80) as u8);
            self.longstr.push(((ch & 0x3F) | 0x80) as u8);
            return 4;
        }
        0
    }

    /// ↔ ProcessStringHexEscape：收集最多 maxdigits 个十六进制字符
    fn process_string_hex_escape(&mut self, maxdigits: usize) -> Result<u32, LexError> {
        self.advance();
        if !self.currdata.is_ascii_hexdigit() {
            return err("hexadecimal number expected");
        }
        let mut buf = Vec::new();
        let mut n = 0;
        while self.currdata.is_ascii_hexdigit() && n < maxdigits {
            buf.push(self.currdata);
            n += 1;
            self.advance();
        }
        let s = std::str::from_utf8(&buf).unwrap_or("");
        Ok(u32::from_str_radix(s, 16).unwrap_or(0))
    }

    /// ↔ SQLexer::ReadString。返回 token（TK_STRING_LITERAL 或 TK_INTEGER 字符常量）。
    fn read_string(&mut self, ndelim: u8, verbatim: bool) -> Result<Token, LexError> {
        self.longstr.clear();
        self.advance();
        if self.is_eob() {
            return err("unfinished string");
        }
        loop {
            while self.currdata != ndelim {
                let x = self.currdata;
                match x {
                    SQUIRREL_EOB => return err("unfinished string"),
                    b'\n' => {
                        if !verbatim {
                            return err("newline in a constant");
                        }
                        self.longstr.push(self.currdata);
                        self.advance();
                        self.currentline += 1;
                    }
                    b'\\' => {
                        if verbatim {
                            self.longstr.push(b'\\');
                            self.advance();
                        } else {
                            self.advance();
                            match self.currdata {
                                b'x' => {
                                    // sizeof(SQChar)*2 = 2 位（8-bit 构建）
                                    let v = self.process_string_hex_escape(2)?;
                                    self.longstr.push(v as u8);
                                }
                                b'U' | b'u' => {
                                    let maxdigits = if self.currdata == b'u' { 4 } else { 8 };
                                    let v = self.process_string_hex_escape(maxdigits)?;
                                    self.add_utf8(v as SQUnsignedInteger);
                                }
                                b't' => {
                                    self.longstr.push(b'\t');
                                    self.advance();
                                }
                                b'a' => {
                                    self.longstr.push(0x07);
                                    self.advance();
                                }
                                b'b' => {
                                    self.longstr.push(0x08);
                                    self.advance();
                                }
                                b'n' => {
                                    self.longstr.push(b'\n');
                                    self.advance();
                                }
                                b'r' => {
                                    self.longstr.push(b'\r');
                                    self.advance();
                                }
                                b'v' => {
                                    self.longstr.push(0x0B);
                                    self.advance();
                                }
                                b'f' => {
                                    self.longstr.push(0x0C);
                                    self.advance();
                                }
                                b'0' => {
                                    self.longstr.push(0);
                                    self.advance();
                                }
                                b'\\' => {
                                    self.longstr.push(b'\\');
                                    self.advance();
                                }
                                b'"' => {
                                    self.longstr.push(b'"');
                                    self.advance();
                                }
                                b'\'' => {
                                    self.longstr.push(b'\'');
                                    self.advance();
                                }
                                _ => return err("unrecognised escaper char"),
                            }
                        }
                    }
                    _ => {
                        self.longstr.push(self.currdata);
                        self.advance();
                    }
                }
            }
            self.advance();
            if verbatim && self.currdata == b'"' {
                // verbatim 字符串中 "" 表示一个引号
                self.longstr.push(self.currdata);
                self.advance();
            } else {
                break;
            }
        }
        let len = self.longstr.len();
        if ndelim == b'\'' {
            if len == 0 {
                return err("empty constant");
            }
            if len > 1 {
                return err("constant too long");
            }
            self.nvalue = self.longstr[0] as SQInteger;
            return Ok(TK_INTEGER);
        }
        Ok(TK_STRING_LITERAL)
    }

    /// ↔ ReadNumber
    fn read_number(&mut self) -> Result<Token, LexError> {
        const TINT: i32 = 1;
        const TFLOAT: i32 = 2;
        const THEX: i32 = 3;
        const TSCIENTIFIC: i32 = 4;
        const TOCTAL: i32 = 5;
        let mut ntype = TINT;
        let firstchar = self.currdata;
        self.longstr.clear();
        self.advance();
        let is_odigit = |c: u8| c.is_ascii_digit() && c <= b'7';
        if firstchar == b'0'
            && (self.currdata.to_ascii_uppercase() == b'X' || is_odigit(self.currdata))
        {
            if is_odigit(self.currdata) {
                ntype = TOCTAL;
                while is_odigit(self.currdata) {
                    self.longstr.push(self.currdata);
                    self.advance();
                }
                if self.currdata.is_ascii_digit() {
                    return err("invalid octal number");
                }
            } else {
                self.advance(); // consume 'x'/'X'
                ntype = THEX;
                while self.currdata.is_ascii_hexdigit() {
                    self.longstr.push(self.currdata);
                    self.advance();
                }
                if self.longstr.len() > 16 {
                    // MAX_HEX_DIGITS = sizeof(SQInteger)*2
                    return err("too many digits for an Hex number");
                }
            }
        } else {
            self.longstr.push(firstchar);
            while self.currdata == b'.'
                || self.currdata.is_ascii_digit()
                || self.currdata == b'e'
                || self.currdata == b'E'
            {
                let c = self.currdata;
                let isexp = c == b'e' || c == b'E';
                if c == b'.' || isexp {
                    ntype = TFLOAT;
                }
                if isexp {
                    if ntype != TFLOAT {
                        return err("invalid numeric format");
                    }
                    ntype = TSCIENTIFIC;
                    self.longstr.push(c);
                    self.advance();
                    if self.currdata == b'+' || self.currdata == b'-' {
                        self.longstr.push(self.currdata);
                        self.advance();
                    }
                    if !self.currdata.is_ascii_digit() {
                        return err("exponent expected");
                    }
                }
                self.longstr.push(self.currdata);
                self.advance();
            }
        }
        match ntype {
            TSCIENTIFIC | TFLOAT => {
                self.fvalue = strtod_compat(&self.longstr) as SQFloat;
                Ok(TK_FLOAT)
            }
            TINT => {
                let mut res: SQUnsignedInteger = 0;
                for &d in self.longstr.iter() {
                    res = res.wrapping_mul(10).wrapping_add((d - b'0') as SQUnsignedInteger);
                }
                self.nvalue = res as SQInteger;
                Ok(TK_INTEGER)
            }
            THEX => {
                let mut res: SQUnsignedInteger = 0;
                for &d in self.longstr.iter() {
                    let v = if d.is_ascii_digit() {
                        (d - b'0') as SQUnsignedInteger
                    } else {
                        (d.to_ascii_uppercase() - b'A' + 10) as SQUnsignedInteger
                    };
                    res = res.wrapping_mul(16).wrapping_add(v);
                }
                self.nvalue = res as SQInteger;
                Ok(TK_INTEGER)
            }
            TOCTAL => {
                let mut res: SQUnsignedInteger = 0;
                for &d in self.longstr.iter() {
                    res = res.wrapping_mul(8).wrapping_add((d - b'0') as SQUnsignedInteger);
                }
                self.nvalue = res as SQInteger;
                Ok(TK_INTEGER)
            }
            _ => Ok(0),
        }
    }

    /// ↔ ReadID
    fn read_id(&mut self) -> Token {
        self.longstr.clear();
        loop {
            self.longstr.push(self.currdata);
            self.advance();
            if !(self.currdata.is_ascii_alphanumeric() || self.currdata == b'_') {
                break;
            }
        }
        Self::get_id_type(&self.longstr)
    }

    /// ↔ _svalue：刚读到的标识符/字符串内容
    pub fn svalue(&self) -> &[u8] {
        &self.longstr
    }
}

/// ↔ scstrtod：**取能构成一个浮点数的最长前缀**，而不是"整体 parse 失败就给 0"。
///
/// 词法器把 `'.'`/`'e'`/数字**全都收进同一个 token**（↔ 参考实现的 `ReadNumber`
/// 循环条件就是 `'.' || isdigit || isexponent`），所以 `1.2.3` 会作为一个浮点
/// token 走到这里，而 CRT 的 `strtod("1.2.3")` 停在第二个小数点 → **1.2**；
/// `1..2` → **1**。原来整体 `parse::<f64>()` 失败退化成 0.0，于是
/// `print(1.2.3)` 参考给 `1.2`、我们给 `0`（`tests-corpus/t_litmap.nut` 抓到的）。
fn strtod_compat(bytes: &[u8]) -> f64 {
    crate::baselib::parse_dec_float(bytes).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexer_over(src: &str) -> SqlLexer {
        let mut pos = 0usize;
        let bytes = src.as_bytes().to_vec();
        SqlLexer::new(Box::new(move || {
            if pos < bytes.len() {
                let c = bytes[pos];
                pos += 1;
                c as SQInteger
            } else {
                0
            }
        }))
    }

    fn lex_all(src: &str) -> Vec<Token> {
        let mut lx = lexer_over(src);
        let mut out = Vec::new();
        while let Ok(t) = lx.lex() {
            if t == 0 {
                break;
            }
            out.push(t);
        }
        out
    }

    #[test]
    fn basic_tokens() {
        let toks = lex_all("local x = 1 + 2.5");
        assert_eq!(
            toks,
            vec![TK_LOCAL, TK_IDENTIFIER, b'=' as Token, TK_INTEGER, b'+' as Token, TK_FLOAT]
        );
    }

    #[test]
    fn operators() {
        let toks = lex_all("a <- b; c ::= d <- 2");
        assert_eq!(
            toks,
            vec![
                TK_IDENTIFIER,
                TK_NEWSLOT,
                TK_IDENTIFIER,
                b';' as Token,
                TK_IDENTIFIER,
                TK_DOUBLE_COLON,
                b'=' as Token,
                TK_IDENTIFIER,
                TK_NEWSLOT,
                TK_INTEGER
            ]
        );
        let toks = lex_all("a <= b >= c <=> d != e == f");
        assert_eq!(
            toks,
            vec![
                TK_IDENTIFIER,
                TK_LE,
                TK_IDENTIFIER,
                TK_GE,
                TK_IDENTIFIER,
                TK_3WAYSCMP,
                TK_IDENTIFIER,
                TK_NE,
                TK_IDENTIFIER,
                TK_EQ,
                TK_IDENTIFIER
            ]
        );
        let toks = lex_all("<< >> >>> && || ++ -- += -= *= /= %= ...");
        assert_eq!(
            toks,
            vec![
                TK_SHIFTL,
                TK_SHIFTR,
                TK_USHIFTR,
                TK_AND,
                TK_OR,
                TK_PLUSPLUS,
                TK_MINUSMINUS,
                TK_PLUSEQ,
                TK_MINUSEQ,
                TK_MULEQ,
                TK_DIVEQ,
                TK_MODEQ,
                TK_VARPARAMS
            ]
        );
        let toks = lex_all("</ class attrs />");
        assert_eq!(toks, vec![TK_ATTR_OPEN, TK_CLASS, TK_IDENTIFIER, TK_ATTR_CLOSE]);
    }

    #[test]
    fn numbers() {
        let mut lx = lexer_over("0x1F 010 42 3.14 1e3 1.5e-2 1.");
        let expected: [(Token, SQInteger, SQFloat); 7] = [
            (TK_INTEGER, 0x1F, 0.0),
            (TK_INTEGER, 0o10, 0.0),
            (TK_INTEGER, 42, 0.0),
            (TK_FLOAT, 0, 3.14),
            (TK_FLOAT, 0, 1000.0),
            (TK_FLOAT, 0, 0.015),
            (TK_FLOAT, 0, 1.0),
        ];
        for (i, (etok, en, ef)) in expected.iter().enumerate() {
            let t = lx.lex().unwrap();
            assert_eq!(t, *etok, "token #{}", i);
            if *etok == TK_INTEGER {
                assert_eq!(lx.nvalue, *en, "int #{}", i);
            } else {
                assert!((lx.fvalue - ef).abs() < 1e-6, "float #{}: {}", i, lx.fvalue);
            }
        }
    }

    /// 多个小数点/指数混在一起时，**整个串仍是一个浮点 token**，值取 `strtod` 的
    /// 最长合法前缀（↔ `ReadNumber` 的循环条件含 `'.'`，而取值走 `scstrtod`）。
    /// 原来是整体 `parse::<f64>()` 失败退化成 0.0 —— `print(1.2.3)` 参考给 `1.2`。
    #[test]
    fn number_with_extra_dots() {
        for (src, want) in [
            ("1.2.3", 1.2f64),
            ("1..2", 1.0),
            ("1.2e3.4", 1200.0),
            ("1.2.3e4", 1.2),
            ("5.e3..", 5000.0),
        ] {
            let mut lx = lexer_over(src);
            assert_eq!(lx.lex().unwrap(), TK_FLOAT, "{}", src);
            assert_eq!(lx.fvalue, want as SQFloat, "{}", src);
            // 剩下的部分不该再产生数字 token
            assert_ne!(lx.lex().unwrap(), TK_FLOAT, "{}: 尾巴", src);
        }
    }

    #[test]
    fn strings_and_escapes() {
        let mut lx = lexer_over("\"a\\tb\\x41\\u00e9\" 'z' @\"raw\\n\"");
        assert_eq!(lx.lex().unwrap(), TK_STRING_LITERAL);
        assert_eq!(lx.svalue(), b"a\tbA\xc3\xa9");
        assert_eq!(lx.lex().unwrap(), TK_INTEGER);
        assert_eq!(lx.nvalue, b'z' as SQInteger);
        assert_eq!(lx.lex().unwrap(), TK_STRING_LITERAL);
        assert_eq!(lx.svalue(), b"raw\\n");
    }

    #[test]
    fn comments_and_lines() {
        let toks = lex_all("// line\n# also line\n/* block\ncomment */ local");
        assert_eq!(toks, vec![TK_LOCAL]);
        let mut lx = lexer_over("a\nb");
        lx.lex().unwrap();
        assert_eq!(lx.currentline, 1);
        lx.lex().unwrap();
        assert_eq!(lx.currentline, 2);
    }

    #[test]
    fn unterminated_string_is_error() {
        let mut lx = lexer_over("\"abc");
        assert!(lx.lex().is_err());
    }
}

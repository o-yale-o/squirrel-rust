//! ↔ sqstdlib/sqstdrex.cpp —— regexp 类。
//!
//! 引擎部分是对 `sqstdrex.cpp` 的**逐行移植**（节点数组 + 回溯匹配），
//! 包括原引擎那些"不像正则"的怪癖，因为脚本能观察到它们：
//!
//! * **贪婪量词其实是"停在第一个能继续的位置"**：`OP_GREEDY` 每吃掉一个字符就试一次
//!   后续节点，一旦后续能匹配就立刻返回，所以 `regexp("<.*>").search("<a><b>")`
//!   给出 `{0,3}`（"<a>"）而不是 `{0,6}`；
//! * `\b` 不是词边界，而是 **空格与非空格之间的转移**（且只看 `*str` 与 `*(str+1)`）；
//! * `search` 每次尝试失败后**不回退到起始节点**（`node` 变量在 do-while 外），
//!   所以"部分匹配后失败"会让起点扫描从中间节点继续；
//! * `^` 锚定的是**本次 search 的 start 偏移**（`_bol = text_begin`），不是字符串开头；
//! * 空主语直接返回不匹配（`if(text_begin >= text_end) return SQFalse`）；
//! * `?` 紧跟量词时是**字面量** `?`（引擎没有惰性量词语法）；
//! * `[a-]` 之类不会报 `unfinished range`（那句检查写在 `*_p++ == ']'` 上，恒假）。
//!
//! 与参考实现的差异只有**安全阀**（参考实现在这些点上会死循环/爆栈，不是可观察语义）：
//! 匹配步数与递归深度上限（见 `MAX_RE_STEPS` / `MAX_RE_DEPTH`）、
//! `start > len` 时返回不匹配（参考实现是指针越界）、高位字节的 ctype 判定按 ASCII 处理
//! （C 的 `isalpha((char)0xE9)` 是负值下标，属 UB）。

use std::rc::Rc;

use squirrel::config::*;
use squirrel::object::Value;
use squirrel::vm::VmData;

// ---------------------------------------------------------------- 常量
// ↔ sqconfig.h（非 UNICODE 构建）：MAX_CHAR = 0xFF，scisprint = isprint
const MAX_CHAR: i32 = 0xFF;
const OP_GREEDY: i32 = MAX_CHAR + 1;
const OP_OR: i32 = MAX_CHAR + 2;
const OP_EXPR: i32 = MAX_CHAR + 3;
const OP_NOCAPEXPR: i32 = MAX_CHAR + 4;
const OP_DOT: i32 = MAX_CHAR + 5;
const OP_CLASS: i32 = MAX_CHAR + 6;
const OP_CCLASS: i32 = MAX_CHAR + 7;
const OP_NCLASS: i32 = MAX_CHAR + 8;
const OP_RANGE: i32 = MAX_CHAR + 9;
const OP_EOL: i32 = MAX_CHAR + 11;
const OP_BOL: i32 = MAX_CHAR + 12;
const OP_WB: i32 = MAX_CHAR + 13;
const OP_MB: i32 = MAX_CHAR + 14;

const SYM_ANY: u8 = b'.';
const SYM_PLUS: u8 = b'+';
const SYM_STAR: u8 = b'*';
const SYM_QUEST: u8 = b'?';
const SYM_BRANCH: u8 = b'|';
const SYM_EOS: u8 = b'$';
const SYM_BOS: u8 = b'^';
const SYM_ESC: u8 = b'\\';

/// 递归层数上限（**次要**保险；真正的限制是下面的栈预算）
const MAX_RE_DEPTH: u32 = 100_000;
/// 递归栈预算：用局部变量地址差估算已用栈深，**比固定层数可靠**——
/// 同一段代码在不同线程上的栈大小并不一样（Rust 主线程默认 1MB；参考实现
/// 实测 4000 层能编、8000 层 `0xC00000FD`）。超过预算就给可捕获错误，
/// 而不是把宿主栈打穿。
///
/// 默认值按"最小的常见栈"取（1MB 主线程 ⇒ 384KB 预算）；CLI 自己会在大栈线程上
/// 调 `set_stack_budget` 放大它（见 `crates/sq`），因此命令行下与参考实现同量级。
static RE_STACK_BUDGET: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(384 * 1024);

/// 设置正则递归栈预算（会在**大栈线程**里执行的宿主可以调大）
pub fn set_stack_budget(bytes: usize) {
    RE_STACK_BUDGET.store(bytes, std::sync::atomic::Ordering::Relaxed);
}

/// 匹配步数上限：参考实现在 `(a*)*` 这类空匹配量词上会**死循环**
const MAX_RE_STEPS: u64 = 4_000_000;

/// 当前栈指针的近似值（只用地址差，不做解引用）
#[inline]
fn stack_probe() -> usize {
    let probe = 0u8;
    &probe as *const u8 as usize
}

/// 编译期栈预算检查
fn stack_left(base: usize) -> bool {
    base.saturating_sub(stack_probe()) < RE_STACK_BUDGET.load(std::sync::atomic::Ordering::Relaxed)
}

// ---------------------------------------------------------------- ctype（C locale）
fn scisprint(c: u8) -> bool {
    (0x20..0x7F).contains(&c)
}

fn scisspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

fn scisdigit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn scisalpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

fn scisalnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn scisxdigit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

fn sciscntrl(c: u8) -> bool {
    c < 0x20 || c == 0x7F
}

fn scispunct(c: u8) -> bool {
    scisprint(c) && !c.is_ascii_alphanumeric() && !scisspace(c)
}

// ---------------------------------------------------------------- 节点
#[derive(Clone, Copy, Default)]
struct Node {
    ty: i32,
    left: i32,
    right: i32,
    next: i32,
}

pub struct Regex {
    nodes: Vec<Node>,
    first: i32,
    /// ↔ `_nsubexpr`（含整体匹配的 0 号组）
    nsubexpr: usize,
}

// ---------------------------------------------------------------- 编译（↔ sqstdrex_compile）
struct Comp {
    p: Vec<u8>,
    i: usize,
    nodes: Vec<Node>,
    nsubexpr: i32,
    depth: u32,
    /// 编译开始时的栈指针（见 `RE_STACK_BUDGET`）
    stack_base: usize,
}

impl Comp {
    /// 模式字节（末尾隐含 NUL：越界读返回 0，↔ C 字符串终止符）
    fn cur(&self) -> u8 {
        if self.i < self.p.len() {
            self.p[self.i]
        } else {
            0
        }
    }

    fn newnode(&mut self, ty: i32) -> i32 {
        let mut n = Node { ty, left: -1, right: -1, next: -1 };
        if ty == OP_EXPR {
            n.right = self.nsubexpr;
            self.nsubexpr += 1;
        }
        self.nodes.push(n);
        (self.nodes.len() - 1) as i32
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.cur() != c {
            return Err("expected paren".to_string());
        }
        self.i += 1;
        Ok(())
    }

    /// ↔ sqstd_rex_escapechar
    fn escapechar(&mut self) -> Result<u8, String> {
        if self.cur() == SYM_ESC {
            self.i += 1;
            match self.cur() {
                b'v' => {
                    self.i += 1;
                    Ok(0x0B)
                }
                b'n' => {
                    self.i += 1;
                    Ok(b'\n')
                }
                b't' => {
                    self.i += 1;
                    Ok(b'\t')
                }
                b'r' => {
                    self.i += 1;
                    Ok(b'\r')
                }
                b'f' => {
                    self.i += 1;
                    Ok(0x0C)
                }
                _ => {
                    let c = self.cur();
                    self.i += 1;
                    Ok(c)
                }
            }
        } else {
            if !scisprint(self.cur()) {
                return Err("letter expected".to_string());
            }
            let c = self.cur();
            self.i += 1;
            Ok(c)
        }
    }

    fn charclass(&mut self, id: u8) -> i32 {
        let n = self.newnode(OP_CCLASS);
        self.nodes[n as usize].left = id as i32;
        n
    }

    /// ↔ sqstd_rex_charnode
    fn charnode(&mut self, isclass: bool) -> Result<i32, String> {
        if self.cur() == SYM_ESC {
            self.i += 1;
            match self.cur() {
                b'n' => {
                    self.i += 1;
                    return Ok(self.newnode(b'\n' as i32));
                }
                b't' => {
                    self.i += 1;
                    return Ok(self.newnode(b'\t' as i32));
                }
                b'r' => {
                    self.i += 1;
                    return Ok(self.newnode(b'\r' as i32));
                }
                b'f' => {
                    self.i += 1;
                    return Ok(self.newnode(0x0C));
                }
                b'v' => {
                    self.i += 1;
                    return Ok(self.newnode(0x0B));
                }
                b'a' | b'A' | b'w' | b'W' | b's' | b'S' | b'd' | b'D' | b'x' | b'X' | b'c'
                | b'C' | b'p' | b'P' | b'l' | b'u' => {
                    let t = self.cur();
                    self.i += 1;
                    return Ok(self.charclass(t));
                }
                b'm' => {
                    self.i += 1;
                    let cb = self.cur();
                    self.i += 1;
                    let ce = self.cur();
                    self.i += 1;
                    if cb == 0 || ce == 0 {
                        return Err("balanced chars expected".to_string());
                    }
                    if cb == ce {
                        return Err("open/close char can't be the same".to_string());
                    }
                    let node = self.newnode(OP_MB);
                    self.nodes[node as usize].left = cb as i32;
                    self.nodes[node as usize].right = ce as i32;
                    return Ok(node);
                }
                0 => return Err("letter expected for argument of escape sequence".to_string()),
                b'b' | b'B' if !isclass => {
                    // 类里 \b 是普通字符 b/B
                    let node = self.newnode(OP_WB);
                    self.nodes[node as usize].left = self.cur() as i32;
                    self.i += 1;
                    return Ok(node);
                }
                _ => {
                    let t = self.cur();
                    self.i += 1;
                    return Ok(self.newnode(t as i32));
                }
            }
        } else if !scisprint(self.cur()) {
            return Err("letter expected".to_string());
        }
        let t = self.cur();
        self.i += 1;
        Ok(self.newnode(t as i32))
    }

    /// ↔ sqstd_rex_class
    fn class_(&mut self) -> Result<i32, String> {
        let ret = if self.cur() == SYM_BOS {
            let r = self.newnode(OP_NCLASS);
            self.i += 1;
            r
        } else {
            self.newnode(OP_CLASS)
        };
        if self.cur() == b']' {
            return Err("empty class".to_string());
        }
        let mut first: i32 = -1;
        let mut chain = ret;
        // ↔ `while(*exp->_p != ']' && exp->_p != exp->_eol)`：编译期 `_eol` 是 NULL，
        // 所以第二个条件恒真——模式在类里结束时由 charnode 对 NUL 报 "letter expected"
        while self.cur() != b']' {
            if self.cur() == b'-' && first != -1 {
                // ↔ `if(*exp->_p++ == ']') "unfinished range"`：这句读到的恒是 '-'，
                // 所以原版的 "unfinished range" 是死代码，这里照搬（只跳过 '-'）
                self.i += 1;
                let r = self.newnode(OP_RANGE);
                if self.nodes[first as usize].ty > self.cur() as i32 {
                    return Err("invalid range".to_string());
                }
                if self.nodes[first as usize].ty == OP_CCLASS {
                    return Err("cannot use character classes in ranges".to_string());
                }
                self.nodes[r as usize].left = self.nodes[first as usize].ty;
                let t = self.escapechar()?;
                self.nodes[r as usize].right = t as i32;
                self.nodes[chain as usize].next = r;
                chain = r;
                first = -1;
            } else if first != -1 {
                let c = first;
                self.nodes[chain as usize].next = c;
                chain = c;
                first = self.charnode(true)?;
            } else {
                first = self.charnode(true)?;
            }
        }
        if first != -1 {
            let c = first;
            self.nodes[chain as usize].next = c;
        }
        // /* hack? */（原版注释）
        self.nodes[ret as usize].left = self.nodes[ret as usize].next;
        self.nodes[ret as usize].next = -1;
        Ok(ret)
    }

    /// ↔ sqstd_rex_parsenumber（含 "overflow in numeric constant"）
    fn parsenumber(&mut self) -> Result<i32, String> {
        let mut ret = self.cur() as i32 - b'0' as i32;
        let mut positions: i64 = 10;
        self.i += 1;
        while scisdigit(self.cur()) {
            ret = ret * 10 + (self.cur() as i32 - b'0' as i32);
            self.i += 1;
            if positions == 1000000000 {
                return Err("overflow in numeric constant".to_string());
            }
            positions *= 10;
        }
        Ok(ret)
    }

    /// ↔ sqstd_rex_element（= 一个元素 + 量词包装）
    fn element_body(&mut self) -> Result<i32, String> {
        let mut ret;
        match self.cur() {
            b'(' => {
                self.i += 1;
                let expr;
                if self.cur() == b'?' {
                    self.i += 1;
                    self.expect(b':')?;
                    expr = self.newnode(OP_NOCAPEXPR);
                } else {
                    expr = self.newnode(OP_EXPR);
                }
                let newn = self.list()?;
                self.nodes[expr as usize].left = newn;
                ret = expr;
                self.expect(b')')?;
            }
            b'[' => {
                self.i += 1;
                ret = self.class_()?;
                self.expect(b']')?;
            }
            SYM_EOS => {
                self.i += 1;
                ret = self.newnode(OP_EOL);
            }
            SYM_ANY => {
                self.i += 1;
                ret = self.newnode(OP_DOT);
            }
            _ => {
                ret = self.charnode(false)?;
            }
        }

        let mut isgreedy = false;
        let mut p0: u32 = 0;
        let mut p1: u32 = 0;
        match self.cur() {
            SYM_STAR => {
                p0 = 0;
                p1 = 0xFFFF;
                self.i += 1;
                isgreedy = true;
            }
            SYM_PLUS => {
                p0 = 1;
                p1 = 0xFFFF;
                self.i += 1;
                isgreedy = true;
            }
            SYM_QUEST => {
                p0 = 0;
                p1 = 1;
                self.i += 1;
                isgreedy = true;
            }
            b'{' => {
                self.i += 1;
                if !scisdigit(self.cur()) {
                    return Err("number expected".to_string());
                }
                p0 = (self.parsenumber()? as u32) & 0xFFFF;
                match self.cur() {
                    b'}' => {
                        p1 = p0;
                        self.i += 1;
                    }
                    b',' => {
                        self.i += 1;
                        p1 = 0xFFFF;
                        if scisdigit(self.cur()) {
                            p1 = (self.parsenumber()? as u32) & 0xFFFF;
                        }
                        self.expect(b'}')?;
                    }
                    _ => return Err(", or } expected".to_string()),
                }
                isgreedy = true;
            }
            _ => {}
        }
        if isgreedy {
            let nnode = self.newnode(OP_GREEDY);
            self.nodes[nnode as usize].left = ret;
            self.nodes[nnode as usize].right = (((p0 & 0xFFFF) << 16) | (p1 & 0xFFFF)) as i32;
            ret = nnode;
        }

        // ↔ 原版这里是**尾递归** `nnode = element(); nodes[ret].next = nnode;`
        // （字符链按 `next` 串起来）。改成循环后，递归深度只与**真正的嵌套**
        // （括号/量词）相关：5000 字的字面量模式不会再消耗 5000 层栈。
        let mut tail = ret;
        while self.cur() != SYM_BRANCH
            && self.cur() != b')'
            && self.cur() != SYM_STAR
            && self.cur() != SYM_PLUS
            && self.cur() != 0
        {
            let nnode = self.element_body()?;
            self.nodes[tail as usize].next = nnode;
            tail = nnode;
        }
        Ok(ret)
    }

    /// ↔ 原版 `element()` 的入口：带递归深度保护（真正的嵌套才增加深度）
    fn element(&mut self) -> Result<i32, String> {
        self.depth += 1;
        if self.depth > MAX_RE_DEPTH || !stack_left(self.stack_base) {
            self.depth -= 1;
            return Err("regexp too deeply nested".to_string());
        }
        let r = self.element_body();
        self.depth -= 1;
        r
    }

    /// ↔ sqstd_rex_list
    fn list(&mut self) -> Result<i32, String> {
        let mut ret: i32 = -1;
        if self.cur() == SYM_BOS {
            self.i += 1;
            ret = self.newnode(OP_BOL);
        }
        let e = self.element()?;
        if ret != -1 {
            self.nodes[ret as usize].next = e;
        } else {
            ret = e;
        }
        if self.cur() == SYM_BRANCH {
            self.i += 1;
            let temp = self.newnode(OP_OR);
            self.nodes[temp as usize].left = ret;
            let tright = self.list()?;
            self.nodes[temp as usize].right = tright;
            ret = temp;
        }
        Ok(ret)
    }
}

// ---------------------------------------------------------------- 匹配（↔ sqstd_rex_matchnode）
struct M<'a> {
    nodes: &'a [Node],
    text: &'a [u8],
    bol: usize,
    eol: usize,
    currsubexp: i32,
    /// (begin, len)；len == 0 表示"没匹配上"（↔ 原版 memset 0 + `match.len > 0` 判定）
    matches: Vec<(usize, usize)>,
    depth: u32,
    steps: u64,
    aborted: bool,
    /// 匹配开始时的栈指针（见 `RE_STACK_BUDGET`）
    stack_base: usize,
}

impl<'a> M<'a> {
    fn new(rx: &'a Regex, text: &'a [u8], bol: usize, eol: usize) -> M<'a> {
        M {
            nodes: &rx.nodes,
            text,
            bol,
            eol,
            currsubexp: 0,
            matches: vec![(0, 0); rx.nsubexpr],
            depth: 0,
            steps: 0,
            aborted: false,
            stack_base: stack_probe(),
        }
    }

    /// 文本第 i 字节（越界 = 隐含 NUL，↔ C 字符串终止符）
    fn ch(&self, i: usize) -> u8 {
        if i < self.text.len() {
            self.text[i]
        } else {
            0
        }
    }

    /// 带符号访问（`*(str-1)`；i < 0 时给 0，参考实现在那一点是越界读）
    fn at(&self, i: i64) -> u8 {
        if i < 0 {
            0
        } else {
            self.ch(i as usize)
        }
    }

    fn node(&self, i: i32) -> &Node {
        &self.nodes[i as usize]
    }

    fn matchcclass(cclass: i32, c: u8) -> bool {
        match cclass as u8 {
            b'a' => scisalpha(c),
            b'A' => !scisalpha(c),
            b'w' => scisalnum(c) || c == b'_',
            b'W' => !scisalnum(c) && c != b'_',
            b's' => scisspace(c),
            b'S' => !scisspace(c),
            b'd' => scisdigit(c),
            b'D' => !scisdigit(c),
            b'x' => scisxdigit(c),
            b'X' => !scisxdigit(c),
            b'c' => sciscntrl(c),
            b'C' => !sciscntrl(c),
            b'p' => scispunct(c),
            b'P' => !scispunct(c),
            b'l' => c.is_ascii_lowercase(),
            b'u' => c.is_ascii_uppercase(),
            _ => false,
        }
    }

    fn matchclass(&self, mut idx: i32, c: u8) -> bool {
        loop {
            let n = self.node(idx);
            match n.ty {
                OP_RANGE => {
                    // 原版是 `char` 与 SQInteger 比较：高位字节（负值）落在任何范围外
                    let sc = c as i8 as i32;
                    if sc >= n.left && sc <= n.right {
                        return true;
                    }
                }
                OP_CCLASS => {
                    if Self::matchcclass(n.left, c) {
                        return true;
                    }
                }
                _ => {
                    if c as i8 as i32 == n.ty {
                        return true;
                    }
                }
            }
            if n.next != -1 {
                idx = n.next;
            } else {
                return false;
            }
        }
    }

    fn matchnode(&mut self, ni: i32, s: usize, next: Option<i32>) -> Option<usize> {
        if self.aborted {
            return None;
        }
        self.steps += 1;
        if self.steps > MAX_RE_STEPS {
            self.aborted = true;
            return None;
        }
        self.depth += 1;
        if self.depth > MAX_RE_DEPTH || !stack_left(self.stack_base) {
            self.depth -= 1;
            self.aborted = true;
            return None;
        }
        let r = self.matchnode_inner(ni, s, next);
        self.depth -= 1;
        r
    }

    fn matchnode_inner(&mut self, ni: i32, s: usize, next: Option<i32>) -> Option<usize> {
        let node = *self.node(ni);
        match node.ty {
            OP_GREEDY => {
                let p0 = ((node.right >> 16) & 0xFFFF) as u32;
                let p1 = (node.right & 0xFFFF) as u32;
                let mut nmaches: u32 = 0;
                let mut str_ = s;
                let mut good = s;
                let greedystop: Option<i32> = if node.next != -1 { Some(node.next) } else { next };

                while nmaches == 0xFFFF || nmaches < p1 {
                    match self.matchnode(node.left, str_, greedystop) {
                        Some(ns) => str_ = ns,
                        None => break,
                    }
                    nmaches += 1;
                    good = str_;
                    if let Some(gs) = greedystop {
                        let gsn = *self.node(gs);
                        if gsn.ty != OP_GREEDY
                            || (gsn.ty == OP_GREEDY && ((gsn.right >> 16) & 0xFFFF) != 0)
                        {
                            let gnext = if gsn.next != -1 {
                                Some(gsn.next)
                            } else if let Some(nx) = next {
                                let nxn = *self.node(nx);
                                if nxn.next != -1 {
                                    Some(nxn.next)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            if self.matchnode(gs, str_, gnext).is_some() {
                                if p0 == p1 && p0 == nmaches {
                                    break;
                                } else if nmaches >= p0 && p1 == 0xFFFF {
                                    break;
                                } else if nmaches >= p0 && nmaches <= p1 {
                                    break;
                                }
                            }
                        }
                    }
                    if str_ >= self.eol {
                        break;
                    }
                }
                if p0 == p1 && p0 == nmaches {
                    return Some(good);
                } else if nmaches >= p0 && p1 == 0xFFFF {
                    return Some(good);
                } else if nmaches >= p0 && nmaches <= p1 {
                    return Some(good);
                }
                None
            }
            OP_OR => {
                let mut temp = node.left;
                let mut asd = s;
                loop {
                    match self.matchnode(temp, asd, None) {
                        Some(n) => asd = n,
                        None => break,
                    }
                    let tn = *self.node(temp);
                    if tn.next != -1 {
                        temp = tn.next;
                    } else {
                        return Some(asd);
                    }
                }
                asd = s;
                temp = node.right;
                loop {
                    match self.matchnode(temp, asd, None) {
                        Some(n) => asd = n,
                        None => break,
                    }
                    let tn = *self.node(temp);
                    if tn.next != -1 {
                        temp = tn.next;
                    } else {
                        return Some(asd);
                    }
                }
                None
            }
            OP_EXPR | OP_NOCAPEXPR => {
                let mut n = node.left;
                let mut cur = s;
                let mut capture: i32 = -1;
                if node.ty != OP_NOCAPEXPR && node.right == self.currsubexp {
                    capture = self.currsubexp;
                    if let Some(slot) = self.matches.get_mut(capture as usize) {
                        *slot = (cur, 0);
                    }
                    self.currsubexp += 1;
                }
                let tempcap = self.currsubexp;
                loop {
                    let subnext = {
                        let nn = *self.node(n);
                        if nn.next != -1 {
                            Some(nn.next)
                        } else {
                            next
                        }
                    };
                    match self.matchnode(n, cur, subnext) {
                        Some(nc) => cur = nc,
                        None => {
                            if capture != -1 {
                                if let Some(slot) = self.matches.get_mut(capture as usize) {
                                    *slot = (0, 0);
                                }
                            }
                            return None;
                        }
                    }
                    let nn = *self.node(n);
                    if nn.next != -1 {
                        n = nn.next;
                    } else {
                        break;
                    }
                }
                self.currsubexp = tempcap;
                if capture != -1 {
                    if let Some(slot) = self.matches.get_mut(capture as usize) {
                        let b = slot.0;
                        slot.1 = cur.saturating_sub(b);
                    }
                }
                Some(cur)
            }
            OP_WB => {
                let c0 = self.ch(s);
                let c1 = self.ch(s + 1);
                let cm1 = self.at(s as i64 - 1);
                let cond = (s == self.bol && !scisspace(c0))
                    || (s == self.eol && !scisspace(cm1))
                    || (!scisspace(c0) && scisspace(c1))
                    || (scisspace(c0) && !scisspace(c1));
                if cond {
                    if node.left == b'b' as i32 {
                        return Some(s);
                    }
                    return None;
                }
                if node.left == b'b' as i32 {
                    return None;
                }
                Some(s)
            }
            OP_BOL => {
                if s == self.bol {
                    Some(s)
                } else {
                    None
                }
            }
            OP_EOL => {
                if s == self.eol {
                    Some(s)
                } else {
                    None
                }
            }
            OP_DOT => {
                if s == self.eol {
                    return None;
                }
                Some(s + 1)
            }
            OP_NCLASS | OP_CLASS => {
                if s == self.eol {
                    return None;
                }
                let m = self.matchclass(node.left, self.ch(s));
                let ok = if m { node.ty == OP_CLASS } else { node.ty == OP_NCLASS };
                if ok {
                    Some(s + 1)
                } else {
                    None
                }
            }
            OP_CCLASS => {
                if s == self.eol {
                    return None;
                }
                if Self::matchcclass(node.left, self.ch(s)) {
                    Some(s + 1)
                } else {
                    None
                }
            }
            OP_MB => {
                let cb = node.left as u8;
                let ce = node.right as u8;
                if self.ch(s) != cb {
                    return None;
                }
                let mut cont: i32 = 1;
                let mut str_ = s;
                loop {
                    str_ += 1;
                    if str_ >= self.eol {
                        break;
                    }
                    let c = self.ch(str_);
                    if c == ce {
                        cont -= 1;
                        if cont == 0 {
                            return Some(str_ + 1);
                        }
                    } else if c == cb {
                        cont += 1;
                    }
                }
                None
            }
            _ => {
                // 字面字符
                if s == self.eol {
                    return None;
                }
                if self.ch(s) as i8 as i32 != node.ty {
                    return None;
                }
                Some(s + 1)
            }
        }
    }
}

// ---------------------------------------------------------------- 公开 API
impl Regex {
    pub fn compile(pattern: &[u8]) -> Result<Regex, String> {
        let mut c = Comp {
            p: pattern.to_vec(),
            i: 0,
            nodes: Vec::new(),
            nsubexpr: 0,
            depth: 0,
            stack_base: stack_probe(),
        };
        let first = c.newnode(OP_EXPR);
        let res = c.list()?;
        c.nodes[first as usize].left = res;
        if c.cur() != 0 {
            return Err("unexpected character".to_string());
        }
        Ok(Regex { nodes: c.nodes, first, nsubexpr: c.nsubexpr as usize })
    }

    /// ↔ sqstd_rex_search(str+start)：`_bol` = start 偏移、`_eol` = 字符串结尾。
    /// 返回 (begin, end, captures) 都是**相对整个 subject** 的下标。
    pub fn search(
        &self,
        text: &[u8],
        start: usize,
    ) -> Option<(usize, usize, Vec<(usize, usize)>)> {
        // 参考实现在 start > len 时做的是越界指针运算（UB），这里给确定性的"不匹配"
        if start > text.len() {
            return None;
        }
        let mut text_begin = start;
        let text_end = text.len();
        if text_begin >= text_end {
            return None;
        }
        let mut m = M::new(self, text, start, text_end);
        let mut node = self.first;
        let mut cur: Option<usize>;
        loop {
            cur = Some(text_begin);
            while node != -1 {
                m.currsubexp = 0;
                match m.matchnode(node, cur.unwrap_or(text_begin), None) {
                    Some(nc) => cur = Some(nc),
                    None => {
                        cur = None;
                        break;
                    }
                }
                node = self.nodes[node as usize].next;
            }
            text_begin += 1;
            if !(cur.is_none() && text_begin != text_end) {
                break;
            }
        }
        if m.aborted {
            return None;
        }
        match cur {
            Some(end) => {
                let begin = text_begin - 1;
                Some((begin, end, m.matches))
            }
            None => None,
        }
    }

    /// ↔ sqstd_rex_match：整串匹配（锚定两端）
    pub fn match_full(&self, text: &[u8]) -> Option<(usize, usize, Vec<(usize, usize)>)> {
        let mut m = M::new(self, text, 0, text.len());
        m.currsubexp = 0;
        let res = m.matchnode(self.first, 0, None);
        if m.aborted {
            return None;
        }
        match res {
            Some(e) if e == text.len() => Some((0, e, m.matches)),
            _ => None,
        }
    }

    pub fn ngroups(&self) -> usize {
        self.nsubexpr.saturating_sub(1)
    }

    pub fn subexpcount(&self) -> usize {
        self.nsubexpr
    }
}

thread_local! {
    static REGEXPS: std::cell::RefCell<Vec<Rc<Regex>>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn get_rx(v: &VmData, idx: SQInteger) -> Option<Rc<Regex>> {
    let o = v.stack_get(idx);
    if let Value::Instance(inst) = &o {
        // __rxid 必须是**整数**且在注册表范围内：null/其它类型/越界一律当作无效对象，
        // 不允许 as_integer() 把 null 当成 0 去取表（那会取到别人的正则）
        let idv = inst.borrow().get(&v.ss.intern(b"__rxid"))?;
        let id = match idv {
            Value::Integer(i) if i >= 0 => i as usize,
            _ => return None,
        };
        return REGEXPS.with(|r| r.borrow().get(id).cloned());
    }
    None
}

pub fn make_regexp_class(ss: &std::rc::Rc<squirrel::state::SharedState>) -> Value {
    let cls = squirrel::class::SQClass::create(ss, None);
    let rxid_key = ss.intern(b"__rxid");

    // 字段：__rxid（REGEXPS 注册表索引）。默认 **null** = 还没构造过
    // （↔ 参考实现用实例 userpointer 判 self != NULL；本实现用这个隐藏字段代替）
    cls.borrow_mut().new_slot(ss, &rxid_key, &Value::Null, false);

    macro_rules! method {
        ($name:expr, $npc:expr, $tm:expr, $f:expr) => {
            let nc = squirrel::closure::SQNativeClosure::create(ss, Rc::new($f), 0);
            {
                let mut n = nc.borrow_mut();
                n.nparamscheck = $npc;
                n.name = ss.intern($name.as_bytes());
                if let Some(tm) = $tm {
                    squirrel::state::compile_typemask(&mut n.typecheck, tm.as_bytes());
                }
            }
            let key = ss.intern($name.as_bytes());
            cls.borrow_mut().new_slot(ss, &key, &Value::NativeClosure(nc), true);
        };
    }

    // ↔ rexobj_funcs：constructor,2,.s / search,-2,.xsn / match,2,.xs / capture,-2,.xsn /
    //                subexpcount,1,.x / _typeof,1,.x
    method!("constructor", 2, Some(".s"), move |v: &mut VmData| {
        // ↔ _regexp_constructor：实例已经构造过（self != NULL）时直接报错，
        // 而不是重新编译——`regexp("a").constructor("s")` 在参考实现里是
        // "invalid regexp object"，不是语法错误
        {
            let self_ = v.stack_get(1);
            if let Value::Instance(inst) = &self_ {
                let cur = inst.borrow().get(&rxid_key).unwrap_or(Value::Null);
                if !cur.is_null() {
                    v.raise_error_str("invalid regexp object");
                    return SQ_ERROR;
                }
            }
        }
        let pat = v.stack_get(2);
        match Regex::compile(pat.as_bytes()) {
            Ok(rx) => {
                let id = REGEXPS.with(|r| {
                    let mut b = r.borrow_mut();
                    b.push(Rc::new(rx));
                    b.len() - 1
                });
                // this.__rxid = id（实例字段赋值）
                let self_ = v.stack_get(1);
                if let Value::Instance(inst) = &self_ {
                    inst.borrow_mut().set(&rxid_key, &Value::Integer(id as SQInteger));
                }
                0
            }
            Err(msg) => {
                v.raise_error_str(&msg);
                SQ_ERROR
            }
        }
    });

    method!("search", -2, Some("xsn"), |v: &mut VmData| {
        let rx = match get_rx(v, 1) {
            Some(r) => r,
            None => return SQ_ERROR,
        };
        let str_ = v.stack_get(2);
        let mut start = 0usize;
        if v.top - v.stackbase > 2 {
            start = v.stack_get(3).as_integer().max(0) as usize;
        }
        match rx.search(str_.as_bytes(), start) {
            Some((b, e, _caps)) => {
                let t = squirrel::table::SQTable::create(&v.ss, 2);
                t.borrow_mut().new_slot(&v.ss.intern(b"begin"), &Value::Integer(b as SQInteger));
                t.borrow_mut().new_slot(&v.ss.intern(b"end"), &Value::Integer(e as SQInteger));
                v.push(Value::Table(t));
                1
            }
            None => 0,
        }
    });

    method!("capture", -2, Some("xsn"), |v: &mut VmData| {
        let rx = match get_rx(v, 1) {
            Some(r) => r,
            None => return SQ_ERROR,
        };
        let str_ = v.stack_get(2);
        let mut start = 0usize;
        if v.top - v.stackbase > 2 {
            start = v.stack_get(3).as_integer().max(0) as usize;
        }
        match rx.search(str_.as_bytes(), start) {
            Some((_b, _e, caps)) => {
                let arr = squirrel::array::SQArray::create(&v.ss, caps.len() as SQInteger);
                for (i, c) in caps.iter().enumerate() {
                    // ↔ match.len > 0 ? 实际区间 : 空匹配（{begin:0,end:0}），不是 null
                    let (s, e) = if c.1 > 0 { (c.0, c.0 + c.1) } else { (0, 0) };
                    let t = squirrel::table::SQTable::create(&v.ss, 2);
                    t.borrow_mut().new_slot(&v.ss.intern(b"begin"), &Value::Integer(s as SQInteger));
                    t.borrow_mut().new_slot(&v.ss.intern(b"end"), &Value::Integer(e as SQInteger));
                    let _ = arr.borrow_mut().set(i as SQInteger, &Value::Table(t));
                }
                v.push(Value::Array(arr));
                1
            }
            None => 0,
        }
    });

    method!("match", 2, Some("xs"), |v: &mut VmData| {
        let rx = match get_rx(v, 1) {
            Some(r) => r,
            None => return SQ_ERROR,
        };
        let str_ = v.stack_get(2);
        v.push(Value::Bool(rx.match_full(str_.as_bytes()).is_some()));
        1
    });

    method!("subexpcount", 1, Some("x"), |v: &mut VmData| {
        let rx = match get_rx(v, 1) {
            Some(r) => r,
            None => return SQ_ERROR,
        };
        v.push(Value::Integer(rx.subexpcount() as SQInteger));
        1
    });

    // ↔ rexobj_funcs 的 _typeof：typeof regexp 实例得到 "regexp"
    method!("_typeof", 1, Some("x"), |v: &mut VmData| {
        let s = v.ss.intern(b"regexp");
        v.push(s);
        1
    });

    Value::Class(cls)
}

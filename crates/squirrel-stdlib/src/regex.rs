//! ↔ sqstdlib/sqstdrex.cpp —— regexp 类。
//!
//! 原版是自包含回溯正则引擎；此处用等价的 Rust 回溯实现，
//! 支持相同语法：字面量、`.`、`[...]`（含范围/取反）、`\d \D \w \W \s \S`、
//! 量词 `* + ?`、分组 `()`（捕获）、 alternation `|`、锚 `^ $`。

use std::rc::Rc;

use squirrel::config::*;
use squirrel::object::Value;
use squirrel::vm::VmData;

#[derive(Clone, Debug)]
enum Node {
    Char(u8),
    Any,
    Class { neg: bool, ranges: Vec<(u8, u8)>, classes: Vec<u8> }, // classes: b'd' b'w' b's' 等
    Start,
    End,
    Group(Box<Node>, usize), // 捕获组（解析期预分配编号）
    NonCap(Box<Node>),      // (?:...)
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Star(Box<Node>, bool),  // greedy
    Plus(Box<Node>, bool),
    Quest(Box<Node>, bool),
    /// ↔ OP_MB：\m(cb)(ce) 嵌套平衡匹配
    MBalanced(u8, u8),
    /// ↔ OP_WB：\b / \B 词边界
    WordBoundary(bool),
}

pub struct Regex {
    root: Node,
    ngroups: usize,
}

struct Parser<'a> {
    pat: &'a [u8],
    pos: usize,
    ngroups: usize,
}

impl<'a> Parser<'a> {
    fn parse_alt(&mut self) -> Node {
        let mut branches = vec![self.parse_concat()];
        while self.pos < self.pat.len() && self.pat[self.pos] == b'|' {
            self.pos += 1;
            branches.push(self.parse_concat());
        }
        if branches.len() == 1 {
            branches.pop().unwrap()
        } else {
            Node::Alt(branches)
        }
    }

    fn parse_concat(&mut self) -> Node {
        let mut items = Vec::new();
        while self.pos < self.pat.len() && self.pat[self.pos] != b'|' && self.pat[self.pos] != b')' {
            let atom = self.parse_atom();
            let atom = self.parse_quant(atom);
            items.push(atom);
        }
        if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Node::Concat(items)
        }
    }

    fn parse_quant(&mut self, atom: Node) -> Node {
        if self.pos >= self.pat.len() {
            return atom;
        }
        let c = self.pat[self.pos];
        let mut greedy = true;
        match c {
            b'*' | b'+' | b'?' => {
                self.pos += 1;
                if self.pos < self.pat.len() && self.pat[self.pos] == b'?' {
                    greedy = false;
                    self.pos += 1;
                }
                match c {
                    b'*' => Node::Star(Box::new(atom), greedy),
                    b'+' => Node::Plus(Box::new(atom), greedy),
                    _ => Node::Quest(Box::new(atom), greedy),
                }
            }
            _ => atom,
        }
    }

    fn parse_atom(&mut self) -> Node {
        let c = self.pat[self.pos];
        match c {
            b'(' => {
                self.pos += 1;
                let noncap = self.pos + 1 < self.pat.len()
                    && self.pat[self.pos] == b'?'
                    && self.pat[self.pos + 1] == b':';
                if noncap {
                    self.pos += 2;
                } else {
                    self.ngroups += 1;
                }
                let gidx = self.ngroups;
                let inner = self.parse_alt();
                // 期待 ')'
                if self.pos < self.pat.len() && self.pat[self.pos] == b')' {
                    self.pos += 1;
                }
                if noncap {
                    Node::NonCap(Box::new(inner))
                } else {
                    Node::Group(Box::new(inner), gidx)
                }
            }
            b'[' => {
                self.pos += 1;
                let mut neg = false;
                if self.pos < self.pat.len() && self.pat[self.pos] == b'^' {
                    neg = true;
                    self.pos += 1;
                }
                let mut ranges = Vec::new();
                let mut classes = Vec::new();
                let mut first = true;
                while self.pos < self.pat.len() && (self.pat[self.pos] != b']' || first) {
                    first = false;
                    let lo = self.parse_class_char();
                    if lo == b'\\' {
                        // 类简写
                        let cls = self.pat.get(self.pos - 1).copied().unwrap_or(b'd');
                        classes.push(cls);
                        continue;
                    }
                    if self.pos + 1 < self.pat.len() && self.pat[self.pos] == b'-' && self.pat[self.pos + 1] != b']' {
                        self.pos += 1;
                        let hi = self.parse_class_char();
                        ranges.push((lo, hi));
                    } else {
                        ranges.push((lo, lo));
                    }
                }
                if self.pos < self.pat.len() {
                    self.pos += 1; // ']'
                }
                Node::Class { neg, ranges, classes }
            }
            b'.' => {
                self.pos += 1;
                Node::Any
            }
            b'^' => {
                self.pos += 1;
                Node::Start
            }
            b'$' => {
                self.pos += 1;
                Node::End
            }
            b'\\' => {
                self.pos += 1;
                let e = *self.pat.get(self.pos).unwrap_or(&b'\\');
                self.pos += 1;
                match e {
                    // ↔ sqstd_rex_charclass 的完整集合
                    b'a' | b'A' | b'w' | b'W' | b's' | b'S' | b'd' | b'D' | b'x' | b'X'
                    | b'c' | b'C' | b'p' | b'P' | b'l' | b'u' => Node::Class {
                        neg: false,
                        ranges: Vec::new(),
                        classes: vec![e],
                    },
                    b'm' => {
                        // ↔ \m(cb)(ce)：嵌套平衡匹配
                        let cb = *self.pat.get(self.pos).unwrap_or(&0);
                        let ce = *self.pat.get(self.pos + 1).unwrap_or(&0);
                        self.pos += 2;
                        Node::MBalanced(cb, ce)
                    }
                    b'b' => Node::WordBoundary(true),
                    b'B' => Node::WordBoundary(false),
                    b'n' => Node::Char(b'\n'),
                    b't' => Node::Char(b'\t'),
                    b'r' => Node::Char(b'\r'),
                    b'f' => Node::Char(0x0C),
                    b'v' => Node::Char(0x0B),
                    b'0' => Node::Char(0),
                    _ => Node::Char(e), // 其余（含已被 class 分支覆盖的）按字面字符

                }
            }
            _ => {
                self.pos += 1;
                Node::Char(c)
            }
        }
    }

    fn parse_class_char(&mut self) -> u8 {
        let c = self.pat[self.pos];
        if c == b'\\' && self.pos + 1 < self.pat.len() {
            self.pos += 2;
            // 标记：返回转义符本身，由调用方判断是否类简写
            let e = self.pat[self.pos - 1];
            match e {
                b'd' | b'w' | b's' | b'D' | b'W' | b'S' => b'\\',
                b'n' => b'\n',
                b't' => b'\t',
                b'r' => b'\r',
                other => other,
            }
        } else {
            self.pos += 1;
            c
        }
    }
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn class_matches(neg: bool, ranges: &[(u8, u8)], classes: &[u8], c: u8) -> bool {
    // ↔ sqstd_rex_matchcclass
    let cls_match = |cls: u8| -> bool {
        match cls {
            b'a' => c.is_ascii_alphabetic(),
            b'A' => !c.is_ascii_alphabetic(),
            b'w' => c.is_ascii_alphanumeric() || c == b'_',
            b'W' => !(c.is_ascii_alphanumeric() || c == b'_'),
            b's' => c.is_ascii_whitespace(),
            b'S' => !c.is_ascii_whitespace(),
            b'd' => c.is_ascii_digit(),
            b'D' => !c.is_ascii_digit(),
            b'x' => c.is_ascii_hexdigit(),
            b'X' => !c.is_ascii_hexdigit(),
            b'c' => c.is_ascii_control(),
            b'C' => !c.is_ascii_control(),
            b'p' => c.is_ascii_punctuation(),
            b'P' => !c.is_ascii_punctuation(),
            b'l' => c.is_ascii_lowercase(),
            b'u' => c.is_ascii_uppercase(),
            _ => false,
        }
    };
    let mut m = ranges.iter().any(|(lo, hi)| c >= *lo && c <= *hi);
    if !m {
        for cls in classes {
            m = cls_match(*cls);
            if m {
                break;
            }
        }
    }
    m != neg
}

/// 回溯匹配器：caps[i] = (start, end)
struct Matcher<'a> {
    text: &'a [u8],
    caps: Vec<Option<(usize, usize)>>,
}

impl<'a> Matcher<'a> {
    fn m(&mut self, node: &Node, pos: usize, k: &mut dyn FnMut(&mut Matcher<'a>, usize) -> bool) -> bool {
        match node {
            Node::Char(c) => {
                if pos < self.text.len() && self.text[pos] == *c {
                    k(self, pos + 1)
                } else {
                    false
                }
            }
            Node::Any => {
                if pos < self.text.len() {
                    k(self, pos + 1)
                } else {
                    false
                }
            }
            Node::Class { neg, ranges, classes } => {
                if pos < self.text.len() && class_matches(*neg, ranges, classes, self.text[pos]) {
                    k(self, pos + 1)
                } else {
                    false
                }
            }
            Node::Start => {
                if pos == 0 {
                    k(self, pos)
                } else {
                    false
                }
            }
            Node::End => {
                if pos == self.text.len() {
                    k(self, pos)
                } else {
                    false
                }
            }
            Node::Group(inner, gidx) => {
                let start = pos;
                self.m_node_capture(inner, *gidx, start, k)
            }
            Node::NonCap(inner) => self.m(inner, pos, k),
            Node::Concat(items) => self.m_concat(items, 0, pos, k),
            Node::Alt(branches) => {
                for b in branches {
                    if self.m(b, pos, k) {
                        return true;
                    }
                }
                false
            }
            Node::Star(inner, greedy) => self.m_repeat(inner, pos, 0, usize::MAX, *greedy, k),
            Node::Plus(inner, greedy) => self.m_repeat(inner, pos, 1, usize::MAX, *greedy, k),
            Node::Quest(inner, greedy) => self.m_repeat(inner, pos, 0, 1, *greedy, k),
            Node::MBalanced(cb, ce) => {
                // ↔ sqstd_rex_matchbalance：嵌套平衡
                if pos >= self.text.len() || self.text[pos] != *cb {
                    return false;
                }
                let mut cont = 1i64;
                let mut i = pos + 1;
                while i < self.text.len() {
                    let c = self.text[i];
                    if c == *ce {
                        cont -= 1;
                        if cont == 0 {
                            return k(self, i + 1);
                        }
                    } else if c == *cb {
                        cont += 1;
                    }
                    i += 1;
                }
                false
            }
            Node::WordBoundary(positive) => {
                let before = pos > 0 && is_word(self.text[pos - 1]);
                let after = pos < self.text.len() && is_word(self.text[pos]);
                let at_boundary = before != after;
                if at_boundary == *positive {
                    k(self, pos)
                } else {
                    false
                }
            }
        }
    }

    fn m_node_capture(
        &mut self,
        inner: &Node,
        gidx: usize,
        start: usize,
        k: &mut dyn FnMut(&mut Matcher<'a>, usize) -> bool,
    ) -> bool {
        let mut kont = |m: &mut Matcher<'a>, end: usize| {
            let old = m.caps[gidx];
            m.caps[gidx] = Some((start, end));
            if k(m, end) {
                true
            } else {
                m.caps[gidx] = old;
                false
            }
        };
        self.m(inner, start, &mut kont)
    }

    fn m_concat(
        &mut self,
        items: &[Node],
        idx: usize,
        pos: usize,
        k: &mut dyn FnMut(&mut Matcher<'a>, usize) -> bool,
    ) -> bool {
        if idx == items.len() {
            return k(self, pos);
        }
        let mut kont = |m: &mut Matcher<'a>, p: usize| -> bool {
            m.m_concat(items, idx + 1, p, k)
        };
        self.m(&items[idx], pos, &mut kont)
    }

    fn m_repeat(
        &mut self,
        inner: &Node,
        pos: usize,
        min: usize,
        max: usize,
        greedy: bool,
        k: &mut dyn FnMut(&mut Matcher<'a>, usize) -> bool,
    ) -> bool {
        // 先收集能匹配的所有延续长度（贪心：多优先；惰性：少优先）
        if greedy {
            let _me: &mut Matcher<'a> = self;
            let mut kont = |m: &mut Matcher<'a>, p: usize| {
                if p == pos {
                    return false; // 空匹配防死循环
                }
                m.m_repeat(inner, p, min.saturating_sub(1), max.saturating_sub(1), greedy, k)
            };
            if self.m(inner, pos, &mut kont) {
                return true;
            }
            if min == 0 {
                return k(self, pos);
            }
            false
        } else {
            if min == 0 && k(self, pos) {
                return true;
            }
            let _me: &mut Matcher<'a> = self;
            let mut kont = |m: &mut Matcher<'a>, p: usize| {
                if p == pos {
                    return false;
                }
                m.m_repeat(inner, p, min.saturating_sub(1), max.saturating_sub(1), greedy, k)
            };
            self.m(inner, pos, &mut kont)
        }
    }
}

// 手动补充 next_group 字段
impl<'a> Matcher<'a> {
    fn new(text: &'a [u8], ngroups: usize) -> Matcher<'a> {
        Matcher {
            text,
            caps: vec![None; ngroups + 1],
        }
    }
}

impl Regex {
    pub fn compile(pattern: &[u8]) -> Option<Regex> {
        let mut p = Parser {
            pat: pattern,
            pos: 0,
            ngroups: 0,
        };
        let root = p.parse_alt();
        if p.pos != pattern.len() {
            return None;
        }
        Some(Regex {
            root,
            ngroups: p.ngroups,
        })
    }

    /// 从 start 开始查找第一个匹配，返回 (match_start, match_end, caps)
    pub fn search(&self, text: &[u8], start: usize) -> Option<(usize, usize, Vec<Option<(usize, usize)>>)> {
        let mut s = start;
        loop {
            if s > text.len() {
                return None;
            }
            let mut m = Matcher::new(text, self.ngroups);
            let mut end_found: Option<usize> = None;
            let root = &self.root;
            let mut kont = |mm: &mut Matcher, end: usize| {
                end_found = Some(end);
                // 保留 caps（第 0 组存整体）
                if mm.caps[0].is_none() {
                    mm.caps[0] = Some((s, end));
                }
                true
            };
            if m.m(root, s, &mut kont) {
                let e = end_found.unwrap_or(s);
                return Some((s, e, m.caps.clone()));
            }
            s += 1;
        }
    }

    /// ↔ sqstd_rex_match：从头完整匹配
    pub fn match_full(&self, text: &[u8]) -> Option<(usize, usize, Vec<Option<(usize, usize)>>)> {
        let mut m = Matcher::new(text, self.ngroups);
        let mut end_found: Option<usize> = None;
        let root = &self.root;
        let s = 0usize;
        let mut kont = |mm: &mut Matcher, end: usize| {
            if end == text.len() {
                end_found = Some(end);
                mm.caps[0] = Some((s, end));
                true
            } else {
                false
            }
        };
        if m.m(root, 0, &mut kont) {
            let e = end_found?;
            return Some((0, e, m.caps.clone()));
        }
        None
    }

    pub fn ngroups(&self) -> usize {
        self.ngroups
    }
}

// ---------------------------------------------------------------------------
// Squirrel 侧的 regexp 类
// ---------------------------------------------------------------------------

thread_local! {
    static REGEXPS: std::cell::RefCell<Vec<Rc<Regex>>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn get_rx(v: &VmData, idx: SQInteger) -> Option<Rc<Regex>> {
    let o = v.stack_get(idx);
    if let Value::Instance(inst) = &o {
        let idv = inst.borrow().get(&v.ss.intern(b"__rxid"))?;
        let id = idv.as_integer() as usize;
        return REGEXPS.with(|r| r.borrow().get(id).cloned());
    }
    None
}

pub fn make_regexp_class(ss: &std::rc::Rc<squirrel::state::SharedState>) -> Value {
    let cls = squirrel::class::SQClass::create(ss, None);
    let rxid_key = ss.intern(b"__rxid");

    // 字段：__rxid（REGEXPS 注册表索引）
    cls.borrow_mut().new_slot(ss, &rxid_key, &Value::Integer(0), false);

    macro_rules! method {
        ($name:expr, $npc:expr, $f:expr) => {
            let nc = squirrel::closure::SQNativeClosure::create(ss, Rc::new($f), 0);
            nc.borrow_mut().nparamscheck = $npc;
            nc.borrow_mut().name = ss.intern($name.as_bytes());
            let key = ss.intern($name.as_bytes());
            cls.borrow_mut().new_slot(ss, &key, &Value::NativeClosure(nc), true);
        };
    }

    // constructor(pattern)
    method!("constructor", -1, move |v: &mut VmData| {
        let pat = v.stack_get(2);
        match Regex::compile(pat.as_bytes()) {
            Some(rx) => {
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
            None => {
                v.raise_error_str("error compiling the regular expression");
                SQ_ERROR
            }
        }
    });

    method!("search", -1, |v: &mut VmData| {
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

    method!("capture", -1, |v: &mut VmData| {
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
                    match c {
                        Some((s, e)) => {
                            let t = squirrel::table::SQTable::create(&v.ss, 2);
                            t.borrow_mut().new_slot(&v.ss.intern(b"begin"), &Value::Integer(*s as SQInteger));
                            t.borrow_mut().new_slot(&v.ss.intern(b"end"), &Value::Integer(*e as SQInteger));
                            let _ = arr.borrow_mut().set(i as SQInteger, &Value::Table(t));
                        }
                        None => {
                            let _ = arr.borrow_mut().set(i as SQInteger, &Value::Null);
                        }
                    }
                }
                v.push(Value::Array(arr));
                1
            }
            None => 0,
        }
    });

    method!("match", 2, |v: &mut VmData| {
        let rx = match get_rx(v, 1) {
            Some(r) => r,
            None => return SQ_ERROR,
        };
        let str_ = v.stack_get(2);
        v.push(Value::Bool(rx.match_full(str_.as_bytes()).is_some()));
        1
    });

    method!("subexpcount", 1, |v: &mut VmData| {
        let rx = match get_rx(v, 1) {
            Some(r) => r,
            None => return SQ_ERROR,
        };
        v.push(Value::Integer(rx.ngroups() as SQInteger));
        1
    });

    Value::Class(cls)
}

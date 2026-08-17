//! ↔ squirrel/sqtable.h + sqtable.cpp
//!
//! 基于 Lua 4.0 的开链哈希表。**物理布局逐位复刻**：哈希函数、节点数组、
//! `_firstfree` 回退扫描、冲突节点搬移、rehash 时机与 Next() 遍历顺序，
//! 因为 foreach 的遍历顺序完全由这些细节决定（差分测试依赖）。
//! C++ 的 `next` 裸指针在 Rust 里改为节点索引（地址比较 == 索引比较）。

use crate::config::*;
use crate::object::{hash_obj, raw_eq, translate_index, Value};

pub const MINPOWER2: SQInteger = 4;

#[derive(Clone)]
pub struct HashNode {
    pub val: Value,
    pub key: Value,
    pub next: Option<usize>, // ↔ _HashNode::next（冲突链下一个节点索引）
}

fn null_node() -> HashNode {
    HashNode {
        val: Value::Null,
        key: Value::Null,
        next: None,
    }
}

pub struct SQTable {
    pub firstfree: usize,
    pub nodes: Vec<HashNode>,
    pub numofnodes: SQInteger,
    pub usednodes: SQInteger,
    pub delegate: Option<crate::object::TableRef>,
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

impl SQTable {
    /// ↔ SQTable::Create + 构造函数
    pub fn create(ss: &crate::state::SharedState, initial_size: SQInteger) -> crate::object::TableRef {
        let mut pow2size = MINPOWER2;
        while initial_size > pow2size {
            pow2size <<= 1;
        }
        let rc = crate::object::TableRef::new(std::cell::RefCell::new(Self::with_nodes(pow2size)));
        crate::object::gc_register!(ss, &rc, Table);
        rc
    }

    fn with_nodes(size: SQInteger) -> SQTable {
        let n = size as usize;
        SQTable {
            firstfree: n - 1,
            nodes: vec![null_node(); n],
            numofnodes: size,
            usednodes: 0,
            delegate: None,
            weakref_slot: std::cell::RefCell::new(None),
        }
    }

    fn mask(&self) -> SQHash {
        (self.numofnodes - 1) as SQHash
    }

    /// ↔ _Get：沿冲突链按 raw+type 匹配
    fn get_node(&self, key: &Value, hash: usize) -> Option<usize> {
        let mut n = Some(hash);
        while let Some(i) = n {
            if raw_eq(&self.nodes[i].key, key) {
                return Some(i);
            }
            n = self.nodes[i].next;
        }
        None
    }

    /// ↔ SQTable::Get（读出的 weakref 值自动解引用）
    pub fn get(&self, key: &Value) -> Option<Value> {
        if key.is_null() {
            return None;
        }
        let h = (hash_obj(key) & self.mask()) as usize;
        self.get_node(key, h).map(|i| self.nodes[i].val.realval())
    }

    /// ↔ GetStr（编译器用：按内容查字符串键，走独立的哈希链匹配）
    pub fn get_str(&self, key: &[u8]) -> Option<Value> {
        let hash = crate::object::hashstr(key);
        let mut n = Some((hash & self.mask()) as usize);
        while let Some(i) = n {
            if let Value::String(s) = &self.nodes[i].key {
                if s.val.as_ref() == key {
                    return Some(self.nodes[i].val.realval());
                }
            }
            n = self.nodes[i].next;
        }
        None
    }

    /// ↔ SQTable::Set
    pub fn set(&mut self, key: &Value, val: &Value) -> bool {
        let h = (hash_obj(key) & self.mask()) as usize;
        if let Some(i) = self.get_node(key, h) {
            self.nodes[i].val = val.clone();
            return true;
        }
        false
    }

    /// ↔ SQTable::NewSlot：true=新建槽，false=键已存在（只更新了值）
    pub fn new_slot(&mut self, key: &Value, val: &Value) -> bool {
        debug_assert!(!key.is_null());
        let h = (hash_obj(key) & self.mask()) as usize;
        if let Some(i) = self.get_node(key, h) {
            self.nodes[i].val = val.clone();
            return false;
        }
        let mut mp = h;

        if !self.nodes[mp].key.is_null() {
            // main pos is not free
            let n = self.firstfree; // free place
            let mph = (hash_obj(&self.nodes[mp].key) & self.mask()) as usize;

            // ↔ `if (mp > n && othern != mp)`：C++ 比较节点地址 == 此处比较索引
            if mp > n && mph != mp {
                // 把占用 main pos 的节点搬到 free 位置
                let mut othern = mph;
                while self.nodes[othern].next != Some(mp) {
                    othern = self.nodes[othern].next.unwrap();
                }
                self.nodes[othern].next = Some(n);
                self.nodes[n].key = self.nodes[mp].key.clone();
                self.nodes[n].val = self.nodes[mp].val.clone();
                self.nodes[n].next = self.nodes[mp].next;
                self.nodes[mp].key = Value::Null;
                self.nodes[mp].val = Value::Null;
                self.nodes[mp].next = None;
            } else {
                // 新节点放进 free 位置，挂进 main pos 的冲突链
                self.nodes[n].next = self.nodes[mp].next;
                self.nodes[mp].next = Some(n);
                mp = n;
            }
        }
        self.nodes[mp].key = key.clone();

        loop {
            // correct `firstfree`
            let ff = self.firstfree;
            if self.nodes[ff].key.is_null() && self.nodes[ff].next.is_none() {
                self.nodes[mp].val = val.clone();
                self.usednodes += 1;
                return true;
            } else if ff == 0 {
                break;
            } else {
                self.firstfree -= 1;
            }
        }
        self.rehash(true);
        self.new_slot(key, val)
    }

    /// ↔ SQTable::Remove
    pub fn remove(&mut self, key: &Value) {
        let h = (hash_obj(key) & self.mask()) as usize;
        if let Some(i) = self.get_node(key, h) {
            self.nodes[i].val = Value::Null;
            self.nodes[i].key = Value::Null;
            self.usednodes -= 1;
            self.rehash(false);
        }
    }

    /// ↔ SQTable::Rehash
    fn rehash(&mut self, force: bool) {
        let mut oldsize = self.numofnodes;
        // prevent problems with the integer division
        if oldsize < 4 {
            oldsize = 4;
        }
        let nelems = self.usednodes;
        let newsize;
        if nelems >= oldsize - oldsize / 4 {
            newsize = oldsize * 2;
        } else if nelems <= oldsize / 4 && oldsize > MINPOWER2 {
            newsize = oldsize / 2;
        } else if force {
            newsize = oldsize;
        } else {
            return;
        }
        // ↔ AllocNodes(newsize)：旧数组暂存到局部（对应 C++ 的 nold）
        let oldnodes = std::mem::replace(&mut self.nodes, vec![null_node(); newsize as usize]);
        self.numofnodes = newsize;
        self.firstfree = newsize as usize - 1;
        self.usednodes = 0;
        for old in &oldnodes {
            if !old.key.is_null() {
                self.new_slot(&old.key, &old.val);
            }
        }
    }

    /// ↔ SQTable::Next：返回 (next_refpos, key, val)；结束返回 None
    pub fn next(&self, getweakrefs: bool, refpos: &Value) -> Option<(SQInteger, Value, Value)> {
        let mut idx = translate_index(refpos) as SQInteger;
        while idx < self.numofnodes {
            let i = idx as usize;
            if !self.nodes[i].key.is_null() {
                let outkey = self.nodes[i].key.clone();
                let outval = if getweakrefs {
                    self.nodes[i].val.clone()
                } else {
                    self.nodes[i].val.realval()
                };
                return Some((idx + 1, outkey, outval));
            }
            idx += 1;
        }
        None
    }

    pub fn count_used(&self) -> SQInteger {
        self.usednodes
    }

    /// ↔ SQTable::Clear
    pub fn clear(&mut self) {
        for n in self.nodes.iter_mut() {
            n.key = Value::Null;
            n.val = Value::Null;
        }
        self.usednodes = 0;
        self.rehash(true);
    }

    /// ↔ SQTable::Finalize（GC 用：清节点、摘委托）
    pub fn finalize(&mut self) {
        for n in self.nodes.iter_mut() {
            n.key = Value::Null;
            n.val = Value::Null;
        }
        self.delegate = None;
    }

    /// ↔ SQTable::Clone（默认构建未定义 _FAST_CLONE：按 Next 顺序重新插入）
    pub fn clone_table(&self, ss: &crate::state::SharedState) -> crate::object::TableRef {
        let nt = SQTable::create(ss, self.numofnodes);
        let mut t = nt.borrow_mut();
        let mut refpos = Value::Null;
        while let Some((next_pos, key, val)) = self.next(false, &refpos) {
            t.new_slot(&key, &val);
            refpos = Value::Integer(next_pos);
        }
        if let Some(d) = &self.delegate {
            t.delegate = Some(d.clone());
        }
        drop(t);
        nt
    }

    /// ↔ SQDelegable::SetDelegate（含环检测）
    pub fn set_delegate(&mut self, mt: Option<crate::object::TableRef>) -> bool {
        let this_identity = self as *const _ as usize;
        crate::object::set_delegate_checked(&mut self.delegate, this_identity, mt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::SQString;
    use std::rc::Rc;

    fn s(v: &str) -> Value {
        Value::String(Rc::new(SQString {
            hash: crate::object::hashstr(v.as_bytes()),
            val: v.as_bytes().into(),
            weakref_slot: std::cell::RefCell::new(None),
        }))
    }

    #[test]
    fn insert_get_remove() {
        let mut t = SQTable::with_nodes(4);
        let k1 = s("hello");
        let v1 = Value::Integer(42);
        assert!(t.new_slot(&k1, &v1));
        assert!(!t.new_slot(&k1, &Value::Integer(43))); // 已存在只更新
        assert_eq!(t.get(&k1).unwrap().as_integer(), 43);
        assert_eq!(t.count_used(), 1);
        t.remove(&k1);
        assert!(t.get(&k1).is_none());
        assert_eq!(t.count_used(), 0);
    }

    #[test]
    fn grow_and_iterate() {
        let mut t = SQTable::with_nodes(4);
        for i in 0..100i64 {
            // 注意：C++ NewSlot 在触发 rehash 的插入上会返回 false（历史怪癖），
            // 此处不检查返回值，只验证键值与计数。
            t.new_slot(&Value::Integer(i), &Value::Integer(i * 2));
        }
        assert_eq!(t.count_used(), 100);
        for i in 0..100i64 {
            assert_eq!(t.get(&Value::Integer(i)).unwrap().as_integer(), i * 2);
        }
        // 遍历产出恰好 100 个不重复键
        let mut seen = std::collections::HashSet::new();
        let mut refpos = Value::Null;
        while let Some((np, k, _)) = t.next(false, &refpos) {
            assert!(seen.insert(k.as_integer()));
            refpos = Value::Integer(np);
        }
        assert_eq!(seen.len(), 100);
    }

    #[test]
    fn hashstr_matches_reference_algorithm() {
        // _hashstr("", 0) = 0；"ab" 手工验算（注意：从末尾向前采样）
        assert_eq!(crate::object::hashstr(b""), 0);
        let h = crate::object::hashstr(b"ab");
        let mut expect: u64 = 2;
        expect ^= (2u64 << 5) + (2u64 >> 2) + b'b' as u64; // 先 s[l-1]='b'
        expect ^= (expect << 5) + (expect >> 2) + b'a' as u64; // 再 s[0]='a'
        assert_eq!(h, expect);
    }

    /// 与原版 C++（sq_static.exe）差分验证：32 个字符串键的遍历顺序必须逐位一致。
    /// 基准生成脚本（build-ref/bin/Release/sq_static.exe）：
    ///   local t = {}; foreach(k in keys) t[k] <- k.len(); foreach(k,v in t) print(k + 换行);
    #[test]
    fn iteration_order_matches_cpp_reference() {
        let keys = [
            "apple", "banana", "cherry", "date", "elderberry", "fig", "grape", "honeydew",
            "kiwi", "lemon", "mango", "nectarine", "orange", "papaya", "quince", "raspberry",
            "strawberry", "tangerine", "ugli", "vanilla", "watermelon", "xylophone", "yam",
            "zucchini", "avocado", "blueberry", "cantaloupe", "dragonfruit", "Eggplant",
            "Frisee", "Grapefruit", "Haricot",
        ];
        let expected = "nectarine,fig,date,yam,dragonfruit,zucchini,orange,tangerine,watermelon,mango,Grapefruit,Haricot,honeydew,grape,avocado,xylophone,apple,Eggplant,lemon,raspberry,kiwi,cherry,quince,blueberry,cantaloupe,Frisee,vanilla,strawberry,papaya,ugli,elderberry,banana";
        let mut t = SQTable::with_nodes(4);
        for k in keys {
            let key = s(k);
            let val = Value::Integer(k.len() as SQInteger);
            t.new_slot(&key, &val);
        }
        let mut order = Vec::new();
        let mut refpos = Value::Null;
        while let Some((np, k, _)) = t.next(false, &refpos) {
            order.push(String::from_utf8_lossy(&k.as_bytes()).into_owned());
            refpos = Value::Integer(np);
        }
        assert_eq!(order.join(","), expected);
    }
}
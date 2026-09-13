//! ↔ squirrel/sqarray.h + SQArray::Extend（sqobject.cpp:208-213）

use crate::config::*;
use crate::object::{translate_index, Value};

pub struct SQArray {
    pub values: Vec<Value>,
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

/// 迭代式释放：深嵌套数组不能靠递归 Drop（见 crate::release）
impl Drop for SQArray {
    fn drop(&mut self) {
        crate::release::release_values(std::mem::take(&mut self.values));
    }
}

impl SQArray {
    /// ↔ SQArray::Create
    pub fn create(ss: &crate::state::SharedState, initial_size: SQInteger) -> crate::object::ArrayRef {
        let rc = crate::object::ArrayRef::new(std::cell::RefCell::new(SQArray {
            values: vec![Value::Null; initial_size.max(0) as usize],
            weakref_slot: std::cell::RefCell::new(None),
        }));
        crate::object::gc_register!(ss, &rc, Array);
        rc
    }

    pub fn get(&self, nidx: SQInteger) -> Option<Value> {
        if nidx >= 0 && nidx < self.values.len() as SQInteger {
            Some(self.values[nidx as usize].realval())
        } else {
            None
        }
    }

    pub fn set(&mut self, nidx: SQInteger, val: &Value) -> bool {
        if nidx >= 0 && nidx < self.values.len() as SQInteger {
            self.values[nidx as usize] = val.clone();
            true
        } else {
            false
        }
    }

    /// ↔ SQArray::Next：返回 (next_refpos, key, val)
    pub fn next(&self, refpos: &Value) -> Option<(SQInteger, Value, Value)> {
        let idx = translate_index(refpos);
        while idx < self.values.len() as SQUnsignedInteger {
            let outkey = Value::Integer(idx as SQInteger);
            let outval = self.values[idx as usize].realval();
            return Some(((idx + 1) as SQInteger, outkey, outval));
        }
        None
    }

    pub fn size(&self) -> SQInteger {
        self.values.len() as SQInteger
    }

    pub fn resize(&mut self, size: SQInteger, fill: &Value) {
        self.values.resize(size.max(0) as usize, fill.clone());
    }

    pub fn append(&mut self, o: &Value) {
        self.values.push(o.clone());
    }

    /// ↔ SQArray::Extend
    pub fn extend(&mut self, a: &SQArray) {
        let xlen = a.size();
        if xlen > 0 {
            for i in 0..xlen {
                self.append(&a.values[i as usize]);
            }
        }
    }

    pub fn top(&self) -> Value {
        self.values.last().cloned().unwrap_or(Value::Null)
    }

    pub fn pop(&mut self) {
        self.values.pop();
    }

    pub fn insert(&mut self, idx: SQInteger, val: &Value) -> bool {
        if idx < 0 || idx > self.values.len() as SQInteger {
            return false;
        }
        self.values.insert(idx as usize, val.clone());
        true
    }

    pub fn remove(&mut self, idx: SQInteger) -> bool {
        if idx < 0 || idx >= self.values.len() as SQInteger {
            return false;
        }
        self.values.remove(idx as usize);
        true
    }

    /// ↔ SQArray::Finalize（GC 用）
    pub fn finalize(&mut self) {
        self.values.clear();
    }

    /// ↔ SQArray::Clone
    pub fn clone_array(&self, ss: &crate::state::SharedState) -> crate::object::ArrayRef {
        let anew = SQArray::create(ss, 0);
        anew.borrow_mut().values = self.values.clone();
        anew
    }
}

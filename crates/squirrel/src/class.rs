//! ↔ squirrel/sqclass.h + sqclass.cpp

use crate::config::*;
use crate::object::{raw_eq, ClassRef, InstanceRef, TableRef, Value};

/// ↔ SQClassMember
#[derive(Clone)]
pub struct SQClassMember {
    pub val: Value,
    pub attrs: Value,
}

impl SQClassMember {
    fn new() -> Self {
        SQClassMember {
            val: Value::Null,
            attrs: Value::Null,
        }
    }
    fn null(&mut self) {
        self.val = Value::Null;
        self.attrs = Value::Null;
    }
}

pub const MEMBER_TYPE_METHOD: SQInteger = 0x0100_0000;
pub const MEMBER_TYPE_FIELD: SQInteger = 0x0200_0000;
pub const MEMBER_MAX_COUNT: SQInteger = 0x00FF_FFFF;

pub fn is_method(o: &Value) -> bool {
    o.as_integer() & MEMBER_TYPE_METHOD != 0
}

pub fn is_field(o: &Value) -> bool {
    o.as_integer() & MEMBER_TYPE_FIELD != 0
}

pub fn make_method_idx(i: SQInteger) -> SQInteger {
    MEMBER_TYPE_METHOD | i
}

pub fn make_field_idx(i: SQInteger) -> SQInteger {
    MEMBER_TYPE_FIELD | i
}

pub fn member_idx(o: &Value) -> usize {
    (o.as_integer() & 0x00FF_FFFF) as usize
}

pub const MT_LAST: usize = 18;

/// ↔ SQClass
pub struct SQClass {
    pub members: TableRef,
    pub base: Option<ClassRef>,
    pub defaultvalues: Vec<SQClassMember>,
    pub methods: Vec<SQClassMember>,
    /// ↔ _metamethods[MT_LAST]
    pub metamethods: Vec<Value>,
    pub attributes: Value,
    pub typetag: SQUserPointer,
    pub hook: Option<crate::userdata::ReleaseHook>,
    pub locked: bool,
    pub constructoridx: SQInteger,
    pub udsize: SQInteger,
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

impl SQClass {
    /// ↔ SQClass::SQClass(ss, base)（sqclass.cpp:13-34）
    pub fn create(ss: &crate::state::SharedState, base: Option<ClassRef>) -> ClassRef {
        let members = match &base {
            Some(b) => b.borrow().members.borrow().clone_table(ss),
            None => crate::table::SQTable::create(ss, 0),
        };
        let rc = ClassRef::new(std::cell::RefCell::new(SQClass {
            constructoridx: -1,
            udsize: 0,
            locked: false,
            typetag: 0,
            hook: None,
            attributes: Value::Null,
            metamethods: vec![Value::Null; MT_LAST],
            defaultvalues: Vec::new(),
            methods: Vec::new(),
            base: None,
            members,
            weakref_slot: std::cell::RefCell::new(None),
        }));
        if let Some(b) = &base {
            let (mut c, bb) = (rc.borrow_mut(), b.borrow());
            c.constructoridx = bb.constructoridx;
            c.udsize = bb.udsize;
            c.defaultvalues = bb.defaultvalues.clone();
            c.methods = bb.methods.clone();
            c.metamethods = bb.metamethods.clone();
            c.base = Some(b.clone());
        }
        crate::object::gc_register!(ss, &rc, Class);
        rc
    }

    /// ↔ SQClass::NewSlot（sqclass.cpp:53-102）
    pub fn new_slot(&mut self, ss: &crate::state::SharedState, key: &Value, val: &Value, bstatic: bool) -> bool {
        let temp = self.members.borrow().get(key);
        let belongs_to_static_table = matches!(val, Value::Closure(_) | Value::NativeClosure(_)) || bstatic;
        if self.locked && !belongs_to_static_table {
            return false; // 类已有实例，不能再修改
        }
        if let Some(t) = &temp {
            if is_field(t) {
                // 覆盖字段默认值
                self.defaultvalues[member_idx(t)].val = val.clone();
                return true;
            }
        }
        if self.members.borrow().count_used() >= MEMBER_MAX_COUNT {
            return false;
        }
        if belongs_to_static_table {
            let mmidx = if matches!(val, Value::Closure(_) | Value::NativeClosure(_)) {
                ss.get_metamethod_idx_by_name(key)
            } else {
                -1
            };
            if mmidx != -1 {
                self.metamethods[mmidx as usize] = val.clone();
            } else {
                let mut theval = val.clone();
                if let (Some(base), Value::Closure(cl)) = (&self.base, val) {
                    let cloned = cl.borrow().clone_closure(ss);
                    cloned.borrow_mut().base = Some(base.clone());
                    theval = Value::Closure(cloned);
                }
                match &temp {
                    None => {
                        // ↔ IsEqual(ss->_constructoridx, key)
                        if raw_eq(&ss.constructoridx.borrow(), key) {
                            self.constructoridx = self.methods.len() as SQInteger;
                        }
                        let mut m = SQClassMember::new();
                        m.val = theval;
                        self.members.borrow_mut().new_slot(
                            key,
                            &Value::Integer(make_method_idx(self.methods.len() as SQInteger)),
                        );
                        self.methods.push(m);
                    }
                    Some(t) => {
                        self.methods[member_idx(t)].val = theval;
                    }
                }
            }
            return true;
        }
        let mut m = SQClassMember::new();
        m.val = val.clone();
        self.members
            .borrow_mut()
            .new_slot(key, &Value::Integer(make_field_idx(self.defaultvalues.len() as SQInteger)));
        self.defaultvalues.push(m);
        true
    }

    /// ↔ SQClass::Get
    pub fn get(&self, key: &Value) -> Option<Value> {
        let idx = self.members.borrow().get(key)?;
        if is_field(&idx) {
            Some(self.defaultvalues[member_idx(&idx)].val.realval())
        } else {
            Some(self.methods[member_idx(&idx)].val.clone())
        }
    }

    /// ↔ SQClass::GetConstructor
    pub fn get_constructor(&self) -> Option<Value> {
        if self.constructoridx != -1 {
            Some(self.methods[self.constructoridx as usize].val.clone())
        } else {
            None
        }
    }

    /// ↔ SQClass::Lock
    pub fn lock(&mut self) {
        self.locked = true;
        if let Some(b) = &self.base {
            b.borrow_mut().lock();
        }
    }

    /// ↔ SQClass::CreateInstance（持 Rc 的关联函数，避免重入借用）
    pub fn create_instance(this: &ClassRef, ss: &crate::state::SharedState) -> InstanceRef {
        if !this.borrow().locked {
            this.borrow_mut().lock();
        }
        SQInstance::create(ss, this)
    }

    /// ↔ SQClass::Next
    pub fn next(&self, refpos: &Value) -> Option<(SQInteger, Value, Value)> {
        let (idx, outkey, oval) = self.members.borrow().next(false, refpos)?;
        let outval = if is_method(&oval) {
            self.methods[member_idx(&oval)].val.clone()
        } else {
            self.defaultvalues[member_idx(&oval)].val.realval()
        };
        Some((idx, outkey, outval))
    }

    /// ↔ SQClass::SetAttributes
    pub fn set_attributes(&mut self, key: &Value, val: &Value) -> bool {
        let idx = self.members.borrow().get(key);
        if let Some(idx) = idx {
            if is_field(&idx) {
                self.defaultvalues[member_idx(&idx)].attrs = val.clone();
            } else {
                self.methods[member_idx(&idx)].attrs = val.clone();
            }
            true
        } else {
            false
        }
    }

    /// ↔ SQClass::GetAttributes
    pub fn get_attributes(&self, key: &Value) -> Option<Value> {
        let idx = self.members.borrow().get(key)?;
        Some(if is_field(&idx) {
            self.defaultvalues[member_idx(&idx)].attrs.clone()
        } else {
            self.methods[member_idx(&idx)].attrs.clone()
        })
    }

    /// ↔ SQClass::Finalize
    pub fn finalize(&mut self) {
        self.attributes = Value::Null;
        for dv in self.defaultvalues.iter_mut() {
            dv.null();
        }
        self.methods.clear();
        for mm in self.metamethods.iter_mut() {
            *mm = Value::Null;
        }
        self.base = None;
    }
}

/// ↔ SQInstance：`_delegate` 即其类的 `_members` 表
pub struct SQInstance {
    /// None 仅出现在 GC Finalize 之后（对应 C++ `_class=NULL`）
    pub class: Option<ClassRef>,
    pub userpointer: SQUserPointer,
    pub hook: Option<crate::userdata::ReleaseHook>,
    pub values: Vec<Value>,
    pub delegate: Option<TableRef>, // == class._members
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

impl SQInstance {
    /// ↔ SQInstance::Create
    pub fn create(ss: &crate::state::SharedState, theclass: &ClassRef) -> InstanceRef {
        let (defaults, members) = {
            let c = theclass.borrow();
            (
                c.defaultvalues.iter().map(|m| m.val.clone()).collect::<Vec<_>>(),
                c.members.clone(),
            )
        };
        let rc = InstanceRef::new(std::cell::RefCell::new(SQInstance {
            class: Some(theclass.clone()),
            userpointer: 0,
            hook: None,
            values: defaults,
            delegate: Some(members),
            weakref_slot: std::cell::RefCell::new(None),
        }));
        crate::object::gc_register!(ss, &rc, Instance);
        rc
    }

    /// ↔ SQInstance::Clone（不复制 hook/userpointer；C++ 克隆构造不复制它们）
    pub fn clone_instance(&self, ss: &crate::state::SharedState) -> InstanceRef {
        let rc = InstanceRef::new(std::cell::RefCell::new(SQInstance {
            class: self.class.clone(),
            userpointer: 0,
            hook: None,
            values: self.values.clone(),
            delegate: self.delegate.clone(),
            weakref_slot: std::cell::RefCell::new(None),
        }));
        crate::object::gc_register!(ss, &rc, Instance);
        rc
    }

    fn the_class(&self) -> &ClassRef {
        self.class.as_ref().expect("instance finalized by GC")
    }

    /// ↔ SQInstance::Get
    pub fn get(&self, key: &Value) -> Option<Value> {
        let idx = self.the_class().borrow().members.borrow().get(key)?;
        if is_field(&idx) {
            Some(self.values[member_idx(&idx)].realval())
        } else {
            Some(self.the_class().borrow().methods[member_idx(&idx)].val.clone())
        }
    }

    /// ↔ SQInstance::Set
    pub fn set(&mut self, key: &Value, val: &Value) -> bool {
        let idx = self.the_class().borrow().members.borrow().get(key);
        if let Some(idx) = idx {
            if is_field(&idx) {
                self.values[member_idx(&idx)] = val.clone();
                return true;
            }
        }
        false
    }

    /// ↔ SQInstance::GetMetaMethod（类上找 metamethod）
    pub fn get_metamethod(&self, mm: usize) -> Option<Value> {
        let c = self.the_class().borrow();
        let v = c.metamethods[mm].clone();
        if v.is_null() {
            None
        } else {
            Some(v)
        }
    }

    /// ↔ SQInstance::InstanceOf
    pub fn instance_of(&self, trg: &ClassRef) -> bool {
        let mut parent = self.class.clone();
        while let Some(p) = parent {
            if ClassRef::ptr_eq(&p, trg) {
                return true;
            }
            parent = p.borrow().base.clone();
        }
        false
    }

    /// ↔ SQInstance::Finalize
    pub fn finalize(&mut self) {
        for v in self.values.iter_mut() {
            *v = Value::Null;
        }
        self.class = None;
        self.delegate = None;
    }
}

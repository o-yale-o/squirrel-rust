//! ↔ squirrel/squserdata.h

use crate::config::*;
use crate::object::Value;

/// ↔ SQRELEASEHOOK
pub type ReleaseHook = Box<dyn FnMut(SQUserPointer, SQInteger) -> SQInteger>;

pub struct SQUserData {
    pub delegate: Option<crate::object::TableRef>,
    pub hook: Option<ReleaseHook>,
    pub typetag: SQUserPointer,
    pub size: SQInteger,
    pub data: Vec<u8>,
    pub(crate) weakref_slot: std::cell::RefCell<Option<Value>>,
}

impl SQUserData {
    /// ↔ SQUserData::Create
    pub fn create(ss: &crate::state::SharedState, size: SQInteger) -> crate::object::UserdataRef {
        let rc = crate::object::UserdataRef::new(std::cell::RefCell::new(SQUserData {
            delegate: None,
            hook: None,
            typetag: 0,
            size,
            data: vec![0; size.max(0) as usize],
            weakref_slot: std::cell::RefCell::new(None),
        }));
        crate::object::gc_register!(ss, &rc, Userdata);
        rc
    }

    pub fn userpointer(&mut self) -> SQUserPointer {
        // C++ 里 userdata 数据块紧跟结构体之后；Rust 用 Vec 缓冲区地址
        self.data.as_mut_ptr() as SQUserPointer
    }

    /// ↔ SQUserData::Finalize
    pub fn finalize(&mut self) {
        self.delegate = None;
    }

    pub fn set_delegate(&mut self, mt: Option<crate::object::TableRef>) -> bool {
        let this_identity = self as *const _ as usize;
        crate::object::set_delegate_checked(&mut self.delegate, this_identity, mt)
    }
}

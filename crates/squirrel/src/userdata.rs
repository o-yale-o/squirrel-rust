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

/// 迭代式释放：delegate 可能链得很深（见 crate::release）。
///
/// 同时这里承担 ↔ `SQUserData::Release()` 的**release hook**：
/// 参考实现是引用计数归零就 `if (_hook) _hook(_data, _size);`，只调一次。
/// 本实现以前只在 GC 的 finalize 路径里跑 hook，而 userdata 通常在 `Rc` 析构时
/// 就已经释放了 —— 于是 `sq_setreleasehook` 注册的回调**根本不会被调用**
/// （P2 的 host_api_ext 测试抓到）。挂在 `Drop` 上即可覆盖两条路径；
/// 先走 finalize 的话 hook 已经 `take()` 走了，Drop 里就是 `None`，不会重复调。
impl Drop for SQUserData {
    fn drop(&mut self) {
        if let Some(d) = self.delegate.take() {
            crate::release::release_value(Value::Table(d));
        }
        if let Some(mut hook) = self.hook.take() {
            // SQUserPointer 在本实现里是 usize（不是裸指针）
            let ptr: SQUserPointer = if self.data.is_empty() {
                0
            } else {
                self.data.as_mut_ptr() as SQUserPointer
            };
            hook(ptr, self.size);
        }
    }
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

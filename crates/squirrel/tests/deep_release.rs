//! 深嵌套值释放的自检：`Rc` 容器的默认 Drop 是递归的，必须在 O(1) 宿主栈内完成
//! 十万层结构的回收（否则会 STATUS_STACK_OVERFLOW abort）。

use squirrel::array::SQArray;
use squirrel::api::Vm;
use squirrel::object::Value;
use squirrel::state::SharedState;
use std::sync::atomic::Ordering::Relaxed;

fn reset_depth() {
    squirrel::release::RELEASE_DEPTH.store(0, Relaxed);
    squirrel::release::MAX_RELEASE_DEPTH.store(0, Relaxed);
}

fn max_depth() -> usize {
    squirrel::release::MAX_RELEASE_DEPTH.load(Relaxed)
}

#[test]
fn deep_array_drop_is_iterative() {
    let ss = SharedState::new();
    reset_depth();
    let mut cur = Value::Null;
    for _ in 0..100_000 {
        let a = SQArray::create(&ss, 0);
        a.borrow_mut().append(&cur);
        cur = Value::Array(a);
    }
    drop(cur);
    assert!(
        max_depth() <= 3,
        "深嵌套数组的释放出现了 {} 层递归",
        max_depth()
    );
}

#[test]
fn deep_table_drop_is_iterative() {
    let ss = SharedState::new();
    reset_depth();
    let mut cur = Value::Null;
    for _ in 0..100_000 {
        let t = squirrel::table::SQTable::create(&ss, 1);
        let k = ss.intern(b"inner");
        t.borrow_mut().new_slot(&k, &cur);
        cur = Value::Table(t);
    }
    drop(cur);
    assert!(max_depth() <= 3, "深嵌套表的释放出现了 {} 层递归", max_depth());
}

#[test]
fn deep_mixed_drop_is_iterative() {    let ss = SharedState::new();
    reset_depth();
    let mut cur = Value::Null;
    for i in 0..50_000 {
        match i % 3 {
            0 => {
                let a = SQArray::create(&ss, 0);
                a.borrow_mut().append(&cur);
                cur = Value::Array(a);
            }
            1 => {
                let t = squirrel::table::SQTable::create(&ss, 1);
                let k = ss.intern(b"inner");
                t.borrow_mut().new_slot(&k, &cur);
                cur = Value::Table(t);
            }
            _ => {
                let a = SQArray::create(&ss, 0);
                a.borrow_mut().append(&cur);
                a.borrow_mut().append(&Value::Integer(i as i64));
                cur = Value::Array(a);
            }
        }
    }
    drop(cur);
    assert!(max_depth() <= 3, "混合嵌套的释放出现了 {} 层递归", max_depth());
}

/// 与 CLI 等价的 1MB 主线程栈条件下也必须迭代释放（CLI 崩溃过：约 1500 层/1MB）
#[test]
fn deep_array_built_by_script_on_small_stack() {
    let h = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            let vm = Vm::new(1024);
            reset_depth();
            let src: &[u8] = b"local n = 20000; local a = []; for (local i = 0; i < n; i++) a = [a]; print(\"b\\n\"); a = null;";
            let r = vm.run_buffer(src, b"deep.nut");
            (r.is_ok(), max_depth())
        })
        .expect("spawn");
    let (ok, d) = h.join().expect("thread");
    assert!(ok, "脚本执行失败");
    assert!(d <= 3, "1MB 栈下释放出现 {} 层递归", d);
}

/// 脚本路径（VM 栈/临时寄存器/root 表都参与）构造的深嵌套值同样必须迭代释放
#[test]
fn deep_array_built_by_script_is_released_iteratively() {
    let vm = Vm::new(1024);
    reset_depth();
    let src: &[u8] =
        b"local n = 20000; local a = []; for (local i = 0; i < n; i++) a = [a]; a = null;";
    let r = vm.run_buffer(src, b"deep.nut");
    assert!(r.is_ok(), "脚本执行失败");
    assert!(
        max_depth() <= 3,
        "脚本构造的深嵌套结构释放出现了 {} 层递归",
        max_depth()
    );
}

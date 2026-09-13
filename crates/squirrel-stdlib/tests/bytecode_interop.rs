//! 字节码互通：直接执行**官方 C++ 实现编译出来的** `.cnut`（夹具文件）。
//!
//! 为什么必须有这组夹具：
//!
//! 1. 参考实现保存 outer 值时写的是 `sizeof(SQUnsignedInteger)`（8 字节），
//!    而字段本身是 4 字节枚举 → **高 4 字节是未初始化内存**，同一个脚本两次编译
//!    内容都不同（实测 `tools/cnut_matrix.py`）。加载器必须只取低 32 位，
//!    否则 `otLOCAL` 会被当成 `otOUTER`，接着就会越界索引。
//! 2. 参考实现自己不检查 outer 索引，坏字节码会让它直接访问违例崩溃
//!    （实测 `ref_closures_badouter.cnut` 在 C++ 上是 0xC0000005）；
//!    本实现必须退化成**可捕获的脚本错误**，绝不能 panic。

use std::cell::RefCell;
use std::rc::Rc;

use squirrel::api::Vm;

/// 参考实现（build-ref）编译出来的闭包/outer 值夹具
const REF_CLOSURES: &[u8] = include_bytes!("fixtures/ref_closures.cnut");
/// 同上，但把第一个 outer 的类型低 32 位改成 otOUTER(=1)、src 保持越界
const REF_BADOUTER: &[u8] = include_bytes!("fixtures/ref_closures_badouter.cnut");

fn make_vm() -> (Vm, Rc<RefCell<Vec<u8>>>, Rc<RefCell<Vec<u8>>>) {
    let vm = Vm::new(1024);
    let out: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    let err: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let o = out.clone();
        let e = err.clone();
        vm.set_print_funcs(
            Some(Rc::new(move |_v, b: &[u8]| o.borrow_mut().extend_from_slice(b))),
            Some(Rc::new(move |_v, b: &[u8]| e.borrow_mut().extend_from_slice(b))),
        );
    }
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    (vm, out, err)
}

fn write_temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(name);
    std::fs::write(&p, bytes).expect("写临时 .cnut 失败");
    p
}

#[test]
fn runs_reference_compiled_bytecode() {
    let (vm, out, err) = make_vm();
    let p = write_temp("sqrust_ref_closures.cnut", REF_CLOSURES);
    let r = squirrel_stdlib::run_file(&vm, p.to_str().unwrap(), true);
    let _ = std::fs::remove_file(&p);
    assert!(r.is_ok(), "参考实现编译的字节码应能正常执行：{:?}", r.err());
    assert_eq!(
        String::from_utf8_lossy(&out.borrow()),
        "11,13\n105\n3\n2,6\n",
        "执行结果应与源码直接运行一致"
    );
    assert!(err.borrow().is_empty(), "不应有错误输出：{:?}", String::from_utf8_lossy(&err.borrow()));
}

#[test]
fn out_of_range_outer_value_reports_error_instead_of_panicking() {
    let (vm, _out, err) = make_vm();
    let p = write_temp("sqrust_ref_closures_badouter.cnut", REF_BADOUTER);
    let r = squirrel_stdlib::run_file(&vm, p.to_str().unwrap(), true);
    let _ = std::fs::remove_file(&p);
    let msg = format!("{:?}", r.as_ref().err());
    let logged = String::from_utf8_lossy(&err.borrow()).into_owned();
    assert!(
        msg.contains("invalid outer value index") || logged.contains("invalid outer value index"),
        "越界 outer 索引必须报可捕获错误（返回值 {:?}，stderr {:?}）",
        r.err(),
        logged
    );
}

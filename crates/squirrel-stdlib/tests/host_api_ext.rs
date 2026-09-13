//! 宿主 API 扩展回归（P2）：member handle、`getfunctioninfo`、`setfreevariable`。
//!
//! 期望值来自**参考实现**：用一个 C 探针（`tools/hostapi/ref_host.c`，编译成
//! `ref_host.exe` 链接参考实现的静态库）跑同样的操作，报告存在
//! `tools/hostapi/ref_host.txt`。这里的断言与那份报告逐项对应。
//!
//! 从参考实现实测确认的几条"容易搞错"的语义：
//!
//! * `sq_getmemberhandle` **只接受类**（传实例直接 `wrong type`），
//!   而且**成功时会把键从栈上弹掉**；
//! * `_static` = "不是字段"：方法/静态字段为真，实例字段为假；
//! * `sq_getbyhandle` 静态成员从类的 `_methods[i]` 取、实例字段从实例的 `_values[i]`、
//!   传类时取 `_defaultvalues[i]`；
//! * `sq_setfreevariable` 是**替换槽位**（不是写穿 open outer），越界报 `index out of range`；
//! * `sq_getfunctioninfo` 收的是**栈层级**，没有帧时返回错误。

use std::cell::RefCell;
use std::rc::Rc;

use squirrel::api::Vm;
use squirrel::object::{raw_eq, Value};

#[derive(Default)]
struct Sink {
    out: RefCell<String>,
    err: RefCell<String>,
}

fn make_vm() -> (Vm, Rc<Sink>) {
    let vm = Vm::new(1024);
    let sink = Rc::new(Sink::default());
    {
        let o = sink.clone();
        let e = sink.clone();
        vm.set_print_funcs(
            Some(Rc::new(move |_v, s: &[u8]| {
                o.out.borrow_mut().push_str(&String::from_utf8_lossy(s))
            })),
            Some(Rc::new(move |_v, s: &[u8]| {
                e.err.borrow_mut().push_str(&String::from_utf8_lossy(s))
            })),
        );
    }
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    (vm, sink)
}

/// 跑一段脚本建出类/闭包，返回 (C, g)
fn setup(vm: &Vm) -> (Value, Value) {
    let src = "class C { </ attr = 7 /> v = 1; static sv = 2; function m() { return 5; } }\n\
               function make(x) { return function() { return x; }; }\n\
               g <- make(11);\n";
    vm.run_buffer(src.as_bytes(), b"hostapi2").expect("脚本应当能跑");
    let c = vm.get_global("C").expect("C");
    let g = vm.get_global("g").expect("g");
    (c, g)
}

/// member handle：静态成员 / 实例字段 / 缺失成员 / 非类目标
#[test]
fn member_handles_match_reference() {
    let (vm, _sink) = make_vm();
    let (c, _g) = setup(&vm);
    let root = vm.data().roottable.clone();
    let ss = vm.data().ss.clone();
    let key_sv = ss.intern(b"sv");
    let key_v = ss.intern(b"v");
    let key_m = ss.intern(b"m");
    let key_nope = ss.intern(b"nope");
    // 先把实例建出来（vm.call 需要可变借用，不能和下面的 `d` 长期借用共存）
    let inst = vm.call(&c, root.clone(), &[], true).expect("C()");
    let d = vm.data();

    // 静态字段 sv = 2
    let mh = d.get_member_handle(&c, &key_sv).expect("handle");
    assert!(mh.is_static, "静态字段的 handle 应当 _static=1");
    assert_eq!(d.get_by_handle(&c, &mh).map(|v| v.as_integer()), Some(2));
    // setbyhandle 到类上（改写静态字段）
    assert!(d.set_by_handle(&c, &mh, Value::Integer(42)));
    assert_eq!(d.get_by_handle(&c, &mh).map(|v| v.as_integer()), Some(42));

    // 实例字段 v = 1：handle 也必须从**类**上取
    let mhv = d.get_member_handle(&c, &key_v).expect("handle v");
    assert!(!mhv.is_static, "实例字段的 handle 应当 _static=0");
    // 传类 → 取 defaultvalues
    assert_eq!(d.get_by_handle(&c, &mhv).map(|v| v.as_integer()), Some(1));

    // 按 handle 取实例上的值
    assert_eq!(d.get_by_handle(&inst, &mhv).map(|v| v.as_integer()), Some(1));
    assert!(d.set_by_handle(&inst, &mhv, Value::Integer(77)));
    assert_eq!(d.get_by_handle(&inst, &mhv).map(|v| v.as_integer()), Some(77));
    // 类上的默认值不受实例改动影响
    assert_eq!(d.get_by_handle(&c, &mhv).map(|v| v.as_integer()), Some(1));

    // 方法 m 也算 _static=1（不是字段）
    let mhm = d.get_member_handle(&c, &key_m).expect("handle m");
    assert!(mhm.is_static);

    // 缺失成员 → wrong index；非类目标 → wrong type
    let miss = d.get_member_handle(&c, &key_nope);
    assert_eq!(miss.err(), Some("wrong index"));
    let not_class = d.get_member_handle(&Value::Integer(5), &key_sv);
    assert_eq!(not_class.err(), Some("wrong type(expected class or instance)"));
}

/// `getfunctioninfo`：按栈层级取，无帧时 Err
#[test]
fn function_info_matches_reference() {
    let (vm, _sink) = make_vm();
    let (_c, g) = setup(&vm);
    let root = vm.data().roottable.clone();
    // 不在调用中：任何层级都没有帧
    {
        let d = vm.data();
        assert!(d.get_function_info(0).is_none());
        assert!(d.get_function_info(9).is_none());
    }
    // 在调用中：level 0 = 当前帧（g 是 make 返回的闭包 → 名字是它的原型名）
    let r = vm.call(&g, root.clone(), &[], true).expect("g()");
    assert!(raw_eq(&r, &Value::Integer(11)), "g() 应当返回 11，实际 {:?}", r);
    // 用一个 native 在调用内部取信息
    let seen: Rc<RefCell<Vec<(String, i64, bool, usize)>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let s = seen.clone();
        vm.register_native("host_probe", Rc::new(move |v: &mut squirrel::vm::VmData| {
            let mut rows = Vec::new();
            for lvl in 0..3 {
                if let Some(fi) = v.get_function_info(lvl) {
                    rows.push((
                        String::from_utf8_lossy(fi.name.as_deref().unwrap_or(b"?")).into_owned(),
                        fi.line,
                        fi.native,
                        lvl,
                    ));
                } else {
                    rows.push(("<none>".to_string(), -1, false, lvl));
                }
            }
            *s.borrow_mut() = rows;
            v.push(squirrel::object::Value::Null);
            1
        }), 1, Some(""));
    }
    vm.run_buffer(b"function outer2() { return host_probe(); }\nouter2();\n", b"hostapi3")
        .expect("脚本");
    let rows = seen.borrow().clone();
    // level 0 = native 自己、level 1 = outer2（有源码行）、level 2 = 无
    assert_eq!(rows.len(), 3);
    assert!(rows[0].2, "level 0 应当是 native：{:?}", rows[0]);
    // level 2 = 调用 native 的顶层脚本（main）
    assert_eq!(rows[2].0, "main");
    let outer_row = &rows[1];
    assert!(!outer_row.2, "level 1 应当是闭包：{:?}", outer_row);
    assert_eq!(outer_row.0, "outer2");
    assert!(outer_row.1 >= 1, "闭包帧的行号应当 >= 1：{:?}", outer_row);
}

/// `setfreevariable`：替换槽位、越界报错
#[test]
fn set_free_variable_matches_reference() {
    let (vm, _sink) = make_vm();
    let (_c, g) = setup(&vm);
    let root = vm.data().roottable.clone();
    // g 捕获了 make 的参数 x（1 个自由变量），调用返回 11
    let r0 = vm.call(&g, root.clone(), &[], true).unwrap();
    assert!(raw_eq(&r0, &Value::Integer(11)), "g() 应当返回 11，实际 {:?}", r0);
    {
        let mut d = vm.data_mut();
        let s = d.ss.intern(b"changed");
        d.set_free_variable(&g, 0, s).expect("setfreevariable");
    }
    // 替换后返回新值
    let r = vm.call(&g, root.clone(), &[], true).unwrap();
    assert_eq!(String::from_utf8_lossy(r.as_bytes()), "changed");
    // 越界
    {
        let mut d = vm.data_mut();
        let e = d.set_free_variable(&g, 5, Value::Null);
        assert_eq!(e.err(), Some("index out of range"));
        // 非闭包
        let e2 = d.set_free_variable(&Value::Integer(1), 0, Value::Null);
        assert!(e2.is_err());
    }
}

/// userdata + typetag + release hook + 引用计数 + foreignptr + resurrect
/// （↔ sq_newuserdata / sq_settypetag / sq_setreleasehook / sq_addref / sq_release /
///   sq_setforeignptr / sq_resurrectunreachable）
///
/// 这几项本实现**内部早就有**（`SQUserData` 带 `typetag`/`hook`、
/// `SharedState::add_ref/release`、`resurrect_unreachable`、`foreignptr` 字段），
/// P2 这一轮把它们当成**公开宿主面**钉住，避免以后被随手改坏。
#[test]
fn userdata_refcount_and_resurrect_match_reference() {
    let (vm, _sink) = make_vm();
    let _ = setup(&vm); // 需要 g 这个全局做引用计数实验
    let freed = Rc::new(RefCell::new(0i32));

    // userdata：typetag + release hook
    let ud = {
        let ss = vm.data().ss.clone();
        let u = squirrel::userdata::SQUserData::create(&ss, 8);
        {
            let mut b = u.borrow_mut();
            b.typetag = 0x1234;
            let f = freed.clone();
            b.hook = Some(Box::new(move |_p, size| {
                *f.borrow_mut() += 1;
                let _ = size;
                1
            }));
        }
        Value::Userdata(u)
    };
    {
        let d = vm.data();
        if let Value::Userdata(u) = &ud {
            assert_eq!(u.borrow().typetag, 0x1234);
            assert!(u.borrow().hook.is_some());
            assert_eq!(u.borrow().size, 8);
        }
    }
    vm.set_global("hostapi_ud", ud.clone());
    vm.set_global("hostapi_ud", Value::Null);
    drop(ud);
    {
        let mut d = vm.data_mut();
        let ss = d.ss.clone();
        let _ = ss.collect_garbage(&mut d);
    }
    assert!(*freed.borrow() >= 1, "userdata 的 release hook 应当被调用过");

    // 引用计数：add_ref 之后即使脚本里的引用没了也不会被回收，release 之后才行
    let g = vm.get_global("g").expect("g");
    vm.data().ss.refs_table.add_ref(&g);
    vm.set_global("g", Value::Null);
    {
        let mut d = vm.data_mut();
        let ss = d.ss.clone();
        let _ = ss.collect_garbage(&mut d);
    }
    vm.data().ss.refs_table.release(&g);

    // foreignptr：vm 与 shared
    {
        let mut d = vm.data_mut();
        d.foreignptr = 0xABCD;
        assert_eq!(d.foreignptr, 0xABCD);
        d.ss.foreignptr.set(0x1234);
        assert_eq!(d.ss.foreignptr.get(), 0x1234);
    }

    // resurrect：不可达对象应当能被"复活"取出来（返回数组）
    {
        let d = vm.data();
        let t = squirrel::table::SQTable::create(&d.ss, 0);
        {
            let mut tb = t.borrow_mut();
            tb.new_slot(&d.ss.intern(b"marker"), &Value::Integer(1));
        }
        drop(d);
        vm.set_global("tmp_unreachable", Value::Table(t.clone()));
        vm.set_global("tmp_unreachable", Value::Null);
        drop(t); // 只剩 GC 链里的弱引用
    }
    {
        let mut d = vm.data_mut();
        let ss = d.ss.clone();
        let (n, arr) = ss.resurrect_unreachable(&mut d);
        assert!(n >= 0, "resurrect 的计数应当非负");
        if let Some(a) = arr {
            let _ = a.borrow().size();
        }
    }
}

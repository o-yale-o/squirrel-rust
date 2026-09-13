//! 迭代式释放（健壮性基础设施）。
//!
//! 问题：`Rc` 容器的默认 Drop 是**递归**的——深嵌套结构（例如脚本里
//! `for(...) a = [a];` 构造出的十万层数组）在释放时会一层层递归，直接打爆宿主栈
//! （不可捕获的 abort，不是脚本错误）。
//!
//! 方案：每个可能持有 `Value` 的容器都实现 `Drop`，把自己的子值交给这里。
//! `take_children` 会在容器被释放前把它「唯一持有」的子值搬进显式工作栈，
//! 于是容器自己的 Drop 面对的是一个空容器，递归深度恒为 O(1)。
//!
//! 刻意**不用 thread_local**：TLS 在线程析构期间不可访问，而 `Rc` 可能正好
//! 存活在别的 TLS 里（例如引擎自己的 placeholder 原型、blob 类），那时访问 TLS
//! 会 panic。这里只依赖局部工作栈，任何时机都安全。

use crate::object::Value;

/// 释放循环的嵌套深度（>1 说明还有递归）。仅用于自检测试。
pub static RELEASE_DEPTH: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// 观察到的最大嵌套深度
pub static MAX_RELEASE_DEPTH: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// 容器 Drop 的入口：把子值交给迭代释放器
pub fn release_values(children: Vec<Value>) {
    if children.is_empty() {
        return;
    }
    use std::sync::atomic::Ordering::Relaxed;
    let d = RELEASE_DEPTH.fetch_add(1, Relaxed) + 1;
    MAX_RELEASE_DEPTH.fetch_max(d, Relaxed);
    let mut work = children;
    while let Some(v) = work.pop() {
        take_children(v, &mut work);
    }
    RELEASE_DEPTH.fetch_sub(1, Relaxed);
}

/// 单个值的迭代释放
pub fn release_value(child: Value) {
    release_values(vec![child]);
}

/// 若 `v` 是「唯一持有」的容器，把它的子值搬进 `work`，随后浅释放 `v` 本身。
///
/// - 引用计数 > 1：这次释放不会销毁容器，本就不会递归，直接放手即可；
/// - 借用冲突等异常情况：容器带着子值正常 Drop，由它自己的 Drop 再次进入本模块
///   （多一层帧，但不会无限递归，也绝不 panic）。
fn take_children(v: Value, work: &mut Vec<Value>) {
    match &v {
        Value::Array(rc) => {
            if std::rc::Rc::strong_count(rc) == 1 {
                if let Ok(mut a) = rc.try_borrow_mut() {
                    work.append(&mut a.values);
                }
            }
        }
        Value::Table(rc) => {
            if std::rc::Rc::strong_count(rc) == 1 {
                if let Ok(mut t) = rc.try_borrow_mut() {
                    for n in t.nodes.drain(..) {
                        work.push(n.key);
                        work.push(n.val);
                    }
                    if let Some(d) = t.delegate.take() {
                        work.push(Value::Table(d));
                    }
                }
            }
        }
        Value::Instance(rc) => {
            if std::rc::Rc::strong_count(rc) == 1 {
                if let Ok(mut i) = rc.try_borrow_mut() {
                    work.append(&mut i.values);
                    if let Some(c) = i.class.take() {
                        work.push(Value::Class(c));
                    }
                    if let Some(d) = i.delegate.take() {
                        work.push(Value::Table(d));
                    }
                }
            }
        }
        Value::Class(rc) => {
            if std::rc::Rc::strong_count(rc) == 1 {
                if let Ok(mut c) = rc.try_borrow_mut() {
                    work.push(Value::Table(c.members.clone()));
                    if let Some(b) = c.base.take() {
                        work.push(Value::Class(b));
                    }
                    for m in c.defaultvalues.drain(..) {
                        work.push(m.val);
                        work.push(m.attrs);
                    }
                    for m in c.methods.drain(..) {
                        work.push(m.val);
                        work.push(m.attrs);
                    }
                    for mm in c.metamethods.drain(..) {
                        work.push(mm);
                    }
                    work.push(std::mem::replace(&mut c.attributes, Value::Null));
                }
            }
        }
        Value::Closure(rc) => {
            if std::rc::Rc::strong_count(rc) == 1 {
                if let Ok(mut c) = rc.try_borrow_mut() {
                    work.append(&mut c.outervalues);
                    work.append(&mut c.defaultparams);
                    if let Some(b) = c.base.take() {
                        work.push(Value::Class(b));
                    }
                }
            }
        }
        Value::NativeClosure(rc) => {
            if std::rc::Rc::strong_count(rc) == 1 {
                if let Ok(mut c) = rc.try_borrow_mut() {
                    work.append(&mut c.outervalues);
                    work.push(std::mem::replace(&mut c.name, Value::Null));
                }
            }
        }
        Value::Generator(rc) => {
            if std::rc::Rc::strong_count(rc) == 1 {
                if let Ok(mut g) = rc.try_borrow_mut() {
                    work.append(&mut g.stack);
                    work.push(std::mem::replace(&mut g.closure, Value::Null));
                }
            }
        }
        Value::Userdata(rc) => {
            if std::rc::Rc::strong_count(rc) == 1 {
                if let Ok(mut u) = rc.try_borrow_mut() {
                    if let Some(d) = u.delegate.take() {
                        work.push(Value::Table(d));
                    }
                }
            }
        }
        // outer 变量单元：闭包链（function(){ return g; }）就是靠它串起来的
        Value::Outer(rc) => {
            if std::rc::Rc::strong_count(rc) == 1 {
                if let Ok(mut o) = rc.try_borrow_mut() {
                    work.push(std::mem::replace(&mut o.value, Value::Null));
                }
            }
        }
        // 线程：挂起中的线程栈里可能压着很深的值
        Value::Thread(rc) => {
            if std::rc::Rc::strong_count(rc) == 1 {
                if let Ok(mut tvm) = rc.try_borrow_mut() {
                    work.append(&mut tvm.stack);
                    work.push(std::mem::replace(&mut tvm.roottable, Value::Null));
                    work.push(std::mem::replace(&mut tvm.lasterror, Value::Null));
                    work.push(std::mem::replace(&mut tvm.errorhandler, Value::Null));
                    work.push(std::mem::replace(&mut tvm.temp_reg, Value::Null));
                    work.push(std::mem::replace(&mut tvm.debughook_closure, Value::Null));
                    for ci in tvm.callsstack.drain(..) {
                        work.push(ci.closure);
                        if let Some(g) = ci.generator {
                            work.push(Value::Generator(g));
                        }
                    }
                }
            }
        }
        // 其余类型不持有 Value（字符串/数值/weakref 的 Weak 句柄不参与释放）
        _ => {}
    }
    // v 在这里释放：子值已被搬空 → 其 Drop 面对的容器是空的，不会再递归
    drop(v);
}

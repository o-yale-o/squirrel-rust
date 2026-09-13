//! **Rust 宿主嵌入 Squirrel 的最小可用骨架**
//!
//! 场景：Rust 服务用 Squirrel 脚本做「规则/配置扩展」——宿主提供能力、脚本写业务规则，
//! 两边互相调用，脚本坏了只影响这一次调用（不崩宿主），改脚本不需要重启进程。
//!
//! 跑法：
//! ```bash
//! cargo run --example embed -p squirrel-stdlib
//! ```
//!
//! 演示了 8 件事（每一段都有 `--- N. ---` 分隔）：
//! 1. 建 VM、把脚本的 `print`/错误输出接到宿主的日志
//! 2. **脚本 → Rust**：注册宿主函数（带参数个数/类型掩码检查、返回错误）
//! 3. **宿主对象**：把 Rust 的状态以「表 + native 闭包」暴露给脚本
//! 4. 跑一段脚本（宿主内嵌源码）
//! 5. **Rust → 脚本**：传表参数调用规则函数、取返回值、遍历结果
//! 6. **错误处理**：脚本抛错 → 宿主拿到 `Err(Value)`，并看到错误钩子打印的调用栈
//! 7. **热重载**：脚本从文件读入 → 改文件 → 重新编译加载（宿主不重启）
//! 8. 指令预算（`op_budget`）：给失控脚本封顶，宿主还能继续服务

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use squirrel::api::Vm;
use squirrel::array::SQArray;
use squirrel::config::*;
use squirrel::object::{get_type_name, Value};
use squirrel::table::SQTable;
use squirrel::vm::VmData;

// ---------------------------------------------------------------------------
// 宿主状态：真实场景里这可能是连接池、配置、指标……这里用一个计数器 + 配置表代替
// ---------------------------------------------------------------------------
#[derive(Default)]
struct HostState {
    /// 脚本调用 `host_log` 的次数
    log_count: u64,
    /// 规则命中次数（由脚本调用 `host.bump()` 累加）
    hits: u64,
    /// 宿主配置（脚本通过 `host_config(key)` 读）
    config: HashMap<String, String>,
}

impl HostState {
    fn new() -> Self {
        let mut config = HashMap::new();
        config.insert("region".to_string(), "cn-north".to_string());
        config.insert("max_amount".to_string(), "1000".to_string());
        HostState {
            config,
            ..Default::default()
        }
    }
}

/// 脚本里的 `print` / 错误输出 → 宿主的 stdout（真实项目里换成 log/tracing 即可）
fn make_vm() -> Vm {
    let vm = Vm::new(1024);
    vm.set_print_funcs(
        Some(Rc::new(|_v, bytes: &[u8]| {
            // 注意：参考实现是非 UNICODE 构建，脚本里可能是任意字节，别假设 UTF-8
            print!("[script] {}", String::from_utf8_lossy(bytes));
        })),
        Some(Rc::new(|_v, bytes: &[u8]| {
            eprint!("[script:err] {}", String::from_utf8_lossy(bytes));
        })),
    );
    // 标准库（print/表/数组/字符串/math/io…）与错误处理器（打印 CALLSTACK/LOCALS）
    squirrel_stdlib::register_all(&mut vm.data_mut());
    squirrel_stdlib::set_error_handlers(&vm);
    vm
}

fn main() {
    println!("=== 1. 建 VM ===");
    let vm = make_vm();
    println!("VM 就绪（初始栈 1024 槽）");

    let state = Rc::new(RefCell::new(HostState::new()));

    // -----------------------------------------------------------------------
    println!("\n=== 2. 注册宿主函数（脚本 → Rust）===");
    // nparamscheck 含 this：`host_log(msg)` 是 1 个参数 → 写 2；typemask 首位 `.` 是 this
    {
        let st = state.clone();
        vm.register_native(
            "host_log",
            Rc::new(move |v: &mut VmData| {
                let msg = v.stack_get(2);
                st.borrow_mut().log_count += 1;
                println!("[host] log#{}: {}", st.borrow().log_count, String::from_utf8_lossy(msg.as_bytes()));
                0 // 不返回值（脚本看到 null）
            }),
            2,
            Some(".s"),
        );
    }
    {
        let st = state.clone();
        vm.register_native(
            "host_config",
            Rc::new(move |v: &mut VmData| {
                let key = String::from_utf8_lossy(v.stack_get(2).as_bytes()).into_owned();
                match st.borrow().config.get(&key) {
                    Some(val) => {
                        let bytes = val.clone().into_bytes();
                        let s = v.ss.intern(&bytes);
                        v.push(s);
                        1
                    }
                    None => {
                        // 宿主里没有这个键 → 抛一个**可被脚本 catch** 的错误
                        v.raise_error_str(format!("no such config key: {}", key));
                        SQ_ERROR
                    }
                }
            }),
            2,
            Some(".s"),
        );
    }
    {
        // 无参宿主函数：nparamscheck = 1（只有 this）
        vm.register_native(
            "host_tick",
            Rc::new(|v: &mut VmData| {
                v.push(Value::Integer(7)); // 假装是宿主提供的单调计数
                1
            }),
            1,
            Some("."),
        );
    }
    println!("已注册 host_log / host_config / host_tick");

    // -----------------------------------------------------------------------
    println!("\n=== 3. 宿主对象：Rust 状态以「表 + native 闭包」暴露给脚本 ===");
    {
        let st = state.clone();
        let ss = vm.data().ss.clone();
        let obj = SQTable::create(&ss, 2);
        let bump = {
            let st = st.clone();
            squirrel::closure::SQNativeClosure::create(
                &ss,
                Rc::new(move |v: &mut VmData| {
                    let n = v.stack_get(2).as_integer(); // host.bump(n)
                    st.borrow_mut().hits += n.max(0) as u64;
                    v.push(Value::Integer(st.borrow().hits as SQInteger));
                    1
                }),
                0,
            )
        };
        {
            let mut nc = bump.borrow_mut();
            nc.nparamscheck = 2;
            nc.name = ss.intern(b"bump");
            squirrel::state::compile_typemask(&mut nc.typecheck, b".n");
        }
        let hits = {
            let st = st.clone();
            squirrel::closure::SQNativeClosure::create(
                &ss,
                Rc::new(move |v: &mut VmData| {
                    v.push(Value::Integer(st.borrow().hits as SQInteger));
                    1
                }),
                0,
            )
        };
        {
            let mut nc = hits.borrow_mut();
            nc.nparamscheck = 1;
            nc.name = ss.intern(b"hits");
            squirrel::state::compile_typemask(&mut nc.typecheck, b".");
        }
        obj.borrow_mut().new_slot(&ss.intern(b"bump"), &Value::NativeClosure(bump));
        obj.borrow_mut().new_slot(&ss.intern(b"hits"), &Value::NativeClosure(hits));
        let key = ss.intern(b"host");
        vm.set_global("host", Value::Table(obj));
        let _ = key;
    }
    println!("已暴露 host.bump(n) 与 host.hits()");

    // -----------------------------------------------------------------------
    println!("\n=== 4. 跑一段脚本（内嵌源码）===");
    let src = r#"
        // 规则脚本：宿主提供能力，这里写业务规则
        local rules = [];

        function add_rule(name, max_amount) {
            rules.append({ name = name, max = max_amount });
            return rules.len();
        }

        // 宿主每次事件都会调它：参数是宿主构造的表，返回值给宿主判断
        function on_event(evt) {
            host_log("on_event: " + evt.kind + " amount=" + evt.amount);
            local hit = null;
            foreach (r in rules) {
                if (evt.amount <= r.max) { hit = r.name; break; }
            }
            if (hit != null) {
                local total = host.bump(1);
                return { action = "allow", rule = hit, hits = total, tick = host_tick() };
            }
            return { action = "deny", reason = "no rule matched" };
        }

        function config_of(k) { return host_config(k); }

        local n = add_rule("small", 100);
        n = add_rule("medium", 1000);
        print("rules=" + n + " region=" + host_config("region") + "\n");
    "#;
    match vm.run_buffer(src.as_bytes(), b"rules.nut") {
        Ok(_) => println!("脚本加载并执行完成"),
        Err(e) => {
            eprintln!("脚本加载失败：{}", e);
            return;
        }
    }

    // -----------------------------------------------------------------------
    println!("\n=== 5. Rust → 脚本：调规则函数并取返回值 ===");
    let on_event = vm.get_global("on_event").expect("on_event 应当存在");
    let root = vm.data().roottable.clone();
    for (kind, amount) in [("order", 50), ("order", 5000)] {
        // 构造参数表：{ kind = "...", amount = n }
        let evt = {
            let ss = vm.data().ss.clone();
            let t = SQTable::create(&ss, 2);
            t.borrow_mut().new_slot(&ss.intern(b"kind"), &ss.intern(kind.as_bytes()));
            t.borrow_mut().new_slot(&ss.intern(b"amount"), &Value::Integer(amount));
            Value::Table(t)
        };
        match vm.call(&on_event, root.clone(), &[evt], true) {
            Ok(ret) => {
                // 返回值是个表，宿主按需取字段（越界/缺失返回 None，不会崩）
                if let Value::Table(t) = &ret {
                    let tb = t.borrow();
                    let ss = vm.data().ss.clone();
                    let action = tb.get(&ss.intern(b"action"));
                    let rule = tb.get(&ss.intern(b"rule"));
                    println!(
                        "[host] kind={} amount={} → action={} rule={}",
                        kind,
                        amount,
                        action.as_ref().and_then(|v| get_type_name(v)).unwrap_or("?"),
                        rule.map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned())
                            .unwrap_or_else(|| "-".to_string())
                    );
                } else {
                    println!("[host] 返回值不是表：{:?}", ret);
                }
            }
            Err(e) => println!("[host] 调用失败：{}", squirrel::api::value_to_error_string(&e)),
        }
    }

    // -----------------------------------------------------------------------
    println!("\n=== 6. 错误处理：脚本抛错 → 宿主拿 Err，脚本自己也能 catch ===");
    // 6a. 脚本没有这个配置键 → 宿主函数抛的错被脚本 catch 住
    let cfg = vm.get_global("config_of").expect("config_of");
    // 注意：`vm.data()` 的借用会活到语句结束，所以先把参数取出来再 call
    let missing_key = vm.data().ss.intern(b"nope");
    match vm.call(&cfg, root.clone(), &[missing_key], true) {
        Ok(_) => println!("[host] 意外成功"),
        Err(e) => println!("[host] 预期内的错误：{}", squirrel::api::value_to_error_string(&e)),
    }
    // 6b. 未捕获的脚本错误 → 宿主拿到 Err(Value)，错误对象**就是脚本抛的东西**
    let bad = vm.compile_buffer(
        b"throw { code = 42, msg = \"bad input\" };\n",
        b"boom.nut",
        true,
    );
    match bad {
        Ok(clo) => {
            let r = vm.call(&clo, root.clone(), &[], true);
            match r {
                Err(e) => {
                    if let Value::Table(t) = &e {
                        let tb = t.borrow();
                        let code = tb
                            .get(&vm.data().ss.intern(b"code"))
                            .map(|v| v.as_integer())
                            .unwrap_or(-1);
                        println!("[host] 捕获到脚本抛的表：code={}（类型 {}）", code, get_type_name(&e).unwrap_or("?"));
                    } else {
                        println!("[host] 捕获到：{}", squirrel::api::value_to_error_string(&e));
                    }
                }
                Ok(v) => println!("[host] 意外成功：{:?}", v),
            }
        }
        Err(()) => println!("[host] 编译就该失败"),
    }

    // -----------------------------------------------------------------------
    println!("\n=== 7. 热重载：改脚本文件 → 重新编译加载（宿主不重启）===");
    let path = std::env::temp_dir().join("sqrust_embed_rules.nut");
    std::fs::write(&path, b"function version() { return \"v1\"; }\n").unwrap();
    // 典型用法：编译新脚本 → 立刻调用它的入口函数（旧定义被覆盖）
    let load_version = |vm: &Vm| -> String {
        let src = std::fs::read(&path).unwrap();
        let clo = vm.compile_buffer(&src, b"reload.nut", true).expect("编译应当成功");
        let root = vm.data().roottable.clone(); // 先绑定：借用不能跨 call
        vm.call(&clo, root.clone(), &[], true).expect("执行 chunk");
        let version = vm.get_global("version").expect("脚本应当定义了 version()");
        let r = vm.call(&version, root, &[], true).expect("调用 version()");
        String::from_utf8_lossy(r.as_bytes()).into_owned()
    };
    println!("[host] 加载到 {}", load_version(&vm));
    std::fs::write(&path, b"function version() { return \"v2\"; }\n").unwrap();
    println!("[host] 改了文件后加载到 {}", load_version(&vm));
    let _ = std::fs::remove_file(&path);

    // -----------------------------------------------------------------------
    println!("\n=== 8. 指令预算：失控脚本封顶，宿主还能继续服务 ===");
    // 顶层死循环（真失控脚本的形态；编译成 chunk 后一调就停不下来）
    let loop_clo = vm
        .compile_buffer(b"local i = 0; while (true) { i = i + 1; }\n", b"spin.nut", true)
        .expect("编译");
    vm.data_mut().op_budget = 200_000; // 约 20 万条指令
    let r = vm.call(&loop_clo, root.clone(), &[], true);
    println!(
        "[host] 失控脚本结果：{}",
        match r {
            Ok(_) => "意外跑完了".to_string(),
            Err(e) => format!("{}（可捕获，宿主存活）", squirrel::api::value_to_error_string(&e)),
        }
    );
    vm.data_mut().op_budget = -1; // 关掉预算

    // -----------------------------------------------------------------------
    println!("\n=== 收尾 ===");
    // ⚠️ 坑：宿主持有自己状态的借用时**不要调进 VM** —— native 里会再借同一个
    // `RefCell`，直接 panic（RefCell already borrowed）。所以用块把借用限制在打印里。
    {
        let st = state.borrow();
        println!(
            "宿主状态：log_count={} hits={} config_keys={}",
            st.log_count,
            st.hits,
            st.config.len()
        );
    }
    // 还能继续正常调用（证明前面那些错误没有把 VM 带坏）
    let again = vm.call(&on_event, root.clone(), &[{
        let ss = vm.data().ss.clone();
        let t = SQTable::create(&ss, 1);
        t.borrow_mut()
            .new_slot(&ss.intern(b"kind"), &ss.intern(b"after-errors"));
        t.borrow_mut().new_slot(&ss.intern(b"amount"), &Value::Integer(10));
        Value::Table(t)
    }], true);
    println!("错误之后仍可调用：{}", if again.is_ok() { "OK" } else { "FAIL" });
    let _ = SQArray::create(&vm.data().ss, 0); // 顺带示引：数组构造在 host 侧也直接可用
}

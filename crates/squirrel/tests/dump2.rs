use squirrel::api::Vm;
use squirrel::object::Value;

#[test]
fn dump_trycatch() {
    let vm = squirrel::api::Vm::new(1024);
    if let Ok(clo) = vm.compile_buffer(b"try { throw \"boom\"; } catch(e) { print(\"caught\"); }", b"cmp2.nut", false) {
        if let Value::Closure(c) = &clo {
            let p = c.borrow().function.clone();
            for (i, ins) in p.instructions.iter().enumerate() {
                println!("[{:03}] {:<12} {} {} {} {}", i, ins.opcode().name(), ins.arg0, ins.arg1, ins.arg2, ins.arg3);
            }
        }
    }
}

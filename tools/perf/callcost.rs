// 微基准：模拟解释器主循环里"一条指令"的成本结构。
// 目的：区分 (1) 语言级函数调用成本  (2) 该移植的结构选择成本。
// 四种形态：
//   noinline : #[inline(never)] 的 &mut self 小方法（= 当前 Rust 引擎的形态）
//   inline   : 同样的小方法但 #[inline(always)]（= A 档想做的最小改动）
//   raw      : 指针提前提入局部变量 + 越界检查（= C++ Squirrel 的形态，B 档）
//   rc-payload: 槽里放引用计数对象，测克隆/析构代价（= Value 的可计数分支）
// 用 volatile 自增量阻断常量折叠，保证每一行都是真实循环。
//
// 与 callcost.cpp 逐行对应，用 build_callcost.bat 一起构建运行。
use std::hint::black_box;
use std::rc::Rc;
use std::time::Instant;

const N: usize = 200_000_000;

static mut INC: u64 = 1;
#[inline(always)]
fn inc() -> u64 {
    unsafe { std::ptr::read_volatile(&raw const INC) }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Plain {
    tag: u64,
    payload: u64,
}

struct Vm {
    stack: Vec<Plain>,
    base: usize,
    rcstack: Vec<Rc<u64>>,
}

impl Vm {
    fn new() -> Vm {
        Vm {
            stack: vec![Plain { tag: 0, payload: 0 }; 64],
            base: 0,
            rcstack: vec![Rc::new(0u64); 64],
        }
    }

    // ---- 形态 1/2：与当前引擎一致的小方法 ----
    #[inline(never)]
    fn stk_noinline(&self, a: usize) -> Plain {
        self.stack[self.base + a]
    }
    #[inline(never)]
    fn set_noinline(&mut self, a: usize, v: Plain) {
        let i = self.base + a;
        if i >= self.stack.len() {
            self.stack.resize(i + 1, Plain { tag: 0, payload: 0 });
        }
        self.stack[i] = v;
    }
    #[inline(always)]
    fn stk_inline(&self, a: usize) -> Plain {
        self.stack[self.base + a]
    }
    #[inline(always)]
    fn set_inline(&mut self, a: usize, v: Plain) {
        let i = self.base + a;
        if i >= self.stack.len() {
            self.stack.resize(i + 1, Plain { tag: 0, payload: 0 });
        }
        self.stack[i] = v;
    }

    // 一条 "ADD slot0, slot0, imm" 指令：读-算-写同一槽，形成真依赖链
    fn run_noinline(&mut self) -> u64 {
        let mut acc = 0u64;
        for _ in 0..N {
            let a = self.stk_noinline(0);
            let r = Plain { tag: a.tag, payload: a.payload.wrapping_add(inc()) };
            self.set_noinline(0, r);
            acc = r.payload;
        }
        acc
    }
    fn run_inline(&mut self) -> u64 {
        let mut acc = 0u64;
        for _ in 0..N {
            let a = self.stk_inline(0);
            let r = Plain { tag: a.tag, payload: a.payload.wrapping_add(inc()) };
            self.set_inline(0, r);
            acc = r.payload;
        }
        acc
    }
    // 形态 3：C++ 引擎的形态 —— 指针/基址提入局部变量
    fn run_raw(&mut self) -> u64 {
        let mut acc = 0u64;
        let p = self.stack.as_mut_ptr();
        let base = self.base;
        for _ in 0..N {
            unsafe {
                let a = *p.add(base);
                let r = Plain { tag: a.tag, payload: a.payload.wrapping_add(inc()) };
                *p.add(base) = r;
                acc = r.payload;
            }
        }
        acc
    }

    // ---- 形态 4：槽里是引用计数对象（克隆/析构要走计数分支） ----
    #[inline(never)]
    fn rcget_noinline(&self, a: usize) -> Rc<u64> {
        self.rcstack[self.base + a].clone()
    }
    #[inline(never)]
    fn rcset_noinline(&mut self, a: usize, v: Rc<u64>) {
        self.rcstack[self.base + a] = v;
    }
    fn run_rc_noinline(&mut self) -> u64 {
        let mut acc = 0u64;
        for _ in 0..N {
            let a = self.rcget_noinline(0); // 计数 +1
            let b = a.clone(); // 计数 +1
            self.rcset_noinline(0, b); // 旧值计数 -1
            acc = *a + inc();
        }
        acc
    }
}

fn ns_per_op(secs: f64) -> f64 {
    secs * 1e9 / N as f64
}

fn main() {
    let mut vm = Vm::new();

    let t = Instant::now();
    let a1 = vm.run_noinline();
    let t1 = t.elapsed().as_secs_f64();

    let t = Instant::now();
    let a2 = vm.run_inline();
    let t2 = t.elapsed().as_secs_f64();

    let t = Instant::now();
    let a3 = vm.run_raw();
    let t3 = t.elapsed().as_secs_f64();

    let t = Instant::now();
    let a4 = vm.run_rc_noinline();
    let t4 = t.elapsed().as_secs_f64();

    println!("rust  noinline  : {:7.2} ns/op   (chk {})", ns_per_op(t1), black_box(a1));
    println!("rust  inline    : {:7.2} ns/op   (chk {})", ns_per_op(t2), black_box(a2));
    println!("rust  raw+local : {:7.2} ns/op   (chk {})", ns_per_op(t3), black_box(a3));
    println!("rust  rc-payload: {:7.2} ns/op   (chk {})", ns_per_op(t4), black_box(a4));
}

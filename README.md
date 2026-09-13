# squirrel-rust

Squirrel 3.2 脚本引擎的 Rust 复刻（仅 Windows），基于官方 C++ 实现
[albertodemichelis/squirrel](https://github.com/albertodemichelis/squirrel)
逐模块移植，目标是**行为与字节码级兼容**。

## 兼容性验证（对齐 C++ Win64 默认构建：`_SQ64`、`SQFloat=f32`、非 UNICODE）

差分一律比对 **stdout + stderr + 退出码**（错误钩子的 CALLSTACK/LOCALS 全在 stderr）。

| 验证项 | 结果 |
|---|---|
| 官方 samples/ 全部 17 个脚本差分 | **17/17 一致** |
| 自建边界语料 40 个（浮点/整数溢出/位运算/错误消息与调用栈/GC/委托/metamethod/闭包/生成器/format/math/BOM…） | **40/40 一致** |
| REPL 交互会话（`=expr`、续行、错误后继续、`quit()`） | **5/5 一致** |
| `.cnut` 字节码：同一脚本双方 `-c` 输出 | **17/17 字节级完全一致** |
| `.cnut` 互通：C++ 编译 → Rust 执行（反向亦同） | **17/17 通过** |
| `cargo test`（单元 + 集成） | **50/50 通过** |

## 构建

```bash
cargo build --release          # 产出 target/release/sq.exe
cargo test                     # 单元 + 集成测试
```

差分测试（需要先构建参考二进制，见下）：

```bash
python tools/diff_test.py                       # 官方 samples
python tools/diff_test.py tests-corpus          # 边界语料
python tools/diff_test.py --no-stderr ...       # 只比 stdout（旧行为）
python tools/repl_diff.py                       # REPL 会话（tests-repl/*.txt）
```

参考 C++ 二进制的构建（一次性）：

```bash
cmake -S D:\OpenSource\squirrel -B build-ref -G "Visual Studio 17 2022" -A x64
cmake --build build-ref --config Release --target sq_static
```

## 用法（sq.exe 与原版 CLI 一致）

```bash
sq.exe script.nut [args...]       # 执行脚本
sq.exe -o out.cnut -c script.nut  # 编译为字节码（-o 必须写在脚本路径之前，与原版一致）
sq.exe -d script.nut              # 生成调试信息
sq.exe                            # 交互式 REPL（支持 =expr 求值、花括号续行、quit()）
```

## 作为 Rust 库使用

```rust
use squirrel::api::Vm;

let vm = Vm::new(1024);
vm.set_print_funcs(Some(/* print 钩子 */), Some(/* error 钩子 */));
match vm.run_buffer(b#"print("Hello World!")"#, b"main.nut") {
    Ok(retval) => { /* 脚本返回值 */ }
    Err(e) => eprintln!("error: {}", e),
}
```

更细粒度的 API：`compile_buffer`（编译为闭包）、`call_top`（调用栈顶闭包）、
`squirrel::serialization::{read_closure, write_closure}`（.cnut 读写）。

## 架构（模块 ↔ C++ 原版文件一一对应）

| Rust 模块 | C++ 对应 | 说明 |
|---|---|---|
| `config` | sqconfig.h | 类型配置（i64/f32/字节串） |
| `object` | sqobject.h/.cpp | 18 种类型的 `Value` 枚举、weakref、哈希 |
| `table` | sqtable.h/.cpp | Lua4 开链哈希表，**物理布局逐位复刻**（foreach 顺序一致） |
| `array`/`userdata` | sqarray.h/squserdata.h | |
| `opcodes` | sqopcodes.h | 61 条指令、`#[repr(C)]` 8 字节编码 |
| `funcproto` | sqfuncproto.h/.cpp | 函数原型 |
| `closure` | sqclosure.h/.cpp | 闭包/outer/generator/nativeclosure |
| `class` | sqclass.h/.cpp | 类/实例/继承/metamethod 槽 |
| `state` | sqstate.h/.cpp | SharedState、字符串表、RefTable、GC（Weak 注册表 + BFS 标记） |
| `lexer` | sqlexer.cpp | 词法（含 `\u`、verbatim 字符串、数字格式） |
| `funcstate` | sqfuncstate.cpp | 寄存器分配、跳转回填、**窥孔优化逐条复刻** |
| `compiler` | sqcompiler.cpp | 单遍递归下降编译器（全部语法） |
| `vm` | sqvm.cpp | Execute 主循环、Get/Set 回退链、异常 trap、线程 |
| `baselib` | sqbaselib.cpp | base 库 + 11 张默认委托表 |
| `serialization` | sqobject.cpp Save/Load | .cnut 读写（字节级兼容） |
| `cformat` | — | CRT `%g/%f/%e` 兼容格式化 |
| squirrel-stdlib | sqstdlib/* | math/system/io/string/aux + 正则（含 `\m` 平衡匹配等扩展） |
| squirrel-stdlib `stream` | sqstdstream/blob/file | `stream` 基类（readblob/readn/writeblob/writen/seek/tell/len/eos/flush）、`blob`（resize/swap2/swap4/索引/foreach/clone）、`file`（构造/close + stdout/stderr/stdin 实例） |
| sq (bin) | sq/sq.c | CLI + REPL |

## 与 C++ 的关键差异（语义保持，实现不同）

- 引用计数 + 仅回收环的辅助 GC → `Rc`/`Weak` + GC 注册表 BFS 标记；
- 栈槽裸指针 + `RelocateOuters` → 栈槽绝对索引（根除重定位）；
- `goto exception_trap` → `Result<_, Value>` + 带标签循环（trap 恢复语义一致）；
- `setjmp/longjmp` 编译错误恢复 → `Result` 传播；
- union 类型双关 → `Value` 枚举 + `to_bits/from_bits`。

## 已复刻的 C++ 怪癖（差分测试覆盖）

- `%` 除零**静默失败**（`_OP_MOD` 无 `_GUARD`，目标槽保持原值）；
- 表构造器内不允许匿名函数值（`{ f = function(){} }` 编译错误）；
- 表内函数用 `function name(){}`；`base` 是关键字不能作标识符；
- REPL 每行独立编译（`local` 不跨行）；`}` 出现在字符串里也会减少花括号计数；
- 运行时错误退出码为 0（retval 不被错误覆盖）；CLI 加载失败时退出码为 −2；
- `-o` 必须写在脚本路径之前，否则被当作脚本参数忽略；
- 整数除法向零截断；`10/4 == 2`；float 打印走 CRT `%g` 语义；
- `escape` 对 ≥0x80 的字节输出 32 位补码十六进制（`\xffffffe9`）；
- `date` 的 `month`/`wday`/`yday` 沿用 `tm` 约定（0 基），`'u'` 走 gmtime。

## 已知限制

- `file` 的 stdin 读取为行缓冲近似（CRT 是完全缓冲，交互式逐字节读有差异）；
- 少数**参考实现自身越界/UB** 的场景取确定性行为（详见 `STATUS.md` 第四节）：
  `escape` 短字符串中的高位字节、`getstackinfos(-1)`、UCRT `date` 上界、
  在已挂起的线程上再次 `call()`、REPL 读到 EOF、空正则模式（参考实现会写 NULL 崩溃）。

## 健壮性约定（硬约束）

> 任何对象在执行 / 解引用 / 索引之前，先做**非空、范围、类型**校验。
> 内部不变式被破坏时必须退化为**可捕获的脚本错误或 `Err`**，
> 不允许 `panic!` / `unreachable!` / 越界索引 / 算术溢出 / 递归爆栈——
> 那是宿主进程的 abort，脚本无法捕获。

已按此约定硬化并加测试守门（`cargo test` 会跑）：

| 测试 | 覆盖 |
|---|---|
| `crates/squirrel-stdlib/tests/robustness.rs` | 退化源码、脚本每个前缀/后缀、对抗性原生调用、畸形 `.cnut`、畸形正则、深递归、深嵌套值释放、超深嵌套编译 |
| `crates/squirrel/tests/deep_release.rs` | 释放循环嵌套深度 ≤ 3（10 万层数组/表/闭包/生成器/线程，含 1MB 栈线程） |

对应实现：`crates/squirrel/src/release.rs`（迭代式释放）、编译器嵌套/栈预算守卫、
反序列化长度上界校验。

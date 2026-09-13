# squirrel-rust

Squirrel 3.2 脚本引擎的 Rust 复刻（仅 Windows），基于官方 C++ 实现
[albertodemichelis/squirrel](https://github.com/albertodemichelis/squirrel)
逐模块移植，目标是**行为与字节码级兼容**。

## 兼容性验证（对齐 C++ Win64 默认构建：`_SQ64`、`SQFloat=f32`、非 UNICODE）

差分一律比对 **stdout + stderr + 退出码**（错误钩子的 CALLSTACK/LOCALS 全在 stderr）。

| 验证项 | 结果 |
|---|---|
| 官方 samples/ 全部 17 个脚本差分 | **17/17 一致** |
| 自建边界语料 **69** 个（浮点/整数溢出/位运算/错误消息与调用栈/GC/委托/metamethod/闭包/生成器/format/math/BOM/弱引用/正则/线程×GC/序列化…） | **69/69 一致** |
| 自动生成的组合矩阵（合计 **5600+ 条**）：运算符 × 类型（3181）、metamethod（343）、格式串（1305）、委托方法（2870）、类/实例（249）、**数值/运算符边界（4533）**、**正则语义（500）**、**生成器/线程（130）**、**词法/优先级/作用域（203）**、**weakref/GC（63）**、**blob/stream 二进制（133）**、**io/system/date（45）**、**异常机制（81）**、**类属性（40）**、**suspend 穿透（16）**、**多线程×GC（16）**、**序列化往返（25）**、根表顺序 | **逐行/逐字节一致** |
| 语法/词法非法源码的**编译错误文本**对拍 | **60/60 一致**（`tools/errmsg_matrix.py`） |
| REPL 交互会话（`=expr`、续行、错误后继续、`quit()`） | **5/5 一致** |
| `.cnut` 字节码：同一脚本双方 `-c` 输出 | samples **17/17**、语料 **69/69 字节级一致**（少数文件只差参考实现自己写入的**未初始化填充**，工具会标成 `PAD-ONLY`，见 STATUS.md 第四节） |
| `.cnut` 互通：C++ 编译 → Rust 执行（反向亦同） | **69/69 通过** |
| 宿主 API：用参考实现的静态库编 C 探针做同序对拍（member handle / `getfunctioninfo` / `setfreevariable` / userdata…） | 报告逐项一致（`tmp-c2/hostapi/`） |
| `cargo test`（单元 + 集成 + 健壮性 + 字节码互通 + 各语义矩阵） | **113/113 通过** |

> 「字节级一致」这条要小心理解：参考实现保存闭包的 outer 值时写 8 字节、字段只有 4 字节，
> 高 4 字节是未初始化内存，**同一个脚本连编两次输出都不同**。本实现写确定的 0，
> 其余字段逐字节相同（`tools/cnut_fields.py` 可逐字段核对，`cnut_matrix.py` 会把
> 只差这一处的文件标成 `PAD-ONLY` 而不是失败）。

## 构建

```bash
cargo build --release          # 产出 target/release/sq.exe
cargo test                     # 单元 + 集成测试
```

差分测试（需要先构建参考二进制，见下）：

```bash
python tools/diff_test.py                       # 官方 samples
python tools/diff_test.py tests-corpus          # 边界语料 + 三张生成矩阵
python tools/diff_test.py --no-stderr ...       # 只比 stdout（旧行为）
python tools/line_diff.py tests-corpus/t_opmx.nut   # 大语料按行定位差异
python tools/repl_diff.py                       # REPL 会话（tests-repl/*.txt）
python tools/cnut_matrix.py tests-corpus        # .cnut 字节一致 + 双向交叉执行
python tools/minimal_diff.py                    # 编出官方 etc/minimal.c 与示例对拍（验收 #5）
python tools/stdin_eof.py                       # stdin 在管道/文件重定向下的原语行为
python tools/audit_globals.py                   # 全局注册（名字/参数个数/类型掩码）
python tools/audit_delegates.py                 # 11 张默认委托表成员
python tools/audit_regorder.py                  # 根表**插入顺序**（迭代顺序由它决定）
python tools/lint_corpus.py                     # 语料语法陷阱 + 逐个试编译
python tools/mem_probe.py x.nut --limit-mb 400  # 带内存/时间上限跑（防失控；逐条排查加 --bisect）
```

python tools/errmsg_matrix.py                   # 语法/词法非法源码的编译错误文本对拍
python tools/audit_capi.py                      # C 嵌入 API 在本仓库的出现情况（粗筛）

矩阵语料都由脚本生成，改语料请改生成器后重新生成：`tools/gen_opmatrix.py`、
`gen_metaprobe.py`、`gen_formatmx.py`、`gen_delmatrix.py`、`gen_oopmatrix.py`、
`gen_nummatrix.py`、`gen_rxmatrix.py`、`gen_genmatrix.py`、`gen_lexmatrix.py`、
`gen_weakmap.py`、`gen_blobmap.py`、`gen_sysmap.py`、`gen_errmap.py`、
`gen_attrmap.py`、`gen_suspmap.py`、`gen_gcx.py`、`gen_ser.py`。

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
sq.exe --max-ops 5000000 script.nut   # 指令预算（本实现额外提供；超出报可捕获错误）
sq.exe --mem-limit 512 script.nut     # 进程内存上限 MB（本实现额外提供；Windows Job Object）
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
- `false < true` 恒成立（`ObjCmp` 对 bool 走指针分支，读的是 union 里的 0/1）；
- `fmod` 在异常输入上的 NaN 符号（`x % 0` 给 `nan`、`-inf % y` 给 `-nan(ind)` 等，见 `vm::sq_fmod`）；
- `format`/`printf`：MSVC 的整套标志语义，含 `%#g` 保留尾随零、`%#.0f` 遇 inf/nan 把小数点
  插在第 2 个字符位置（`i.nf`）、`%o` 的 `#` 补前导 0、整型给精度时忽略 `0`、
  inf/nan 忽略 `0`、宽度/精度各最多 2 位数字；
- 表构造器内不允许匿名函数值（`{ f = function(){} }` 编译错误）；
- 表内函数用 `function name(){}`；`base` 是关键字不能作标识符；
- REPL 每行独立编译（`local` 不跨行）；`}` 出现在字符串里也会减少花括号计数；
- 运行时错误退出码为 0（retval 不被错误覆盖）；CLI 加载失败时退出码为 −2；
- `-o` 必须写在脚本路径之前，否则被当作脚本参数忽略；
- 整数除法向零截断；`10/4 == 2`；float 打印走 CRT `%g` 语义；
- `escape` 对 ≥0x80 的字节输出 32 位补码十六进制（`\xffffffe9`）；
- `date` 的 `month`/`wday`/`yday` 沿用 `tm` 约定（0 基），`'u'` 走 gmtime；
- `print` 按裸字节输出（非 UNICODE 构建），`%s`/`print` 在第一个 NUL 处截断。

## 从 C++ 版迁移过来

有一类地方是**故意与参考实现不一致**的（参考实现在那些输入上会崩溃 / 内存损坏 /
把宿主吃光，我们给可捕获的错误），逐条带最小复现整理在 [`MIGRATION.md`](MIGRATION.md)；
另有 15 条"看着像 bug、其实两边一致"的怪癖清单也在那份文档里。

## 已知限制

- **本实现额外提供的加固**（参考实现没有对应能力，见 STATUS.md 三·补20）：
  `--max-ops N` 指令预算（尾调用死循环也能封顶）、`--mem-limit N` 进程内存上限
  （Windows Job Object）、未捕获 `stack overflow` 的调用栈 dump 限深
  （1000 帧，实测 stderr 12.3MB → 0.07MB）、编译器嵌套上限可调
  （库默认 200 层／CLI 800 层；实测参考实现 790 层能编、795 层 `0xC00000FD` 崩溃，
  本实现 790 层同样能编、更深的给可捕获的 `nesting too deep`）；
- **失控递归**：参考实现的 `EnterFrame` 没有栈上限，非尾递归的无限递归会一直涨到进程被
  系统杀掉（实测一条 3 行脚本把宿主吃到 32GB）。本实现封顶（1M 槽 / 20 万帧）并报
  **可捕获的** `stack overflow`，错误处理器另有固定余量保证「报得出、抓得住」；
- **两个不同对象之间的 `<=>`**：参考实现按裸指针序比较，跨进程/跨字节码不可复现
  （同一脚本 `-1` 与 `1` 都出现过），语料只比较同一个对象；
- **不可 seek 流（stdin/stdout/stderr 走管道时）的 `eos()/tell()/len()`**：参考实现基于 CRT 的
  `ftell`/`fseek`，在管道上 `fseek(SEEK_END)` 会失败并**丢弃已缓冲的输入**
  （8 字节管道读完 2 字节后剩的 6 字节直接消失，其后 `readn` 报 `io error`），
  `len()` 返回的是 CRT 内部状态（实测同一进程里先 ≥4、后 0、5000 字节管道上 904）。
  本实现给出确定性行为（`tell()`=已消费字节数、不可 seek 时 `len()`=0），
  文件重定向与参考实现逐字节一致（实测脚本：`python tools/stdin_eof.py`、
  `tools/stdin_diff.py`、`tools/probes/stream_semantics.nut`）；
- 少数**参考实现自身越界/UB** 的场景取确定性行为（详见 `STATUS.md` 第四节）：
  `escape` 短字符串中的高位字节、`getstackinfos(-1)` / `thread.getstackinfos(<负 level>)`、
  UCRT `date` 上界、在已挂起的线程上再次 `call()`、REPL 读到 EOF、
  空正则模式（参考实现会写 NULL 崩溃）、`blob.resize(<负>)` 与 `blob.readblob(<负>)`
  （参考实现直接访问违例）、`.cnut` 里 outer 类型字段的高 4 字节（未初始化内存）、
  坏字节码里的越界 outer 索引（参考实现访问违例崩溃）、
  `format` 规格串恰好 20 字符（参考实现栈溢出崩溃）。

## 健壮性约定（硬约束）

> 任何对象在执行 / 解引用 / 索引之前，先做**非空、范围、类型**校验。
> 内部不变式被破坏时必须退化为**可捕获的脚本错误或 `Err`**，
> 不允许 `panic!` / `unreachable!` / 越界索引 / 算术溢出 / 递归爆栈——
> 那是宿主进程的 abort，脚本无法捕获。

已按此约定硬化并加测试守门（`cargo test` 会跑）：

| 测试 | 覆盖 |
|---|---|
| `crates/squirrel-stdlib/tests/robustness.rs` | 退化源码、脚本每个前缀/后缀、对抗性原生调用（含 `format` 的宽度/精度/标志/`INT64_MIN` 边界）、畸形 `.cnut`、畸形正则、深递归、**失控递归（必须可捕获且有界，不许吃光宿主内存）**、深嵌套值释放、超深嵌套编译 |
| `crates/squirrel-stdlib/tests/bytecode_interop.rs` | 直接执行**参考实现编译出来的** `.cnut`；夹具被改成越界 outer 索引时必须报可捕获错误（参考实现会访问违例崩溃） |
| `crates/squirrel-stdlib/tests/stream_semantics.rs` | file/blob 的 `tell/len/eos/flush` 与参考实现逐字一致（`eos()` 真为 `1`、假为 `null`；`writen` 的参数顺序是 `(值, 格式)`） |
| `crates/squirrel/tests/deep_release.rs` | 释放循环嵌套深度 ≤ 3（10 万层数组/表/闭包/生成器/线程，含 1MB 栈线程） |

对应实现：`crates/squirrel/src/release.rs`（迭代式释放）、编译器嵌套/栈预算守卫、
反序列化长度上界校验、`CLOSURE_OP` 的 outer/默认参数槽范围检查。

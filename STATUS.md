# squirrel-rust 状态评估（STATUS）

> 评估方式：不采信 README 的既有数字，逐项用官方 C++ 参考二进制（`build-ref/bin/Release/sq_static.exe`）
> 与 `target/release/sq.exe` 双引擎复跑；每一条结论都附可复现命令与实测输出。
> 评估时间：本次会话；基线提交 `b85d17e`。

## 一、已完成且已独立验证

| 验证项 | 复现命令 | 实测结果 |
|---|---|---|
| 官方 samples 差分（stdout + stderr + 退出码） | `python tools/diff_test.py` | **17/17 一致** |
| 边界语料差分 | `python tools/diff_test.py tests-corpus` | **54/54 一致** |
| 类/实例语义矩阵（168 条自动 + 81 条语义） | 同上 | **逐行一致** |
| 委托方法 × 参数形态矩阵（2723 条自动 + 147 条语义） | 同上 | **逐行一致** |
| 根表插入/迭代顺序 | `python tools/diff_test.py tests-corpus`（`t_roottable.nut`） | **逐行一致**（注册顺序已逐库对齐） |
| 运算符 × 类型矩阵（3181 条断言） | `python tools/diff_test.py tests-corpus` | **逐行一致** |
| metamethod 全矩阵（343 个用例 × 18 个 metamethod） | 同上 | **逐行一致** |
| `format`/`printf` 格式串矩阵（1305 行） | 同上 | **逐行一致** |
| REPL 交互会话差分 | `python tools/repl_diff.py` | **5/5 一致** |
| `.cnut` 字节级一致 | `python tools/cnut_matrix.py` | samples **17/17**、语料 **54/54**（少数文件会被标成 `PAD-ONLY`：只差参考实现的未初始化填充，见第四节） |
| `.cnut` 双向互通 | `python tools/cnut_matrix.py tests-corpus` | **54/54 一致**（含参考实现编译的字节码在本实现上执行） |
| 指令覆盖 | `cargo test --test opcode_coverage` | **61/61 条指令被语料覆盖** |
| 宿主 API 嵌入（验收 #5） | `cargo run --example minimal` + `cargo test --test host_api` + `python tools/minimal_diff.py` | **5/5 通过**；`minimal_diff.py` 把官方 `etc/minimal.c` 真编出来跑，stdout/退出码与本实现**逐字节一致**（`Called foo(), i=1, f=2.5, s='teststring'`） |
| 流语义（file/blob 的 tell/len/eos/flush） | `python tools/line_diff.py tools/probes/stream_semantics.nut` | **逐行一致**（`cargo test --test stream_semantics` 守门） |
| stdin / 管道语义 | `python tools/stdin_eof.py` | 文件重定向 **5/5 一致**；管道上 `tell()/eos()` 对齐，残留见第四节 |
| 委托表成员 | `python tools/audit_delegates.py` | **10 张表 88 个成员全一致** |
| 全局注册（nparamscheck + 掩码） | `python tools/audit_globals.py` | **62/62 全一致** |
| 单元 + 集成 + 健壮性测试 | `cargo test` | **75/75 通过** |
| 全局环境覆盖 | `getroottable()` 快照对比 | root 表 **77 个名字逐字一致** |
| 原生错误消息文本 | 自动生成的错误对拍语料（331 处调用） | **逐行一致** |
| CLI 保真 | `-v` / `-h` / 编译错误消息 / `-o` 必须前置的怪癖 / 退出码 | 与原版一致 |

结论：**引擎主体（词法、编译器、VM、对象模型、类/OOP、生成器、GC、序列化、主要标准库）与 CLI 已完工，保真度高。**

## 二、未完成 / 实际不可用

以下为本次评估新发现（README「已知限制」仅覆盖 B 与部分 E）。

### A. math 库 18 个函数全部不可用 ★最严重

`crates/squirrel-stdlib/src/lib.rs` 的 `m1!` 宏把类型掩码原样注册，漏掉了 C++ 掩码里代表 `this` 的
占位 `.`，并且 `nparamscheck` 恒为 2。

```text
复现：tmp-c2/math_globals.nut
REF : sqrt OK float 2 / pow OK float 8 / rand OK integer 41 ...
RUST: sqrt FAIL parameter 0 has an invalid type 'table' ; expected: 'integer|float'
      pow  FAIL wrong number of parameters
      rand FAIL wrong number of parameters        → 18/18 全部失败（仅 PI、RAND_MAX 常量可用）
```

C++ 侧定义（`sqstdlib/sqstdmath.cpp:63-80`）：`sqrt,2,.n` / `atan2,3,.nn` / `pow,3,.nn` / `rand,1,NULL`。

同类注册错误还有 `writeclosuretofile`：Rust 用 `"cs"`，C++ 是 `".sc"`（`sqstdio.cpp:468`）。

**为什么没被测出来**：`tests-corpus/` 与官方 `samples/` 中**没有任何一处**调用 math 全局函数：

```text
grep -E '\b(sqrt|sin|cos|asin|acos|log|log10|tan|atan|atan2|pow|floor|ceil|exp|srand|fabs|abs)\s*\('
     tests-corpus/*.nut samples/*.nut   → 无匹配
```

### B. `getstackinfos` 的 locals（README 已承认；当前唯一红灯）

`tests-corpus/t_stkinfo.nut`（未提交的 WIP）**FAIL**，语料 29/30。不只是格式简化，值还整体错位：

```text
REF : a=42 x=85 s="str" b=43 t={table} this={table}
RUST: a=43 x="str" s={table} b=85 t=(function : 0x...) this=42 infos={table}
```

即：名字与栈槽错位一格、多出正在赋值的局部变量 `infos`、`this` 读成了 `42`。

### C. REPL 两个缺陷（README 声称「支持 `=expr` 求值」）

- `=2*3` **直接 panic**：`crates/squirrel/src/vm.rs:1988` `unreachable!("execute on non-closure")`；
- `quit()` **不退出**：`crates/sq/src/main.rs:53` 把 `outervalues[0]` 写成占位 0、随后 `let _ = done;`，
  退出判断读到的永远是 0，只能靠 EOF 结束，并比原版多打一个 `sq>`。

### D. 错误钩子 / 调用栈打印与原版不一致（全在 stderr）

```text
REF : *FUNCTION [level2()] tmp-c2\err_a.nut line [1]      + LOCALS 段
RUST: *FUNCTION [unknown()] source line [-1]              （多一帧、源文件名变 "source"、无 LOCALS）
```

README 中「sqstdaux 调用栈打印格式逐字复刻」目前不成立。

### E. 源码 BOM / UCS2 嗅探缺失

带 UTF-8 BOM 或 UTF-16LE 的 `.nut`，原版正常执行，Rust 版报 `error expression expected`
（`crates/sq/src/main.rs:201` 注释已承认）。Windows 编辑器存盘极易触发。

### F. 零散差异（15 条错误消息对拍中的 3 条 + 格式差异）

| 项 | REF | RUST |
|---|---|---|
| `format("%e", 1234.5)` | `1.234500e+03` | `1.2345e+03`（丢尾零） |
| `split("a,b,,c", ",")` | 4 段（保留空段） | 丢空段 |
| `escape("a\nb")` | `a\x0ab` | `a\nb` |
| `({}).rawget("x")` | `the index doesn't exist` | `the index 'x' does not exist` |
| `loadfile("缺失")` | `cannot open the file` | `cannot open file '... (os error 2))'` |
| `format()` 无参 | `wrong number of parameters` | 正常返回 |

### G. 测试基础设施盲区

`tools/diff_test.py` 只比较 **stdout + 退出码**；而错误钩子输出全在 **stderr**。因此
`t_bit.nut`、`t_blob.nut` 这类走未捕获错误路径的用例虽然「通过」，其错误文本差异从未被检查
（D/F 类差异因此长期潜伏）。

### H. README 文档不准

- 语料数写 26/26，实际 **30 个、29 通过**；
- 用法示例 `sq.exe -c script.nut -o out.cnut` 有误：原版与 Rust 版都要求 `-o` 写在脚本路径**之前**
  （`sq -o out.cnut -c script.nut`），行为一致、文档错误。

## 三、修复清单与进度（全部已修复并回归）

- [x] A. math 库注册（`m1!` 补 `.`、`atan2/pow` nparams=3、`rand` nparams=1、`abs` 取整语义、
      `writeclosuretofile` → `.sc` 且栈槽反了）+ 新增 `tests-corpus/t_math.nut`、`t_closure_io.nut`
- [x] B. `getstackinfos` locals 槽位与内容（`t_stkinfo.nut` 转绿并扩充到 10 组断言）
- [x] C. REPL `=expr` panic（根因：宿主调用入口只接受脚本闭包）与 `quit()` 退出
- [x] D. `diff_test.py` 比较 stderr + 退出码；`sqstd_printcallstack` 重写（level 从 1 起、
      真实源文件名、LOCALS 0..9 段与类型化格式）；新增 `tools/repl_diff.py` + `tests-repl/`
- [x] E. BOM / UCS2 源码嗅探（`sqstd_loadfile` 全规则）+ `t_bom_cli.nut`、`t_loadfile_bom.nut`
- [x] F. `%e` 保留尾零、`split` 空段与分隔符为空、`escape` 控制字符、`rawget` 消息、
      `format()` 无参/类型检查、`date` 的 0 基月份 + `'u'/'l'` + localtime/CRT 范围
- [x] G. 正则引擎对齐：`{n,m}` 量词、`*?` 是「量词 + 字面问号」（参考实现只解析一次量词）、
      `^` 只在表达式起始处生效、`subexpcount` 含整体匹配、`typeof` 返回 `regexp`、
      `constructor` 参数检查、编译错误文本（`letter expected` 等）
- [x] H. blob 全局 `casti2f/castf2i/swap2/swap4/swapfloat`：掩码缺 `this` 占位（18 个 math
      函数同款 bug）导致全部不可调用；`swap4` 只交换低 32 位；`castf2i` 取位模式
- [x] I. **健壮性硬化**（见下节）：恶意 `.cnut`、超深源码、深嵌套值三条路径都能打死宿主
- [x] 全量回归：`cargo test` **63/63**、samples 17/17、语料 **41/41**、REPL 会话 5/5、
      `.cnut` 字节一致 17/17 + 交叉执行 17/17

## 三·补、健壮性约定（编码基本习惯）

> **任何对象在执行/解引用/索引之前，先做非空、范围、类型校验。**
> 引擎内部不变式被破坏时，必须退化为**可捕获的脚本错误或 `Err`**，
> 绝不允许 `panic!` / `unreachable!` / 越界索引 / 算术溢出 / 递归爆栈——
> 那些是宿主进程的 abort，脚本无法捕获，也不能算「参考实现也这样」。

这条约定由两个测试文件守门（新增原生函数、语法特性后如果引入可被脚本触发的 panic，
它们会直接变红）：

| 文件 | 覆盖 |
|---|---|
| `crates/squirrel-stdlib/tests/robustness.rs` | 退化源码、完整脚本的每个前缀/后缀、80+ 对抗性原生调用、畸形/截断/篡改 `.cnut`、畸形正则、深递归与恶意执行流、**深嵌套值释放**、**超深嵌套编译** |
| `crates/squirrel/tests/deep_release.rs` | 直接测释放循环的嵌套深度 ≤ 3（10 万层数组/表/混合结构，含 1MB 栈线程） |

本轮审计发现并修掉的三条「能打死宿主」的路径（都不需要恶意宿主，普通脚本即可触发）：

| 路径 | 现象 | 修复 |
|---|---|---|
| 畸形 `.cnut` 的计数 | `ninstructions as usize * 8` 溢出 panic（debug 直接崩） | 先按「每元素最小字节数 × 剩余字节」校验全部计数，再 `checked_mul`，并加函数嵌套深度上限、`stacksize` 上限 |
| 超深源码 | 约 700 层括号（release）/ 150 层（debug）就把宿主栈打爆（`STATUS_STACK_OVERFLOW`） | 编译期嵌套守卫：层数上限 200 **且** 实测宿主栈用量 ≤ 512KB，超限报可捕获的 `nesting too deep` |
| 深嵌套值释放 | `for(...) a = [a];` 十万层后 `a = null` → `Rc` 递归 Drop 爆栈 | 新增 `crates/squirrel/src/release.rs`：显式工作栈迭代释放；Array/Table/Instance/Class/Closure/NativeClosure/Generator/UserData/**Outer**/**Thread** 都实现 Drop 移交子值，嵌套深度恒为 O(1) |

同时把审计出的其它 panic 点改成优雅降级：`Instance::the_class()`（GC finalize 后返回
`None` 而不是 `expect`）、VM 的 `etraps.last()` 与生成器 `generator.unwrap()`（畸形字节码
下报脚本错误）、`closure` 的 trap 搬运、`cformat` 的 `split_once/parse` 解包、`array.pop`
的 `unwrap`、`serialization` 的 `raw` 长度算术。

> 顺带发现：参考实现自己也会**写 NULL 崩溃**——`regexp("")` 触发 Windows 崩溃框
> （不是死循环，是模态框让子进程看起来卡住）。本实现对该输入报可捕获的
> `letter expected`，并不把这条 UB 复刻进语料。

## 三·补2、验收标准补齐（宿主 API / 指令覆盖 / 委托表）

| 计划书验收项 | 现状 |
|---|---|
| #5 用 Rust 宿主 API 复刻 `etc/minimal.c` 嵌入示例 | ✅ `crates/squirrel-stdlib/examples/minimal.rs`（输出与 C++ 版一致：`Called foo(), i=1, f=2.5, s='teststring'`）+ `crates/squirrel-stdlib/tests/host_api.rs`（4 个测试：注册 native、宿主调脚本取返回值、dofile 后按名调用、错误路径与调用栈、全局读写、从宿主构造类并调方法）。为此给 `api.rs` 补了 `get_global/set_global/register_native/call` |
| 61 条指令被差分语料覆盖 | ✅ `crates/squirrel/tests/opcode_coverage.rs` 统计语料+samples 的指令直方图，**61/61**；新增 `tests-corpus/t_ops.nut`（本地/字段/数组元素/全局的自增自减、`!`、位运算、复合赋值、三目、`in`、`delete`、`clone`、`instanceof`）补齐原本缺失的 `Inc/PInc/Not` |
| 默认委托表成员审计 | ✅ `tools/audit_delegates.py`：C++ 的 10 张 `_*_default_delegate_funcz` 与 Rust 的 `fill_*_ddel` 逐项对比，**88 个成员 + nparamscheck + 掩码全部一致** |

**这一轮最有价值的发现是测试方法本身的漏洞**：`diff_test.py` 原本只比 stdout/stderr/退出码，
而**两边报同一个编译错误时完全一致 → 假绿**。我新写的 5 个广度语料里有 4 个其实根本没编译通过
（`{ b <- 2 }`、`local base`、`class X {} return` 同行、`const` 在函数内、`x &= 1`、
`f.rawcall()`、`-8 >>> 1`、裸枚举名……全是 Squirrel 不接受、两边报同样错误的写法），
于是"PASS"了一整轮却什么都没测到。类似地**脚本中途以未捕获错误结束**（两边一样地早停）
也会假绿：原语料里的 `t_edge1.nut`（`65.tochar()` 词法成浮点）与 `t_lambda.nut`
（具名参数块用错）都只跑了几行，而我生成的错误对拍语料 429 行只跑到 25 行。

现在有三重守卫（都已生效，并且各自当场抓到了问题）：

- `diff_test.py` 对 `Error [ ...` 开头的输出判 **SUSPECT 失败**；
- `diff_test.py` 对**未捕获错误导致提前结束**判 **SUSPECT 失败**（带白名单：测错误打印的用例）；
- `tools/lint_corpus.py` 扫语法陷阱 + 逐个试编译；`opcode_coverage.rs` 列出编译失败的语料。

修好语法后立刻暴露的问题（全部已修）：

1. **`o.setdelegate(o)` 让 Rust 侧 panic**（`RefCell already mutably borrowed`，rc=101）——
   环检测的 `this_identity` 用了内部结构体地址、与遍历时的 `Rc::as_ptr` 不一致，且 `borrow()`
   撞上调用方持有的 `borrow_mut`。改为身份统一用 `Rc::as_ptr` + 遍历用 `try_borrow`
   （借用失败等价于 C++ 的 `temp == this`，判环返回 false）。
2. **委托环错误文本**：参考是 `delegate cycle`（非表 this / 非表非空参数是 `unexpected type X`）。
3. **含 NUL 字符串的打印**：C++ 的 `_printfunc(v,"%s",str)` 在第一个 NUL 处截断
   （`print((0).tochar())` 整行不输出），Rust 侧原来把 NUL 也写出去 → 现在 `print/error/printf`
   都按 `%s` 语义截断。
4. **`rawset` 语义不完整**（自动错误对拍语料抓到）：实例/数组失败时参考用
   `Raise_IdxError(key)` 报 `the index 'X' does not exist`，而端口只返回 `SQ_ERROR` 不设消息
   （脚本 catch 到的是 **null**）；另外缺 `null key` 检查、缺数组分支、class 分支不该看返回值。
5. **`tointeger(base)` 进制**：参考支持 0（自动识别）/2..36，端口只认 10/16；
   `"z".tointeger(36)`、`"1".tointeger(0)` 都对不上 → 已按 CRT `strtol` 语义重写。
6. **`clock` / `split` 注册参数**（全局注册审计抓到）：`clock` 应为 nparams=0、`split` 应为 -3。
7. **`-0.0` 打印**：MSVC `%g` 输出 `-0`，端口丢符号 → 已修。
8. **具名参数块**的语义确认：`{ key = value }` 是 SET 到**调用返回值**上（配合返回实例/表的调用），
   例如 `C() { a = 1, b = 2 }`；写成 `print(kw(1) { b = 2 })` 两边都报错，原语料写法有误。

## 三·补3、第三轮：三张自动生成矩阵 + 字节码互通

前两轮的教训是「**没测到的路径等于没写**」，而手工挑样例总会漏。这一轮把「运算符 × 类型」
「metamethod × 触发方式」「格式串 × 值 × 标志」三张组合矩阵**直接生成**成语料
（共 4800+ 条断言，全部逐行对拍），另外把 `.cnut` 互通做成了可复现脚本。

| 生成器 | 产物 | 规模 |
|---|---|---|
| `tools/gen_opmatrix.py` | `tests-corpus/t_opmx.nut` | 3181 条：13 种运算符 × 12 种类型两两组合 + 一元/索引/成员/默认委托/赋值/自增自减 |
| `tools/gen_metaprobe.py` | `tests-corpus/t_meta3.nut` | 343 条：18 个 metamethod 的正常路径 / 返回值类型错 / 抛异常 / 参数个数错 / 值不是函数 / 没有 metamethod 时的回退与报错 |
| `tools/gen_formatmx.py` | `tests-corpus/t_formatmx.nut` | 1305 条：29 个浮点值 × 32 种说明符、10 个整数值 × 29 种说明符、字符串宽度精度、类型接受与解析错误 |

**这一轮抓到的真问题（全部已修）**：

1. **`false < true` 判反**（矩阵一跑就现形）。参考实现的 `ObjCmp` 对 `OT_BOOL` 走
   `default:` 分支，而 `_userpointer(o)` 读的是 union 里的整数（bool 就是 0/1），
   所以 `false <=> true` 恒为 −1。本实现原来按身份地址比较，结果随机。
2. **`fmod` 的 NaN 符号位**。参考走 UCRT 的 `fmod`，异常输入上的 NaN 符号与 Rust 的 `%`
   不同（`1.0 % 0.0` 参考给 `nan`、Rust 给 `-nan(ind)`）。新增 `vm::sq_fmod` 逐条对齐
   （±0/±inf/nan 全组合 64 条实测），并配单元测试。
3. **`_nexti` 取不到值时的错误文本**：参考实现先把 `Raise_IdxError` 的结果**覆盖**成
   `_nexti returned an invalid idx`，本实现直接透出了 `the index '0' does not exist`。
4. **`format` 的整套语义**（这一轮最大的一块，全部按 `sqstd_format` + MSVC printf 实测复刻）：
   - 类型接受：`%s` 只收字符串；整型说明符收 `integer|float|bool`（float 截断，
     越界给 `INT64_MIN`）；浮点说明符收 `integer|float`（**bool 不行**）；
     错误文本 `string/integer/float expected for the specified format`。
   - 标志：`-` `+` ` ` `#` `0`。`%#x/%#X` 前缀 `0x/0X`、`%#o` 补前导 0、
     `%#g` **保留尾随零**、`%#f/%#e` 一定带小数点、整型给了精度时忽略 `0`、
     inf/nan 忽略 `0`（空格填充）。
   - 精度：整型是最少位数、字符串是截断、`%.d` 是精度 0（`%.0d` 打印 0 得空串）。
   - 限制：宽度/精度各最多 2 位数字（`width format too long` / `precision format too long`）、
     整段最长 20 字符（`format too long`）、`%` 结尾先报 `not enough parameters`。
   - MSVC 怪癖：`%#.0f`/`%#.0e` 遇到 inf/nan 会把小数点插在**整段第 2 个字符**位置
     （`i.nf`、`-.inf`、`n.an`、`-.nan(ind)`），其它精度不插。
   - `%d` 的 `INT64_MIN`：原来用 `abs()`，release 下打出 `--9223372036854775808`、
     debug 下直接 panic → 改 `unsigned_abs()`。
5. **非 UTF-8 字节被吞**（`PrintFn` 从 `&str` 改成 `&[u8]`）。参考实现是非 UNICODE 的
   `char` 构建，`print((255).tochar())` 写出的是裸字节 `0xFF`；本实现原先所有输出都要过
   `from_utf8_lossy`，被换成 `U+FFFD`（`format("%c", 255)` 实测 3 字节 vs 1 字节）。
   `print`/`error`/`printf`/CALLSTACK/LOCALS 全部改为裸字节，并加测试
   `host_api::print_writes_raw_bytes`。
6. **加载参考实现编译的 `.cnut` 会让宿主 panic**（本轮最严重的一条）。参考实现保存 outer
   值时写了 8 字节而字段只有 4 字节，高 4 字节是垃圾 → 本实现原来直接 `i64` 判零，
   把 `otLOCAL` 读成 `otOUTER`，接着 `outervalues[src]` 越界 panic
   （`vm.rs:1704`，rc=101）。修法：加载时掩码低 32 位；`CLOSURE_OP` 的 outer 索引、
   默认参数槽、绑定目标全部加范围检查，退化为可捕获的 `invalid outer value index`。
   新增夹具测试 `crates/squirrel-stdlib/tests/bytecode_interop.rs`。

**顺带修好的测试方法学问题**：

- `diff_test.py`/`line_diff.py` 的指针归一化从 `0x[0-9a-fA-F]+` 收紧到 `{8,}`——
  原先会把 `format("%#x", 1)` 的正常输出也归一化成 `0xPTR`，等于自己把差异抹掉。
- `tools/lint_corpus.py` 的规则全部换成**实测踩过的坑**（位运算复合赋值、`>>>` 其实存在、
  `rawcall` 当成员、`65.tochar()` 词法陷阱、函数体内 `const`），原来那条
  「`class {}` 后同行紧跟语句」是纯误报（`class X {} return 1;` 完全合法），已被删掉。
- 新增 `tools/line_diff.py`（按行定位差异）、`tools/bisect_cases.py`（把探针逐条单独跑，
  找出崩溃/差异的那一条）、`tools/cnut_fields.py`（按 `Save` 布局逐字段比对 `.cnut`）。

## 三·补4、第四轮：计划书验收项与流语义

计划书里两项一直没被**独立验证**的验收标准，这一轮补齐，并顺带把 README 里那句
「stdin 行缓冲差异」查清楚——结论是它不是平台怪癖，而是三个真 bug。

| 验收项 | 证据 |
|---|---|
| #5 用 Rust 宿主 API 复刻 `etc/minimal.c` | `crates/squirrel-stdlib/examples/minimal.rs`（注册 native/调用脚本函数/传参取返回值/错误处理）+ `tests/host_api.rs` 5 个测试；`tools/minimal_diff.py` 用 vswhere 找到 MSVC、**真的编译官方 `etc/minimal.c`**（链接 build-ref 的静态库），两边 stdout 与退出码逐字节一致 |
| 61 条指令被差分语料覆盖 | `crates/squirrel/tests/opcode_coverage.rs`：语料 + samples 共 51 个文件，**覆盖 61/61**；`tests-corpus/t_ops.nut` 补的 `Inc/PInc/Not` 等缺失指令早已到位 |

**「stdin 行缓冲差异」的真相**（`tools/stdin_eof.py` / `tools/stdin_blob.nut` 实测）：

| 现象 | 根因 | 处理 |
|---|---|---|
| 文件流（含 stdin 重定向自文件）上 `tell()` 恒为 0、`len()` 恒为 0 | 本实现只对 `file()` 打开的句柄实现了位置查询，标准流直接返回 0 | 已修：`FileBackend` 自己维护逻辑读写位置（↔ CRT `ftell`），文件大小用 `GetFileSizeEx` 查询（不 seek，避免破坏管道状态）。**文件重定向 5/5 对齐** |
| 管道里喂 `"A\nBC"` 后 `readblob(4)` 只返回 2 字节 | 早先按字节读、遇到 `'\n'` 就停——那是控制台的行语义，被错安到了管道上 | 已修：改成「读到填满请求长度或 EOF」（↔ CRT `fread`），并且不再把请求长度夹到不可信的 `len()` 上（管道上会夹成 0 → 永远 `no data left to read`） |
| 管道上 `eos()` 恒为真（`while(!stdin.eos())` 一次都不进） | 同上，`tell()==len()` 恒成立 | 已修：`tell()` 现在给出已消费字节数，`eos()`/`len()` 随之与参考实现对齐 |

修复后 `tools/stdin_eof.py`：文件重定向 **5/5 一致**；管道上「读满 → 越界读报 io error」
的错误路径、`tell()`、`eos()` 都与参考实现一致，剩余差异只剩参考实现自己的
`fseek(SEEK_END)` 失败残留（见第四节）。

顺带对齐了两个「反直觉」的返回值（`crates/squirrel-stdlib/tests/stream_semantics.rs` 守门）：

- `eos()` 真返回 **`1`**、假返回 **`null`**（不是 `true`/`false`）；
- `flush()` 失败返回 `1`、成功返回 `null`；
- `writen` 的参数顺序是 **`(值, 格式)`**，与 `readn(格式)` 相反（`sqstdstream.cpp` 读的是 arg3）。

## 三·补5、第五轮：委托方法矩阵（含 ASan 定位参考实现越界）

这一轮把「委托方法 × 参数形态」也做成生成矩阵（`tools/gen_delmatrix.py` → `tests-corpus/t_delmatrix.nut`，
2723 条自动 + 147 条语义），成员名同样取自 C++ 的 `*_default_delegate_funcz` 表，参数形态覆盖
空参/整数/负数/浮点/null/布尔/字符串/空串/数组/表/闭包/比较器/多参数。

**过程中参考实现开始不稳定崩**（同一文件 5 次跑 3 次崩、2 次正常，就是那个崩溃框）。
没有猜，而是用 **AddressSanitizer 重编参考实现**（`tmp-c2/build_asan.bat`）直接拿到报告：

```text
ERROR: AddressSanitizer: heap-buffer-overflow READ of size 4
    #0 sq_stackinfos      squirrel/sqdebug.cpp:35
    #1 __getcallstackinfos squirrel/sqbaselib.cpp:117
    #2 thread_getstackinfos squirrel/sqbaselib.cpp:1271
```

`thread.getstackinfos(<负数>)`：`sq_stackinfos` 的边界判断是 `cssize > level`，对负 level 恒真，
于是去读一个空的调用栈（越界读 + 偶发访问违例）。同类还有两条，都由矩阵二分 + 逐条单跑定位：

| 参考实现 UB 输入 | 现象 |
|---|---|
| `blob.resize(<负数>)` | `SQBlob::Resize` 直接 `realloc(负长度)` → 访问违例 0xC0000005 |
| `blob.readblob(<负数>)` | `sq_getscratchpad(v,-1)` + `fread(buf,1,(size_t)-1)` → 访问违例 |
| `thread.getstackinfos(<负数>)` | 越界读调用栈（ASan 报告如上），偶发崩溃 |

这三类已从语料里按形态排除（`gen_delmatrix.py` 的 `BLOCK` 表），并在 ASan 构建下复跑到
**零报告**，参考实现 5/5 稳定跑完。

**矩阵接着抓出的真差异（全部已修）**：

1. **`array.apply` 返回值错 + 栈泄漏**：参考实现返回**数组本身**（`__map_array` 之后 `sq_pop(v,1)`
   让 this 成为返回值）；本实现返回了最后一个映射结果。根因是 `map_array` 没有消费压入的实参
   （我们的 `call_value` 等价于 `SQVM::Call`，不会像 `sq_call` 那样 `Pop(params)+Push(res)`），
   于是每处理一个元素栈就长 2 格——大数组会顶爆栈，返回值也取错槽。已按 `__map_array` 逐句对齐。
2. **根表插入顺序 4 处不同**（脚本可观察：`foreach (k,v in getroottable())`）：
   bloblib 的 `stream` 类位置与 `casti2f`/`castf2i` 顺序、`blob` 类应在 swap* 之后；
   iolib 的 `stdout/stdin/stderr` 顺序（本实现写成了 stdout/stderr/stdin）；systemlib 的 `date`
   位置（应在 `remove` 之前）；stringlib 的 `regexp`（应在 `format` 之前）。
   定位方法：给参考实现副本的 `sq_newslot` 加一行追踪 + 我们这边临时同样追踪，逐条对比插入序列
   （现在固化成了 `tools/audit_regorder.py` 静态审计 + `tests-corpus/t_roottable.nut` 语料）。
3. **`closure.call` 的尾调用怪癖**：参考实现判断的是**最后一个实参**是否为非生成器闭包，
   被调用者则是 `this`；命中就走 `sq_tailcall`（原生闭包 → `only closure can be tail called`）。
   本实现按接收者判断，错误文本因此不同。
4. **`closure.getinfos`**：`varargs` 是整数 0/1（不是布尔）；原生闭包的 `typecheck` 为空时是
   **null**（不是空数组）。
5. **`closure.getroot`/`setroot`**：对原生闭包报 `closure expected`；`setroot` 的非表参数报
   `invalid type`。原实现返回 `SQ_ERROR` 却没设消息，脚本 catch 到的是上一次的陈旧错误文本。
6. **`class.getattributes(null)`**：参考实现返回**类自身**的 attributes（没有就是 null），
   本实现报 `wrong index`。
7. **`weakref.weakref()`**：weakref 自身也是引用计数对象，`w.weakref()` 给的是「指向 w 的 weakref」
   （`SQRefCounted::GetWeakRef`），因此 `w.weakref() == w` 恒为 false、`typeof` 仍是 weakref。
   本实现原来返回 `w` 自己（后来又错改成返回目标对象）。
8. **`regexp.constructor`**：已构造过的实例再调必须报 `invalid regexp object`（参考实现看实例
   userpointer 是否为空）；顺带把 `get_rx` 的 `__rxid` 读取改成严格校验（null/非整数/越界一律
   视为无效，不允许 `as_integer()` 把 null 当 0 去取别人的正则）。
9. `stream/blob/file` 三个类被构造了两次（bloblib 与 iolib 各一次）→ 现在幂等缓存，
   root 里的类与内部引用指向同一对象。

**测试方法学**：`diff_test.py` 的「未捕获错误 → SUSPECT」守卫加了完成标记豁免——生成的矩阵语料
以 `<NAME>-DONE` 结尾，若 stdout 里有这个标记就说明脚本跑完了；此时 stderr 可能是「错误被脚本
catch 住、错误处理器仍打印」的那类情况（两边逐字一致才算过），不该误判为提前结束。

## 三·补6、第六轮：类/实例语义矩阵 + 一次真实的内存爆炸

### 语料

`tools/gen_oopmatrix.py` → `tests-corpus/t_oopmatrix.nut`：168 条自动（5 个类夹具 ×
6 种接收者 × 28 条访问路径：读字段/读静态/读缺失/属性/调方法/调静态方法/缺失调用/
typeof/rawget/rawin/in/getclass/getbase/getattributes/delete/赋值/newslot/加字段/
tostring/len/weakref/成员数/类共享）+ 81 条语义（默认字段值是否共享、构造器默认参数与
varargs、`base.constructor()`、三层继承与 `base` 链、`instanceof` 链、成员锁定、
attributes、`this`/`base` 在方法/lambda 里、类作为值、实例 clone 等）。

### 内存爆炸：真实事故、根因与修复

**触发**：无界的**非尾递归**。最小复现是三行脚本，不需要大输入、不需要畸形数据：

```nut
class C11 { constructor() { local x = C11(); } }   // 或
function f(n) { local x = f(n + 1); return x; }    // 漏了 base case
```

**现象**：宿主内存 7GB → 22GB → 32GB 后被系统杀掉。

**为什么**：参考实现的 `SQVM::EnterFrame` 只做 `_stack.resize(newtop + …)`，
**没有任何上限**；本实现原来逐字照搬。参考实现同样会涨（实测 10 秒 191MB 且在涨），
本实现涨得更快（同 10 秒 601MB）。

**修复**（`config.rs` + `vm.rs`）：

| 措施 | 说明 |
|---|---|
| `MAX_STACK_SLOTS = 1M` / `MAX_CALL_FRAMES = 200k` | 超过就报**可捕获**的 `stack overflow`；正常深递归只需几千帧，实测构造器无限递归峰值 601MB → **32–51MB** |
| `STACK_RESERVED_SLOTS/FRAMES`（错误处理器/调试钩子余量） | 栈满时错误处理器自己也要压帧；没有余量就会出现「报了 stack overflow 却什么都不打印」（参考实现因为无上限所以能打印） |
| 未捕获时仍打印错误 + 调用栈 | 与参考实现一致（只是我们的调用栈最多 20 万帧，参考实现到不了那么深就先 OOM 了） |

这是**有意偏离参考实现**的一处，已记入第四节：宁可报可捕获错误，也不让脚本拖死宿主。

### 测试方法学（这轮又抓出两个假绿/假故障）

1. **`diff_test.py` 把「两边都超时」当 PASS**：原来的 `t_deep.nut` 是
   `function rec(n) { return rec(n + 1); }` —— **尾调用**会复用帧，两边都是死循环，
   于是这条用例长期假绿。现在**任一侧超时一律 FAIL**，且 `t_deep.nut` 重写成有界版本
   （深度 2000 的非尾递归 + 20000 次尾递归 + 深帧里抛错被 catch，末尾 `DEEP-DONE`）。
2. **不可复现的语料**：`D3() <=> D3()`（两个不同实例）走的是 `ObjCmp` 的裸指针序，
   参考实现自己换个进程就可能给出 `-1` 或 `1`（cnut 交叉执行抓到 `-1` vs `1`）。
   已改成同一实例自比（`_rawval` 相等 → 0），并在生成器里写明原因。
3. **`tools/mem_probe.py` 自己的管道死锁 bug**：它原来只在进程结束后才读 stdout，
   子进程输出写满管道缓冲就阻塞在 write 上，看起来像"脚本死循环"——我为此追了
   一小时的"神秘卡死"（ddmin 缩到"删掉任意一条就不卡"这种自相矛盾的结论）。
   现在用独立线程并发读干管道。**教训**：测量工具本身也要先自证。

## 三·补7、第七轮：生成器/线程语义矩阵（跨 VM outer 的真实 panic + 三处栈纪律）

### 语料

`tools/gen_genmatrix.py` → `tests-corpus/t_genmatrix.nut`：**130 条**用例，全自动生成，
每轮都能重跑。覆盖 10 组：resume 次数 × 状态机、resume 作用于 13 种非生成器值、
11 种 yield 值形态、8 种 suspend 值形态、线程 4 状态 × 5 操作、参数/返回值边界、
错误传播、**跨 VM outer**、生成器/线程作为值（表/数组/类成员/嵌套/foreach）、
线程栈状态（失败/正常结束后再 call、wakeup 的栈纪律）。全部 130 条 stdout 逐字节一致
（`python tools/line_diff.py tests-corpus/t_genmatrix.nut`）。

### 1. 跨 VM 的 open outer：从"读不到"到 panic，再到正确的共享单元

**现象**：`newthread(function() { … inner.call() … })` 报
`the index 'call' does not exist`——闭包捕获的局部变量（open outer）在另一个线程里读到了 null。

**根因**：C++ 的 `SQOuter::_valptr` 是**指向创建者 VM 栈槽的裸指针**。本实现当初把
open outer 存成「宿主 VM + 槽位索引」，跨线程时去 `borrow()` 对方的 `RefCell`，而对方
正在执行自己的代码（可变借用中）→ 直接 panic
（`RefCell already mutably borrowed`，`vm.rs:860`）。

**修复**（`closure.rs` + `vm.rs`）：栈槽与 outer **共享一个 `Rc<RefCell<Value>>` 单元**：

| 位置 | 做法 |
|---|---|
| `SQOuter::open` | `Option<Rc<RefCell<Value>>>`（不再是「宿主 VM + 索引」） |
| `VmData::open_cells` | 槽位 → 单元 的表；`find_outer` 建单元并登记 |
| `stk` / `stk_set` | 本 VM 读写被抓取的局部变量时**同步单元**（表空则零开销直通） |
| `outer_read` / `outer_write` | 只碰单元，**绝不碰对方的 VM 结构** → 跨线程安全 |
| `close_outers` | 从单元取值写回 `outer.value`、断开、清表 |

语义与 C++ 等价（线程读到的就是宿主栈槽的当前值，宿主也能看到线程的写入），
且不再有任何跨 VM 借用。

### 2. `thread_call` 的栈纪律：两处真实缺陷

↔ `sq_call` 的 `if(!v->_suspended) v->Pop(params)` + `thread_call` 的
`sq_move(v,thread,-1); sq_pop(thread,1)`。原来的实现**成功路径只弹 1 格**、
**失败路径一格不弹**，于是线程栈每次调用都长高：

| 场景 | 参考实现 | 修复前 | 修复后 |
|---|---|---|---|
| `t.call(1,2)` 后再 `t.call()` | `r1` / `r2` | 第二次 `attempt to call 'table'` | 一致 |
| 抛错的线程再 `call()` | 再次执行闭包、再次抛 `boom3` | `attempt to call 'table'` | 一致 |
| 挂起中的线程 `wakeup()` | 保留 suspend 现场 | 一致 | 一致（按 `suspended` 分支保留） |

### 3. `wakeup()` 不带值时，`suspend()` 的返回值应为 `null`

↔ `sq_wakeupvm` 的 `else if(target != -1) v->GetAt(…).Null()`。本实现漏了这一支，
于是 `local v = suspend("x"); … t.wakeup()`（不传值）拿到的是槽里残留的对象——
实测是 `suspend` 那个 native 闭包（`got:(function : 0x…)`）。参考实现给 `got:null`。

### 4. 方法学：矩阵自己也会有"空洞"

写完矩阵后逐条核对输出，发现**整整一组用例是退化的**：这些用例先对 **fresh** 线程
`wakeup()` 想到达挂起态，而 fresh 线程的 `wakeup()` 必然报
`cannot wakeup a idle thread`——两侧报同一个错，于是"全绿"，但根本没测到挂起态。
改成先用 `call()` 跑到挂起态之后，立刻抓出上面第 2、3 条真实缺陷。
**教训**：组合矩阵要核对"每一行是否真的走到了预期路径"，而不只是"两侧是否一致"。

### 5. 固化

`crates/squirrel-stdlib/tests/thread_semantics.rs`：5 个 Rust 测试把本轮结论钉死
（跨 VM outer 读写、失败后重跑、实参/变长/挂起的栈纪律、`wakeup()` 无值传 null、
线程里创建生成器再回主线程 resume）。期望值全部先用**参考实现**实测确认
（`python tmp-c2/check_thread_tests.py` 把测试里的脚本抽出来两侧对跑）。

## 三·补8、第八轮：正则引擎逐行移植 + 两处回收缺口（2GB → 18MB）

### 语料

`tools/gen_rxmatrix.py` → `tests-corpus/t_rxmatrix.nut`：**500 条**自动用例
（33 种模式 × 13 种主语的 search/capture/subexpcount/match 四件套、17 条非法模式的
错误文本、22 条 API 边界、12 条 start 偏移、19 条引擎细节）。另加
`tests-corpus/t_churn.nut`（对象/字符串 churn 语义）。

### 1. 旧引擎不是参考实现：9 处真实差异

原来那版正则引擎是"按正则的常识"写的，跟 `sqstdrex.cpp` 差得很远。矩阵第一轮就抓出 9 条：

| 形态 | 参考实现 | 旧实现 |
|---|---|---|
| `regexp("<.*>").search("<a><b>")` | `{0,3}`（贪婪量词**停在第一个能继续的位置**） | `{0,6}` |
| `\bfoo\b` 搜 `"a foo b"` | **不匹配**（`\b` 是「空格↔非空格转移」，不是词边界） | `{2,5}` |
| `^b` 带 `start=1` 搜 `"ab"` | `{1,2}`（`^` 锚定的是 **start 偏移**） | 不匹配 |
| `""` 上 `search(".*")` | 不匹配（`searchrange` 开头就有空串早退） | `{0,0}` |
| `[z-a]` / `[\d-z]` / `[a-` | `invalid range` | 编译通过 / 别的错误 |
| `a{1000000000}` | `overflow in numeric constant` | 编译通过 |
| `(a)+` 对 `"aaa"` 的 capture | `[{0,3},{0,1}]`（记**第一次**迭代） | `[{0,3},{2,3}]` |
| `[\xe9]` | `letter expected`（`scisprint` 拦下高位字节） | 编译通过 |

结论：不再"修差异"，而是把引擎**按 `sqstdrex.cpp` 逐行重写**（节点数组 + 回溯匹配，
`OP_GREEDY` / `OP_OR` / `OP_EXPR` / `OP_WB` / `OP_CLASS` / `OP_MB` 一一对应），
连原版的死代码与怪癖一起搬（例如 `[a-]` 那句 `unfinished range` 写在恒假的
`*_p++ == ']'` 上；`search` 失败后 `node` 不回退到起始节点）。改完 500/500 一致。

### 2. 空主语、量词链、深嵌套：三个"参考实现会挂"的点

* `(a*)*` 这类**空匹配量词**：原版 `OP_GREEDY` 在 `s` 不前进时靠 `s >= _eol` 才跳出，
  有死循环风险 → 本实现加**步数上限**（`MAX_RE_STEPS`，超限按不匹配处理）。
* 5000 字面量字符的模式：原版是尾递归 `element()`，本实现照抄会吃 5000 层栈
  → 改成**循环**串 `next`，递归深度只与真正的嵌套相关。
* 深嵌套括号：实测参考实现 4000 层能编、8000 层就 `0xC00000FD`（栈溢出）。
  本实现原来**4000 层就爆栈**——原因是 Rust 可执行文件的**主线程栈默认只有 1MB**，
  而 MSVC 的 `sq_static.exe` 帧更小、能撑到 4000 层。修法两步：
  1. 递归改用**栈预算**判定（比较局部变量地址差，默认 384KB，见 `RE_STACK_BUDGET`），
     固定层数只作次要保险——固定层数在不同线程/平台上根本不可靠；
  2. CLI 把脚本执行放到 **16MB 栈的线程**上并把正则预算提到 8MB
     （`crates/sq/src/main.rs`，`set_stack_budget`）。
  结果（`tmp-c2/rx_depth.py`）：

  | 嵌套层数 | 参考实现 | 本实现 |
  |---|---|---|
  | 1000 / 2000 / 3600 / 4000 | 正常 | 正常（逐字节一致） |
  | 8000 / 20000 | **`0xC00000FD` 崩溃** | 正常 |
  | 40000 / 80000 | **崩溃** | 可捕获错误 `regexp too deeply nested` |

### 3. 内存：字符串实习表的键是永久垃圾（实测 2GB）

churn 对拍（20 万次迭代，`tools/mem_probe.py` 量峰值）：

| 形态 | 修复前 | 修复后 | 参考实现 |
|---|---|---|---|
| `s += "x"` ×20 万（`str`） | **2000MB（撞 2GB 上限被杀）** | **18.0MB** | 5.2MB |
| `"v" + i` ×20 万（`str2`） | 54.3MB | 17.7MB | 4.6MB |
| `local t = {}` ×20 万（`table`） | 28.7MB | 12.9MB | 4.5MB |
| `local a = [i]` ×20 万（`array`） | 17.1MB | 8.2MB | 4.6MB |
| `local f = function(){}` ×20 万（`closure`） | 24.7MB | 7.9MB | 4.6MB |

根因两条，**都与可观察语义无关**（只是回收时机）：

1. **字符串实习表把已死字符串的键留着**：C++ 的 `SQString` 引用计数归零时会
   `SQStringTable::Remove`，而本实现的值虽然是 `Weak`（字符串对象本身由 `Rc` 即时释放），
   HashMap 的**键**（字符串字节）却一直留着——`s += "x"` 两万次正好留下所有中间串
   （长度之和 ≈ 232MB，与实测峰值一模一样）。现在按"插入字节数 4MB **或** 插入条目数 4096"
   清一遍弱引用已死的条目。
2. **GC 注册表累积已死条目**：`gc_chain` 存的是 `Weak`，对象被 `Rc` 释放后条目还在，
   纯 churn 脚本会攒下几十 MB。现在表长到阈值就 `retain` 掉弱引用失效的条目
   （那正是"对象已析构"，与 GC 的 `None => 已析构` 分支同义）。

参考实现是靠引用计数**即时**回收的，本实现是 `Rc` + 显式 GC，所以这两处"清理死条目"
只是把时机补回来，不改变任何脚本可见结果。

### 4. 固化

* `crates/squirrel-stdlib/tests/regex_semantics.rs`：4 个 Rust 测试（引擎怪癖、
  编译错误文本、search 偏移/捕获形状、**churn 后实习表与注册表条目数有界**）。
* `tools/gen_rxmatrix.py` 与 `tests-corpus/t_rxmatrix.nut`、
  `tests-corpus/t_churn.nut` 进语料，`tools/diff_test.py tests-corpus` 覆盖。
* 探针：`tmp-c2/rxpath.nut`（病理形态 16 条，两侧一致）、`tmp-c2/churn.nut`（内存对拍）、
  `tmp-c2/rx_depth.py`（嵌套深度上限实测）。

## 三·补9、第九轮（P1-1）：数值 / 运算符边界矩阵

`tools/gen_nummatrix.py` → `tests-corpus/t_nummatrix.nut`：**4533 条**自动用例
（13 个整数边界值 + 15 个浮点值 + 10 种其它类型 × 运算符/比较/转换/字面量/自增/复合赋值，
另含 math 边界与移位越界计数），两侧 stdout **逐字节一致**。

### 抓到的真实差异：移位计数越界

x86 的硬件移位只用**计数低 6 位**，MSVC 版 Squirrel 就是这个语义：

```
1 << 63          = -9223372036854775808
1 << 64          = 1                    （== 1 << 0）
1 << 2147483647  = -9223372036854775808 （== 1 << 63）
1 << -1          = -9223372036854775808 （== 1 << 63）
```

本实现原来对 `i2 < 0 || i2 >= 64` 一律返回 0，矩阵一次抓出 18 条差异。
现改为 `wrapping_shl` / `wrapping_shr`（语义正是"只用低 6 位"），
并把 `64 / 65 / 127 / 128 / -1 / -64 / -65 / 2147483647 / INT64_MAX` 这些计数全部纳入语料守门。

### 确认的参考实现崩溃：`INT64_MIN / -1`（用户见过的崩溃框）

x86 的 `idiv` 在商溢出时触发**硬件异常**，参考实现直接
`STATUS_INTEGER_OVERFLOW (0xC0000095)` 崩溃：

```nut
(-9223372036854775807 - 1) / -1    // 参考实现：崩溃
(-9223372036854775807 - 1) % -1    // 参考实现：崩溃（同一个 #DE）
```

本实现给回绕结果（`INT64_MIN` / `0`），不崩。这条**不能进差分语料**
（参考实现活不下来），由 `numeric_semantics.rs::min_div_minus_one_does_not_crash` 单独守门，
并在工具里用 `ref-crash-skip` 标记让对拍脚本跳过它。

### 顺带确认的两个怪行为（两侧一致，照抄）

* `1 % 0`：被**编译器常量折叠**成 `1`；
* 运行时的 `a % b`（`b == 0`）**不报错**，函数返回 `null`（错误对象是 null）；
  而 `a / b`（`b == 0`）报可捕获的 `division by zero`。

### 工具层修复（重要）

对拍脚本 spawn 参考实现时**必须先关掉系统错误对话框**（`SEM_NOGPFAULTERRORBOX`），
否则参考实现一崩就弹模态框把整批对拍挂住。上一轮补的抑制代码有个 bug：
`tmp-c2/` 下的脚本只把**自身目录**加进 `sys.path`，而 `safe_run.py` 在 `tools/`，
导入失败又被 `except` 吞掉 → 抑制其实从未生效（用户连续两次看到崩溃框就是这个原因）。
现已改为把「脚本目录 / 上级 tools / 自身 tools」都加进路径，并且**导入失败会打印警告**
而不是静默跳过；`tmp-c2/check_suppress.py` 可一键验证当前 error mode 是否含 `0x2`。

## 三·补10、第十轮（P1-2）：weakref / GC 回收语义矩阵

`tools/gen_weakmap.py` → `tests-corpus/t_weakmap.nut`：**63 条**用例
（两轮探针 `tmp-c2/weakprobe.nut` + `weakprobe2.nut` 的并集，逐字节一致）。

### 这一轮**没有发现差异**

值得说明的是：本实现是 `Rc` + GC 链（链里存 `Weak`），参考实现是手写引用计数 + 环回收，
两边在"对象到底什么时候死"上居然完全一致 —— 包括下面这些容易想当然的点：

| 形态 | 两侧一致的结果 | 说明 |
|---|---|---|
| `{ local o = mk(); w = o.weakref(); }` 之后 `w.ref()` | `table`（还活着） | 栈上的残留槽仍指着它（C++ 的 `_stack` 未清空，本实现同样） |
| 同上，再 `collectgarbage()` | `null` | GC 之后就不再是根 |
| 自环 `t.self <- t.weakref()`（**弱**自环） | `null` | 弱引用不保活 |
| `collectgarbage()` 的**返回值** | 单环 → `1`、双环 → `2`、普通垃圾 → `0` | 返回值就是回收个数，逐一对齐 |
| 强引用进数组 | 对象活着 | 容器保活、弱引用不保活 |
| 字符串弱引用 | `w.ref() == v` | 字符串实习 → 同一个对象 |
| 线程/生成器/类/实例/blob/regexp 弱引用 | 全部可用 | |

### 顺带确认的两条"参考实现自己也怪"的接口事实（两侧一致，照抄）

* `(5).weakref()` **能**建出 weakref（`typeof` 是 `weakref`），但它的 `ref()` 报
  `the index 'ref' does not exist`（整数弱引用没挂 weakref 委托表）；
* `(null).weakref()` 直接报 `the index 'weakref' does not exist`；
* 脚本侧**没有** `getweakrefval` 这个全局（那是 C API `sq_getweakrefval`），
  只有 `w.ref()`。

### 固化

`crates/squirrel-stdlib/tests/weakref_semantics.rs`（2 个测试：各类型弱引用；
弱引用不保活 + GC 计数），期望值先经参考实现实测确认。

## 三·补11、第十一轮（P1-3）：词法 / 优先级 / 作用域 + 编译错误对拍

### 语料与工具

* `tools/gen_lexmatrix.py` → `tests-corpus/t_lexmatrix.nut`：**203 条**（转义表、
  `\x`/`\u`/`\U` 的字节值、verbatim 串、优先级与结合性、短路求值、作用域遮蔽、
  循环里的闭包捕获、`++/--` 返回值、语句形态、`__LINE__`、类/函数语法面）。
  用 `tmp-c2/byte_diff.py` 按**字节**对比，两侧逐字节一致。
* `tools/errmsg_matrix.py`：**55 条**语法/词法非法源码，逐条 `-c` 只编译并对比
  编译器错误消息与行列号 —— **55/55 一致**（含 `unrecognised escaper char`、
  `expected '{'`、`empty class` 那一类）。语法错的脚本会中断整个文件，
  所以不能塞进普通语料，必须单独这样跑。

### 抓到的真实差异：字符串下标是**有符号**的

参考实现是非 UNICODE 构建（`SQChar` = `char`），`SQString::_val[idx]` 取出来会
**符号扩展**：

```nut
"\xff"[0]     // 参考实现：-1     本实现（改前）：255
"\u00e9"[0]   // 参考实现：-61    本实现（改前）：195   （字符串是 UTF-8 两字节 C3 A9）
foreach (c in "\u00e9") …   // 同一条路径，也拿到 -61 / -87
```

`\u`/`\U` 的**编码**两侧本来就一致（都产出 UTF-8 字节 ✓ 已实测 `\u00e9` → `C3 A9`、
`\U0001F600` → `F0 9F 98 80`），差的只是取字节时的符号。已改 `vm.rs` 的字符串 `_get`
与 `SQString::next`（foreach 用）为 `as i8 as SQInteger`。

**blob 不同**：`b[0] = 255` 取回 `255`（`unsigned char`）——已实测确认两侧一致，
语料里同时守住这两种语义。

### 顺带隔离的一处参考实现 UB：`escape()` 遇高位字节

`escape()` 用 `(char)c` 传给 `%02x`，高位字节会写出 8 位补码（`\xffffffe9`），
**并越界写它自己的 scratchpad**：实测输出会串行（`esc_xff=1:\xff` 后面直接粘上
下一个用例的尾巴），还混进别的错误消息片段（`stream' does not exist`）。
所以语料里高位字节只比**字节值**（`bytes()` 转储），不比 `escape()`；
已知偏离记在第四节。

### 固化

`crates/squirrel-stdlib/tests/lexer_semantics.rs`（2 个测试：字符串字节有符号性、
优先级/短路/作用域/循环捕获/自增返回值），期望值先经参考实现实测确认。

## 三·补12、第十二轮（P1-4）：blob / stream 二进制矩阵

`tools/gen_blobmap.py` → `tests-corpus/t_blobmap.nut`：**133 条**
（源探针 `tmp-c2/blobprobe.nut` 56 条 + 生成式扩充：`writen`/`readn` 的
**格式 × 值** 往返矩阵、`seek`/`resize` 组合、`swap2/swap4`、`readblob`/`writeblob`、
文件流往返与模式），两侧**逐字节一致**。

覆盖到的类型分派（↔ `sqstdstream.cpp`）：`l`(int64) `i`(int32) `s`(short) `w`(u16)
`c`(char) `b`(uchar) `f`(float) `d`(double)，含截断（`writen(70000,'s')` → `70 11`）、
小端字节序、`readn` 到 EOF 报 `io error`、`readblob` 无数据报 `no data left to read`、
`writen` 非法格式报 `invalid format`、`swap2/swap4` 的成对/四字节翻转。

### 抓到的一处参考实现崩溃：非法文件模式

参考实现把模式串**直接**交给 `fopen`，非法模式会落到 UCRT 的 invalid parameter
handler（`__fastfail`），实测这些全部 `0xC0000409` 崩溃：

```
file(p, "")  file(p, "z")  file(p, "zz")  file(p, "R")  file(p, "RB")
file(p, "rx")  file(p, "rw")  file(p, "rbb")  file(p, "trash")  file(p, "r\tb")
```

而 `r rb r+ rb+ rt w wb w+ wb+ a ab a+ ab+ w+b` 与 `"r b"`（空格）都能正常打开。
本实现原来用 `mode.contains('r'/'w'/'a')` 解析，垃圾模式会被当成只读打开 ✗；
现改为**显式白名单校验**（首字符 r/w/a，其余只允许 `+ b t c n`、不重复、空格忽略），
非法模式给可捕获的 `cannot open file` —— 接受集与参考实现实测一致，崩的那一档
变成可捕获错误（记入第四节）。

### 工具

`tmp-c2/bisect_cases2.py`：按**括号配对**切分探针用例并二分定位"让参考实现崩/卡住"
的那一条（原来的 `find_bad.py` 按行切，遇到多行用例会切坏）。这一轮就是靠它定位的
（现象是参考实现**完全没有输出**——stdout 是块缓冲，崩溃时全丢）。

### 固化

`crates/squirrel-stdlib/tests/stream_semantics.rs` 扩到 4 个测试：
`writen/readn` 类型分派与字节序、非法模式可捕获（`ref-crash-skip`）、
外加原有的文件/blob 流语义与 `readblob` 夹取。

## 四、残留差异（参考实现自身是 UB / 无法复刻的部分）

| 项 | 说明 |
|---|---|
| **失控递归的内存上限** | 参考实现的 `EnterFrame` 无上限，非尾递归的无限递归会把宿主吃到 OOM（实测 32GB 被系统杀）；本实现封顶（1M 槽 / 20 万帧）并报可捕获的 `stack overflow`。**这是有意偏离**：宁可报可捕获错误，也不让脚本拖死宿主 |
| 两个不同类型/同类型对象之间的 `<=>` | 参考实现按裸指针序（`_userpointer(o1) < _userpointer(o2)`），跨进程/跨字节码不可复现（cnut 交叉执行实测同一脚本 `-1` vs `1`）。本实现按自身身份地址，同样只保证"同一次运行内一致"。语料里因此只比较**同一个对象** |
| `blob.resize(<负数>)` / `blob.readblob(<负数>)` | 参考实现不检查负数：前者 `realloc(负长度)`，后者 `sq_getscratchpad(-1)` + `fread(buf,1,(size_t)-1)`，实测都访问违例（0xC0000005）。本实现报可捕获错误，语料按形态排除 |
| `thread.getstackinfos(<负数 level>)` | `sq_stackinfos` 用 `cssize > level` 判界，负 level 恒真 → 越界读空调用栈（ASan 实测 `sqdebug.cpp:35` heap-buffer-overflow），偶发整进程崩溃。本实现返回 null，语料按形态排除 |
| 不可 seek 流上 `len()`/`eos()`/`readblob` | 参考实现的 `EOS()` 是 `Tell()==Len()`，而 `Len()` 靠 `fseek(SEEK_END)`：管道上这次调用**失败**，之后 `ftell` 给出的是 CRT 内部状态（实测同一进程里 `readblob(4)` 之前 Len()≥4、之后 Len()=0；5000 字节管道上又变成 904=5000−4096）。更糟的是失败的 `fseek` 会**丢弃已缓冲的输入**：8 字节管道上读完 2 字节后剩下的 6 字节直接消失，于是 `readn` 报 io error。本实现给出确定性行为（`tell()`=已消费字节数、不可 seek 时 `len()`=0），不复刻这个数据丢失 |
| `.cnut` 的 outer 类型字段 | 参考实现用 `sizeof(SQUnsignedInteger)`（8 字节）写一个 4 字节枚举，高 4 字节是**未初始化内存**：同一脚本连编两次字节都不同（实测 `ff245324…` vs `e4e3c42e…`），而 `e4e3c42e…` 恰好等于本实现的输出。因此含闭包的语料会有几个文件字节不同，`tools/cnut_matrix.py` 把这种只差高 4 字节的情况单独标成 `PAD-ONLY`（低 32 位类型仍逐位相同，其余字段全部一致）；本实现写确定的 0。加载侧已按低 32 位掩码，参考实现产出的字节码能正常执行（51/51 交叉执行一致） |
| 坏字节码里的越界 outer 索引 | 参考实现不检查该索引，夹具 `ref_closures_badouter.cnut` 让 C++ 侧访问违例崩溃（0xC0000005）；本实现报可捕获的 `invalid outer value index`（`crates/squirrel-stdlib/tests/bytecode_interop.rs` 守门） |
| `format` 规格串恰好 20 字符 | `validate_format` 的 `n-start > MAX_FORMAT_LEN` 差一，随后 `memcpy` 越界写 `fmt[20]`：`format("%" + 20 个 '+' + "d", 1)` 实测 `STATUS_STACK_BUFFER_OVERRUN` 崩溃。本实现从 20 起就报 `format too long`，不复刻 |
| `escape` 高位字节 | C++ 用 `(char)c` 传给 `%02x`：≥0x80 输出 8 位补码（`\xffffffe9`），**并越界写自己的 scratchpad**。实测（P1-3）输出会**串行**（`esc_xff=1:\xff` 后面直接粘下一个用例的尾巴）并混进别的错误消息片段（`stream' does not exist`）。本实现取确定性的补码值；语料里高位字节只比字节值、不比 `escape()` |
| `date` 上界 | UCRT 的上界常量未逐秒定位：实测 32536800000（3001-01-19）可用、32536857599 失败。本实现取前者为上界 |
| `getstackinfos(-1)` | 参考实现 `_callsstack[cssize]` 越界读（UB，实测返回一个表）；本实现返回 `null`，不 panic |
| 挂起线程上再 `call()` | 参考实现的 `suspend` 不关帧，`sq_call` 取 `GetUp(-(nparams+1))` 拿到的正是**挂起值**（实测无论传几个实参都是 `attempt to call 'string'`）；本实现关帧后取到的是环境表，文本为 `attempt to call 'table'`。两者都是可捕获错误，只有类型名不同；`wakeup`/`getstatus`/`getstackinfos` 等合法操作两侧完全一致（`tmp-c2/layout.nut`） |
| 正则引擎的怪癖 | `sqstdrex.cpp` 的行为与"正则常识"差很远，已**逐行移植**（见三·补8）：`.*` 停在第一个能继续的位置、`\b` 是空格转移、`^` 锚定 search 的 start、空主语不匹配、`?` 紧跟量词时是字面量、`[a-]` 不报 `unfinished range`（那句检查恒假）、失败后不回退起始节点。这些都不是 UB，所以照搬 |
| 正则引擎的怪癖 | `sqstdrex.cpp` 的行为与"正则常识"差很远，已**逐行移植**（见三·补8）：`.*` 停在第一个能继续的位置、`\b` 是空格转移、`^` 锚定 search 的 start、空主语不匹配、`?` 紧跟量词时是字面量、`[a-]` 不报 `unfinished range`（那句检查恒假）、失败后不回退起始节点。这些都不是 UB，所以照搬 |
| 正则的递归/步数上限 | 参考实现没有上限：空匹配量词（`(a*)*`）有死循环风险，嵌套超过 4000 层就 `0xC00000FD`（实测 8000 层崩溃）。本实现给 `MAX_RE_STEPS` + **栈预算**（按局部变量地址差估算，CLI 下 8MB、库默认 384KB），超限按不匹配或可捕获错误处理。现在 4000 层以内与参考一致，8000 层以上参考崩溃而我们正常/报错 |
| 移位计数越界 | x86 硬件移位只用计数低 6 位（`1 << 64` == `1 << 0`、`1 << -1` == `1 << 63`）。C++ 侧形式上是 UB，但 MSVC 实测就是这个语义，本实现按同一语义（`wrapping_shl/shr`）对齐并把越界计数纳入语料 |
| `INT64_MIN / -1`、`INT64_MIN % -1` | x86 `idiv` 商溢出触发硬件异常，参考实现 `STATUS_INTEGER_OVERFLOW(0xC0000095)` **崩溃**（用户实际见过那个崩溃框）；本实现回绕成 `INT64_MIN` / `0`。不进差分语料（参考实现活不下来），由 `numeric_semantics.rs` 守门 |
| `file(路径, <非法模式>)` | 模式串直接交给 `fopen`，非法模式落到 UCRT invalid parameter handler（`__fastfail`）：实测 `""` / `"z"` / `"R"` / `"rbb"` / `"trash"` / `"r\tb"` 全部 `0xC0000409` 崩溃；合法的 `r rb r+ rb+ rt w wb w+ wb+ a ab a+ ab+ w+b` 与 `"r b"` 正常。本实现白名单校验（接受集与实测一致），非法模式给可捕获 `cannot open file` |
| 宿主栈大小与"Rust 不如 C++"的错觉 | Rust 可执行文件主线程栈默认 1MB（Windows），比 MSVC 版能用的深度小；CLI 已改成在 16MB 栈线程上跑。**注意**：库调用方（FFI/host api）仍在自己的栈上执行，所以正则递归用栈预算保护，不假定有大栈 |
| 字符串/对象的回收时机 | 参考实现用引用计数**即时**回收；本实现是 `Rc` + 显式 GC，靠"清死条目"把内存拉回同一量级（20 万次拼接 2000MB → 18MB，参考 5.2MB）。`collectgarbage()` 的返回计数因此不同（本实现已经自动清过一部分） |
| REPL 读到 EOF | 参考实现在 EOF 后死循环（`getchar()` 返回 −1 被当作字符累积）；本实现直接退出 |
| UCS2 非 ASCII 内容 | 参考实现在非 UNICODE 构建下把码元交给 `isalnum` 等（越界 UB）；本实现按码元喂词法器，ASCII 等价 |
| 空正则模式 `regexp("")` | 参考实现写 NULL 崩溃（弹模态崩溃框）；本实现报 `letter expected`，不复刻该 UB |
| `castf2i` 高位字节 | 参考实现把 `f32` 的 4 字节当 `i64` 读（顺带读相邻栈内存，UB）；本实现只取确定的低 32 位（`f.to_bits()`），低 32 位与参考一致 |
| 超深嵌套 / 深嵌套值 | 参考实现分别会 `STATUS_STACK_OVERFLOW`；本实现给出可捕获错误或迭代释放（见上节） |
| 不可 seek 流的 `eos()/tell()/len()` | 实测（`tools/eos_diff.py`）：参考实现走 CRT `ftell`/`fseek`，在管道上 `fseek(SEEK_END)` 失败会破坏流（其后 `readn` 报 `io error`），`stdout.eos()` 返回 `null`、`stdin.eos()` 视剩余数据而 `null`/抛错。本实现返回正常值，不复刻该平台副作用 |
| `split(<非串>, <非串>)` | 参考实现不检查 `sq_getstring` 的返回值，`str` 未初始化即被 `while(*end)` 解引用 → 写/读非法地址崩溃（Windows 崩溃框）。本实现报 `parameter 1 has an invalid type ...`。因此该形态已从自动生成的错误对拍语料里排除 |
| `tointeger(base)` 非法进制 | 参考实现把它透给 CRT `strtol`：base=1 / 负数 / >36 是未定义行为，实测 `STATUS_STACK_BUFFER_OVERRUN` 崩溃。本实现报 `cannot convert the string`。合法进制（0=自动识别、2..36、16 带 `0x`）已逐一对齐（`tools/toint_probe.py`） |

> 注：`}` 出现在字符串里仍会减少花括号计数（参考实现的怪癖），本实现按原样复刻——因此
> 这类会话会让参考实现读到 EOF 后卡死，无法作为差分会话用例。

## 五、复现脚本

| 文件 | 用途 |
|---|---|
| `tools/probes/math_globals.nut` | A 的证据：逐个调用 18 个 math 全局函数 |
| `tools/probes/errmsg.nut` | F 的错误消息对拍（15 条） |
| `tools/probes/breadth.nut` | 约 90 项标准库广度探针 |
| `tools/probes/locals_fmt.nut` / `err_obj.nut` | D 的 CALLSTACK/LOCALS 与错误对象格式 |
| `tools/probes/nan_probe.nut` | `fmod`/除法/unary minus 的 NaN 符号矩阵（±0/±1/2.5/±inf/nan 全组合） |
| `tools/probes/fmt_flags.nut` / `fmt_hash.nut` / `fmt_types.nut` | printf 标志、`#` 与 inf/nan、类型接受的实测记录 |
| `tools/probes/closure_outer.nut` | 闭包/outer 值夹具源码（编译成 `crates/squirrel-stdlib/tests/fixtures/*.cnut`） |
| `tools/gen_genmatrix.py` / `tools/gen_rxmatrix.py` | 生成 `tests-corpus/t_genmatrix.nut`（生成器/线程 130 条）与 `t_rxmatrix.nut`（正则 500 条） |
| `tmp-c2/show.py` | 按前缀对比两侧输出（`python tmp-c2/show.py <file.nut> [前缀…]`） |
| `tmp-c2/rxpath.nut` / `tmp-c2/rx_depth.py` | 正则病理形态（空匹配量词、深嵌套、长模式）与嵌套深度上限实测 |
| `tmp-c2/churn.nut` / `tmp-c2/mem_cases.py` | 对象/字符串 churn 的内存对拍（`python -c` 调 `tools/mem_probe.py` 见三·补8） |
| `tmp-c2/check_rs_scripts.py` | 把 Rust 测试里的脚本抽出来两侧对跑（期望值先经参考实现确认） |
| `tmp-c2/*.nut` | 一次性探针（BOM/UCS2、date 边界、escape 高位），已被 .gitignore 忽略 |

对拍方式（stderr 也要比，否则错误路径差异不可见）：

```powershell
python tools/diff_test.py                 # 官方 samples（stdout + stderr + 退出码）
python tools/diff_test.py tests-corpus    # 边界语料 + 三张生成矩阵
python tools/diff_test.py --no-stderr ... # 退回只比 stdout
python tools/repl_diff.py                 # REPL 交互会话
python tools/line_diff.py tests-corpus/t_opmx.nut   # 大语料按行定位差异
python tools/bisect_cases.py tools/probes/fmt_types.nut   # 逐条单独跑探针
python tools/cnut_matrix.py tests-corpus  # .cnut 字节一致 + 双向交叉执行
python tools/cnut_fields.py a.cnut b.cnut # .cnut 逐字段比对（定位差异字段）
python tools/lint_corpus.py               # 语料语法陷阱 + 逐个试编译
```

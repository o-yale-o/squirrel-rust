# squirrel-rust 状态评估（STATUS）

> 评估方式：不采信 README 的既有数字，逐项用官方 C++ 参考二进制（`build-ref/bin/Release/sq_static.exe`）
> 与 `target/release/sq.exe` 双引擎复跑；每一条结论都附可复现命令与实测输出。
> 评估时间：本次会话；基线提交 `b85d17e`。

## 一、已完成且已独立验证

| 验证项 | 复现命令 | 实测结果 |
|---|---|---|
| 官方 samples 差分（stdout + stderr + 退出码） | `python tools/diff_test.py` | **17/17 一致** |
| 边界语料差分 | `python tools/diff_test.py tests-corpus` | **51/51 一致** |
| 运算符 × 类型矩阵（3181 条断言） | `python tools/diff_test.py tests-corpus` | **逐行一致** |
| metamethod 全矩阵（343 个用例 × 18 个 metamethod） | 同上 | **逐行一致** |
| `format`/`printf` 格式串矩阵（1305 行） | 同上 | **逐行一致** |
| REPL 交互会话差分 | `python tools/repl_diff.py` | **5/5 一致** |
| `.cnut` 字节级一致 | `python tools/cnut_matrix.py` | samples **17/17**；语料 **48/51**（3 个只差参考实现的未初始化填充，见第四节） |
| `.cnut` 双向互通 | `python tools/cnut_matrix.py tests-corpus` | **51/51 一致**（含参考实现编译的字节码在本实现上执行） |
| 指令覆盖 | `cargo test --test opcode_coverage` | **61/61 条指令被语料覆盖** |
| 宿主 API 嵌入（验收 #5） | `cargo run --example minimal` + `cargo test --test host_api` | **5/5 通过**，输出与 C++ minimal.c 一致 |
| 委托表成员 | `python tools/audit_delegates.py` | **10 张表 88 个成员全一致** |
| 全局注册（nparamscheck + 掩码） | `python tools/audit_globals.py` | **62/62 全一致** |
| 单元 + 集成 + 健壮性测试 | `cargo test` | **72/72 通过** |
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

## 四、残留差异（参考实现自身是 UB / 无法复刻的部分）

| 项 | 说明 |
|---|---|
| `.cnut` 的 outer 类型字段 | 参考实现用 `sizeof(SQUnsignedInteger)`（8 字节）写一个 4 字节枚举，高 4 字节是**未初始化内存**：同一脚本连编两次字节都不同（实测 `ff245324…` vs `e4e3c42e…`），而 `e4e3c42e…` 恰好等于本实现的输出。因此「字节完全一致」在这 3 个含闭包的语料上不可达；本实现写确定的 0。加载侧已按低 32 位掩码，参考实现产出的字节码能正常执行（51/51 交叉执行一致） |
| 坏字节码里的越界 outer 索引 | 参考实现不检查该索引，夹具 `ref_closures_badouter.cnut` 让 C++ 侧访问违例崩溃（0xC0000005）；本实现报可捕获的 `invalid outer value index`（`crates/squirrel-stdlib/tests/bytecode_interop.rs` 守门） |
| `format` 规格串恰好 20 字符 | `validate_format` 的 `n-start > MAX_FORMAT_LEN` 差一，随后 `memcpy` 越界写 `fmt[20]`：`format("%" + 20 个 '+' + "d", 1)` 实测 `STATUS_STACK_BUFFER_OVERRUN` 崩溃。本实现从 20 起就报 `format too long`，不复刻 |
| `escape` 高位字节 | C++ 用 `(char)c` 传给 `%02x`：≥0x80 输出 8 位补码（`\xffffffe9`）。当字符串很短、scratchpad 不够时参考实现会越界写出**不确定内容**（实测 `escape("\xe9")` 输出 `\xffaaaaa\xffffffe9`）。本实现取确定性的补码值，短字符串场景与参考不同 |
| `date` 上界 | UCRT 的上界常量未逐秒定位：实测 32536800000（3001-01-19）可用、32536857599 失败。本实现取前者为上界 |
| `getstackinfos(-1)` | 参考实现 `_callsstack[cssize]` 越界读（UB，实测返回一个表）；本实现返回 `null`，不 panic |
| 挂起线程上再 `call()` | 参考实现的 `suspend` 不关帧，故错误文本是 `attempt to call 'string'`；本实现关帧后取到的是环境表，文本为 `attempt to call 'table'`（都用 `wakeup` 时行为一致） |
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

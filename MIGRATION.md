# 从 C++ 版 Squirrel 迁移到本实现：会撞到什么

> 本实现的目标是「脚本行为对齐 C++ 版（Win64、`_SQ64`、`SQFloat=f32`、非 UNICODE）」，
> 已经用 **69 个边界语料 + 5600 多条生成矩阵 + 60 条编译错误对拍 + `.cnut` 字节级互通**
> 把关（详见 `STATUS.md` 与 `README.md`）。
>
> 但有一类地方是**故意不一致**的：参考实现在那些输入上会**崩溃 / 内存损坏 / 把宿主吃光**，
> 而我们选择「给可捕获的错误」。这份文档把这类点逐条列出来，每条都能一条命令复现。

## 0. 一句话总览

| 类别 | 参考实现 | 本实现 | 你需要做什么 |
|---|---|---|---|
| 参考实现自身的 UB / 崩溃（约 20 种形态） | `0xC0000005` / `0xC0000095` / `0xC0000409` / `0xC00000FD` 崩溃，或静默内存损坏 | **可捕获的脚本错误** | 如果你的脚本靠"崩了重启"兜底，改成 `try/catch` |
| 资源无上限（递归、指令数、内存） | 一直涨到系统杀进程 | 有上限，报可捕获错误；CLI 另有 `--max-ops` / `--mem-limit` | 正常脚本无感；死循环脚本会从"挂死"变成"报错" |
| 不可复现的取值（裸指针序等） | 同一脚本换次运行就不同 | 确定性值（只保证"同一次运行内一致"） | 不要依赖跨进程稳定的比较结果 |
| 平台副作用（CRT 在管道上的 `fseek`） | 会丢弃已缓冲输入 | 确定性行为 | 用管道喂 stdin 的脚本行为会更"正常" |

## 1. 参考实现会崩溃、我们给可捕获错误

每条都实测过（复现命令在表格下方）。

| 形态 | 参考实现实测 | 本实现 |
|---|---|---|
| `(-9223372036854775807 - 1) / -1`（以及 `% -1`） | `STATUS_INTEGER_OVERFLOW (0xC0000095)` —— x86 `idiv` 商溢出硬件陷 | 回绕成 `INT64_MIN` / `0` |
| `file(路径, "")`、`"z"`、`"R"`、`"rbb"`、`"trash"`、`"r\tb"` | `0xC0000409` —— 模式串直接进 `fopen`，落到 UCRT invalid parameter handler | 白名单校验（接受集与实测一致），报 `cannot open the file` |
| `getstackinfos(-1)` / `thread.getstackinfos(<负 level>)` | 越界读空调用栈（ASan: `sqdebug.cpp:35` heap-buffer-overflow），偶发整进程崩 | 返回 `null` |
| `blob.resize(<负数>)` / `blob.readblob(<负数>)` | 访问违例 `0xC0000005` | 可捕获错误 |
| `split(<非串>, <非串>)` | `str` 未初始化即被解引用 → 崩溃框 | `parameter 1 has an invalid type …` |
| `regexp("")` | 写 NULL 崩溃（弹模态框） | `letter expected` |
| `format("%" + 20 个 '+' + "d", 1)` | `STATUS_STACK_BUFFER_OVERRUN` | 从 20 起就报 `format too long` |
| `tointeger(<非法进制>)`（base=1 / 负数 / >36） | `STATUS_STACK_BUFFER_OVERRUN` | `cannot convert the string` |
| 嵌套括号 795 层 | `STATUS_STACK_OVERFLOW (0xC00000FD)` | 编译期 `nesting too deep`（790 层及以内**两侧都能编**） |
| 深嵌套值（10 万层容器）释放 | 栈溢出崩溃 | 迭代式释放 |
| 坏字节码里的越界 outer 索引 | 访问违例崩溃 | `invalid outer value index` |
| `escape()` 遇 ≥0x80 字节 | 用 `(char)c` 喂 `%02x`，越界写 scratchpad：**输出串行**、混进别的错误消息片段 | 确定性 32 位补码（`\xffffffe9`），但**我们也不再复刻那次越界** |
| `throw` 一个**正在执行的 VM 自己**、线程递归调用自己 | 未定义/死循环 | 不进语料；本实现不会崩 |

```bash
# 复现（两边对比；tmp-c2/ 下都是本地工具，参考实现崩溃已被抑制，不会弹框）
python tmp-c2/mode_probe.py            # 非法文件模式：ref rc=3221226505 vs 我们可捕获
python tmp-c2/p3_nest_range.py         # 嵌套括号 790/795 层
# 语料里对应的守门测试：
cargo test --release -p squirrel-stdlib --test numeric_semantics   # INT64_MIN / -1
cargo test --release -p squirrel-stdlib --test stream_semantics    # 非法模式
cargo test --release -p squirrel-stdlib --test exception_semantics # getstackinfos(-1)
cargo test --release -p squirrel-stdlib --test robustness_p3       # dump 限深 / 嵌套上限 / 指令预算
python tools/errmsg_matrix.py                                      # 编译错误文本 60/60
```

## 2. 资源上限（参考实现没有）

| 项 | 参考实现 | 本实现 |
|---|---|---|
| 非尾递归无限递归 | 一直涨：实测 3 行脚本把宿主吃到 **32GB** 被系统杀（C++ 侧 10 秒 191MB 且在涨） | 封顶（1M 槽 / 20 万帧），报**可捕获**的 `stack overflow`，峰值 ~30MB |
| 尾调用死循环 | 永远不返回（只能 Ctrl+C） | 默认照旧；加 `--max-ops N` 后报 `execution budget exceeded` |
| 内存总量 | 无上限 | `--mem-limit N`（Windows Job Object）：实测 127.5MB → 38.1MB 被截停 |
| 未捕获 `stack overflow` 的调用栈 dump | 到不了那么深（先 OOM） | 最多 1000 帧 + 一行「省略 N 层」：stderr **12.3MB → 0.07MB** |
| 正则递归/步数 | 空匹配量词有死循环风险；4000 层嵌套能吃满栈 | 步数上限 + 栈预算；超限按不匹配/可捕获错误 |
| 字符串/对象回收时机 | 引用计数**即时**回收 | `Rc` + GC 链，清了死条目后同量级（20 万次拼接 2000MB → 18MB，参考 5.2MB） |

```bash
python -c "import sys,os;ROOT=r'D:\OpenSource\squirrel-rust';sys.path.insert(0,os.path.join(ROOT,'tools'));from mem_probe import run_with_cap,RUST,REF;[print(t, run_with_cap(e,[os.path.join(ROOT,'tmp-c2','p3_dump.nut')],2000,30)[1:3]) for t,e in (('rust',RUST),('ref',REF))]"
target\release\sq.exe --max-ops 500000 tmp-c2\p3_loop.nut    # execution budget exceeded
target\release\sq.exe --mem-limit 64 tmp-c2\p3_mem.nut       # 进程被截停
```

## 3. 取值不可复现 → 确定性

| 形态 | 参考实现 | 本实现 |
|---|---|---|
| 两个不同类型/同类型对象之间的 `<=>` | 按裸指针序（`_userpointer(o1) < _userpointer(o2)`），跨进程/跨字节码不可复现（cnut 交叉执行实测同一脚本 `-1` vs `1`） | 按自身身份地址，同样只保证"同一次运行内一致" |
| 在已挂起的线程上再次 `call()` | `suspend` 不关帧，错误文本是 `attempt to call 'string'` | 关帧后取到环境表 → `attempt to call 'table'`（**两者都是可捕获错误**，只有类型名不同） |
| REPL 读到 EOF | `getchar()` 返回 −1 被当字符累积 → **死循环** | 直接退出 |
| 不可 seek 流上的 `len()/eos()/tell()` | 靠 CRT `ftell`/`fseek`：管道上 `fseek(SEEK_END)` 失败并**丢弃已缓冲输入**（实测 8 字节管道读完 2 字节后剩 6 字节消失），`len()` 返回内部状态（904 等） | `tell()`=已消费字节数、不可 seek 时 `len()`=0；文件重定向下与参考逐字节一致 |
| `.cnut` 里 outer 类型字段的高 4 字节 | 写 8 字节、字段 4 字节 → 高 4 字节是**未初始化内存**（同脚本连编两次都不同） | 写确定的 0（低 32 位与参考逐位相同，其余字段全同） |
| `date()` 上界 | UCRT 常量未逐秒定位（32536800000 可用、32536857599 失败） | 取前者为上界 |

```bash
python tools/stdin_eof.py          # 管道/重定向下 len/eos/tell 的行为
python tools/cnut_matrix.py tests-corpus   # 字节一致 + 双向交叉执行（含 PAD-ONLY 说明）
```

## 4. 保真复刻的"怪癖"（这些**不要**当成 bug）

迁移时最容易被误认为"本实现有 bug"、其实是**两边一致**的行为：

| 怪癖 | 说明 |
|---|---|
| `%` 除零 | 运行时 `a % b`（b==0）**不报错**、函数返回 `null`；`1 % 0` 被编译期常量折叠成 `1` |
| 移位计数 | 只用计数低 6 位：`1 << 64` == `1`、`1 << -1` == `1 << 63` == `INT64_MIN` |
| 字符串下标 | **有符号**：`"\xff"[0]` == `-1`、`"\u00e9"[0]` == `-61`（blob 则是 0..255） |
| `escape()` | 高位字节输出 32 位补码 `\xffffffe9` |
| 正则 `.*` | **不是贪婪**：`<.*>` 对 `"<a><b>"` 给 `{0,3}`（停在第一个能继续的位置） |
| 正则 `\b` | 不是词边界，是"空格↔非空格转移" |
| 正则 `^` | 锚定的是**本次 search 的 start 偏移**，不是字符串开头 |
| 空主语 | `search` 对空串直接不匹配；`match("")` 才是 true |
| `?` 紧跟量词 | 是**字面量** `?`（引擎没有惰性量词） |
| 属性块 `</ … />` | **必须独占一行**；成员之后写、单行写第二个块都是语法错 |
| `function named() {}` | 只能作**语句**：`local f = function named(){}` 报 `expected '('` |
| `dofile`/`loadfile` 第 2 参 | 是 `printerror` **布尔**，不是脚本实参 |
| 序列化默认参数 | 往返**会丢**默认值（`g(1)` 里缺省参数变 `null`） |
| 未捕获错误的退出码 | 是 0（`retval` 不被错误覆盖）；CLI 加载失败才是 −2 |
| tail call | `return inner()` 会**复用当前帧**，出错时调用栈里看不到这一层 |
| string 库是**全局函数**，不是字符串方法 | `strip/lstrip/rstrip/split/escape/startswith/endswith/format/printf` 都要写成 `startswith("abc","a")`；`"abc".startswith("a")` 报 `the index 'startswith' does not exist` |
| `startswith`/`endswith` 严格 3 参数 | 没有 start 偏移；少传/多传都是 `wrong number of parameters` |
| `startswith`/`endswith` 空模式 | 恒为 `true`（`memcmp(...,0)==0`），包括 `startswith("", "")` |
| `split` 按 **C 字符串**扫描 | 遇到**嵌入 NUL 就停**，其后整段丢掉：`split("a\x00,b", ",")` 是 `["a"]`，不是 `["a\x00","b"]`；分隔串里含 NUL 时同样先停在 NUL |
| `lstrip` 结果在**嵌入 NUL 处截断** | 它用 `sq_pushstring(v,start,-1)`（负长度 → `strlen`）：`lstrip(" a\x00b")` 是 `"a"`（长度 1） |
| `rstrip`/`strip`/`escape`/`startswith` 不受 NUL 影响 | 这些走**显式长度**：`rstrip("a\x00b ")` 长度 3、`strip("  a\x00b  ")` 长度 3、`escape("a\x00b")` 是 4 个字符 `a\0b` |
| `escape` 不吃具名转义 `\t\n\r` | `scisprint` 为假 → 走 `\xNN`：`escape("a\tb")` 是 `a\x09b`（`\0` 是唯一例外，转成两个字符 `\0`） |
| `print` 在**嵌入 NUL 处截断** | 两边一致（截断后连换行一起丢，下一行会接到同一行上）——所以语料里含 NUL 的用例改成打印各元素**字节长度**来观测 |

## 5. 迁移检查清单

1. **有没有依赖"崩溃"的行为**？——参考实现会在上面第一类输入上崩；改成 `try/catch`。
2. **有没有无限循环/无界递归的兜底**？——加 `--max-ops` / `--mem-limit`，或依赖默认的递归上限。
3. **有没有跨进程比较对象（`<=>`、`==` 之外的会话）**？——不要依赖指针序。
4. **有没有用管道喂 stdin 且依赖 `len()/eos()`**？——参考实现那边本身不可靠，本实现是确定值。
5. **正则依赖贪婪语义**？——两边一致地"不贪婪"，但如果你的代码是按标准正则写的，**在 C++ 版上就已经是错的**。
6. **嵌入宿主（C/C++）**？——本实现目前**没有 C ABI**（141 个 `sq_*` 未导出）；Rust 宿主的 API 见
   `crates/squirrel/src/hostapi.rs` 与 `crates/squirrel-stdlib/tests/host_api*.rs`。
   是否需要 C ABI 仍在等确认（`TODO.md` 的 P0）。
7. **字符串处理里有没有嵌入 NUL**（二进制数据塞进 string）？——`split`/`lstrip` 在两边都是
   "到 NUL 为止"，别指望拿到 NUL 之后的内容；要处理二进制请用 `blob`。

## 6. 完整清单

本文是**面向迁移**的摘要。逐条实测数据、ASan 输出、涉及的源文件行号都在
[`STATUS.md`](STATUS.md) 第四节的表里（约 26 条），每条都注明了"参考实现为什么是 UB"。

string 库全局函数那一族（第 4 节最后 8 行）的守门位置：
`crates/squirrel-stdlib/tests/string_semantics.rs`（3 个测试）+ 语料
`tests-corpus/t_delmatrix.nut` 的 `m_s_*` 段（229 条语义用例，两侧逐字节一致）。

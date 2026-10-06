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
| **不可达的 file/blob/stream 何时释放** | 引用计数归零 / 帧弹出 / GC 时释放 | **已对齐**（P14）：后端注册表原来只增不减、导致未显式 `close()` 的实例活到进程结束；现在实例上挂一个 userdata（`__be`），其 release hook 在实例死亡时释放注册表槽位。仍建议脚本显式 `close()` |
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
| 在已挂起的线程上再次 `call()` | `suspend` 不关帧，于是第二次 `call()` 拿**被挂起帧里的残留栈槽**当函数调 —— 那个槽正好还是 `suspend(x)` 传入的 `x`：实测 `x=42`→报 `attempt to call 'integer'`、`"s"`→`'string'`、`null`→`'null'`、`[1]`→`'array'`、`{}`→`'table'`、`1.5`→`'float'`、`true`→`'bool'`。**若 `x` 是闭包，参考实现会把它当函数调用并执行成功**（实测副作用真的发生） | 关帧后恒为可捕获的 `attempt to call 'table'`，**并且绝不执行残留闭包**（这是安全不变量，有用例守门）。差异只在错误文本的类型名上；语料 `t_threadmap.nut` 把这个类型名归一化，只比较"是否报了 attempt to call" |
| REPL 读到 EOF | `getchar()` 返回 −1 被当字符累积 → **死循环** | 直接退出 |
| 不可 seek 流上的 `len()/eos()/tell()` | 靠 CRT `ftell`/`fseek`：管道上 `fseek(SEEK_END)` 失败并**丢弃已缓冲输入**（实测 8 字节管道读完 2 字节后剩 6 字节消失），`len()` 返回内部状态（904 等） | `tell()`=已消费字节数、不可 seek 时 `len()`=0；文件重定向下与参考逐字节一致 |
| `.cnut` 里 outer 类型字段的高 4 字节 | 写 8 字节、字段 4 字节 → 高 4 字节是**未初始化内存**（同脚本连编两次都不同） | 写确定的 0（低 32 位与参考逐位相同，其余字段全同） |
| `date()` 上界 | UCRT 常量未逐秒定位（32536800000 可用、32536857599 失败） | 取前者为上界 |
| `escape()` 遇 ≥0x80 字节 | 用 `(char)c` 喂 `%02x`，越界写 scratchpad：**输出串行**、混进别的错误消息片段 | 确定性 32 位补码（`\xffffffe9`），但**我们也不再复刻那次越界** |
| **容器的超大一次分配**（`array(N)` / `resize(N)` / `blob(N)` 的 N ≥ 2³¹） | 无任何守卫：`array(9223372036854775807)`、`resize(0x7FFFFFFF)` → `0xC0000005` **崩溃**；`resize(4294967296)` → 分配几十 GB，Windows 错误报告把整台机器挂死；`blob(4).readblob(-1)` 同样挂死（负尺寸转无符号后巨型分配） | 按 `MAX_CONTAINER_BYTES`（2 GiB）封顶：超限报**可捕获**的 `container too big`，不 panic、不挂死（`robustness.rs` 的 `container_allocations_are_bounded_and_catchable` 守门）；负尺寸维持原有语义（`resizing to negative length` / `cannot create blob with negative size` / readblob 夹 0）。合法大分配（如 `array(100000, 7)`）不受影响；要更紧的进程级上限用 CLI 的 `--mem-limit` |
| **高位字节 × 字符串库整体是参考 UB 沼泽** | `toupper/tolower` 把 `(signed char)` 高位字节当负数喂 CRT（UB，输出乱码碎片）；`escape` 的越界读会**污染其后所有 stdout**（同样的字符串，字面量直写、经变量、不同长度，输出的乱码形态都不同、不可复现）；连普通 `len` 的表现都随上下文漂移 | 安全子集与参考一致：**比较是逐字节 memcmp**（`"\xff" <=> "\xffx"` = −1，前缀短者小）、高位字节字符串**可以当表键**（按内容哈希）、`len` 正常。除此之外这一族**不做差分语料**（参考输出不可复现，语料会变成摆设）；本实现侧全部给确定性输出 |
| **`newthread(<原生闭包>)`** | **创建时即崩溃**（实测 WER 弹窗、无任何输出；`tmp-c2/gt_isolate.py` 逐条隔离）—— 给线程装原生闭包当函数体，参考实现取函数信息时走的是脚本闭包的路径 | 正常创建线程对象（`typeof` = `thread`），调用时报可捕获的 `wrong number of parameters`（`gt_semantics.rs` 的 `newthread_with_native_closure_is_safe_here` 钉住）。正常脚本不会这么写，属安全超集 |

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
| 序列化默认参数 | 往返**会丢**默认值，但**只丢"被直接序列化的那个闭包自己"的**：`writeclosuretofile(P, function(a, b = 100) {...})` 读回后 `g(1)` 里 `b` 是 `null`；而闭包**内部**嵌套函数的默认值（`f(1)` → `b=100`）、它**返回出去**的内部闭包（`inner=77`）、以及 varargs（`vargv.len()`）都完好 |
| `writeclosuretofile` 的**检查顺序**与残留文件 | 顺序是"掩码 → 开文件 → 序列化"：① 第 3 参过不了 `.sc` 掩码（传整数）→ 参数类型错，**不会建文件**；② 过了掩码**先 `fopen(...,"wb+")`**：路径不可写 → `cannot open the file`（**哪怕第 3 参是 native closure、后面还会类型不符**）；③ 路径可写则文件**立刻建出来**，之后无论序列化失败（自由变量）还是 native closure 不匹配，那个 **0 字节文件都会留在磁盘上**。写脚本时别用 `writeclosuretofile` 的返回值判断"文件有没有生成"，要自己检查 |
| 文件句柄的**共享模式**（Windows） | 参考实现的 `fopen` 用 `_SH_DENYNO`（`FILE_SHARE_READ/WRITE`，**不含** `FILE_SHARE_DELETE`）：**句柄还开着时 `remove()` 会失败**（`remove() failed`，文件留在磁盘上） | **已对齐**（P14）：显式 `share_mode(FILE_SHARE_READ \| FILE_SHARE_WRITE)`；前提是后端随实例死亡释放（同一轮修掉了注册表只增不减的问题）。语料 `t_file.nut` 的 `share_remove_*` 钉住 |
| `file.seek()` 的偏移是 **32 位**的 | `sqstd_fseek` 里是 `fseek(file,(long)offset,…)`，Windows 上 `long` 只有 32 位 → 偏移**先截成 int32**：`seek(4294967301)` 其实 `fseek(…,5)`、`seek(INT64_MAX)` 低 32 位是 `-1` → CRT 失败返回 `-1`、`seek(INT64_MIN)` → `0`、`seek(2147483648)` → `-1`。**`blob.seek()` 不截断**（不走 CRT，两种情况下都返回 `-1`/正常值）。写脚本时别用超过 2 GiB 的偏移；语料 `t_file.nut` 的 `trunc_*` 10 条钉住 | **已对齐**（P15-2）：`offset as i32 as SQInteger`；`blob` 路径不截断 |
| **尾调用会提前释放当前帧的局部** | `return f(x);` 复用当前帧，但帧里的局部（如打开的文件）**保留到外层帧弹出**才释放 | 我们目前更早释放 —— `local a = file(p,"wb+"); return try_remove(p);` 里我们能删掉文件（句柄已关），参考实现报 `remove() failed`；改成 `local r = try_remove(p); return r;`（非尾调用）两边就一致。已记为 `TODO.md` P14 子问题 2，语料刻意回避这一格 |
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
| `RAND_MAX` 是 **32767** | MSVC 的 CRT 就这么定义，不是 `INT_MAX`；`rand()` 的序列也按 MSVC CRT 复刻 |
| `abs()` 是 **32 位**的 | 参考实现写的是 `abs((int)n)`：先截成 int32 再取绝对值 —— `abs(2147483648)`→`-2147483648`、`abs(-2147483649)`→`2147483647`、`abs(INT64_MAX)`→`1`、`abs(INT64_MIN)`→`0`、`abs(-3000000000)`→`1294967296` |
| `abs()` 只吃整数 | `abs(-7.9)`→`7`（先截断再绝对值，返回 **integer**）；浮点版是 `fabs`。`floor`/`ceil` 返回的也是 **float** 而不是 int |
| 数学函数定义域外**不报错** | `sqrt(-1)`/`log(-1)`/`asin(2)`/`pow(-1,0.5)` → NaN（打印成 MSVC 的 `-nan(ind)`）、`log(0)`→`-inf`、`exp(f32::MAX)`→`inf` |
| `pow(±NaN, 奇整数)` 返回**正** NaN | 参考实现走 `(SQFloat)pow((double),(double))`，CRT 的"奇次幂"分支会清掉符号位，而 x86 上 `0.0/0.0` 是**负** NaN —— 所以 `pow(0.0/0.0,-1.0)` 是 `nan` 而不是 `-nan(ind)`；偶数/非整数指数（含 `NaN`、`inf`）则**保留**输入 NaN 的符号。判定只能看文本渲染：与 NaN 的比较恒为 false，`1.0/NAN < 0` 那类写法两边都会"一致"却什么都没测到。语料 `t_mathmap.nut` 的 `sgn_pow_*` 钉住（`atan2` 的 NaN/inf 传播两边本来就一致） |
| 数值字面量与 `"…".tointeger()` 是**两条不同的路径** | 同一个数字，写法不同结果就不同：`99999999999999999999999999` 作为**字面量**走词法器 `LexInteger`（**回绕** → `-2537764290115403777`），作为**字符串**走 `str2num`/`_strtoi64`（**按符号饱和** → `INT64_MAX`）。十六进制字面量走 `LexHexadecimal`（同样回绕，且超过 **16 位**直接报 `too many digits for an Hex number`），`0xffffffffffffffff` 是 **-1**；八进制没有 `0b`/`0o` 前缀（`0b101` 是 `0` 后面跟标识符）。**`1.2.3` 是一个合法的浮点 token**（`ReadNumber` 会把第二个 `.` 也吃掉，取值走 `strtod` → `1.2`；`1..2` → `1`）。语料 `t_litmap.nut` 把 889 条形态钉住 |
| `slice`/`find`/`insert`/`remove` 的**索引值语义** | 负数从尾部数、越界夹进 `[0, len]`、**start > end 给空结果**、浮点先截断；`find` 的起始偏移同样夹取，找不到返回 `null`；`insert` 允许 `idx == len`（尾部）、`remove` 不允许；`resize` 负数报 `resizing to negative length`、缺省 fill 是 `null`。语料 `t_idxmap.nut`（128 条）钉住 |
| `.5` 不是字面量 | 小数点开头在词法阶段进不了数字分支（`ReadNumber` 只在首位是数字时进入）→ 报 `expression expected`；`0.5` 才是合法写法 |
| **循环委托在赋值时就报错** | `setdelegate` 的环检测发生在**赋值当下**：`a` 已挂 `b` 再 `b.setdelegate(a)` 直接抛 `delegate cycle`（自挂、三角环同理）——任何环都构造不出来，不存在"等查找时才发现"。`setdelegate` 只接受 `null\|table`（类/实例/数组报参数类型错），实例和数组**没有**这个方法 |
| **`raw*` / `<-` / `delete` 与委托链的关系** | `rawget/rawin/rawset` **不穿链**；`b.q <- 5`（无 `_newslot` 元方法时）挂到**接收者自身**而不是链上游；**`delete b.q` 也不穿链**（只删自身的槽，穿链要靠 `_delslot` 元方法）——这三条最容易想反。`foreach`/`len` 同样只看**自己的槽** |
| **`foreach` 期间的容器变更**（参考实现全程不崩，结果两侧逐字节一致） | 表：迭代中删当前键 → 计数照旧、剩 1；删**别的**键 → 其后的键被**跳过**；`clear()` 后迭代立即终止。数组：`append` 会**延长**迭代；`remove`/缩容 `resize` 让迭代**提前结束**；变长 `resize` 按**旧长度**走完。语料 `t_delegiter.nut`（59 条）钉住 |
| **生成器/线程与异常的交错** | 生成器死后 `resume` 报 `resuming dead generator`（正常跑完/抛错两种死法都算）；体内抛错传到**恢复点**的 catch；`yield` 在 try 里 + 体内 throw → 走生成器**自己**的 catch（`catch_yields` 能连续 yield）；**`yield` 是语句不是表达式**（`local a = yield x` 是编译错）；含 yield 的普通函数自动成为生成器。线程状态串是 **`idle/suspended/…`（没有 "dead"）**，跑完回 idle 且**可以再 `call()` 从头重跑**；`wakeup`/`wakeupthrow` 打在 idle 上报 `cannot wakeup a idle thread`；**`wakeupthrow` 注入的错误会被挂起点外层的 try 接住**；`wakeup` 不带参数 → `suspend` 表达式的值是 `null`，多带的参数被忽略；挂起线程的 `getstackinfos` 返回 null；**主线程里 `suspend()` 静默退出**（无错误文本，try/catch 接不住）。语料 `t_gtmap.nut`（37 条）钉住 |
| **类属性：时序 / 共享 / 继承边界** | 属性挂在**类**上，实例不快照——实例化之后 `setattributes`，实例透过 `getclass()` 立即可见；**继承后改父类属性，子类不受影响**（extend 时拷贝进子类），但父类成员的属性在子类**可读可设**（`setattributes` 返回 null）；三层继承看**最近的重写层**。**类级属性（null 键）不随 extends 继承**。属性表是**共享引用**：`getattributes("v").a.n = 42` 对后续读取可见，而给属性表**加新键** `x.same = "yes"` 报 `the index 'same' does not exist`——这是 Squirrel 核心语义"普通 `=` 不建新槽（要用 `<-`）"，不是属性特有。`newmember` 的 **4 参形式** `(name, val, attrs, isstatic)` 属性与 static 都生效；`clone` 一个 **class** 报 `cloning a class`；实例**没有** `setattributes/getattributes`；`delete C.v` 报 `attempt to delete a slot from a class`；`rawset` 新成员属性为 null。语料 `t_attrtiming.nut`（28 条）钉住 |
| **链式赋值 `x = y = z`**（P23 修复） | 编译器把连续两条 MOVE 合并成一条 `DMOVE(d0←s0, d1←s1)`，执行是**严格顺序**（先 `y←z` 再 `x←y`，第二读看到第一写）——所以链式赋值三者同为 3。本实现曾把两个源**先快照再写**（并行语义），链式赋值的外层拿到**旧值**（`x=0`）——已修为顺序语义（`vm.rs` 的 `Op::DMove`，带注释）。`x = (y = z)`、`o.m = o.n = 5`、RHS 是函数调用，全都同一条路径 | 语料 `t_evalmap.nut` 的 `asg_chain`/`chain3`/`chain4` 钉住 |
| **`bindenv` × 编译闭包**（**未修的引擎分歧**，TODO P28） | `compilestring` 产出的闭包经 `f.bindenv(env)` 后，**裸自由标识符不读 env**：`local env = { marker = "env-ok" }; local g = compilestring("return marker", "X").bindenv(env); g()` → `the index 'marker' does not exist`（读的是 root；`setdelegate` 有无无关，4 种形态见 `tmp-c2/bind_iso.nut`）。而普通（文件编译）闭包 bindenv 后正常（`marker=env-ok`）；`return this` 也正常（t_surface2 的用例只钉了"typeof 是 table"，**区分不了 root 与 env**——这正是它漏掉这条的原因） | 本实现裸标识符读 bindenv 的 env（`env-ok`）——行为上更像"bindenv 该有的样子"，但与参考不同。**复现与判别**见 `tmp-c2/bind_iso.nut`；语料只收一致子集（`t_bindenv.nut`）。要修需动自由标识符解析 + `_env` 调用路径 |
| **`bindenv` × 编译闭包的**参考侧行为**随调用结构漂移**（P29 补充） | 同一段 `compilestring("return this.marker", …) + bindenv(env)`：写在**具名顶层函数**里或**顶层语句**里 → 参考正常返回 `env-ok`；写在**匿名闭包实参**里（`s("x", function() { … })` 形态）→ 参考稳定报 `the index 'marker' does not exist`（同文件重跑可复现）。即参考实现这一路径**依赖帧布局/调用结构**，机制无法从外部完全确定 | 本实现**两种结构下都给 env**（确定性）。判别与复现文件：`tmp-c2/bisect_bind.nut`（4 变体全一致）、`tmp-c2/bind_ctx.nut`（同进程对照）、`tmp-c2/c_exact.nut`（失败形态）。忠实复刻参考的帧布局依赖行为不可行，维持偏离 |
| **`&&` / `||` 返回操作数值本身**（不是布尔） | `true && 2` → **2**、`"" && 5` → **5**（`""` 是真值）、`false && f()` **不调用** `f` 且结果是被短路的左操作数 `false`；`1 && 2 || 3 && 4` → **2**（`&&` 先结合，`||` 左真值后右半整体被短路——`3 && 4` 根本不求值）。判假用 `SQVM::IsFalse`：只有 `null/false/0/0.0` 假，`""`/`[]`/`{}` 都真 |
| **switch** | `case` 标签可以是**任意表达式**（含变量、运行期求值）；重复 case 允许、**先到先得**；浮点 case 与整数 case 交叉匹配（`case 2:` 接住 `2.0`）；`case` 写在 `default` **之后**是语法错 `expected '}'`（default 在前合法）；switch 里的 `break` 归 **switch** 不归外层循环；case 体里可以声明 local。语料 `t_evalmap.nut`（50 条）钉住 |
| **编译期折叠的边界** | `%` 会被编译期折叠：`7 % 0` → **7**（`a % 0` → `a`，已在 MIGRATION 早前条目记录）；整数 `/` **不**折叠：`1 / 0` 是运行时 `division by zero`；浮点折叠走 double：`1.0/0.0`→inf、`1.0%0.0`→nan；INT64 字面量折叠**回绕**：`MAX+1`→MIN、`3037000500*3037000500`→`-9223372036709301616`；`1<<64`→1、`1<<-1`→INT64_MIN；`"a"+"b"` 字符串拼接在运行时做（无折叠） |
| **错误转储（stderr 的 CALLSTACK/LOCALS）只在跨界时打印** | 错误跨越**线程边界**（含 `wakeupthrow` 注入、生成器在线程内抛错）与**原生调用边界**（sort/map 比较器回调）时：即使外层脚本 try/catch 接住，stderr 照样打印 `AN ERROR HAS OCCURRED [msg]` + CALLSTACK + LOCALS（含脚本相对路径），且脚本**继续跑完**；**纯脚本帧之间**被 catch 的抛错不产生任何转储。另外 `_get` 元方法里解析落回委托链的名字会**递归爆栈**（`Native stack overflow`，两侧一致、可被 catch）。语料 `t_errdumpframes.nut`（13 条）钉住 |
| **`suspend` 在生成器体内 = 静默退出** | 与主线程 `suspend()` 同一族：生成器体内 `suspend(...)` 无错误文本、try/catch 接不住，脚本**静默终止**（挂在该用例之后的代码全部丢失）。两侧一致；语料要避免这种形态（t_clonestack 的生成器 docstring 有记录） |
| **`clone` 是浅拷贝、委托保留、类型受限** | 嵌套表**共享**（`b.inner.n = 42` 两边都变）；数组顶层独立（append/赋值不影响原数组）但**元素共享**；**委托保留**（clone 后能读链上的键）；委托链上的普通 `=` 是**链写**——命中链上第一个有该键的表并写进去（`b.g = 99` 会写进 base）。**不可 clone**：标量、null、generator、thread 一律报 `cloning a xxx` |
| **`getstackinfos` 的帧与 locals** | level 0 是它自己（`func="getstackinfos"`、locals 空）；**匿名函数的 func 是 `"unknown"`**；`src` 是**相对路径**（compilestring 则是给定的名字）；`locals` 含参数与已声明局部（后声明的也在，值为 null），且返回的是**副本**（改它不影响真实局部）；**生成器帧查不到外层局部**；线程体内 `getstackinfos(1)` 是线程体自己的帧（thread **方法** `th.getstackinfos(1)` 对挂起线程才是 null）；**尾调用逐层顶掉帧**（`return inner()` 之后 getstackinfos 看不到中间帧）。语料 `t_clonestack.nut`（21 条）钉住 |
| **表键严格按类型区分** | `t[1]`、`t[1.0]`、`t["1"]`、`t[true]` 是**四个不同的槽**——值相等也不互通（`t[1.0]` 读不到 `t[1]` 写的值，报 `the index '1' does not exist`）；`-0.0` 与 `0` 是两个键；**null 不能当键**（`null cannot be used as index`）；混合键型的遍历顺序是**哈希序**（`bool;string;float;integer;`）。**数组下标**则做 `tointeger` 转换：`a[1.0]`→`a[1]`、`a[1.9]`→`a[1]`（截断）、`a[-0.0]`→`a[0]`，但 `a["1"]` 报错。语料 `t_keymap.nut`（30 条）钉住 |
| **循环变量的闭包捕获** | `for`/`foreach` 的循环变量被闭包捕获时**共享同一个槽**——循环结束后所有闭包看到的都是最终值（`3,3,3` / `30,30,30`）；**循环体内 `local j` 才逐次独立**（`0,10,20`）；共享 `i` 的两个闭包一侧修改对另一侧可见（`2,102,102,202`）；捕获循环变量的生成器在循环结束后 resume 看到最终值。要逐次独立就在循环体里 `local j = i`。语料 `t_keymap.nut` 钉住 |
| **比较器的返回值走 `sq_getinteger`** | 收 **integer / float / bool**：float 按 `(SQInteger)` **截断**（`0.5`→0、`1.9`→1、`-1.9`→`-1`），bool 给 0/1；`null`/字符串/数组等报 `numeric value expected as return value of the compare function`（数组保持原顺序）。所以 `a.sort(function(x, y) { return x - y; })` 在**浮点数组**上也能用。另外比较器恒以 **3 个槽**调用（`this`=根表 + 两个元素），声明 3 个形参反而报 `wrong number of parameters`；返回常量会让堆排给出确定但"乱"的排列（恒 `-1` → `inconsistent compare function`） |
| **`map`/`apply` 的回调只支持 1~3 个形参** | `__map_array` 每轮只压 `2 + (nArgs>=3) + (nArgs>=4)` 个值，调用却按 `GetUp(-(nArgs+1))` 取目标，只有 `nArgs == pushed` 时调用的才是闭包自己：**0 个形参**（nArgs=1）会去调那个数组、**4 个及以上形参**（nArgs>=5）会去调闭包下面那一槽（同样是数组）—— 两种情况都报 `attempt to call 'array'`。空数组不进循环、**不报错**。`filter`/`reduce` 则是固定槽数的普通调用（`wrong number of parameters (3 passed, N required)`），`find` 根本不调回调 |
| 容器回调的**实参顺序不一致** | `map`/`apply` 给回调的是 **(值, 下标, 数组)**，而 `filter` 给的是 **(下标, 值)** —— 顺序相反；`reduce` 是 `(累加值, 当前值)`；三者的 `this` 都是那个容器本身。回调的实参**个数**还取决于闭包**声明的形参个数**（`nArgs` 含 `this`），不是固定的。`array.find(x)` 是"找值返回**下标**"，不是 `find(predicate)`；`table.map` 返回的是 **array**；`table.keys(cb)` 的回调参数个数永远对不上（`TABLE_TO_ARRAY_FUNC` 的固定槽数），一定报 `wrong number of parameters` |
| 回调抛异常 | 错误原样往外传播（`sort` 会停在原地，数组保持调用前的顺序；`apply` 已处理的元素不回滚） |
| `format` 的数字走 **f32** | `format("%.17g", 2147483647)` 是 `2147483648`（整数被转成 `SQFloat`）——想精确打印整数就别套 `%g` |
| `assert` 的数字 0 是**假** | 判假用 `SQVM::IsFalse`：`null`/`false`/**`0`**/**`0.0`** 为假，而 `""`/`[]`/`{}`/`function(){}`/`blob(0)` 都是**真**（`assert("")` 通过） |
| `assert(cond, msg)` 的消息会先 `tostring` | `assert(false, 123)` 的错误文本是 `123`；`sq_tostring` 对容器给的是 `(table : 0x…)`，**不调 `_tostring`** |
| `error(msg)` **不抛异常** | 只调 errorfunc（打到 stderr）然后返回 `null`，脚本继续往下跑；参数个数严格为 2 |
| `array(size, fill?)` | `size` 走 `tointeger` 截断（`array(2.9)` 长度 2、`array(0.5)` 空）；多余实参忽略；`array` 没有 `getclass` |
| 非引用类型的 `weakref()` 返回**自身** | `(5).weakref()` 还是 `5`（`type()` 给 `integer`）；只有 table/array/… 才真给 weakref |
| **字符串 → 数字**（`"…".tointeger()` / `.tofloat()`） | 先扫 `'.'`/`'e'`/`'E'` 决定走浮点还是整数（`iseintbase = base > 13`；扫描是 **C 字符串**，第一个 NUL 之后不看）。整数路径是 MSVC `_strtoi64`：**溢出按符号饱和**，`"9223372036854775808"`→`INT64_MAX`、`"-9223372036854775809"`→`INT64_MIN`（不是回绕）；非法字符只是"停止"，所以 `"42abc"`→`42`、`"0x10"`→`0`。**`tofloat` 也走同一条路**：没有 `.`/`e` 就先当整数，于是 `"99999999999999999999999999".tofloat()` 是 `9.2233720368547758e+18`（饱和后的 INT64_MAX），**不是** 1e26 |
| `"…".tointeger(base)` | 第二参是**进制**（`"ff".tointeger(16)`→255），且字符串的委托表 `nparamscheck = -1` —— 多传实参不报错；`base` 只接受 **0 或 2..36**：base 0 自动识别（`0x`→16、前导 `0`→8），**base 1 / 负数 / >36 会让参考实现直接崩溃**（CRT invalid parameter handler），本实现给可捕获的 `cannot convert the string`。`tofloat` **没有** base 参数（传了就是 `wrong number of parameters`） |
| 浮点路径是 MSVC `strtod` | 十进制、**十六进制浮点**（`"0x1.8p3"`→12、`"0x1.8"`→1.5、`"0x.8p3"`→4，`p` 指数可缺可残缺）、`inf`/`nan`（大小写不敏感，`"inf.e"`→inf、`"nan.0"`→nan）都在其列 —— 但**只有含 `.` 或 `e` 时才走浮点路径**：`"inf".tofloat()` 走整数路径 → 转换失败。strtod 给的 NaN 是**全 1 尾数**（f32 `0x7FFFFFFF`/`0xFFFFFFFF`），打印成 `nan`/`-nan`，与 `0.0/0.0` 的 x86 indefinite（`0xFFC00000` → `-nan(ind)`）**不是同一个 NaN** |
| `dummy(...)` 不做参数个数检查 | 它是 `nparams=0`，`dummy(1,2,3,…)` 恒返回 `null` |
| **更新流（`"wb+"`）写入后必须先定位再读** | C 标准（C11 7.21.5.3）规定更新流"输出后不能直接输入"，中间必须有 `fflush` 或文件定位调用；少了这一步就是 UB —— **实测参考实现会给出"读越界却把文件撑大"的怪行为**（4 字节文件 `readn('l')` 之后 `len()` 变 12，而且同一段代码只因"读完后是否调用 `len()`"就一个报错一个成功）。本实现在这种序列上给确定行为（`io error`），但**不要依赖任何一侧**：写脚本时老老实实 `seek` 或 `flush` |

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

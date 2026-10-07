# TODO：对标 C++ 版 Squirrel 的功能面盘点与优先级

> 本文是**按当前实测**排的待办清单（不是猜测）。每条都给：为什么排这个位置、怎么验证、
> 大致工作量（"轮" ≈ 一次 生成矩阵 → 差分 → 修 → 进语料 → 加测试 → 提交 的循环）。
> 盘点脚本全部可重跑，见文末"复现命令"。
>
> **进度（最新）：P1 / P2 / P3 / P4 全部完成**，唯一未做的是 **P0（C ABI）**，
> 它按约定"等用户确认宿主形态后再定"。
>
> | 档 | 内容 | 状态 |
> |---|---|---|
> | P1 | 脚本语义未覆盖面 10 项（数值/weakref+GC/词法/blob/io/异常/属性/suspend/线程×GC/序列化） | ✅ 三·补9 ~ 三·补18，语料 57 → 69 个文件 |
> | P2 | 宿主 API 补齐（member handle / `getfunctioninfo` / `setfreevariable` + 公开面） | ✅ 三·补19，顺带修一个 panic 与一个"release hook 从不触发" |
> | P3 | 健壮性加固（dump 限深 / `--mem-limit` / `--max-ops` / 压缩器嵌套上限可调） | ✅ 三·补20，全部实测有效 |
> | P4 | 文档（README 对齐实测 + 迁移指南 `MIGRATION.md`） | ✅ 本节之后 |
> | P5 | string 库全局函数语义（**新发现的一整族未覆盖面**） | ✅ 见下 |
> | P6 | math 库定义域/边界（**验证型：逐位一致，无需修改**） | ✅ 见下 |
> | P7 | base 库零覆盖全局（`assert`/`error`/`array`/`type`/`dummy`） | ✅ 见下 |
> | P8 | 随机差分模糊测试（结构化生成 + 语料变异，自动收缩） | ✅ 见下 |
> | P9 | stream/blob「位置状态 × API」矩阵（1150 条，验证型） | ✅ 见下 |
> | P10 | 验证入口入库 + 语料卫生（写文件审计）+ 模糊测试放大到 1000 | ✅ 见下 |
> | P11 | 线程/生成器「状态 × API」矩阵（162 条）+ 把一处**安全偏离**测清楚 | ✅ 见下 |
> | P12 | 序列化「值形状」矩阵（67 条）+ 把"默认参数会丢"的范围测准 | ✅ 见下 |
> | P13 | 命令行/宿主行为矩阵（34 条）+ 3 处 CLI 差异修复 | ✅ 见下 |
> | P14 | 文件句柄共享模式对齐 + 发现后端注册表只增不减（**已修**） | ✅ 见下 |
> | P15 | 3000 种子扫描的 3 处差异线索（`pow` NaN 符号 / `file.seek` 32 位截断 / `getstackinfos` 负 level） | ✅ 见下 |
> | P16 | 字符串 → 数字（`tointeger`/`tofloat` 的解析路径） | ✅ 见下 |
> | P17 | 数值字面量（词法器路径，与 `tointeger` **不是**同一条路） | ✅ 见下 |
> | P18 | 容器回调乱来（比较器返回值 / 回调形参个数 / 回调抛异常） | ✅ 见下 |
> | P19 | 索引值边界 + 容器分配上限（健壮性缺口） | ✅ 见下 |
> | P20 | 委托链动力学 + foreach 期间容器变更（验证型） | ✅ 见下 |
> | P21 | 生成器/线程 × 挂起-恢复 × 异常交错（验证型 + 1 个参考 UB 归档） | ✅ 见下 |
> | P22 | 类属性：时序/共享/继承边界（验证型） | ✅ 见下 |
> | P23 | 编译期折叠 / switch 值形态 / 链式赋值（真 bug：DMOVE 顺序） | ✅ 见下 |
> | P24 | 错误转储 × 复杂帧（线程/生成器/原生回调边界，验证型） | ✅ 见下 |
> | P25 | 表键身份 + 循环变量闭包捕获（验证型） | ✅ 见下 |
> | P26 | clone 深度/共享 + getstackinfos locals 内容（验证型） | ✅ 见下 |
> | P27 | 高位字节 × 字符串库（**参考 UB 沼泽**，不做语料、只划边界） | ✅ 见下 |
> | P28 | bindenv × 编译闭包（**未修的引擎分歧**，已精确记录） | ✅ 见下 |
> | P29 | bindenv 分歧的**根因收敛**：参考侧行为随调用结构漂移，判定不复刻 | ✅ 见下 |
> | P30 | 表迭代顺序 × 随机变更序列（生成式差分 80 程序零差异，验证型） | ✅ 见下 |
> | P31 | dofile/loadfile 边界矩阵（验证型） | ✅ 见下 |
> | P32 | **file 文本模式翻译**（真 bug：`\n` 不落盘 `\r\n`）+ b+t 拒绝 | ✅ 见下 |
> | P33 | compilestring/dofile 诊断归属（验证型） | ✅ 见下 |
> | P33 | compilestring/dofile 诊断归属（验证型） | ✅ 见下 |
> | P29 | bindenv 分歧的**根因收敛**：参考侧行为随调用结构漂移，判定不复刻 | ✅ 见下 |
> | P0 | **C ABI**（141 个 `sq_*` + 33 个 `sqstd_*`） | ⏸ 等确认：只用命令行→不需要；要嵌 C/C++ 宿主→主线 |
>
> 基线：语料 **74/74**、samples **17/17**、编译错误对拍 **60/60**、
> `.cnut` **74/74**（字节一致 + 双向交叉执行）、opcode **61/61**、`cargo test` **143/143**、lint 0。
>
> **一条命令跑完**：`python tools/check_all.py`（13 项，任一项失败退出码非 0；`--quick` 跳过 cargo test）。

### P14：后端注册表只增不减（**已修**）+ 两个后续子问题

**已修（本轮）**：`stream.rs` 的 `BACKENDS`（thread-local `Vec<Option<BackendRef>>`）原来
**只增不减** —— `register_backend` 只 push，没有任何地方置 `None`，于是**每个创建过的
file/blob/stream 后端都被永久持有**：没显式 `close()` 的实例活到进程结束、文件句柄一直开着
（无界增长，属健壮性项）。

修法：实例上挂一个 userdata 成员 `__be`，其 release hook 在**实例死亡时**清掉注册表槽位
（`bind_backend_inst`；并做**指针身份校验**，避免跨线程 sid 碰撞误清别人的槽）。
验证：2000 个 blob 的循环里释放钩子触发 **2004 次**（全部释放 ✓）；`t_streammap.nut`
（上千个 file 实例、从不 GC）在打开 share-delete 限制的前提下依然全绿 —— 说明不是把泄漏藏起来。

**踩到的坑（值得记）**：`SQInstance::set` 对**类里没声明的键**直接返回 `false`
（Squirrel 语义），所以 `__be` 必须像 `__sid` 一样先 `new_slot` 声明成类字段；
不声明时 userdata 只被临时引用持有、立刻析构 → 钩子提前清槽 → 刚建出来的 blob/file
全变成 `the stream is invalid`。

**子问题 1（已修）**：不可达实例的句柄何时释放 —— 见上，现在与参考实现一致。

**子问题 2（已定位到具体形态，待修）**：文件句柄共享模式已**对齐**（`share_mode(FILE_SHARE_READ|WRITE)`，
滞留修复之后 12 项回归全绿）。但顺带发现一处更深的差异：**尾调用会提前释放当前帧的局部**。

    函数里 `local a = file(p,"wb+"); return try_remove(p);`      ← `return f(x)` 是尾调用
      我们 → remove=ok（句柄已关），参考 → remove() failed（句柄还开着）
    改成 `local r = try_remove(p); return r;`（非尾调用）→ 两边一致（都 failed）
    同一函数里再插一句 `a.len()` 或 `a.tell()`，两边也一致

已排除的：`enter_frame` 的 tailcall 分支**没有**清任何槽（只更新 stackbase/top 与 ncalls）；
参考实现的 `_OP_TAILCALL` 用的基址也是 `_stackbase`（和我们一样），参数也同样是搬到自己槽位。
所以释放点还在别处（`start_call`/`close_outers`/`last_top` 的某一步），下一步继续查。
语料 `t_file.nut` 刻意回避这一格（用非尾调用写法），避免把已知差异混进语料。

**方法论坑（差点又误判一次）**：想用"在 release hook 里打点"定位释放时机时，把打点从 stderr
改成 stdout 也**看不出顺序** —— 脚本的 `print` 走我们自己的 print 回调、Rust 的 `println!`
走标准库缓冲，两者交错不可靠，于是"释放发生在尾调用之前"这个推断作废。下次要么两边用同一个
出口，要么改用**纯脚本可观测**的判别（例如同时检查句柄是否还活着）。

**时间盒**：这一格已投入两轮、边际收益下降，本轮记清楚后换别的工作（补了共享模式的 Rust
守门测试）；`tools/fuzz_diff.py --mutate --seeds 3000` 在后台跑，下轮按它的结果决定优先级。

**3000 种子扫描结果（下一轮的线索）**：2430 ok / **3 diff** / 2 ref-ub / 558 语法被改坏 /
1 known-dev / 4 timeout / 2 ref-limit。三处 diff 的复现文件已存好（`tmp-c2/fuzz/fail_*.nut`）：
- `seed=2057`（来自 `t_stkinfo.nut`）：**0 行不同但行数 8 vs 14** —— 参考实现中途出错、少打了几行；
- ~~`seed=2554`（来自 `t_math.nut`）：1 行不同~~ → **已修**（P15-1，`pow` 的 NaN 符号规则）；
- ~~`seed=2930`（来自 `t_streammap.nut`）：1 行不同 —— 很可能就是上面那个尾调用格~~
  → **已修**（P15-2，`file.seek` 的 32 位偏移截断），**不是**尾调用格。

#### P15 完成情况（3000 种子扫描的 3 处线索）

| # | 项 | 结果 |
|---|---|---|
| P15-1 | **`pow` 的 NaN 符号** | 参考实现 `(SQFloat)pow((double),(double))` 在**指数是奇整数**时清掉底数 NaN 的符号位 → `pow(0.0/0.0, -1.0)` 是 `nan`（x86 上 `0.0/0.0` 是**负** NaN，平时看到的是 `-nan(ind)`）；纯 f32 的 `powf` 不清符号。已按规则复刻（f32 最大奇整数 `2^24-1=16777215` 为界；`16777217` 被舍入成 `2^24`，是偶数）。探针 4 个（正/负 NaN 底数 × 22 个指数，两个方向都验过）全部一致；语料新增 `sgn_*` 节 **203 条**（`pow` 符号 2×24 + 底数×NaN 指数 8 + `atan2` 12×12 全交叉），`math_semantics.rs` 加守门测试。**`atan2` 的 NaN/inf 传播本来就一致**，无需改 |
| P15-2 | **`file.seek` 的偏移是 32 位** | `sqstd_fseek` 写的是 `fseek(file,(long)offset,…)`，Windows 上 `long`=32 位 → 偏移**先截成 int32** 再传给 CRT：`seek(2^32+5)`→`5`、`seek(INT64_MAX)`→`-1`（低 32 位是 `0xFFFFFFFF` → CRT 失败）、`seek(INT64_MIN)`→`0`、`seek(2^31)`→`-1`。已逐字复刻（`offset as i32 as SQInteger`）；**`blob` 的 seek 不截断**（它不走 CRT）。语料 `t_file.nut` 用 `trunc_*` 10 条钉住 |
| P15-3 | `seed=2057`：`getstackinfos(INT64_MIN)` 的**负 level 越界读** | 参考实现拿到垃圾表继续往下跑（8 vs 14 行），属**已记录的偏离**（我们给可捕获错误），归为 known-dev，不进语料 |

#### P16 完成情况（字符串 → 数字）

**为什么这是结构性空白**：`t_delmatrix.nut` 里 `tointeger`/`tofloat` 的 106/108 个用例
**全是参数掩码测试**（`("").tointeger("s")` 之类），"字符串怎么被解析"一次都没测到，
`global_coverage.py` 也看不见（它只数"函数有没有被调用过"）。于是
`sqbaselib.cpp::str2num` 这条路径整块没被覆盖。

| # | 项 | 结果 |
|---|---|---|
| P16-1 | **整数路径溢出**（真实差异，已修） | 参考实现是 MSVC `_strtoi64`：**按符号饱和** —— `"9223372036854775808"`/`"18446744073709551616"`/`"999…9"` → `INT64_MAX`，`"-9223372036854775809"`/`"-18446744073709551616"` → `INT64_MIN`。我们原来是 `wrapping_mul/add` + `wrapping_neg`（回绕），同一个脚本会拿到完全不同的整数。**`tofloat` 也走这条路径**（字符串里没有 `.`/`e` 就先当整数），所以 `"99999999999999999999999999".tofloat()` 也从 `-2.53e18` 变成 `9.2233720368547758e+18` |
| P16-2 | **NUL 截断**（已修） | `str2num` 决定走整数还是浮点的扫描是 `while((c=*e) != '\0')` —— C 字符串。我们原来遍历整个字节切片，于是 `"12\x00.5"` 我们走浮点、参考走整数。改成先按第一个 NUL 截断 |
| P16-3 | **`strtod` 的十六进制浮点 / inf / nan**（已修） | 手写解析器原来在 `"0x1.8p3"` / `"inf.0"` / `"nan.0"` 上报 `cannot convert the string`，参考实现给出 12 / inf / nan。补了 C99 十六进制浮点（`p` 指数可缺可残缺 —— MSVC 的超集语义）、`inf`/`infinity`/`nan` 前缀，按就近舍入 + sticky 位实现 |
| P16-4 | **NaN 的位模式**（已修） | 参考实现的 `strtod("nan")` 给的是**全 1 尾数** NaN（f32 `0x7FFFFFFF`/`0xFFFFFFFF`），打印 `nan`/`-nan`；而 `0.0/0.0` 的 x86 indefinite 是 `0xFFC00000`，打印 `-nan(ind)`。用 `blob.writen(v,'f')` 取出 4 字节才看清（`tmp-c2/probe_nanbits.nut`），现按位复刻 |
| P16-5 | **`base` 参数** | 语料原本只测过"第 2 参类型不对"，没测过**进制**。补了 base 0/2..36 的矩阵（`iseintbase = base > 13` 让 `"1e3".tointeger(16)`=483、base 14 时 `'e'` 又变成非法数字 → `1`）。**base 1/负数/>36 会让参考实现崩溃**（实测 exit=0xC0000409），属参考实现自身 UB，不进语料，本实现给可捕获错误 |
| P16-6 | 新语料 `tests-corpus/t_tonummap.nut` | `tools/gen_tonummatrix.py`：154 条手工边界 + 1000 条定种子随机串 × 3 种观测（`tointeger`/`tofloat`/两者 `typeof`）+ 35×11 的 base 矩阵，**3859 行**，两侧逐字节一致。另用 `tmp-c2/bigtonum.py` 做了 8250 个随机串 / 16501 行的一次性大样本差分：0 差异 |

**方法论收获**：`global_coverage.py` 只能发现"函数从没被调用过"，发现不了"函数被调用了、
但**输入形状**从没被覆盖过"。这一轮的两个差异（P15 的 `pow`、P16 的字符串解析）
都是这一类的第二层。

#### P17 完成情况（数值字面量）

`t_lexmatrix.nut` 覆盖的是字符串转义 / Unicode / 原样串 / 优先级，**数值字面量一个都没有**；
`errmsg_matrix.py` 的 60 条只对拍编译错误文本、不看值。于是 `sqlexer.cpp::ReadNumber`
这条与 `"…".tointeger()` **完全不同**的代码路径也是空白。

| # | 项 | 结果 |
|---|---|---|
| P17-1 | **`1.2.3` / `1..2` 被词法器吃成同一个浮点 token**（真实差异，已修） | `ReadNumber` 的循环条件是 `'.' \|\| isdigit \|\| isexponent`，所以第二个 `.` 也进 `_longstr`，取值走 `scstrtod` → **最长合法前缀**：`1.2.3` = **1.2**、`1..2` = **1**。我们的 `strtod_compat` 是"整体 `parse::<f64>()` 失败就退化成 0.0"，于是 `print(1.2.3)` 参考给 `1.2`、我们给 **0**。改成复用 `baselib::parse_dec_float`（同样是"只消费合法前缀"的语义），并加词法单测 |
| P17-2 | 与 `tointeger` 的**语义分叉**（验证型，已进迁移文档） | 同一个数字写成字面量和写成字符串结果**不同**：`99999999999999999999999999` 字面量**回绕**（`-2537764290115403777`），字符串**饱和**（`INT64_MAX`）；十六进制字面量同样回绕，且 >16 位报 `too many digits for an Hex number`（`0xffffffffffffffff` = **-1**）；八进制没有 `0b`/`0o` 前缀；`.5` 不是合法字面量（报 `expression expected`）。这些与参考**本来一致**，属覆盖型收获 |
| P17-3 | 新语料 `tests-corpus/t_litmap.nut` | `tools/gen_litmatrix.py`：221 条手工边界 + 680 条定种子随机（十进制/十六进制/八进制/浮点/残缺指数）× `compilestring("return <lit>;", "litmap")` + try/catch，**889 行**；另有 13 条直接写在脚本里的字面量与 8 条表达式/容器键/比较上下文。两侧逐字节一致 |
| P17-4 | **假绿的教训** | 先用一次性脚本 `tmp-c2/lit_diff.py` 扫了 76 条字面量，报 "0 diff" —— 实际是**模板里的 `%%` 没被 `.format()` 还原**，浮点全被打成同一个字符串，把 `1.2.3` 的差异整个盖住了。语料生成器用的是正确的 `obs()`，一跑就抓到。**结论：一次性脚本自己也要有"输出确实有区分度"的自检**，否则"0 diff"毫无意义 |

**收尾说明**：P15–P17 之后暂停"结构性未覆盖面"的主动扫描，进入定版归档（用户决定）。

#### P18 完成情况（容器回调乱来）

**为什么是结构性空白**：`t_delmatrix.nut` 覆盖"方法 × 接收者 × **实参类型**"，
`t_meta3.nut` 覆盖"元方法抛异常"，但没有任何用例让**回调自己乱来** ——
`grep` 全语料"抛异常的回调"= **0 条**。这一族按现有生成器的结构碰不到：它需要把
"回调的**行为**"当输入维度，而不是"实参的类型"。

| # | 项 | 结果 |
|---|---|---|
| P18-1 | **比较器返回值只收 Integer**（真实差异，已修） | 参考实现走 `sq_getinteger`：`sq_isnumeric` 掩码**包含 float**（值取 `tointeger(o)` = `(SQInteger)` 截断），bool 另有分支给 0/1。我们只收 `OT_INTEGER`，于是 **`a.sort(function(x, y) { return x - y; })` 在浮点数组上直接报错** —— 这是脚本里最常见的比较器写法。已按 `c_f32_to_i64`（= cvttss2si）复刻截断 |
| P18-2 | **`map`/`apply` 的回调只支持 1~3 个形参**（真实差异，已修） | `__map_array` 每轮只压 `2 + (nArgs>=3) + (nArgs>=4)` 个值，调用按 `GetUp(-(nArgs+1))` 取目标；只有 `nArgs == pushed` 时目标才是闭包自己。0 形参（nArgs=1）→ 去调刚压进去的 `src`；4 个及以上形参（nArgs>=5）→ 去调闭包**下面**那一槽（在 `array_map`/`array_apply` 帧里同样是 `this`）→ 两种情况都报 `attempt to call 'array'`。我们原来是"乱读一个栈槽还成功"。**空数组不进循环 → 不报错**这条也一并复刻 |
| P18-3 | 回调实参形状（验证型，已进迁移文档） | `map`/`apply` 给 **(值, 下标, 数组)**、`filter` 给 **(下标, 值)**（顺序相反）、`reduce` 给 `(累加值, 当前值)`，`this` 都是容器；**实参个数由闭包声明的形参个数决定**（含 `this`）。另外 `array.find(x)` 是"找**值**返回下标"、根本不调回调；`table.map` 返回 **array**；`table.keys(cb)` 的回调参数个数永远对不上 |
| P18-4 | 新语料 + 守门测试 | `tools/gen_cbmatrix.py` → `tests-corpus/t_cbmap.nut`（111 条：比较器返回值 21 种 + 浮点数组/稳定性/形参个数/`this`；`map`/`apply`/`filter`/`reduce`/`find` × 形参 0~5；回调抛异常 9 条；非函数回调 6 条）。守门测试 `crates/squirrel-stdlib/tests/callback_semantics.rs`（3 个）。**观测纪律**：一律 `dump()` 打印内容，绝不直接 print 容器 —— 指针文本两侧本来就不同，一次性脚本里已经因此踩过一次假绿 |
| P18-5 | 不测的（参考实现自身 UB） | 比较器返回 **NaN / ±inf**：`(SQInteger)((SQFloat)o)` 是 UB（本机上两侧**碰巧**都给 INT64_MIN → `inconsistent compare function`，但那是巧合）；排序过程中增删数组元素（`_sort_compare` 的检查发生在 `sq_call` 之后，指针可能已失效）。两者都不进语料 |

#### P19 完成情况（索引值边界 + 容器分配上限）

**起因**：扫"方法 × **值**形状"这一族（`t_delmatrix` 只测过实参**类型**，双参数组合、
INT64 极值、浮点索引、find 起始偏移都是空白）。索引语义本身**两侧一致**（验证型，
118 条探针逐字节一致）；但顺着"还能传什么值"往下探，挖出了**我们自己的健壮性缺口**。

| # | 项 | 结果 |
|---|---|---|
| P19-1 | **超大一次分配不受限**（**本实现的 bug**，已修） | `array(9223372036854775807)` / `array(1).resize(0x7FFFFFFF)` → Rust Vec 容量溢出 **panic**（参考实现同样输入是 `0xC0000005` 崩溃）；`resize(2147483647)`/`resize(4294967296)` → 分配十几 GB **挂死**（参考实现把 Windows 错误报告挂到整机死锁，用户还得点掉弹窗）。修法：`config.rs` 新增 `MAX_CONTAINER_BYTES`（2 GiB）+ `container_fits()`，接到 `base_array`/`array_resize`/`blob` 构造与 `resize`/`readblob`/blob `write` 增长路径 —— 超限一律**可捕获**的 `container too big`，合法尺寸（`array(100000,7)`）不受影响 |
| P19-2 | `blob(4).readblob(-1)` | 参考实现把负尺寸转无符号后巨型分配**挂死**（逐条隔离实测）；本实现夹 0 → `no data left to read`。归入有意偏离 |
| P19-3 | 索引值语义（验证型） | `slice` 负数/夹取/start>end→空、`find` 偏移夹取、`insert` 允许 `idx==len`、`remove` 不允许、`resize` 负值错误、浮点截断 —— 两侧本来一致；blob 的 seek 越界/写入增长（`seek(4)`+写→len 5）也一致 |
| P19-4 | 新语料 + 守门测试 | `tools/gen_idxmatrix.py` → `tests-corpus/t_idxmap.nut`（**128 条**：slice 双参 18 组 ×2 + 单参极值 ×2 + find 偏移 13 + insert/remove ×2 + resize 安全值 ×2 + 下标极值 + blob seek/增长/readblob）。`robustness.rs` 加 `container_allocations_are_bounded_and_catchable`（9 条超限必须 `container too big`、3 条负值保持原语义、3 条合法尺寸照常，且测试必须**秒回**——守卫在分配前拒绝） |
| P19-5 | 探针纪律的再确认 | 参考实现跑危险值会**整机挂死**（WER 弹窗阻塞）——必须逐条隔离 + 每条带超时（`Start-Process` + `WaitForExit`），绝不能整文件跑。这轮已经被弹窗坑过一次 |

#### P20 完成情况（委托链动力学 + foreach 期间容器变更）

两族都是现有生成器结构性地碰不到的：`t_delmatrix` 测"方法 × 实参类型"、
`t_meta3` 测"单个元方法"，链与迭代的**动力学**（深度/循环/跨类型/raw*；
增/删/缩/清期间迭代）没有用例。**验证型轮**：探针 40 + 20 条两侧逐字节
一致（含 12 种循环委托形态、foreach-mutation 逐条隔离跑通）。

| # | 项 | 结果 |
|---|---|---|
| P20-1 | **循环委托在赋值时被拒** | `setdelegate` 的环检测发生在**赋值当下**（`b.setdelegate(a)` 在 `a` 已挂 `b` 时抛 `delegate cycle`），不是查找时才发现；自挂/三角环同理。语料夹具曾试图在顶层造环，脚本直接死——顺带把这条语义钉死了 |
| P20-2 | raw*/`<-`/`delete` 与链的关系 | `rawget/rawin/rawset` 不穿链；`<-` 挂**接收者自身**；**`delete` 也不穿链**（穿链要靠 `_delslot`）——最后这条初版期望就写反了，靠差分语料纠正 |
| P20-3 | foreach 期间的容器变更 | 表：删当前键计数照旧、删别的键**跳过其后**、`clear()` 即停；数组：`append` **延长**迭代、`remove`/缩容**提前结束**、变长按旧长度走。参考实现全程不崩（逐条隔离验证），结果逐字节一致 |
| P20-4 | 新语料 + 守门测试 | `tools/gen_delegiter.py` → `tests-corpus/t_delegiter.nut`（**59 条**）；`deleg_iter_semantics.rs`（3 个测试：赋值时环检测 / 链上查找与可见性 / 迭代期变更） |
| P20-5 | 工具坑（再记） | ① `base` 是关键字不能当变量名；② class 不能写成表达式当实参；③ PowerShell 往返改 .nut 会把中文注释搞成乱码（Get-Content 默认编码）——修 .nut 一律用 write 工具 |

#### P21 完成情况（生成器/线程 × 挂起-恢复 × 异常交错）

`t_suspmap`（16 条）/`t_threadmap`（162 条）测的是状态机主干；"**异常与挂起交错**"
（yield 在 try 里、恢复点在 try 里、死态复活、wakeupthrow 注入、错误后重启…）
没有用例。**验证型轮**：探针 34 + 7 条两侧逐字节一致，无引擎差异。

| # | 项 | 结果 |
|---|---|---|
| P21-1 | 生成器死态与异常交错（验证型） | 死后 `resume` 一律 `resuming dead generator`（跑完/抛错两种死法）；体内抛错传到**恢复点**的 catch；`yield` 在 try 里 + 体内 throw → 走生成器**自己的** catch（还能连续 yield）；**`yield` 是语句不是表达式**（`local a = yield x` 编译错）；含 yield 的普通函数自动成为生成器 |
| P21-2 | 线程状态与注入（验证型） | 状态串 `idle/suspended/…`——**没有 "dead"**，跑完回 idle 且**可再 `call()` 重跑**；`wakeup`/`wakeupthrow` 打在 idle 报 `cannot wakeup a idle thread`；**`wakeupthrow` 注入的错误被挂起点外层的 try 接住**；`wakeup` 无参 → suspend 表达式得 `null`、多参忽略；挂起线程 `getstackinfos` → null；**主线程 `suspend()` 两侧同样静默退出**（try 接不住，一致） |
| P21-3 | **`newthread(<原生闭包>)`**（参考 UB，归档） | 参考实现**创建时即崩溃**（实测 WER 弹窗 ×3、无输出；逐条隔离见 `tmp-c2/gt_isolate.py`）。本实现正常创建、调用报 `wrong number of parameters` —— 安全超集，已记 MIGRATION 偏离表并有守门测试钉住 |
| P21-4 | 新语料 + 守门测试 | `tools/gen_gtmatrix.py` → `tests-corpus/t_gtmap.nut`（**37 条**，不含两个会中止/挂死参考实现的形态）；`gt_semantics.rs`（3 个测试：生成器生命周期与交错 / 线程 wakeup 与注入 / newthread 原生闭包的安全偏离） |

#### P22 完成情况（类属性：时序/共享/继承边界）

`t_attrmap`（40 条）测的是**形态**（`</ ... />` 写法、基本 get/set、可见性）；
本轮补**动力学**：实例化后改属性、继承后改父类、给继承来的成员 set/get、
属性表共享引用、newmember 4 参、clone 边界、参数个数。**验证型**：30 条探针
两侧逐字节一致，无引擎差异。

| # | 项 | 结果 |
|---|---|---|
| P22-1 | 时序与继承（验证型） | 属性挂类上、实例不快照（`setattributes` 后实例透过 `getclass()` 立即可见）；**继承后改父类属性子类不受影响**（extend 拷贝）；父类成员属性在子类**可读可设**；三层继承看最近的重写层；**类级属性（null 键）不随 extends 继承** |
| P22-2 | 属性表共享引用（验证型） | `getattributes("v").a.n = 42` 对后续读取可见；给属性表**加新键** `x.same = "yes"` 报 `the index does not exist` —— 这是核心语义"普通 `=` 不建新槽"，不是属性特有（初看以为是属性实现差异，隔离后确认是核心语义） |
| P22-3 | 边界形态（验证型） | `newmember` 4 参 `(name, val, attrs, isstatic)`；`clone` 一个 **class** 报 `cloning a class`；实例没有 `setattributes/getattributes`；`delete C.v` 报 `attempt to delete a slot from a class`；`rawset` 新成员属性 null；`_tostring` 挂属性后 `getattributes("_tostring")` 报 `wrong index`（参考如此）；两个方法的参数个数都是死规矩 |
| P22-4 | 新语料 + 守门测试 | `tools/gen_attrtiming.py` → `tests-corpus/t_attrtiming.nut`（**28 条**，独立自洽）；`attributes_semantics.rs` 追加 `attribute_timing_sharing_and_inheritance`（24 条断言整段对拍） |
| P22-5 | 顺带发现的既有隐患 | `tools/gen_attrmap.py` 的用例源是 **gitignored 的 `tmp-c2/attrprobe.nut`**——本地丢了生成器就废。本轮没动它（避免节外生枝），已在此记录 |

#### P23 完成情况（编译期折叠 / switch 值形态 / 链式赋值）

`t_ops`（4533 条）测的是**运行时**运算符 × 类型；编译期折叠边界、switch 值形态、
短路求值顺序、赋值作表达式都没有用例。探针 55 条抓到 **1 个真 bug**。

| # | 项 | 结果 |
|---|---|---|
| P23-1 | **DMOVE 执行顺序**（**真 bug**，已修） | 窥孔把连续两条 MOVE 合并成一条 `DMOVE(d0←s0, d1←s1)`；参考实现**严格顺序**执行（`STK(arg0)=STK(arg1); STK(arg2)=STK(arg3);`）。链式赋值 `x = y = z` 恰好是 `DMOVE(y←z, x←y)`——arg3 与 arg1 **同一寄存器**，第二读必须看到第一写。我们把两个源先快照再写（并行语义），链式赋值的外层拿到**旧值**：`local x=0;local y=0;local z=3; x=y=z;` 参考给 x=3、我们给 **x=0**。RHS 是字面量时碰巧没事（常量直接 Load），所以一直没暴露。定位路径：差分 → 逐条二分（`constant too long` 是 **`'..'` 字符字面量**的报错，绕开）→ `tests/dump_chain.rs` 转储字节码发现 DMOVE 参数正确、执行错误 → vm.rs |
| P23-2 | switch 值形态（验证型） | case 标签可以是**任意表达式**（含变量）；重复 case **先到先得**；浮点/整数 case 交叉匹配；**case 在 default 之后是语法错**（`expected '}'`，default 在前合法）；switch 里的 `break` 归 switch；case 体可声明 local；`case null`/`case true` 都行 |
| P23-3 | 编译期折叠边界（验证型） | `7 % 0` → **7**（% 被编译期折叠，早前已记录）；整数 `/` **不**折叠（运行时 `division by zero`）；浮点折叠 `1.0/0.0`→inf、`1.0%0.0`→nan；INT64 字面量折叠**回绕**；`1<<64`→1、`1<<-1`→INT64_MIN |
| P23-4 | `&&`/`||` 返回操作数值本身（验证型） | `true && 2`→2、`"" && 5`→5（`""` 真值）、`false && side()` **不调用** side；`1 && 2 \|\| 3 && 4` → 2 且右半整体被短路。守门测试用 `side()` 打点 + 出现次数断言精确验证"谁被调用了几次" |
| P23-5 | 新语料 + 守门测试 | `tools/gen_evalmatrix.py` → `tests-corpus/t_evalmap.nut`（**50 条**）；`eval_semantics.rs`（3 个测试：链式赋值 / 逻辑操作符与短路 / switch 与折叠）。调试用转储测试 `dump_chain.rs` 用完即删 |

#### P24 完成情况（错误转储 × 复杂帧）

`t_errdump1-3` 各自只有**一个**顶层 throw 的转储（第一个抛错就中止脚本，一个文件
只能钉一条）。本轮钉的是**错误跨越边界时**的转储行为。**验证型**：16 + 6 条探针
（逐条隔离，stdout + stderr + 退出码三方对拍）两侧逐字节一致，无引擎差异。

| # | 项 | 结果 |
|---|---|---|
| P24-1 | **转储只在跨界时打印**（验证型） | 错误跨越**线程边界**（含 wakeupthrow 注入、生成器在线程内抛错）与**原生调用边界**（sort/map 比较器回调）时：即使外层 try/catch 接住，stderr 照样打印 `AN ERROR HAS OCCURRED + CALLSTACK + LOCALS`（含脚本相对路径），脚本**继续跑完**；纯脚本帧之间被 catch → **stderr 干净** |
| P24-2 | `_get` 元方法的名字解析递归（验证型） | 委托链的 `_get` 调用外部 local 函数 `boom4(k)`，而 `boom4` 的解析又落回委托链 → 无限递归 → `Native stack overflow`，**两侧一致**（连被 catch 后继续跑都一致），照实入语料 |
| P24-3 | 其余 14 种帧形态（验证型） | 生成器抛错（纯脚本帧，无转储）、rethrow、嵌套 try、多 local、构造器抛错、`_nexti` 抛错、varargs、default 参数表达式抛错、具名 `compilestring` 源（转储中的源名一致）、深层调用链——全部一致 |
| P24-4 | 新语料 + 守门测试 | `tools/gen_errdumpframes.py` → `tests-corpus/t_errdumpframes.nut`（**13 条**：5 条跨界转储 + 8 条脚本帧无转储，脚本不中止、有 DONE 标记）；`exception_semantics.rs` 追加 `stderr_dump_applies_at_thread_and_native_boundaries` |
| P24-5 | 工具提示 | `tools/safe_run.py` 里已有 `suppress_crash_dialogs()`（corpus_health 在用）——以后给参考实现喂危险值的探针应该复用它，而不是手动点弹窗 |

#### P27 完成情况（高位字节 × 字符串库：参考 UB 沼泽，只划边界不做语料）

探针目标：字符串库（toupper/tolower/escape/slice/find/比较/排序/format/表键）×
**高位字节内容**。结论：**这一族在参考实现上是 UB 沼泽，不做差分语料**——
但把边界划清楚了。

| # | 项 | 结果 |
|---|---|---|
| P27-1 | **`toupper`/`tolower` 高位字节 = 参考 UB** | 参考实现把 `(signed char)` 高位字节当负数喂 CRT `toupper`（UB）：输出乱码碎片，甚至夹带 `std_stream` 相关错误。本实现 `to_ascii_uppercase`（高位字节原样保留、ASCII 正常映射）——**确定性输出**，记为有意偏离，不进语料 |
| P27-2 | **`escape` 越界读污染后续输出**（早前已归档的偏离，本轮补实证） | 同一个字符串 `"\xff"`：字面量直写、经变量、放在不同长度的输出里，参考的 escape 输出形态**各不相同**（`\xff`/`\xffff`/`\xffffffff`…）——越界读把**之前 stdout 的碎片**漏进来了。本实现的确定性 32 位补码形态即早前归档的偏离 |
| P27-3 | 安全子集（验证型，与参考一致） | **比较是逐字节 memcmp**（`"\xff" <=> "\xffx"` = −1、前缀短者小、`"\xc1" <=> "\xdc"` 按字节）；高位字节字符串**可以当表键**（按内容哈希，`("\xff" in x)` 为 true）；`len` 正常计数 |
| P27-4 | 方法论 | 一旦某族的输出要经过"参考实现 UB 的函数"，**后续所有输出都不可信**（越界读会串行）——这类族只能划边界 + 文档，不能造语料。判别方法：同一输入分别以"字面量/变量/不同长度"三种形态跑，输出形态不同即 UB 实锤 |
| P28-4 | 附带发现 | `compilestring("return function() { yield 1; }")` 产出的**yield 函数不自动变生成器**：`resume` 报 `trying to resume a 'function',only genenerator can be resumed`（两侧一致）——对比：直接 `function() { yield 1; }` 字面量调用就是生成器。已入语料 `cs_gen` |

#### P29 完成情况（bindenv 分歧的根因收敛：判定不复刻）

带着 P28 的复现去修，先做了**根因收敛**，结论是**不复刻**——把精确事实补进
MIGRATION（新增"参考侧行为随调用结构漂移"一行）。

| # | 项 | 结果 |
|---|---|---|
| P29-1 | 读参考源码 | `StartCall` 对 `_env` 的处理是**无条件覆盖** `this`（`sqvm.cpp:422`，脚本/原生闭包共用）——理论上 bindenv 应始终生效 |
| P29-2 | **参考侧行为随调用结构漂移** | 同一段 `compilestring("return this.marker") + bindenv(env)`：写在**具名顶层函数**体内或**顶层语句**里 → 参考正常 `env-ok`；写在**匿名闭包实参**里（`s("x", function() { … })` 形态）→ 参考稳定报 `the index 'marker' does not exist`（同文件重跑可复现）。对照文件：`tmp-c2/bisect_bind.nut`（4 变体全一致）、`tmp-c2/bind_ctx.nut`（同进程对照）、`tmp-c2/c_exact.nut`（失败形态）。**结论：参考的这条路径依赖帧布局/调用结构**（内部机制无法从行为侧完全确定），忠实复刻不可行——维持 P28 的偏离记录，不再追 |
| P29-3 | 假设排除记录 | ① 不是 escape 越界读污染（本族无 escape）；② 不是"编译过自由标识符就污染"（先编译 `return marker` 再跑正常用例，参考也正常）；③ 不是 delegate 有无（有无 setdelegate 各自都有成功与失败形态）；④ 真正的判别变量是 **compilestring 的调用所在帧** |
| P29-4 | 对脚本作者的建议（已写进 MIGRATION） | 别对 **compilestring 的闭包**用 bindenv 承载变量传递——参考实现这一路径随调用结构漂移，迁移到本实现后行为只会更确定；要传环境就用**显式参数**或普通闭包（普通闭包的 bindenv 两侧一致） |

#### P30 完成情况（表迭代顺序 × 随机变更序列）

任何脚本 `foreach` 一张表，输出顺序都依赖**哈希表布局**——键的哈希函数、探测
策略、扩容时机、删除后的空洞处理，任何一处不一致都会让输出顺序漂移。这一族
用**生成式差分**压测：随机"插入/覆盖/删除"序列（混合 int/string/float/bool
键型，插入量足以触发多次扩容），foreach 输出与参考逐字节对拍。

| # | 项 | 结果 |
|---|---|---|
| P30-1 | 生成式差分 | **80 个随机程序**（单程序 10~200 操作，混合 8 类键型生成器）零差异——哈希函数、探测、扩容、空洞处理与参考完全一致 |
| P30-2 | 新语料 + 守门测试 | `tools/gen_iterorder.py` → `tests-corpus/t_iterorder.nut`（**12 条定种子序列**，每条 10~120 操作）；`key_capture_semantics.rs` 追加 `table_iteration_order_matches_reference_layout`（4 个确定性形态：40 键扩容序 / 删除偶数键后的序 / 字符串键序 / 混合键型序） |
| P30-3 | 为什么值得单开一轮 | 迭代顺序是**全语料的隐性前提**——86 个文件里凡是 `foreach` 表的输出都隐式依赖它；这轮把它从"隐式前提"变成"显式钉死"，未来任何哈希表实现调整都会立即被 12 条序列 + 4 个确定性形态暴露 |

#### P31 完成情况（dofile/loadfile 边界矩阵）

`t_loadfile_bom` 测的是**编码**（BOM/UTF-16）；本轮测**路径与错误形态**——
不存在、空串、目录作为路径、`./` 与 `\` 形态、带空格路径、文件中途抛错、
语法错文件、空文件/纯注释文件、第 2 参（printerror）语义。**验证型**：31 行
探针两侧逐字节一致（含 stderr），无引擎差异。

| # | 项 | 结果 |
|---|---|---|
| P31-1 | 错误文本统一 | 不存在/空串/**目录**作为路径，全都是 `cannot open the file`（目录不特殊报错）；**`loadfile` 打不开是抛错而非返回 null**（容易想反） |
| P31-2 | 第 2 参语义 | printerror 只控制 stderr 打印，**不影响抛错**——`false` 也照样抛 `cannot open the file`；`loadfile` 的第 2 参同理 |
| P31-3 | 穿透与副作用 | dofile 返回文件的 return 值、文件里 `<-` 的槽进**根表**、文件中途抛错**穿透**到调用方（before 打印 after 不打印）；loadfile 闭包**可重复调用**（每次完整重跑）；语法错文件的编译错误同样穿透 |
| P31-4 | 新语料 + 守门测试 | `tools/gen_dofilemap.py` → `tests-corpus/t_dofilemap.nut`（**23 条**，脚本自建自删伴生文件、无残留）；`stream_semantics.rs` 追加 `dofile_loadfile_boundaries` |

#### P32 完成情况（file 文本模式翻译：真 bug）

file 的模式串只出现过 5 种（wb+/rb/wb/w/ab）——**文本模式**（无 `b` 的所有模式）
完全没测过。file_modes 驱动（逐条隔离 + 弹窗抑制）一跑就抓到 **10/16 形态差异**。

| # | 项 | 结果 |
|---|---|---|
| P32-1 | **文本模式翻译缺失**（**真 bug**，已修） | 参考实现走 CRT 文本流：写入 `\n` 落盘 `\r\n`（`file(p,"w")` 写 97,10,98 → 磁盘 97,**13**,10,98）、读取 CRLF→LF、**遇 Ctrl+Z（0x1A）截断**（读短 → readn 报 `io error`）；seek 后读**裸字节**（绕过翻译，实测 seek(2) 读到物理 10）；`tell/len` 报**物理字节数**。我们永远二进制——写文本文件的真实脚本落盘字节与参考不同。已实现（`FileBackend` 的 `text/pushback/text_eof/raw_after_seek` 状态 + LF→CRLF 写翻译 + CRLF→LF 读翻译 + Ctrl+Z 截断 + seek 后 raw） |
| P32-2 | **`b` 与 `t` 同用**（参考崩溃 → 有意偏离） | `file(p,"wbt")`：参考 fopen 判无效参数 → `0xC0000409` 崩溃（实测 rc=3221226505）；本实现给可捕获的 `cannot open file`。`mode_is_valid` 拒绝 b+t 组合 |
| P32-3 | 新语料 + 守门测试 | `tools/gen_filemodes.py` → `tests-corpus/t_filemodes.nut`（**15 种模式** × 写入/tell/len/读回，`wbt` 按纪律剔除——参考崩溃无法对拍）；`stream_semantics.rs` 追加 `text_mode_translation_and_bt_rejection` |
| P32-4 | 驱动纪律（再记） | file_modes 驱动初版 16 用例**共享同一个探针文件**且顺序执行——追加模式的历史在两侧分叉（尤其 wbt 崩溃后），制造出假差异。修法：每用例独立文件名 + 脚本内自删；与前几轮的"逐条隔离"互补 |

#### P33 完成情况（compilestring/dofile 诊断归属：验证型）

`errmsg_matrix` 测**文件编译**的错误文本、`t_errdumpframes` 测**线程/原生边界**
的转储——**compilestring/dofile 的源名与行号归属**（多行源的运行期错误行号、
`getstackinfos(1).line` 在编译代码内的取值、嵌套 dofile 的转储源名链）没有用例。
**验证型**：12 行 stdout + 完整 stderr（含跨 dofile 嵌套的转储）两侧逐字节一致，
无引擎差异。

| # | 项 | 结果 |
|---|---|---|
| P33-1 | `getstackinfos(1).line` 在编译代码内返回**实际行号** | 多行源第 3 行 → 3；`.src` 返回 `compilestring` 给定的源名。`si_line`/`si_src` 钉住 |
| P33-2 | 编译错误的 catch 文本**不含行号** | 就是 `expression expected` 这类消息本身；行号只在 stderr 的编译处理器输出里（`行 = (N) 列 = (M)` 格式），且多行源与单行源形态一致 |
| P33-3 | **嵌套 dofile** 的转储源名链 | `d4.nut`→`dofile()`→`d3.nut`→`dofile()`→主脚本，逐层 CALLSTACK/LOCALS 逐字节一致（`df_nested` 钉住） |
| P33-4 | 新语料 + 守门测试 | `tools/gen_diagmap.py` → `tests-corpus/t_diagmap.nut`（**10 条**，脚本自建自删伴生文件）；`exception_semantics.rs` 追加 `compilestring_line_and_src_attribution` |






#### P28 完成情况（bindenv × 编译闭包：未修的引擎分歧，已精确记录）

探针目标：compilestring 诊断一致性（源名/行号经 getstackinfos 与错误传播）。
大部分一致（源名归属、行号、同名/空名/无名、嵌套编译、编译产物与线程/生成器
交互——`cs_gen` 还钉了"compilestring 产出的 yield 函数**不会**自动变生成器，
resume 报 `trying to resume a 'function'`"），但抓到 **1 个未修的引擎分歧**：

| # | 项 | 结果 |
|---|---|---|
| P28-1 | **bindenv × 编译闭包的裸自由标识符**（**未修**） | `compilestring("return marker", "X").bindenv(env)()`：参考报 `the index 'marker' does not exist`（裸标识符读 root，env 的槽看不见；`setdelegate` 有无无关——A/B/C/E 四形态复现见 `tmp-c2/bind_iso.nut`）；本实现读 env 给 `env-ok`。**普通（文件）闭包** bindenv 两侧都正常。修复需动自由标识符解析与 `_env` 调用路径，超出本轮预算——**延后**，精确复现已入 MIGRATION |
| P28-2 | `t_surface2` 的盲区（方法论） | 既有用例 `compilestring_env` 只钉了 `return this` 是 **table**——root 也是 table，**区分不了 this=env 还是 this=root**，所以这条分歧躲过了全部既有语料。教训：身份断言要用**独占槽**（如 `("marker" in this)`）而不是 typeof |
| P28-3 | 一致子集入语料 | `tools/gen_bindenv.py` → `tests-corpus/t_bindenv.nut`（**6 条**：普通闭包 bindenv、编译闭包不 bindenv 时 this=root、bindenv 返回新闭包、编译闭包读全局、bindenv(root)、this==env 判别）；本轮其余一致语义：源名归属（`getstackinfos(1).src`=给定名）、行号（多行源第 3 行抛错 `L3`、`getstackinfos(1).line`=3）、同名/空名/无名（无名源的 src 是 `unnamedbuffer`）、嵌套编译（源名 INNER/OUTER 各自正确）、编译产物与线程/生成器交互 |
| P28-4 | 附带发现 | `compilestring("return function() { yield 1; }")` 产出的**yield 函数不自动变生成器**：`resume` 报 `trying to resume a 'function',only genenerator can be resumed`（两侧一致）——对比：直接 `function() { yield 1; }` 字面量调用就是生成器。已入语料 `cs_gen` |



#### P25 完成情况（表键身份 + 循环变量闭包捕获）

`t_delmatrix` 测"方法 × 实参类型"、`t_ops` 测运算符——**表的键按什么身份区分**
与**循环变量的闭包捕获**这两族没有用例。**验证型**：31 条探针两侧逐字节一致，
无引擎差异。语义点：

| # | 项 | 结果 |
|---|---|---|
| P25-1 | **表键严格按类型区分**（验证型） | `t[1]`、`t[1.0]`、`t["1"]`、`t[true]` 是**四个不同的槽**（值相等也不互通）；`-0.0` 与 `0` 是两个键；**null 不能当键**（`null cannot be used as index`）；混合键型遍历是**哈希序**（`bool;string;float;integer;`） |
| P25-2 | 数组下标做 `tointeger`（验证型） | `a[1.0]`→`a[1]`、`a[1.9]`→`a[1]`（截断）、`a[-0.0]`→`a[0]`；`a["1"]` 报错；`a[-1] = v` 报 `the index '-1' does not exist`（普通 `=` 不建新槽，核心语义） |
| P25-3 | **循环变量的闭包捕获**（验证型） | `for`/`foreach` 的循环变量被闭包捕获时**共享同一个槽**——循环结束后所有闭包看到最终值（`3,3,3` / `30,30,30`）；**循环体内 `local j` 才逐次独立**（`0,10,20`）；捕获循环变量的生成器 resume 看到最终值；共享 `i` 的两个闭包互相可见修改（`2,102,102,202`）。要逐次独立就循环体内 `local j = i` |
| P25-4 | 新语料 + 守门测试 | `tools/gen_keymatrix.py` → `tests-corpus/t_keymap.nut`（**30 条**）；新守门测试 `key_capture_semantics.rs`（2 个：表键严格类型 / 循环捕获共享与隔离） |







**新增守门（本轮）**：`tools/check_generators.py`（已接进 check_all 成第 9 步）—— 跑一遍全部
`tools/gen_*.py`，比对内容哈希，抓"生成物被手改 / 生成器忘记重新生成"。第一次跑就抓到
`t_deep.nut` 的生成器还在产出**无限尾递归**版（会挂住两侧），已回填；另 4 个已知漂移在白名单。
两个坑：按原始字节比会被 autocrlf 的 CRLF 干扰（改成内容哈希）；检查器会重写语料（改成
跑前存字节、跑完无条件写回，现在完全无副作用）。


>
> **覆盖面扫描工具**：`tools/global_coverage.py` —— 把 62 个已注册全局函数逐个对照语料
> 调用次数，直接列出"零覆盖/低覆盖"。P7 的 5 个项就是它扫出来的（当时只有这 5 个是 0 次）。

### P12 完成情况（序列化 × 值形状）

`t_sermap.nut` 只有 25 条、测的是"能不能写/读文件"；**值本身的往返**没有系统覆盖。
新矩阵 `tools/gen_sermatrix.py` → `tests-corpus/t_sershape.nut`（67 条：值形状 56 + 传参 4 +
错误路径 7），每条都是"写出去 → 读回来 → 调用 → show(返回值) + typeof"。

| # | 项 | 结果 |
|---|---|---|
| P12-1 | 关键设计（第一版写错的地方） | 值必须是闭包里的**字面量**才参与往返；用外部 `local` 会先撞"自由变量闭包不能序列化"，**根本走不到值序列化**（探针实测 100 层/自引用那些用例全被这条挡住，是假覆盖） |
| P12-2 | 往返正确性 | 标量（含 `INT64_MIN`、`-0.0` 的符号）、字符串（含转义与高位字节）、容器（嵌套/整数键/混键）、嵌套与递归闭包、varargs、类（含继承/属性/`_tostring`）、生成器闭包、blob/regexp 全部**两侧逐字节一致** |
| P12-3 | **把"默认参数会丢"的范围测准** | 老文档只写"往返会丢默认值"；实测**只丢"被直接序列化的那个闭包自己"的**默认值（`g(1)` → `b=null`），而闭包**内部**嵌套函数的默认值（`b=100`）、它**返回出去**的内部闭包（`inner=77`）、varargs（`n=3`）全都完好 → 已改写 `MIGRATION.md` |
| P12-4 | 两处参考实现的怪错误文本（逐字复刻） | `writeclosuretofile(P, print)` → `wrong argument type, expected 'function' got 'function'`；`writeclosuretofile(P, 1)` → `... expected: 'function|function'` |
| P12-5 | 安全边界（探针实测） | 自引用结构两侧报同样的错 ✓ 可进语料；**深嵌套值到 ~10000 层时参考实现栈溢出崩溃**（`0xC00000FD`，属已记录的"深嵌套值释放"族），语料只用到几层 |
| P12-6 | 守门测试 `crates/squirrel-stdlib/tests/serialization_shapes.rs`（4 个） | `cargo test` 134 → 138。踩坑：同一文件里多个 `#[test]` 由 cargo **并行**跑，共用一个临时 `.cnut` 路径会互相覆盖（实测 `null` 用例返回了另一个用例的 120）→ 每个测试用独立文件名 |
| P12-7 | 生成器自检又抓到两个真问题 | ① `local PATH` 声明在函数之后 → 函数里解析不到（`the index 'PATH' does not exist`）→ 改用 root 表全局；② 只含 `suspend` 不含量 `yield` 的函数**不是生成器**，直接调用会把主 VM 挂起、整个脚本停在那里（DONE 打不出来）→ 换成"生成器闭包往返"用例 |
| P12-8 | **2000 个变异种子的扫描抓到并修掉一处真实差异**：`writeclosuretofile` 的**检查顺序** | 参考实现是"掩码 → **先 fopen("wb+")** → 再序列化"：路径不可写时即使第 3 参是 native closure 也报 `cannot open the file`；路径可写时文件**立刻建出**，之后无论序列化失败（自由变量）还是 native closure 类型不符，都留下一个 **0 字节文件**。我们原来是"先序列化再写文件"→ 两处错误文本都反了。改完 10 个当时失败的种子全部转 `ok`。进语料 6 条 + Rust 测试 1 个（`cargo test` 138 → 139） |
| P12-9 | 顺带修掉模糊测试的**两个环境干扰源** | ① 变异可能把写入改成追加/把读取挪到写入之前 → 两个引擎共用工作目录时，第二个引擎会读到第一个留下的文件（刷出一批"2 行不同"的**偶发假差异**）→ 改成每个引擎一个**独立且每轮清空**的目录；② 变异把字符串换成 `"abc"` 时若那处是文件路径，会在 cwd 建出文件（曾在仓库根目录留下 `abc`）→ 私有 cwd 一并挡掉 |

### P11 完成情况（线程/生成器 状态 × API）

沿用 P9 的方法论（差异常藏在**状态迁移**里）：8 个线程状态 + 4 个生成器状态 × 全部委托 API
（`call`/`wakeup`/`wakeupthrow`/`getstatus`/`getstackinfos`/`weakref`/`tostring`），
外加 10 条多步迁移序列。观测口径 **前置状态 | 返回值 | 错误 | 后置状态**。

| # | 项 | 结果 |
|---|---|---|
| P11-1 | `tools/gen_threadmatrix.py` → `tests-corpus/t_threadmap.nut`（162 条，带生成器自检） | 两侧逐字节一致（类型名归一化后，见下） |
| P11-2 | 钉住的状态机语义 | 线程只有 **idle/suspended**（跑完**回 idle**，不是 dead）→ 对跑完的线程再 `call()` 会**重跑线程体**；`wakeup()` 无值也能唤醒且**返回线程体的返回值**；`wakeupthrow` 把错误抛给**调用方**；对 idle 线程 wakeup → `cannot wakeup a idle thread`（状态不变）；生成器有 **dead** 态、未 resume 过就是 `suspended`、**没有 `getstackinfos`** |
| P11-3 | 复刻了参考实现的拼写错误 | `trying to resume a 'integer',only genenerator can be resumed`（**genenerator**）逐字一致 |
| P11-4 | **把一处安全偏离测清楚**（本轮最重要的发现） | 对已挂起线程再 `call()`，参考实现拿被挂起帧里的**残留栈槽**当函数调，那个槽正好是 `suspend(x)` 的 `x`：实测 `42`→`'integer'`、`"s"`→`'string'`、`null`→`'null'`、`[1]`→`'array'`、`{}`→`'table'`、`1.5`→`'float'`、`true`→`'bool'`。**`x` 是闭包时参考实现会把它执行掉**（探针实测副作用发生、`call()` 返回闭包的返回值）；我们恒为可捕获的 `attempt to call 'table'` 且**绝不执行残留闭包** |
| P11-5 | 守门测试 `crates/squirrel-stdlib/tests/thread_states.rs`（3 个） | 含安全性不变量 `second_call_does_not_run_stale_closure`；`cargo test` 131 → 134 |
| P11-6 | 语料侧的处理 | 把那个类型名**归一化**成 `attempt to call <type>`（只比较"是否报 attempt to call"），既保住"会不会报错"的验证，又不钉一个不稳定的字符串 |

### P10 完成情况（验证入口 + 语料卫生 + 模糊测试放大）

| # | 项 | 结果 |
|---|---|---|
| P10-1 | 验证入口入库：`tools/check_all.py` | 10 项（8 项差分/审计 + health + writes + cargo test）；原来的 `tmp-c2/final_check.py` 在 gitignore 里、不可复现，且步骤名写死数量（`corpus(69)` 早已过时）→ 改成薄壳转发 |
| P10-2 | **语料写文件审计**：`tools/audit_corpus_writes.py`（静态）+ `check_all` 里的**动态**守卫 | 静态：`t_surface2.nut` 漏删 `robust_surface3.txt`（**就是被误提交进仓库的那个**）、`t_surface5.nut`、`t_closure_io.nut` 三处补上清理。但静态审计有盲点：它只看"写之后有没有 remove"，抓不到**"remove 在一条走不到的路径上"** —— `closure_roundtrip_closure_env` 先写出 `robust_surface2.cnut` 再抛 `a closure with free variables bound cannot be serialized`，于是 remove 永远不执行（文件被 `*.cnut` 忽略，`git status` 也看不见），**每次差分都留一个**。修法：用例改成"catch 记错误 → 无条件清理 → 再 throw 回去"（可观察行为不变），并给 `check_all.py` 每步加**仓库根目录文件集合快照**，多出文件即 FAIL —— 这类问题只能动态抓 |
| P10-3 | 模糊测试变异模式放大到 **1000 个种子** | **830 ok / 0 差异**；167 个变异把语法改坏（预期内）；1 个已知偏离（线程二次 call 的类型名）；2 个超时 → 分类后 1 个"两侧都挂"（死循环，语义一致）+ 1 个 **ref-limit**（变异把 `depth(n-1)` 改成 `depth(n*1)` → 无限递归：参考实现无上限，一路涨到系统杀进程；我们封顶给可捕获错误 = MIGRATION.md 记录的资源上限偏离） |
| P10-4 | 模糊测试健壮性：超时不再让整轮扫描崩溃 | `subprocess.TimeoutExpired` 之前直接把 1000 个种子的扫描打断在中间（实测 seed 451）；现在按哨兵码返回并分类成 `timeout` / `ref-limit`，单脚本上限 60s → 20s |

**结论**：1000 个变异里**没有一个无法解释的差异** —— 剩下的两类分别是"已在案的有意偏离"
（错误文本类型名、资源上限）。

### P9 完成情况（stream/blob 位置状态 × API 矩阵）

动机：P8 的模糊测试在 stream/blob 上抓到的两个差异都**藏在"调用之后的读写位置"里**，
而原有语料每条都先 `seek(0)` 重置，结构性地看不到状态维度。所以把"状态"当作矩阵第二维：
12 个 blob 状态 / 10 个 file 状态 × 全部 API（`tell/len/eos`、`seek` 的 7 偏移 × 3 origin、
`readn` 8 种格式、`writen` 11 种（含 int↔float 隐式转换）、`readblob` 含越界、
`writeblob`、`resize`、`swap2/swap4`、`flush`、`clone`），**观测调用返回值 + 调用后的
tell/len/eos**，出错也照样观测。

| # | 项 | 结果 |
|---|---|---|
| P9-1 | `tools/gen_streammatrix.py` → `tests-corpus/t_streammap.nut`（1150 条） | **两侧逐字节一致（1151 行，0 差异）** |
| P9-2 | 作用 | 系统性验证了 P8 修的两处（`swap2/4` 不动位置、文件流负绝对偏移返回 -1）在整个叉积上都成立 |
| P9-3 | **生成器自检**（关键）：跑一遍自己的产物，出现 `!outer:` 就报错退出 | 抓出 3 个生成器 bug —— 见下 |
| P9-4 | 发现的一类**语料书写陷阱**：更新流写入后直接读是 C 标准的 UB | 见下 |
| P9-5 | `tools/corpus_health.py`：语料"假绿"体检（用例是否真的执行到了） | 全部 72 个文件无假绿；见下 |

**P9-5 体检口径**：跑一遍语料，按用例标签聚合"错误行占比"，并重点标记
**"全部用例都出错且集中在同一两个错误上"** —— 那是最强的假绿信号（用例体自己坏了，
例如调用了不存在的辅助函数，此时两侧会"一致地"报同一个错，差分照样 PASS）。
结果：没有命中。错误占比高的几个文件都是设计使然（`t_delmatrix` 81%，因为通用参数形态里
"空参"对多数方法就是 arity 错误；`t_errmsg2` 76% 是错误消息矩阵；`t_opmx` 55% 是
运算符 × 类型矩阵，大量组合本就是类型错误），且没有一个文件是 100% 集中报错。

**自检抓到的三个生成器 bug（都属于"差分测不出来"的那类）**：

1. Python 模板里写成 `''b''`（双单引号）→ 生成坏代码 → **1150 条用例全部在同一个地方
   炸掉、两侧一致 → 差分报 PASS 但什么都没验证**（假绿）。抽查输出才发现；
2. `function show(r, d)` 没有默认值而调用处只传一个 → Squirrel 把 `this` 算进参数个数，
   于是 `2 passed, 3 required` → 改成 `d = 0`；
3. `clone` 文件流会抛 `this object cannot be cloned`，我把它写在 `try` 外面 →
   外层 catch 吃掉 → 把 clone 也纳入观测。

**顺带发现的一类 UB（不是我们的偏离，是书写陷阱）**：C11 7.21.5.3 规定更新流
（`"wb+"`）**输出后不能直接输入**，中间必须有 `fflush` 或定位调用。我最初的状态构造器
写完之后直接读 → 12 条 file 读用例报差异 ✗，参考实现的表现是"读越界却把文件撑大"
（4 字节文件 `readn('l')` 后 `len()` 变 12），而且**同一段代码只因"读完之后是否调用
`len()`"就一个报错一个成功** —— 典型的未定义行为。补一次 `seek(当前位置)` 之后全部一致。
这条已写进 `MIGRATION.md` 的怪癖表（给写脚本的人看）。

### P8 完成情况（随机差分模糊测试）

`tools/fuzz_diff.py`，两种模式，**并且真的抓到了两个语义差异**：

| 模式 | 做法 | 结果 |
|---|---|---|
| 结构化生成（默认） | 随机组合字面量池（边界整数/浮点/NaN/±inf/嵌入容器）、二元与一元运算、容器与字符串方法、安全全局、循环/foreach/switch/try-catch/**闭包捕获**/**类与继承**/**生成器**/**委托 metamethod** | **820 个程序 / 约 1.6 万条用例，0 差异** |
| 语料变异（`--mutate`） | 随机挑 `tests-corpus/` 里的程序，做 1~3 处局部变异（整数/浮点/字符串字面量换成边界值、换运算符、复制一行），跳过含随机源与文件 IO 的行 | **400 个变异：2 处真实差异（已修）+ 1 处已在案的有意偏离** |

**抓到的两个真实差异（都已修 + 进语料 + 加测试）**：

| # | 差异 | 实测 | 修复 |
|---|---|---|---|
| P8-1 | `blob.swap2()/swap4()` **推进了读写位置** | `tell()` 我们给 4/8，参考给 0（参考直接改 `GetBuf()`，从不碰 `_ptr`） | 交换后恢复位置（`stream.rs`）；连带修掉"swap2 后 seek(-1) 失败、位置停在末尾 → `readn` 越界报 `io error`"的连锁现象 |
| P8-2 | 文件流上**负绝对偏移被夹成 0 并当成功** | `f.seek(-1,'b')` 我们给 0，参考给 **-1**（CRT `fseek(f,-1,SEEK_SET)` 失败） | `SQ_SEEK_SET` 且 `offset < 0` 直接返回 -1 |

**变异才碰得到的原因**：语料里原有的 `swap2` 用例都在 `seek(0)` 之后（位置 0 时"整块交换"与"就地交换"结果相同），所以这个差异一直藏着；变异把 `seek(-1)` 插进来才暴露。

设计要点（都是踩过坑才定下来的）：

1. **每个用例一行** `t("cNNN", function(){...})` + 用例内 `try/catch` 打印错误文本 ——
   于是"运行时报错"也是一种被比较的观测，不会让整份脚本失败；发生差异时**按行删除自动收缩**；
2. **确定性**：不用 `rand/time/clock/date/getenv/system/file`；容器统一用带**深度上限**的
   递归 `show()` 渲染（防环 + 不打印裸指针）；变异模式的跳过清单同一套；
3. **参考实现的 UB 不进参数池**：`tointeger` 坏进制（负数/1/>36）会 `0xC0000409`，
   第一版生成器里误留了 `"a".tointeger(99)`，被判成 `ref-ub` ——现在生成器只喂合法进制；
4. **ref-ub 自动定位**：stdout 是带缓冲的，"零输出"既可能是编译期崩溃也可能是运行期崩溃，
   所以崩了就跑**前缀二分**把那条用例找出来；
5. **生成器自检**：两侧都跑不动 = 生成器产出了非法程序（`both-fail`），要修生成器而不是
   记成差异。实测修掉三处：`===`（不是 Squirrel 语法）、表字面量整数键要写 `[1] = v`、
   `-` 粘到 `-0.5` 上会变成自减 `--0.5`；
6. **完成判据要分模式**：变异对象是语料文件，它们没有 `FUZZ-DONE` 标记 —— 判据必须退化成
   `rc == 0`，否则全被判成 `both-fail`（这个 bug 实际发生过，也让收缩函数失效过一轮）；
7. `print` 在嵌入 NUL 处截断是**两边一致**的，所以模糊测试里不生成 NUL 字面量
   （否则输出的可见部分会比真实值短，掩盖差异）。

**已知局限**：结构化生成只做 2 层嵌套、不生成线程/文件/序列化/正则深水区；变异模式对
长语料（如 4533 行的数值矩阵）每次要跑整份，成本偏高。

### P7 完成情况（base 库零覆盖全局）

`tools/global_coverage.py` 的结果：62 个全局里只有 5 个在语料中出现 0 次 ——
`assert` / `error` / `array` / `type` / `dummy`，全是 `sqbaselib.cpp` 里的错误与构造路径。

| # | 项 | 结果 |
|---|---|---|
| P7-1 | `tools/gen_baselibmap.py` → `tests-corpus/t_baselibmap.nut`（90 行） | **stdout 与 stderr 都逐字节一致，0 差异** |
| P7-2 | `assert` 判假：`null`/`false`/**`0`**/**`0.0`** 为假，`""`/`[]`/`{}`/函数/blob 为真 | 实测纠正了我的源码推断（我以为只认 null/false），已记入 `MIGRATION.md` |
| P7-3 | `assert(cond, msg)` 的消息先 `sq_tostring`；容器给 `(table : 0x…)` 且**不调 `_tostring`** | 同上 |
| P7-4 | `error(msg)` **不抛异常**：调 errorfunc（stderr）后返回 `null`，脚本继续；`array(size)` 的 `tointeger` 截断与多余实参忽略；`type` 是 `typeof` 别名；`dummy` 的 `nparams=0` 不检查个数 | 同上 |
| P7-5 | 守门测试 `crates/squirrel-stdlib/tests/baselib_semantics.rs`（5 个） | `cargo test` 124 → 129 |
| P7-6 | 生成器踩坑：`class` 声明不能当表达式（`type(class C {})` → 两侧同样报 `expected '{'`） | 改成顶层声明 + 按名引用 |

### P6 完成情况（math 库定义域/边界，验证型）

动机：`t_math.nut` 只覆盖了"基础形态"，而且打印用默认浮点格式（约 6 位有效数字），
**尾位差异看不见**；定义域边界完全没测。新矩阵统一用 `%.17g`（f32 的精确值）+
整数按十进制原样打印。

| # | 项 | 结果 |
|---|---|---|
| P6-1 | `tools/gen_mathmatrix.py` → `tests-corpus/t_mathmap.nut`（一元 312 + 二元 31 + 常量/错误 34 = 377 行） | **两侧逐字节一致，0 差异** |
| P6-2 | 结论：我们与参考**逐位一致**（含 `sin(1e30)` 的参数归约、`sqrt(-1)`=`-nan(ind)`、`PI`=(float)M_PI） | 无需修改 |
| P6-3 | 顺带钉住的怪癖：`RAND_MAX`=**32767**（MSVC CRT）、`abs` 是 `abs((int)n)` 的 **32 位截断**、`abs` 只吃整数、`floor/ceil` 返回 float | 已进 `MIGRATION.md` |
| P6-4 | 守门测试 `crates/squirrel-stdlib/tests/math_semantics.rs`（4 个） | `cargo test` 120 → 124 |
| P6-5 | **踩坑记录**：观测器不能给整数套 `%.17g` —— 那会把整数转成 f32，`abs(2147483647)` 被打成 `2147483648`，整数用例全部失效（第一版矩阵就是这样"假绿"的） | 写进 `MIGRATION.md` 怪癖表 |

### P5 完成情况（string 库全局函数语义）

发现方式：补"字符串方法语义矩阵"时先 grep 语料，`startswith/endswith/strip/lstrip/rstrip/escape`
**出现 0 次**。查参考实现后确认原因：它们**不是字符串的成员方法**，而是 string 库的
**全局函数**（`startswith(str, cmp)`）；`"abc".startswith("a")` 在两个引擎里都是
`the index 'startswith' does not exist`。按委托表逐个成员生成的矩阵因此**结构性地碰不到这一族**
（`gen_delmatrix.py` 解析的是 `_string_default_delegate_funcz`）。

| # | 项 | 结果 |
|---|---|---|
| P5-1 | `gen_delmatrix.py` 新增 `m_s_*` 语义段 **229 条**（全局调用语法） | 两侧逐字节一致 |
| P5-2 | **修 `split`**：参考实现 `while(*end != '\0')` 按 C 字符串扫描，遇嵌入 NUL 就停、其后整段丢掉 | 原来给 `["a\x00","b"]`，现在给 `["a"]` ✓ |
| P5-3 | **修 `lstrip`**：参考实现 `sq_pushstring(v,start,-1)` 负长度走 `strlen`，结果在第一个 NUL 处截断 | 原来给 `"a\x00b"`(3)，现在给 `"a"`(1) ✓ |
| P5-4 | 确认 `rstrip`/`strip`/`escape`/`startswith`/`endswith` 走**显式长度**，不受 NUL 影响 | 加测试钉住，防止以后"顺手统一"改错 |
| P5-5 | 守门测试 `crates/squirrel-stdlib/tests/string_semantics.rs`（3 个） | `cargo test` 117 → 120 |
| P5-6 | 迁移指南补 8 条"两边一致怪癖"（含 `print` 在 NUL 处截断 → 语料改用字节长度观测） | `MIGRATION.md` 第 4 节 |

### P2 完成情况

| # | 子项 | 结果 |
|---|---|---|
| P2-1 | member handle | `hostapi.rs` 的 `get_member_handle`/`get_by_handle`/`set_by_handle`；用**参考实现的静态库编 C 探针**实测出三条关键语义：**只接受类**、成功时消费键、`_static` = 不是字段 |
| P2-2 | `getfunctioninfo` | 按栈层级取（name/source/line/native）；`native` 区分 native closure |
| P2-3 | `setfreevariable` | 替换槽位语义；**修出 panic**：`_OP_GETOUTER` 假设槽里一定是 `Outer` |
| P2-4 | userdata/typetag/releasehook/refcount/foreignptr/resurrect | 内部早有，钉成公开宿主面；**修出"release hook 从不触发"**（挂到 `Drop`） |
| P2-5 | 宿主 API 测试 | `host_api_ext.rs` 4 个测试 |


## P1 完成情况（2026-09 本轮）

| # | 面 | 语料 | 结果 |
|---|---|---|---|
| P1-1 | 数值/运算符边界 | `t_nummatrix.nut` 4533 条 | 修**移位计数掩码**（x86 低 6 位）；定位 `INT64_MIN / -1` 硬件陷（参考实现崩） |
| P1-2 | weakref / GC | `t_weakmap.nut` 63 条 | 零差异；`collectgarbage()` 回收计数逐一对齐 |
| P1-3 | 词法/编译面 | `t_lexmatrix.nut` 203 条 + `errmsg_matrix.py` 60 条 | 修**字符串下标有符号性**（`"\xff"[0]` == -1）；编译错误文本全一致 |
| P1-4 | blob / stream 二进制 | `t_blobmap.nut` 133 条 | 修**非法文件模式**（参考实现 `0xC0000409` 崩，改成白名单校验） |
| P1-5 | io / system / date | `t_sysmap.nut` 45 条 | 修 `remove`/`rename`（抛错 + 返回 null）、`system`（继承 stdio）、`clock`（CPU 时间） |
| P1-6 | 异常机制 | `t_errmap.nut` 81 条 + `t_errtail.nut` | 零语义差异；未捕获错误的 CALLSTACK/LOCALS dump **逐字节一致** |
| P1-7 | 类属性 `</ … />` | `t_attrmap.nut` 40 条 | 修 `setattributes(null, …)` 设类级属性（返回旧值） |
| P1-8 | suspend 穿透 native | `t_suspmap.nut` 16 条 + `t_suspend_main.nut` | 零差异；`_get` 里 suspend 报的是"元方法不能扩栈" |
| P1-9 | 多线程 × GC | `t_gcx.nut` 16 条 | 修 **panic**（线程里 `collectgarbage()` 撞 RefCell）+ **当前 VM 根集合缺失**（线程栈对象被误回收） |
| P1-10 | 序列化往返 | `t_sermap.nut` 25 条 | 修 native closure 错误文本；确认默认参数值往返丢失（两侧一致） |


---

## 0. 今天的盘点结论（先看这个）

### 0.1 已经对齐的面（有审计脚本或语料守门）

| 面 | 审计方式 | 结果 |
|---|---|---|
| 全局函数（base/math/string/system/io/blob 六张注册表） | `tools/audit_globals.py` | 62/62，名字+nparamscheck+类型掩码全一致 |
| 10 张默认委托表 | `tools/audit_delegates.py` | 0 处不一致 |
| 根表插入顺序（脚本可观察：`foreach(getroottable())`） | `tools/audit_regorder.py` | 顺序一致 |
| metamethod 集合 | 对照 `squirrel/sqobject.h` 的 `MM_*` | 18 个完全一致（`_add…_inherited`） |
| 标准库类方法（**今天新做的审计**） | 对照 `_blob_methods` / `_stream_methods` / `_file_methods` / `rexobj_funcs` | blob 10/10、stream 10/10、file 3/3、regexp 6/6，**无缺口** |
| 脚本可见语义 | 语料 57 + 正则矩阵 500 + 生成器/线程矩阵 130 + samples 17 + `.cnut` 57（字节一致 + 双向交叉执行）+ opcode 61 | 全绿 |
| Rust 单元/集成测试 | `cargo test` | 85/85 |

### 0.2 整块缺失的面

| 面 | 现状 | 影响 |
|---|---|---|
| **C ABI（嵌入 API）** | `include/squirrel.h` 导出 **141 个 `sq_*`**，`sqstdlib` 再导出 **33 个 `sqstd_*`**；我们的宿主 API 是 **Rust 的**，只对齐到 `etc/minimal.c` 的用法（`Vm::new/compile_buffer/call/run_buffer/get_global/set_global/register_native/set_print_funcs`） | **如果宿主是 C/C++ 程序**（把 Squirrel 当内嵌脚本引擎），则无法替换/迁移 —— 这是唯一"整块"缺失的功能面 |

### 0.3 宿主 API 的零散缺口（今天 grep 实测）

| 能力 | 参考实现 | 我们 |
|---|---|---|
| member handle 快速成员访问 | `sq_getmemberhandle` / `sq_getbyhandle` / `sq_setbyhandle` | **0 处**，无实现 |
| 函数信息（名字/源/行/是否 native） | `sq_getfunctioninfo` | **0 处**（脚本侧 `getstackinfos` 有，宿主侧没有） |
| 宿主给闭包绑自由变量 | `sq_setfreevariable` | **0 处**（内部有 outer，但没有宿主入口） |
| userdata + typetag | `sq_newuserdata` / `sq_settypetag` / `sq_gettypetag` / `sq_getobjtypetag` | 数据结构有（`Value::Userdata` 带 `typetag`），**没有公开宿主面** |
| release hook | `sq_setreleasehook` / `sq_getreleasehook`（vm/shared 也有） | 有内部实现（9 处），无公开宿主面 |
| 外部指针 | `sq_setforeignptr` / `sq_getforeignptr`（vm/shared） | 字段有（`foreignptr`），无公开宿主面 |
| 调试钩子 | `sq_setdebughook` / `sq_setnativedebughook` | 脚本级 `setdebughook` 有，宿主级无 |
| 引用计数 API | `sq_addref` / `sq_release` / `sq_getrefcount` / `sq_getvmrefcount` | `RefTable` 内部有，无公开宿主面 |
| 不可达对象复活 | `sq_resurrectunreachable` | 有实现（6 处），无公开宿主面 |

### 0.4 已确认的**有意偏离**（不是缺口，见 STATUS 第四节）

递归/内存上限、指针序比较、参考实现自身的 UB 输入（`getstackinfos(-1)`、`blob.resize(-1)`、
`split(非串)`、`regexp("")`、`format` 20 字符、`tointeger(base=1)`）等 ~26 条 —— 我们给可捕获错误，
参考实现崩溃。**这些不打算"修回去"。**

---

## 1. 优先级

### P0 —— 先回答一个问题：宿主形态

**问题**：你是把 Squirrel 当**命令行解释器**用，还是**嵌进 C/C++ 程序**当脚本引擎用？

* 只当解释器 → 跳过 P0，从 P1 开始（下面是脚本语义覆盖面）。
* 要嵌入且想保持 C/C++ 宿主代码不变 → **必须做 C ABI 层**，这是唯一的"整块"工作：

  | 子项 | 内容 | 轮 |
  |---|---|---|
  | P0-1 | `extern "C"` 导出 141 个 `sq_*`：句柄/栈 API/调用/编译/表与类/userdata/序列化/GC/调试 | 3–5 |
  | P0-2 | 33 个 `sqstd_*`：文件/blob/regex 引擎入口/错误处理/注册函数 | 1–2 |
  | P0-3 | 提供 `squirrel.h` 兼容头（或直接用原头），保证 `etc/*.c` 能直接编译 | 1 |
  | P0-4 | 内存所有权与错误语义对照（谁分配谁释放、`sq_addref` 契约、异常 vs 返回值） | 1 |
  | P0-5 | 用参考实现的 `etc/minimal.c`、`sq.c`、官方 samples **原样编译**跑通做验收 | 1 |

  验收：`minimal.c` / `sq.c` 不改一行，链接我们的库后行为与 C++ 版逐字节一致。

### P1 —— 脚本语义的未覆盖面（按 风险 × 成本 排序，每项 ≈ 1 轮）

排名依据：这块是"用户天天写到的语法/运算"，一旦有差异最容易撞上；且都能用现成的
生成式差分方法一次扫一大片。

| # | 面 | 为什么排这里 | 验证方式 |
|---|---|---|---|
| P1-1 | **数值与运算符边界**：整数溢出（回绕 vs 报错）、位运算与负数、移位 ≥64、`/` `%` 的符号与除零、`tointeger/tofloat` 的舍入与进制、NaN/±inf 比较、`+=`/`-=` 越界 | 最常用、最容易藏差异；现有 `nan_probe.nut`/`toint_probe.py` 只覆盖了一角 | `tools/gen_nummatrix.py` → 运算×类型×边界值矩阵；进语料 + Rust 测试 |
| P1-2 | **weakref / GC / 回收语义**：weakref 对每种类型、`getweakrefval` 对已死对象、weakref 进容器、`collectgarbage()` 返回计数、`resurrectunreachable`、release hook 触发时机 | 与内存/生命周期强相关；我们刚改过回收时机，需要守门 | `t_weakmap` 矩阵 + `collectgarbage` 前后对拍（注意参考实现的 refcount 时机差异要单列） |
| P1-3 | **词法/编译器面**：字符串转义全表、verbatim `@"..."`、数字字面量（hex/科学计数/前导 0）、运算符优先级与结合性、作用域与遮蔽、`__LINE__`、注释嵌套、`::`/`<-`/`clone`/`delete`/`in`/`instanceof` | 编译器是我们自己移植的（有 61 opcode 覆盖，但"语法形态"覆盖不全） | `tools/gen_lexmatrix.py`：表达式/语句形态矩阵，逐行差分；另加"编译错误消息"对拍 |
| P1-4 | **blob / stream 二进制**：`readn`/`writen` 的类型分派、`readblob`/`writeblob`、`swap2/4` 端序、`seek` 模式、内存 blob vs 文件的 `tell/len/eos`、`resize/append` | 二进制处理最容易"看起来对但字节不对" | `t_blobmap`：写→读回环 + 每种类型/边界；已有 `stream_semantics.rs` 可扩展 |
| P1-5 | **io / system**：`getenv`、`system`、`clock`/`time`/`date` 全字段、`remove`/`rename` 错误路径、文件打开模式（`r/w/a/rb/wb` + 非法模式）、`close` 二次调用 | 平台相关，容易被"想当然" | `t_iomatrix`（`date` 只比结构不比时间；`system` 用固定命令） |
| P1-6 | **异常机制**：嵌套 trap、`throw` 非字符串、错误处理器自己抛错、`getstackinfos` 各层级、未捕获错误的 stderr 格式 | 错误路径是"平时不跑、出事才跑"的代码 | 现有 `t_try2` 扩充 + stderr 对拍（已有归一化） |
| P1-7 | **类属性 `</ ... />` 与 `getattributes/setattributes`** 全矩阵 | OOP 矩阵覆盖了一部分，属性语法本身没系统测 | `gen_attrmatrix.py` |
| P1-8 | **suspend 穿透 native/metamethod** 的错误路径（`cannot suspend through native calls/metamethods`） | 生成器/线程矩阵没覆盖 | 探针 + 进语料 |
| P1-9 | **多线程 × GC**：多线程持有对象、GC 时的根集合、线程死亡后的对象生命周期 | 我们刚改过 outer/回收，交叉面风险高 | 定向探针 + Rust 测试 |
| P1-10 | **序列化往返**（`-c` 编译 + `sqstd_writeclosuretofile`），含闭包/类/默认参数/调试信息 | `.cnut` 已 57/57 字节一致，但"脚本侧写文件"的路径没测 | 扩 `cnut_matrix.py` 的命令行形态 |

### P2 —— 宿主 API 补齐（Rust 面，让宿主能做完 C API 能做的事）

前置：P0 的答案。若只当解释器用，这一档可以不做。

| # | 子项 | 说明 | 轮 |
|---|---|---|---|
| P2-1 | member handle（`getmemberhandle`/`getbyhandle`/`setbyhandle`） | 宿主高频成员访问用；现在完全没有 | 1 |
| P2-2 | `getfunctioninfo`（名字/源/起始行/native） | 宿主做诊断/热更新用 | 0.5 |
| P2-3 | `setfreevariable`（宿主给闭包绑自由变量） | 宿主把 C 数据塞进脚本闭包的标准做法 | 0.5 |
| P2-4 | 公开宿主面：userdata+typetag、release hook、foreignptr、refcount、resurrect、debughook、`stackinfos/getlocal` | 内部都有，缺"稳定 API + 文档 + 测试" | 1–2 |
| P2-5 | `host_api.rs` 测试扩到覆盖上述全部（现在只复刻 `minimal.c`） | 守门 | 1 |

### P3 —— 健壮性加固（**C++ 版没有**，属于我们主动多做的）

| # | 子项 | 现状 | 说明 |
|---|---|---|---|
| P3-1 | 未捕获 `stack overflow` 的调用栈 dump 限深 | 现在最多 20 万帧 → stderr 9–11MB | 参考实现到不了那么深（先 OOM），所以这不是保真项；限到 N 帧 + "… 省略 M 帧"更实用 |
| P3-2 | 宿主内存配额（`--mem-limit`，Windows Job Object） | 无 | 参考实现完全没有；防"脚本把宿主吃光" |
| P3-3 | 脚本超时 / 指令预算（`--max-ops`） | 无 | 同上，属于加固 |
| P3-4 | 库调用方的栈假设 | 正则已用"栈预算"保护（默认 384KB），脚本递归靠 `MAX_CALL_FRAMES` | 若还有别处按递归深度假设栈大小，一并改成栈预算 |

### P4 —— 文档与收尾

| # | 子项 |
|---|---|
| P4-1 | README 与实测对齐（历史遗留的"声称支持"逐条核对） |
| P4-2 | 把 STATUS 第四节（~26 条有意偏离）整理成**迁移指南**：从 C++ 版迁过来的用户会撞到哪些点、我们为什么这么选 |
| P4-3 | 每个偏离项补一个最小复现用例（现在多数只有一句描述） |

---

## 2. 复现命令（盘点用的）

```powershell
python tools/audit_globals.py       # 全局函数 62/62
python tools/audit_delegates.py     # 10 张默认委托表
python tools/audit_regorder.py      # 根表插入顺序
python tools/audit_capi.py          # C 嵌入 API 在 Rust 源码里的引用痕迹（粗筛，见 0.2/0.3）
python tools/diff_test.py tests-corpus   # 语料 57（含正则 500 条、生成器/线程 130 条）
python tools/diff_test.py                # samples 17
python tools/cnut_matrix.py tests-corpus # 字节一致 + 双向交叉执行 57/57
python tools/lint_corpus.py              # 语料自检
cargo test --release                     # 85 个测试
```

类方法面的对照（0.1 最后一行）是手工做的，对照对象：

```
D:\OpenSource\squirrel\sqstdlib\sqstdblob.cpp      _blob_methods[]   （10 项）
D:\OpenSource\squirrel\sqstdlib\sqstdstream.cpp    _stream_methods[] （10 项）
D:\OpenSource\squirrel\sqstdlib\sqstdio.cpp        _file_methods[]   （3 项）
D:\OpenSource\squirrel\sqstdlib\sqstdstring.cpp    rexobj_funcs[]    （6 项）
```

---

## 3. 不建议现在做（附理由）

| 事项 | 理由 |
|---|---|
| 把 §4 的 UB 形态"修回"参考实现的行为 | 那是崩溃/内存损坏；用户明确要求健壮性优先 |
| 逐字复刻参考实现的引用计数时机 | Rust 侧用 `Rc` + GC 链；现在把"清死条目"补齐后内存同量级（20 万次拼接 18MB vs 参考 5.2MB），再往下做收益很小 |
| 为"栈大小"逐个函数加保护 | 已用栈预算 + CLI 大栈线程解决；剩下的是库调用方自己的栈假设（P3-4 统一处理） |

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
> | P0 | **C ABI**（141 个 `sq_*` + 33 个 `sqstd_*`） | ⏸ 等确认：只用命令行→不需要；要嵌 C/C++ 宿主→主线 |
>
> 基线：语料 **72/72**、samples **17/17**、编译错误对拍 **60/60**、
> `.cnut` **72/72**（字节一致 + 双向交叉执行）、opcode **61/61**、`cargo test` **131/131**、lint 0。
>
> **一条命令跑完**：`python tools/check_all.py`（10 项，任一项失败退出码非 0；`--quick` 跳过 cargo test）。
>
> **覆盖面扫描工具**：`tools/global_coverage.py` —— 把 62 个已注册全局函数逐个对照语料
> 调用次数，直接列出"零覆盖/低覆盖"。P7 的 5 个项就是它扫出来的（当时只有这 5 个是 0 次）。

### P10 完成情况（验证入口 + 语料卫生 + 模糊测试放大）

| # | 项 | 结果 |
|---|---|---|
| P10-1 | 验证入口入库：`tools/check_all.py` | 10 项（8 项差分/审计 + health + writes + cargo test）；原来的 `tmp-c2/final_check.py` 在 gitignore 里、不可复现，且步骤名写死数量（`corpus(69)` 早已过时）→ 改成薄壳转发 |
| P10-2 | **语料写文件审计**：`tools/audit_corpus_writes.py` | 查"谁在仓库根目录写文件却不删"。根因确认：`t_surface2.nut` 漏删 `robust_surface3.txt` —— **就是那个被误提交进仓库的文件**；另有 `t_surface5.nut`、`t_closure_io.nut` 两处。三处都补了 `try { remove(...) } catch`，改完仍与参考逐字节一致 |
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

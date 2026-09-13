# TODO：对标 C++ 版 Squirrel 的功能面盘点与优先级

> 本文是**按当前实测**排的待办清单（不是猜测）。每条都给：为什么排这个位置、怎么验证、
> 大致工作量（"轮" ≈ 一次 生成矩阵 → 差分 → 修 → 进语料 → 加测试 → 提交 的循环）。
> 盘点脚本全部可重跑，见文末"复现命令"。

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

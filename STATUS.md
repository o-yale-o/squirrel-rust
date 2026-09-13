# squirrel-rust 状态评估（STATUS）

> 评估方式：不采信 README 的既有数字，逐项用官方 C++ 参考二进制（`build-ref/bin/Release/sq_static.exe`）
> 与 `target/release/sq.exe` 双引擎复跑；每一条结论都附可复现命令与实测输出。
> 评估时间：本次会话；基线提交 `b85d17e`。

## 一、已完成且已独立验证

| 验证项 | 复现命令 | 实测结果 |
|---|---|---|
| 官方 samples 差分（stdout + 退出码） | `python tools/diff_test.py` | **17/17 一致** |
| `.cnut` 字节级一致 | 双方 `-o out -c x.nut` 后比对 SHA256 | **17/17 一致** |
| `.cnut` 双向互通 | C++ 编译→Rust 执行 / Rust 编译→C++ 执行 | **17/17 一致** |
| 单元 + 集成测试 | `cargo test` | **50/50 通过**（33 单元 + 1 dump + 16 集成） |
| 全局环境覆盖 | `getroottable()` 快照对比 | root 表 **77 个名字逐字一致** |
| 原生错误消息文本 | 15 条参数/类型错误对拍 | **12/15 逐字一致** |
| CLI 保真 | `-v` / `-h` / 编译错误消息 / `-o` 必须前置的怪癖 | 与原版一致 |

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

## 三、修复清单与进度

- [ ] A. math 库注册（`m1!` 补 `.`、`atan2/pow` nparams=3、`rand` nparams=1、`abs` 取整语义、
      `writeclosuretofile` → `.sc`）+ 新增 `tests-corpus/t_math.nut`
- [ ] B. `getstackinfos` locals 槽位与内容（`tests-corpus/t_stkinfo.nut` 转绿）
- [ ] C. REPL `=expr` panic 与 `quit()` 退出
- [ ] D. `diff_test.py` 增加 stderr 比较 → 修调用栈打印（源文件名、多余帧、LOCALS 段）
- [ ] E. BOM / UCS2 源码嗅探
- [ ] F. `%e`、`split` 空段、`escape`、`rawget`/`loadfile` 消息、`format()` 无参
- [ ] 全量回归：`cargo test` + samples 17/17 + 语料全绿 + `.cnut` 双向互通

## 四、复现脚本

| 文件 | 用途 |
|---|---|
| `tools/probes/math_globals.nut` | A 的直接证据：逐个调用 18 个 math 全局函数 |
| `tools/probes/errmsg.nut` | F 的错误消息对拍（15 条） |
| `tools/probes/breadth.nut` | 约 90 项标准库广度探针，一次跑出 A/D/F 全部差异 |

对拍方式（stderr 也要抓，否则 D/F 类差异不可见）：

```powershell
& .\build-ref\bin\Release\sq_static.exe tools\probes\breadth.nut 1> ref.out 2> ref.err
& .\target\release\sq.exe            tools\probes\breadth.nut 1> rs.out  2> rs.err
Compare-Object (Get-Content ref.out) (Get-Content rs.out)
```

差分执行模板（注意 `-o` 位置）：

```powershell
python tools/diff_test.py                 # 官方 samples
python tools/diff_test.py tests-corpus    # 边界语料
```

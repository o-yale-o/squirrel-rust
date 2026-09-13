# -*- coding: utf-8 -*-
"""跑宿主 API 的 C 探针并打印报告（参考实现侧）。

用法：
    python tools/hostapi/build_ref_host.bat   # 先编译（需要 MSVC + 参考实现的 .lib）
    python tools/hostapi/run_probe.py        # 跑并打印（同时可用于刷新 ref_host.txt）

注意：探针链接的是**参考实现**的静态库，所以它的输出就是"参考实现在这些宿主 API 上的
行为"，Rust 侧 `crates/squirrel-stdlib/tests/host_api_ext.rs` 的断言与它逐项对应。
"""
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(ROOT, "tools"))
from safe_run import run as safe_run  # noqa: E402

EXE = os.path.join(ROOT, "tools", "hostapi", "ref_host.exe")

if not os.path.exists(EXE):
    print("探针还没编译：先运行 tools/hostapi/build_ref_host.bat", file=sys.stderr)
    sys.exit(1)

out, err, rc = safe_run(EXE, [], timeout=30)
print("rc =", rc)
print(out.decode("utf8", "replace").replace("\r\n", "\n"), end="")
if err:
    print("stderr:", err.decode("utf8", "replace")[:300])

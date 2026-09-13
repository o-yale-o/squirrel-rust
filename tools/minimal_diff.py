# -*- coding: utf-8 -*-
"""验收项 #5 的独立证据：把官方 `etc/minimal.c` 真的编出来，与 Rust 示例对拍。

做三件事：
1. 用 vswhere 找到 MSVC，编译 `D:\\OpenSource\\squirrel\\etc\\minimal.c`
   （链接 build-ref 产出的 squirrel_static.lib / sqstdlib_static.lib）；
2. 把 `etc/test.nut` 放到工作目录，让 C++ 版跑 `sqstd_dofile("test.nut")`；
3. 跑 Rust 版 `cargo run --example minimal`，逐字节比对 stdout / 退出码。

MSVC 不存在时跳过（打印构建方法），不算失败。

用法：
    python tools/minimal_diff.py
"""

try:  # 关掉系统错误对话框：参考实现在 UB 形态上会真崩溃并弹模态框
    import os as _os, sys as _sys
    _d = _os.path.dirname(_os.path.abspath(__file__))
    for _p in (_d, _os.path.join(_d, "..", "tools"), _os.path.join(_d, "tools")):
        if _os.path.isdir(_p):
            _sys.path.insert(0, _p)
    from safe_run import suppress_crash_dialogs as _scd
    _scd()
    del _d, _p, _os, _sys, _scd
except Exception as _e:  # pragma: no cover
    import sys as _sys2
    print("warning: 无法关闭崩溃对话框: %r" % (_e,), file=_sys2.stderr)
import os
import shutil
import subprocess
import sys

ROOT = r"D:\OpenSource\squirrel-rust"
REF_SRC_ROOT = r"D:\OpenSource\squirrel"
TMP = os.path.join(ROOT, "tmp-c2")
EXE_CPP = os.path.join(TMP, "minimal_c.exe")
EXE_RUST = os.path.join(ROOT, "target", "release", "examples", "minimal.exe")
LIB_SRC = os.path.join(ROOT, "build-ref", "lib", "Release")
LIB_DST = os.path.join(TMP, "lib")

VSWHERE = os.path.join(
    os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)"),
    "Microsoft Visual Studio", "Installer", "vswhere.exe",
)


def find_vcvars():
    if not os.path.exists(VSWHERE):
        return None
    try:
        out = subprocess.run(
            [VSWHERE, "-latest", "-products", "*",
             "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
             "-property", "installationPath"],
            capture_output=True, timeout=60,
        ).stdout.decode("utf-8", "replace").strip().splitlines()
    except Exception:
        return None
    if not out:
        return None
    bat = os.path.join(out[0], "VC", "Auxiliary", "Build", "vcvars64.bat")
    return bat if os.path.exists(bat) else None


def build_cpp(vcvars):
    os.makedirs(os.path.join(TMP, "obj"), exist_ok=True)
    os.makedirs(LIB_DST, exist_ok=True)
    # minimal.c 里有 #pragma comment(lib,"squirrel.lib")，把静态库按原名复制过去
    for src, dst in (("squirrel_static.lib", "squirrel.lib"), ("sqstdlib_static.lib", "sqstdlib.lib")):
        s = os.path.join(LIB_SRC, src)
        if not os.path.exists(s):
            print("缺少 %s，先构建 build-ref（见 README）" % s)
            return False
        shutil.copyfile(s, os.path.join(LIB_DST, dst))
    bat = os.path.join(TMP, "build_minimal.bat")
    with open(bat, "w", encoding="utf-8", newline="\r\n") as f:
        f.write(
            "@echo off\n"
            'call "%s" >nul\n' % vcvars +
            "cd /d %s\n" % TMP +
            "cl /nologo /W3 /MD /D_SQ64 /I %s /I %s\\squirrel /I %s\\sqstdlib "
            "%s\\etc\\minimal.c /Fe:minimal_c.exe /Fo:obj\\ "
            "/link /LIBPATH:%s ucrt.lib vcruntime.lib legacy_stdio_definitions.lib\n"
            % (os.path.join(REF_SRC_ROOT, "include"), REF_SRC_ROOT, REF_SRC_ROOT,
               REF_SRC_ROOT, LIB_DST) +
            "exit /b %ERRORLEVEL%\n"
        )
    p = subprocess.run(["cmd", "/c", bat], capture_output=True, timeout=600)
    if p.returncode != 0:
        sys.stderr.write(p.stdout.decode("latin-1", "replace")[-1500:])
        print("编译 etc/minimal.c 失败（rc=%d）" % p.returncode)
        return False
    return os.path.exists(EXE_CPP)


def main():
    os.makedirs(TMP, exist_ok=True)
    # etc/test.nut 必须叫 test.nut 且在 C++ 版的工作目录里
    shutil.copyfile(os.path.join(REF_SRC_ROOT, "etc", "test.nut"), os.path.join(TMP, "test.nut"))

    vcvars = find_vcvars()
    if not vcvars:
        print("跳过：找不到 MSVC（vswhere/vcvars64.bat）。手动构建方式见本文件 build_cpp()。")
        return 0
    if not build_cpp(vcvars):
        return 1

    build = subprocess.run(["cargo", "build", "--release", "--example", "minimal"],
                           cwd=ROOT, capture_output=True, timeout=900)
    if build.returncode != 0:
        sys.stderr.write(build.stderr.decode("latin-1", "replace")[-800:])
        return 1

    cpp = subprocess.run([EXE_CPP], cwd=TMP, capture_output=True, timeout=60)
    rust = subprocess.run([EXE_RUST], cwd=ROOT, capture_output=True, timeout=60)
    a, b = cpp.stdout.replace(b"\r\n", b"\n"), rust.stdout.replace(b"\r\n", b"\n")
    print("C++  etc/minimal.c   rc=%d stdout=%r" % (cpp.returncode, cpp.stdout))
    print("Rust examples/minimal rc=%d stdout=%r" % (rust.returncode, rust.stdout))
    ok = a == b and cpp.returncode == rust.returncode and not cpp.stderr and not rust.stderr
    print("\n%s：stdout/退出码逐字节一致" % ("一致" if ok else "不一致"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

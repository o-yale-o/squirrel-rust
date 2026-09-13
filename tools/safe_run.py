# -*- coding: utf-8 -*-
"""安全启动参考实现：先关掉 Windows 崩溃对话框（SEM_NOGPFAULTERRORBOX），
这样参考实现自身的越界/UB 崩溃只会表现为退出码，不会弹出模态框打断交互。

用法（作为库）：
    from safe_run import run
    out, err, rc = run(REF, ["script.nut"], stdin=b"")
"""
import ctypes
import subprocess
import sys

SEM_FAILCRITICALERRORS = 0x0001
SEM_NOGPFAULTERRORBOX = 0x0002
SEM_NOOPENFILEERRORBOX = 0x8000

_prev = None


def suppress_crash_dialogs():
    """对当前进程设置错误模式；子进程会继承该设置"""
    global _prev
    if sys.platform != "win32":
        return
    try:
        k = ctypes.windll.kernel32
        _prev = k.SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX)
    except Exception:
        pass


def run(exe, args, stdin=b"", timeout=30):
    suppress_crash_dialogs()
    p = subprocess.run([exe] + list(args), input=stdin, capture_output=True, timeout=timeout)
    return p.stdout, p.stderr, p.returncode


if __name__ == "__main__":
    suppress_crash_dialogs()
    exe = sys.argv[1]
    out, err, rc = run(exe, sys.argv[2:])
    sys.stdout.write(out.decode("latin-1"))
    sys.stderr.write(err.decode("latin-1"))
    print("\n[rc=%d]" % rc)

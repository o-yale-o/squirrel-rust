# -*- coding: utf-8 -*-
"""带内存/时间上限地跑脚本：超限就杀掉并报告（防止再次出现 32G 失控）。

用法：
    python tools/mem_probe.py script.nut [--limit-mb 512] [--timeout 10] [--exe RUST|REF]
    python tools/mem_probe.py tests-corpus/t_oopmatrix.nut --bisect   # 逐条用例找失控的那条
"""
import ctypes
import ctypes.wintypes as wt
import os
import subprocess
import threading
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from safe_run import suppress_crash_dialogs  # noqa: E402

ROOT = r"D:\OpenSource\squirrel-rust"
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")
RUST = os.path.join(ROOT, "target", "release", "sq.exe")

PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
PROCESS_VM_READ = 0x0010


class PROCESS_MEMORY_COUNTERS(ctypes.Structure):
    _fields_ = [
        ("cb", wt.DWORD),
        ("PageFaultCount", wt.DWORD),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
    ]


def proc_mb(pid):
    k = ctypes.windll.kernel32
    h = k.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, False, pid)
    if not h:
        return None
    try:
        c = PROCESS_MEMORY_COUNTERS()
        c.cb = ctypes.sizeof(c)
        if ctypes.windll.psapi.GetProcessMemoryInfo(h, ctypes.byref(c), c.cb):
            return c.WorkingSetSize / (1024 * 1024)
        return None
    finally:
        k.CloseHandle(h)


def run_with_cap(exe, args, limit_mb, timeout):
    """返回 (rc, peak_mb, out, err, killed_reason)

    注意：**必须并发把管道读干**。早先这里只在进程结束后才 `communicate()`，
    子进程输出超过管道缓冲（几十 KB）就会阻塞在 write 上——看起来像"脚本死循环"，
    实际是探针自己造成的死锁（t_oopmatrix 整跑就这样被误判过）。

    另外：参考实现在 UB 形态上会**真的崩溃**，跑之前必须关掉系统错误对话框
    （`SEM_NOGPFAULTERRORBOX`），否则会弹出模态框把整批对拍挂住。
    """
    suppress_crash_dialogs()
    p = subprocess.Popen([exe] + list(args), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                         stdin=subprocess.DEVNULL)
    bufs = {p.stdout: [], p.stderr: []}

    def drain(stream, sink):
        try:
            while True:
                chunk = stream.read(65536)
                if not chunk:
                    break
                sink.append(chunk)
        except Exception:
            pass

    threads = [threading.Thread(target=drain, args=(s, b), daemon=True) for s, b in bufs.items()]
    for t in threads:
        t.start()
    peak = 0.0
    reason = None
    t0 = time.time()
    while True:
        if p.poll() is not None:
            break
        m = proc_mb(p.pid)
        if m is not None:
            peak = max(peak, m)
            if m > limit_mb:
                reason = "MEM %d MB > %d MB" % (m, limit_mb)
                p.kill()
                break
        if time.time() - t0 > timeout:
            reason = "TIMEOUT %.1fs" % (time.time() - t0)
            p.kill()
            break
        time.sleep(0.02)
    try:
        p.wait(timeout=10)
    except subprocess.TimeoutExpired:
        p.kill()
    for t in threads:
        t.join(timeout=5)
    out = b"".join(bufs[p.stdout])
    err = b"".join(bufs[p.stderr])
    for s in bufs:
        s.close()
    return p.returncode, peak, out, err, reason


def bisect(path, exe, limit_mb, timeout):
    src = open(path, encoding="utf-8").read().split("\n")
    head_end = next((i for i, l in enumerate(src) if l.startswith('s("')), 0)
    head = src[:head_end]
    cases = [l for l in src[head_end:] if l.startswith('s("')]
    tmp = os.path.join(ROOT, "tmp-c2", "mem_case.nut")
    print("共 %d 条用例，逐条跑（上限 %d MB / %.1fs）" % (len(cases), limit_mb, timeout))
    bad = []
    for i, c in enumerate(cases):
        with open(tmp, "w", encoding="utf-8", newline="\n") as f:
            f.write("\n".join(head) + "\n" + c + "\n")
        rc, peak, out, err, reason = run_with_cap(exe, [tmp], limit_mb, timeout)
        if reason:
            tag = c.split('"')[1] if '"' in c else "?"
            print("  #%d %-40s %s (峰值 %.0f MB)" % (i, tag, reason, peak))
            bad.append((i, tag, reason))
    print("\n失控用例 %d 条" % len(bad))
    return bad


def main(argv):
    limit_mb = 512
    timeout = 10.0
    exe = RUST
    path = None
    skip_next = False
    for i, a in enumerate(argv):
        if skip_next:
            skip_next = False
            continue
        if a == "--limit-mb":
            limit_mb = int(argv[i + 1])
            skip_next = True
        elif a == "--timeout":
            timeout = float(argv[i + 1])
            skip_next = True
        elif a == "--exe":
            exe = REF if argv[i + 1].upper() == "REF" else RUST
            skip_next = True
        elif not a.startswith("--"):
            path = a
    if not path:
        print(__doc__)
        return 2
    if "--bisect" in argv:
        bisect(path, exe, limit_mb, timeout)
        return 0
    rc, peak, out, err, reason = run_with_cap(exe, [path], limit_mb, timeout)
    print("rc=%s 峰值=%.0f MB%s" % (rc, peak, (" [" + reason + "]") if reason else ""))
    sys.stdout.write(out.decode("latin-1")[-800:])
    if err:
        sys.stderr.write(err.decode("latin-1")[-400:])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

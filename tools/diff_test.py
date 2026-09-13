# -*- coding: utf-8 -*-
"""差分测试：同一 .nut 脚本分别在原版 sq_static.exe 与 Rust sq.exe 上执行，
比对 stdout、stderr 与退出码。

stderr 也要比——错误钩子（AN ERROR HAS OCCURRED / CALLSTACK / LOCALS）全部写 stderr，
只比 stdout 会让整条错误路径失去覆盖。加 --no-stderr 可退回旧行为。
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
import re
import subprocess
import sys
import os

REF = r"D:\OpenSource\squirrel-rust\build-ref\bin\Release\sq_static.exe"
RUST = r"D:\OpenSource\squirrel-rust\target\release\sq.exe"
SAMPLES = r"D:\OpenSource\squirrel\samples"


def run_one(exe, path, timeout=10):
    try:
        p = subprocess.run([exe, path], capture_output=True, timeout=timeout)
        return p.stdout, p.stderr, p.returncode
    except subprocess.TimeoutExpired:
        return b"<TIMEOUT>", b"<TIMEOUT>", -999


def norm(b):
    b = b.replace(b"\r\n", b"\n")  # CRT stdout/stderr 文本模式翻译，非语言语义
    # 只把**指针**归一化：至少 8 位十六进制。早先写成 0x[0-9a-fA-F]+ 会把
    # `format("%#x", 1)` 这类正常输出也吃掉（0x1 → 0xPTR），等于掩盖差异。
    return re.sub(rb"0x[0-9a-fA-F]{8,}", b"0xPTR", b)


def load_failed(out):
    """CLI 的「加载/编译失败」路径：stdout 以 Error [ 开头。

    两边报同一个编译错误时，stdout/stderr 完全一致 —— 但如果用例本意是跑语义，
    这其实是**假绿**（什么都没测到）。除白名单外一律判失败。
    """
    return out.lstrip().startswith(b"Error [")


# 允许「故意加载失败」的用例（目前没有）
EXPECT_LOAD_FAILURE = set()

# 允许「脚本中途以未捕获错误结束」的用例（它们就是测错误打印的）
EXPECT_UNCAUGHT = {
    "t_bit.nut", "t_blob.nut", "t_deep.nut",
    "t_errdump1.nut", "t_errdump2.nut", "t_errdump3.nut",
    # 线程 call() 参数不匹配时，错误处理器会按参考实现打印一次调用栈（两边一致）
    "t_errmsg2.nut",
    # 故意用 loadfile(path, true) 触发编译错误打印
    "t_loadfile_bom.nut",
}


def main(paths, check_stderr=True):
    npass = nfail = 0
    for path in paths:
        out_r, err_r, rc_r = run_one(REF, path)
        out_u, err_u, rc_u = run_one(RUST, path)
        name = os.path.basename(path)
        # 超时必须判失败：两边都超时会产出完全相同的 `<TIMEOUT>`，
        # 当成 PASS 会让「死循环语料」长期假绿（t_deep.nut 原本是尾调用死循环）。
        if out_r == b"<TIMEOUT>" or out_u == b"<TIMEOUT>" or rc_r == -999 or rc_u == -999:
            nfail += 1
            print("FAIL %s：脚本超时（语料必须能在超时时间内跑完）" % name)
            continue
        same = norm(out_r) == norm(out_u) and rc_r == rc_u
        if check_stderr:
            same = same and norm(err_r) == norm(err_u)
        if same and load_failed(norm(out_r)) and name not in EXPECT_LOAD_FAILURE:
            same = False
            print("SUSPECT %s：脚本本身编译/加载失败（未测到语义）" % name)
            print("  output: %r" % norm(out_r)[:200])
        if same and norm(err_r).strip() and name not in EXPECT_UNCAUGHT:
            # 未捕获错误会让脚本**中途停止**，后面的断言根本没跑 —— 这种「两边一样地早停」
            # 曾经骗过差分测试（新语料 429 行只跑到 25 行仍报 PASS）。
            # 例外：脚本自己打印了完成标记（生成的矩阵语料都以 `<NAME>-DONE` 结尾），
            # 说明它跑完了；此时 stderr 只可能是「错误被脚本 catch 住、但错误处理器仍打印」
            # 的那类情况（两边逐字一致才算通过，前面已经比过 stderr 了）。
            if b"-DONE" in norm(out_r):
                npass += 1
                print("PASS %s（stderr 非空但已跑到 -DONE）" % name)
                continue
            same = False
            print("SUSPECT %s：脚本以未捕获错误提前结束（后面的用例没跑到）" % name)
            print("  stderr: %r" % norm(err_r)[:200])
        if same:
            npass += 1
            print("PASS %s" % name)
        else:
            nfail += 1
            print("FAIL %s (rc %d vs %d)" % (name, rc_r, rc_u))
            if norm(out_r) != norm(out_u):
                print("  stdout ref : %r" % norm(out_r)[:300])
                print("  stdout rust: %r" % norm(out_u)[:300])
            if check_stderr and norm(err_r) != norm(err_u):
                print("  stderr ref : %r" % norm(err_r)[:300])
                print("  stderr rust: %r" % norm(err_u)[:300])
    print("\n%d passed, %d failed" % (npass, nfail))
    return 0 if nfail == 0 else 1


if __name__ == "__main__":
    argv = [a for a in sys.argv[1:] if not a.startswith("--")]
    check_stderr = "--no-stderr" not in sys.argv[1:]
    args = []
    for a in argv:
        if os.path.isdir(a):
            args += [os.path.join(a, f) for f in sorted(os.listdir(a)) if f.endswith(".nut")]
        else:
            args.append(a)
    if not args:
        args = [os.path.join(SAMPLES, f) for f in sorted(os.listdir(SAMPLES)) if f.endswith(".nut")]
    sys.exit(main(args, check_stderr))

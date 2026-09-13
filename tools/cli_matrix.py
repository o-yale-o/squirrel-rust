# -*- coding: utf-8 -*-
"""命令行 / 宿主行为矩阵：把 `sq.exe` 当黑盒，逐条比对两侧。

为什么值得单独做：引擎语义有 74 个语料守着，但**CLI 这一层**（参数解析、退出码、
stdout/stderr 路由、`-c` 产物、REPL、坏参数）几乎没有覆盖（只有 REPL 5 条 + BOM 1 条）。
宿主行为也是交付物的一部分：`--max-ops`/`--mem-limit` 还是我们额外加的选项。

比对口径按用例给：`exact`（stdout+stderr+rc）、`rc`（只比退出码，用于"用法文本本来就
该不同"的用例，比如未知参数会列出我们额外的选项）、`stdout_rc`。

用法：
    python tools/cli_matrix.py            # 全部用例
    python tools/cli_matrix.py -v         # 打印每条的输出摘要
"""
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RUST = os.path.join(ROOT, "target", "release", "sq.exe")
REF = os.path.join(ROOT, "build-ref", "bin", "Release", "sq_static.exe")
WORK = os.path.join(ROOT, "tmp-c2", "cli")
# 单条上限：CLI 用例都很快，只有"参考实现 EOF 死循环"那几条要等超时 ——
# 8 秒足够区分"挂住"与"正常退出"，又不至于让 check_all 变慢太多。
TIMEOUT = 8

sys.path.insert(0, os.path.join(ROOT, "tools"))
try:
    from safe_run import suppress_crash_dialogs
    suppress_crash_dialogs()
except Exception:
    pass

GOOD_SCRIPT = 'print("hello\\n");\nprint(vargv.len() + "\\n");\n'
SYNTAX_ERR = 'local x = ;\n'
THROWS = 'throw "boom";\n'
STACKINFO = 'print(getstackinfos(0).line + "\\n");\n'


def setup(files):
    shutil.rmtree(WORK, ignore_errors=True)
    os.makedirs(WORK, exist_ok=True)
    for name, content in (files or {}).items():
        with open(os.path.join(WORK, name), "w", encoding="utf-8", newline="\n") as f:
            f.write(content)


def norm(t):
    t = t.replace("\r", "")
    t = re.sub(r"0x[0-9a-fA-F]{8,}", "0xPTR", t)
    # 注意：**不要**剥离 "Error " 之类前缀 —— 曾经为了去掉 exe 路径写过
    # `^.*?(?=\[)`，结果把 `Error [cannot open the file]` 的 "Error " 也吃掉了，
    # 差点把一处真实差异看成一致。subprocess 拿到的流本来就是干净的。
    return t


def run(exe, argv, stdin_data, cwd):
    try:
        p = subprocess.run([exe] + argv, input=stdin_data, capture_output=True,
                           cwd=cwd, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        return "TIMEOUT", "", ""
    return p.returncode, norm(p.stdout.decode("utf8", "replace")), \
        norm(p.stderr.decode("utf8", "replace"))


# (标签, argv, stdin, 比对口径, 需要准备的文件)
CASES = [
    # ---- 版本 / 用法
    ("version_v", ["-v"], b"", "stdout_rc", None),
    ("help_h", ["-h"], b"", "rc", None),
    ("unknown_flag", ["-x"], b"", "rc", None),
    ("unknown_flag_long", ["-zz"], b"", "rc", None),
    ("only_dash", ["-"], b"", "rc", None),
    ("dash_dash", ["--"], b"", "rc", None),
    ("version_with_script", ["-v", "good.nut"], b"", "stdout_rc", {"good.nut": GOOD_SCRIPT}),
    # ---- 正常执行
    ("run_script", ["good.nut"], b"", "exact", {"good.nut": GOOD_SCRIPT}),
    ("run_script_args", ["good.nut", "a", "b"], b"", "exact", {"good.nut": GOOD_SCRIPT}),
    ("run_script_debug", ["-d", "good.nut"], b"", "exact", {"good.nut": STACKINFO}),
    ("run_with_o_no_c", ["-o", "ignored.cnut", "good.nut"], b"", "exact",
     {"good.nut": GOOD_SCRIPT}),
    ("run_missing_file", ["nope.nut"], b"", "exact", None),
    ("run_syntax_error", ["bad.nut"], b"", "exact", {"bad.nut": SYNTAX_ERR}),
    ("run_throw", ["throws.nut"], b"", "exact", {"throws.nut": THROWS}),
    ("run_directory", ["."], b"", "exact", None),
    ("run_path_with_space", ["my script.nut"], b"", "exact", {"my script.nut": GOOD_SCRIPT}),
    # ---- -c 编译
    ("compile_c", ["-c", "good.nut"], b"", "exact", {"good.nut": GOOD_SCRIPT}),
    ("compile_c_o", ["-c", "-o", "custom.cnut", "good.nut"], b"", "exact",
     {"good.nut": GOOD_SCRIPT}),
    ("compile_c_debug", ["-d", "-c", "good.nut"], b"", "exact", {"good.nut": STACKINFO}),
    ("compile_c_syntax_error", ["-c", "bad.nut"], b"", "exact", {"bad.nut": SYNTAX_ERR}),
    ("compile_c_bad_out", ["-c", "-o", "no_such_dir/x.cnut", "good.nut"], b"", "exact",
     {"good.nut": GOOD_SCRIPT}),
    # 参考实现在 REPL 读到 EOF 时会死循环（MIGRATION.md 记录的已知偏离）：
    # `-c -o` 后面缺值 → 参数循环吃掉 `-o` 后进 REPL → stdin 是空 → 死循环。
    ("compile_c_o_missing_value", ["-c", "-o"], b"", "known-dev", {"good.nut": GOOD_SCRIPT}),
    ("compile_c_missing_file", ["-c", "nope.nut"], b"", "exact", None),
    # ---- .cnut 执行（先用 -c 产出，再由脚本自身跑一次）
    ("run_cnut", ["good.cnut"], b"", "exact", None),
    # ---- 我们的额外长选项（参考实现没有 → 只验证我们这边能跑通）
    ("extra_max_ops", ["--max-ops", "1000000", "good.nut"], b"", "ours",
     {"good.nut": GOOD_SCRIPT}),
    ("extra_max_ops_bad", ["--max-ops", "abc", "good.nut"], b"", "ours",
     {"good.nut": GOOD_SCRIPT}),
    # ---- REPL（喂 stdin 后退出；参考实现 EOF 行为是已知偏离，只比退出码）
    ("repl_expr", [], b"=1+1\nquit()\n", "rc", None),
    ("repl_print", [], b"print(\"x\\n\")\nquit()\n", "rc", None),
    ("repl_empty_stdin", [], b"", "known-dev", None),
    ("repl_error_then_quit", [], b"nope(\nquit()\n", "rc", None),
    ("repl_compile_error", [], b"local = ;\nquit()\n", "rc", None),
    ("repl_half_line", [], b"print(1\n", "known-dev", None),
]


def main():
    verbose = "-v" in sys.argv
    failed = []
    print("%-24s %-8s %s" % ("case", "state", "detail"))
    print("-" * 92)
    for tag, argv, stdin_data, mode, files in CASES:
        # run_cnut 需要先编出 good.cnut：用两侧各自编译，再各自执行
        results = {}
        for side, exe in (("rust", RUST), ("ref", REF)):
            setup(files)
            if tag == "run_cnut":
                subprocess.run([exe, "-c", "good.nut"], capture_output=True, cwd=WORK,
                               timeout=TIMEOUT)
            results[side] = run(exe, argv, stdin_data, WORK)

        r, f = results["rust"], results["ref"]
        if mode == "exact":
            same = r == f
        elif mode == "stdout_rc":
            same = (r[0], r[1]) == (f[0], f[1])
        elif mode == "ours":
            # 我们额外提供的选项（参考实现没有）：只要求我们这边按预期工作
            same = r[0] != "TIMEOUT"
        elif mode == "known-dev":
            # 参考实现的已知偏离：REPL 读到 EOF 会死循环（MIGRATION.md 第 3 节）
            same = f[0] == "TIMEOUT" and r[0] != "TIMEOUT"
        else:
            same = r[0] == f[0]
        state = "OK" if same else "DIFF"
        detail = ""
        if not same:
            failed.append(tag)
            detail = "rust=%s" % (r,)
        elif verbose:
            detail = "rc=%s out=%r" % (r[0], r[1][:60])
        print("%-24s %-8s %s" % (tag, state, detail[:150]))
        if not same:
            print("%-24s %-8s ref =%s" % ("", "", str(f)[:150]))

    # 顺带核对 -c 产物是否字节一致（默认输出名是 out.cnut，显式 -o 各给一个）
    for tag, argv, outfile in (
        ("compile_c", ["-c", "-o", "a.cnut", "good.nut"], "a.cnut"),
        ("compile_d_c", ["-d", "-c", "-o", "b.cnut", "good.nut"], "b.cnut"),
    ):
        outs = {}
        for side, exe in (("rust", RUST), ("ref", REF)):
            setup({"good.nut": STACKINFO if "-d" in argv else GOOD_SCRIPT})
            subprocess.run([exe] + argv, capture_output=True, cwd=WORK, timeout=TIMEOUT)
            p = os.path.join(WORK, outfile)
            outs[side] = open(p, "rb").read() if os.path.exists(p) else None
        same = outs["rust"] is not None and outs["rust"] == outs["ref"]
        if not same:
            failed.append(tag + "(bytes)")
        print("%-24s %-8s %s" % (tag + "(bytes)", "OK" if same else "DIFF",
                                 "%s vs %s bytes" % (
                                     len(outs["rust"] or b""), len(outs["ref"] or b""))))

    shutil.rmtree(WORK, ignore_errors=True)
    print("-" * 92)
    if failed:
        print("失败：%s" % ", ".join(failed))
        return 1
    print("全部通过（%d 条）" % (len(CASES) + 2))
    return 0


if __name__ == "__main__":
    sys.exit(main())

# -*- coding: utf-8 -*-
"""校验「生成式语料」与它的生成器一致：跑一遍所有 tools/gen_*.py，看 tests-corpus/ 有没有变。

为什么需要：语料里很大一部分是生成出来的（数值/正则/委托/线程/序列化…）。如果有人手改了
生成的 .nut、或者改了生成器却忘了重新生成，**差分测试照样会绿**（两侧跑的是同一份被改过的
文件），问题会一直潜伏。这个检查跑一遍所有生成器，比对文件哈希。

顺带：生成器自己带的"自检"（跑一遍产物、出现 !outer: 就报错退出）也会在这一步执行。

用法：
    python tools/check_generators.py
退出码非 0 表示有不一致（会列出文件名）。
"""
import hashlib
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TOOLS = os.path.join(ROOT, "tools")
CORPUS = os.path.join(ROOT, "tests-corpus")
GEN_TIMEOUT = 300


def snapshot():
    """按**内容**哈希（行尾归一化）。

    不能按原始字节比：本仓库 checkout 时 core.autocrlf 会把工作区文件转成 CRLF，
    而生成器写的是 LF —— 逐字节比会把**所有**生成式语料都判成"变了"（踩过）。
    """
    out = {}
    for name in sorted(os.listdir(CORPUS)):
        if name.endswith(".nut"):
            with open(os.path.join(CORPUS, name), "rb") as f:
                data = f.read().replace(b"\r\n", b"\n")
            out[name] = hashlib.sha256(data).hexdigest()
    return out


# 已知漂移：这些文件的**生成物**在某轮被直接改进过（更正确），生成器没跟着改。
# 它们跑生成器会变，属已知债务；这里显式列出，好让"新出现的漂移"仍然能被抓到。
# t_deep 已回填进生成器（旧版是无限尾递归，谁重新生成一次就会挂住两侧，必须修）。
KNOWN_DRIFT = {
    "t_deleg.nut",
    "t_lambda.nut",
    "t_meta.nut",
    "t_suspend_main.nut",
}


def main():
    gens = sorted(n for n in os.listdir(TOOLS)
                  if n.startswith("gen_") and n.endswith(".py"))
    if not gens:
        print("没有找到 tools/gen_*.py")
        return 1
    # 先把语料原样存下来：跑生成器会**改写**文件（已知漂移那几个会被写回旧版），
    # 检查完要还原，否则这个检查自己就把工作区弄脏了。
    saved = {}
    for name in sorted(os.listdir(CORPUS)):
        if name.endswith(".nut"):
            with open(os.path.join(CORPUS, name), "rb") as f:
                saved[name] = f.read()
    before = snapshot()
    print("跑 %d 个生成器…" % len(gens))
    failed = []
    for name in gens:
        r = subprocess.run([sys.executable, os.path.join(TOOLS, name)],
                           capture_output=True, cwd=ROOT, timeout=GEN_TIMEOUT)
        if r.returncode != 0:
            failed.append(name)
            tail = (r.stdout + r.stderr).decode("utf8", "replace").strip().splitlines()
            print("  生成器失败：%s" % name)
            for l in tail[-3:]:
                print("     " + l[:120])
    after = snapshot()

    changed = sorted(n for n in set(before) | set(after)
                     if before.get(n) != after.get(n))
    unexpected = [n for n in changed if n not in KNOWN_DRIFT]
    # 还原：生成器会**重写所有**它负责的文件 —— 即使内容一致，换行风格也可能从
    # checkout 出来的 CRLF 变成 LF，git status 就会显示一片 modified。所以无条件把
    # 跑之前存下的字节写回去（74 个小文件，代价可忽略）。
    restored = 0
    for name, data in saved.items():
        with open(os.path.join(CORPUS, name), "wb") as f:
            f.write(data)
        restored += 1
    print("\n语料文件 %d 个；跑完生成器后变化的：%d（其中已知漂移 %d）"
          % (len(after), len(changed), len(changed) - len(unexpected)))
    for n in changed:
        print("  %s%s" % (n, "（已知漂移）" if n in KNOWN_DRIFT else "  <== 新漂移"))
    if failed or unexpected:
        print("\n不一致：生成器失败 %d 个、新漂移 %d 个" % (len(failed), len(unexpected)))
        print("（改了生成器要重新生成并提交；手改生成式语料会被这里抓到）")
        return 1
    print("生成式语料与生成器一致（%d 个生成器全部可重跑；已知漂移 %d 个，已还原 %d 个文件）"
          % (len(gens), len(changed), restored))
    return 0


if __name__ == "__main__":
    sys.exit(main())

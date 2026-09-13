// 深递归 + 尾递归的**有界**回归用例。
//
// 为什么必须有界：
//   - `return rec(n+1)` 是尾调用，两边的帧都会被复用 → 无限循环（原来这条就是假绿）；
//   - 非尾递归的无限递归在参考实现里没有栈上限，会把内存吃光（实测 32GB 被系统杀掉）。
// 本实现给 VM 栈封了顶并报可捕获的 `stack overflow`（见 STATUS.md「残留差异」）。
function depth(n) {
    if (n <= 0) return 0;
    local x = depth(n - 1);   // 非尾递归：真的建 n 层帧
    return x + 1;
}
print("depth=" + depth(2000) + "\n");

function tail(n) {
    if (n <= 0) return "tail-done";
    return tail(n - 1);       // 尾调用：帧被复用
}
print(tail(20000) + "\n");

// 深帧里抛错：错误处理器必须能在深栈上跑（栈有上限时更要留余量）
function deep_throw(n) {
    if (n <= 0) throw "deep-boom";
    return deep_throw(n - 1);
}
try { deep_throw(500); } catch (e) { print("caught=" + e + "\n"); }

print("DEEP-DONE\n");

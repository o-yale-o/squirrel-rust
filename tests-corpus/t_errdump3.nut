// 运行期错误（非 throw）：CALLSTACK 多帧 + LOCALS 逐层打印
function level3() {
    local z = null;
    return z.missing;
}
function level2(a) {
    local b = a + 1;
    return level3();
}
function level1() {
    return level2(41);
}
level1();

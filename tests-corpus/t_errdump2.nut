// 非字符串错误对象：AN ERROR HAS OCCURRED 只内联真字符串，其余 [unknown]
function inner(v) {
    local copy = v;
    throw copy;
}
function middle(v) {
    local two = v * 2;
    return inner(v);
}
middle(21);

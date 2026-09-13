// stdin/文件流在 EOF 处的原语行为（逐条单独跑，避免前一次调用污染流状态）
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
// 读一个字节，然后再读一个（越界）
s("read1", function() { return stdin.readn('b'); });
s("read2", function() { return stdin.readn('b'); });
s("read3", function() { return stdin.readn('b'); });
s("tell_after", function() { return stdin.tell(); });
s("len_after", function() { return stdin.len(); });
s("eos_after", function() { return stdin.eos(); });
s("eos1", function() { return stdin.eos(); });
s("readblob", function() { return stdin.readblob(4).len(); });

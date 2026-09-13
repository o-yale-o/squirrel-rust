// stdin 的 readblob/readn 在「请求长度 > 可用/含换行」时的行为
function s(tag, f) {
    try { print(tag + "=" + f() + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
s("blob4_len", function() { return stdin.readblob(4).len(); });
s("tell1", function() { return stdin.tell(); });
s("blob4b_len", function() { return stdin.readblob(4).len(); });
s("tell2", function() { return stdin.tell(); });
s("blob100_len", function() { return stdin.readblob(100).len(); });
s("tell3", function() { return stdin.tell(); });
s("eos", function() { return stdin.eos(); });
s("len", function() { return stdin.len(); });

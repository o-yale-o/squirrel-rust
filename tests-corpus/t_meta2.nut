local proxy = {};
proxy.setdelegate({
    function _get(k) { return "got:" + k; }
    function _set(k, v) { ::print("set " + k + "=" + v + "\n"); }
});
print(proxy.anything + "\n");
proxy.foo = 5;

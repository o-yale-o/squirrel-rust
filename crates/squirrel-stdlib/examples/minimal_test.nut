// ↔ 原版 etc/test.nut：供宿主示例调用 foo() 的脚本
function foo(i, f, s)
{
    print("Called foo(), i=" + i + ", f=" + f + ", s='" + s + "'\n");
}

// 顺带验证宿主方向的能力：脚本调用宿主注册的原生函数
host_result <- "unset";
function call_host(a, b) {
    host_result = host_add(a, b);
    return host_result;
}

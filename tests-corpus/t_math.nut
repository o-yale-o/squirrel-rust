// math 库（↔ sqstdmath.cpp）：注册掩码含 this 占位、nparamscheck、返回类型、错误消息
function show(name, f) {
    try {
        local v = f();
        print(name + "=" + typeof v + ":" + v + "\n");
    } catch (e) {
        print(name + "!" + e + "\n");
    }
}
show("sqrt_f", function() { return sqrt(2.0); });
show("sqrt_i", function() { return sqrt(9); });
show("sin", function() { return sin(0.0); });
show("cos", function() { return cos(0.0); });
show("asin", function() { return asin(1.0); });
show("acos", function() { return acos(0.0); });
show("log", function() { return log(2.718281828); });
show("log10", function() { return log10(1000.0); });
show("tan", function() { return tan(1.0); });
show("atan", function() { return atan(1.0); });
show("atan2", function() { return atan2(1.0, 2.0); });
show("pow", function() { return pow(2.0, 10.0); });
show("pow_frac", function() { return pow(2.0, -1.0); });
show("floor", function() { return floor(-2.5); });
show("floor_t", function() { return typeof floor(2.5); });
show("ceil", function() { return ceil(-2.5); });
show("ceil_t", function() { return typeof ceil(2.5); });
show("exp", function() { return exp(2.0); });
show("fabs", function() { return fabs(-3.5); });
show("abs_i", function() { return abs(-7); });
show("abs_f", function() { return abs(-7.9); });
show("abs_t", function() { return typeof abs(-7.9); });
show("abs_big", function() { return abs(-3000000000); });
show("srand_ret", function() { return srand(1); });
show("rand1", function() { srand(1); return rand(); });
show("rand_seq", function() {
    srand(42);
    return "" + rand() + "," + rand() + "," + rand();
});
show("rand_alone", function() { return typeof rand(); });
show("pi", function() { return PI; });
show("randmax", function() { return RAND_MAX; });
show("sqrt_noargs", function() { return sqrt(); });
show("sqrt_badtype", function() { return sqrt("x"); });
show("pow_onearg", function() { return pow(1.0); });
show("rand_extra", function() { return rand(1); });
print("done\n");

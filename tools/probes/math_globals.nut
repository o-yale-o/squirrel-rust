function t(name, f) {
    try { local v = f(); print(name + " OK " + typeof v + " " + v + "\n"); }
    catch (e) { print(name + " FAIL " + e + "\n"); }
}
t("sqrt", function() { return sqrt(4.0); });
t("sin", function() { return sin(0.0); });
t("cos", function() { return cos(0.0); });
t("asin", function() { return asin(1.0); });
t("acos", function() { return acos(1.0); });
t("log", function() { return log(1.0); });
t("log10", function() { return log10(100.0); });
t("tan", function() { return tan(0.0); });
t("atan", function() { return atan(1.0); });
t("atan2", function() { return atan2(1.0, 1.0); });
t("pow", function() { return pow(2.0, 3.0); });
t("floor", function() { return floor(2.7); });
t("ceil", function() { return ceil(2.1); });
t("exp", function() { return exp(1.0); });
t("srand", function() { srand(1); return "seeded"; });
t("rand", function() { return rand(); });
t("fabs", function() { return fabs(-2.5); });
t("abs", function() { return abs(-7); });
t("PI", function() { return PI; });
t("RAND_MAX", function() { return RAND_MAX; });

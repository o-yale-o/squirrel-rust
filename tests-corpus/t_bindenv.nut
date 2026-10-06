// 自动生成：bindenv × 编译闭包矩阵（tools/gen_bindenv.py）
//
// **只收两侧一致的子集**：compilestring 闭包 bindenv 后的**裸自由标识符**
// 在参考实现里不读 env（读 root），本实现读 env——该分歧 4 种形态见
// MIGRATION.md 的 bindenv 行（未修，TODO P28）。一致子集钉住：
// 普通闭包 bindenv 正常、编译闭包不 bindenv 时 this=root、
// bindenv 返回新闭包、bindenv(root) 后读不到 env 的槽。
function t(tag, f) {
    try { local v = f(); print(tag + "=" + d(v) + "|" + typeof v + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}
function d(x) { return "" + x; }
env0 <- { };

t("norm_bound", function() { local env = { marker = "env-ok" }; local probe = function() { return marker; }; local g = probe.bindenv(env); return g(); });
t("plain_unbound", function() { local env = { marker = "env-ok" }; local f = compilestring("return marker", "D"); return f(); });
t("bind_returns_new", function() { local f = compilestring("return 1", "CC7"); local g = f.bindenv(env0); return (f == g) ? "same" : "new"; });
t("cs_root_read", function() { local f = compilestring("return typeof print", "CC4"); return f(); });
t("cs_bind_root", function() { local f = compilestring("return \"marker=\" + ((\"marker\" in this) ? this.marker : \"?\")", "CC6"); local g = f.bindenv(getroottable()); return g(); });
t("this_is_env", function() { local env = { marker = "env-ok" }; local probe = function() { return (this == env) + "/" + (this == getroottable()); }; local g = probe.bindenv(env); return g(); });
print("BINDENV-DONE\n");

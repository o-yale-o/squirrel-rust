// split / escape 细节（↔ sqstdstring.cpp _string_split / _string_escape）
local a = split("a,b,,c", ",");
print("len=" + a.len() + " ");
foreach (v in a) print("[" + v + "]");
print("\n");

local b = split("a,b,,c", ",", true);
print("skip=" + b.len() + " ");
foreach (v in b) print("[" + v + "]");
print("\n");

local c = split(",a,", ",");
print("edges=" + c.len() + " ");
foreach (v in c) print("[" + v + "]");
print("\n");

print("multi=" + split("a::b:c", ":;").len() + "\n");
print("none=" + split("abc", ",").len() + "\n");
try { split("abc", ""); } catch (e) { print("empty: " + e + "\n"); }
try { split(1, ","); } catch (e) { print("type: " + e + "\n"); }

// escape：可打印字符原样（引号/反斜杠加转义），控制字符与 0 走 \xNN / \0
print(escape("hello\x01world") + "\n");
print(escape("a\tb\nc") + "\n");
print(escape("a\\b'c\"d") + "\n");
print(escape("\x7f") + "\n");
print(escape("aaaaaaaaa\xe9") + "\n");
print(escape("") + "|" + escape("").len() + "\n");

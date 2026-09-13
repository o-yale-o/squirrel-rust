// format/printf 的 %e/%E/%f/%g/%G 精度与错误路径（↔ CRT vsnprintf）
print(format("%e|%E|%f|%g|%G", 1234.5, 1234.5, 1234.5, 1234.5, 1234.5) + "\n");
print(format("%.2e|%.0e|%.3f|%.2g|%.10g", 1234.5, 1234.5, 1234.5, 1234.5, 1.0 / 3.0) + "\n");
print(format("%e|%e|%e", 0.0, -0.0, 1.0) + "\n");
print(format("%e|%e", 0.000123, 123456789.0) + "\n");
print(format("%d|%5d|%-5d|%05d", 42, 42, 42, 42) + "|\n");
print(format("%s|%10s|%-10s|", "ab", "ab", "ab") + "|\n");
print(format("%x|%X|%o|%c|%%", 255, 255, 8, 65) + "\n");
print(format("%.3s|%.2d", "abcdef", 5) + "\n");
try { format(); } catch (e) { print("noargs: " + e + "\n"); }
try { format(42); } catch (e) { print("badtype: " + e + "\n"); }
try { format("%q", 1); } catch (e) { print("badconv: " + e + "\n"); }
printf("%s-%d\n", "pf", 7);

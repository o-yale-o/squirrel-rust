for(local i = 0; i < 3; i++) {
    try {
        if (i == 1) continue;
        if (i == 2) break;
        print("i=" + i + "\n");
    } catch(e) { print("c"); }
}
print("---\n");
try {
    try {
        throw "inner";
    } catch(e) {
        throw "outer:" + e;
    }
} catch(e2) { print(e2 + "\n"); }

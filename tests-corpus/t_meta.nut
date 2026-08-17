class V {
    x = 0; y = 0;
    constructor(a, b) { x = a; y = b; }
    function _add(o) { return V(x + o.x, y + o.y); }
    function _sub(o) { return V(x - o.x, y - o.y); }
    function _mul(o) { return V(x * o, y * o); }
    function _div(o) { return V(x / o, y / o); }
    function _modulo(o) { return V(x % o, y % o); }
    function _unm() { return V(-x, -y); }
    function _tostring() { return "(" + x + "," + y + ")"; }
    function _cmp(o) { if (x < o.x) return -1; if (x > o.x) return 1; return 0; }
    function _typeof() { return "V"; }
}
local a = V(1,2), b = V(3,4);
print((a + b) + "\n");
print((b - a) + "\n");
print((a * 3) + "\n");
print((b / 2) + "\n");
print((-a) + "\n");
print(a < b); print(" ");
print(a > b); print("\n");
print(typeof a); print("\n");
print(a <=> b); print("\n");

// '#' 标志作用在 inf/nan 上的行为（各精度）
function rt(x) { return x; }
local inf = rt(1.0) / rt(0.0);
local ninf = -inf;
local qnan = rt(0.0) / rt(0.0);
local pqnan = -qnan;
local names = ["inf", "-inf", "qnan", "pqnan"];
local vals = [inf, ninf, qnan, pqnan];
local verbs = ["%#f", "%#.0f", "%#.1f", "%#.3f", "%#e", "%#.0e", "%#.1e",
               "%#g", "%#.0g", "%#.1g", "%#.3g", "%#G", "%#E", "%#F"];
foreach (v in verbs) {
    local line = v + " : ";
    foreach (x in vals) {
        line += "[" + format(v, x) + "]";
    }
    print(line + "\n");
}

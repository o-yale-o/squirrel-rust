// date：固定时间戳（不依赖墙上时钟），tm_mon/tm_wday/tm_yday 约定
function dump(tag, d) {
    print(tag + ": " + d.year + "-" + d.month + "-" + d.day + " " + d.hour + ":" + d.min + ":" + d.sec
        + " wday=" + d.wday + " yday=" + d.yday + "\n");
}
dump("epoch-l", date(0, 'l'));
dump("epoch-u", date(0, 'u'));
dump("fixed-l", date(1700000000));
dump("fixed-u", date(1700000000, 'u'));
dump("leap-l", date(951782400, 'u'));
dump("y2k-u", date(946684800, 'u'));
dump("big-u", date(4102444800, 'u'));
print("typeof: " + typeof date(0).sec + typeof date(0).month + typeof date(0).year + "\n");
// CRT 范围：gmtime 自 −43200 起，localtime 不接受负值，超大值一律失败
function tryDate(tag, ts, fmt) {
    try {
        local d = date(ts, fmt);
        print(tag + " ok " + d.year + "\n");
    } catch (e) {
        print(tag + " " + e + "\n");
    }
}
tryDate("min-u", -43200, 'u');
tryDate("under-u", -43201, 'u');
tryDate("neg-l", -1, 'l');
tryDate("huge-u", 999999999999, 'u');
tryDate("huge-l", 999999999999, 'l');

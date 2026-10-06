// 自动生成：表迭代顺序 × 随机变更序列矩阵（tools/gen_iterorder.py）
//
// 每条用例是一段随机的 插入/覆盖/删除 序列（混合 int/string/float/bool 键型，
// 插入量足以触发多次扩容），最后 foreach 输出键值顺序——**顺序依赖哈希表
// 布局**，与参考逐字节一致（生成式差分：80 个随机程序零差异）。
function d(x) { return "" + x; }
function t(tag, f) {
    try { local v = f(); print(tag + "=" + d(v) + "|" + typeof v + "\n"); }
    catch (e) { print(tag + "!" + e + "\n"); }
}

t("seq1", function() {
    local t = {};
    t[130] = 15;
    t["3.5"] <- 83;
    t["bha"] <- 49;
    t[14] <- 34;
    t["bfa"] = 2;
    t[12] <- 87;
    t[true] <- 92;
    t["hhd"] <- 44;
    t["hea"] <- 53;
    try { delete t[95]; } catch (e) { }
    t[3.0] = 95;
    t[true] <- 64;
    try { delete t["eeh"]; } catch (e) { }
    try { delete t[true]; } catch (e) { }
    t[15] = 31;
    t[true] = 53;
    t[3] = 89;
    t[-4] = 56;
    t[398] = 20;
    t[true] <- 47;
    t[15] <- 5;
    t[true] <- 82;
    t["add"] <- 51;
    t[2] <- 34;
    t[12] = 94;
    t["k16"] <- 99;
    t[true] = 7;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq2", function() {
    local t = {};
    try { delete t[2]; } catch (e) { }
    t["k9"] <- 32;
    t[18] = 87;
    t[true] <- 81;
    t[3] <- 56;
    t[0] <- 46;
    t[1] <- 54;
    try { delete t["k17"]; } catch (e) { }
    t["acf"] <- 22;
    t[3] <- 86;
    t["3.5"] = 94;
    t[4] <- 45;
    t["1.5"] <- 96;
    t["5.5"] <- 67;
    t[15.0] <- 64;
    t[5] <- 58;
    try { delete t["2.5"]; } catch (e) { }
    t["3.5"] = 84;
    t[-3] <- 78;
    t["2.5"] <- 38;
    try { delete t[true]; } catch (e) { }
    t["hfb"] <- 43;
    t["baa"] = 34;
    t[386] = 66;
    t[7.0] <- 26;
    try { delete t[13]; } catch (e) { }
    try { delete t[1]; } catch (e) { }
    t["k7"] <- 86;
    t[488] <- 8;
    t[11] <- 32;
    t["k5"] <- 66;
    t[true] = 75;
    t["caa"] <- 44;
    try { delete t[146]; } catch (e) { }
    t[9] <- 57;
    t[8] = 96;
    t["k15"] <- 28;
    t[-4] <- 3;
    t["k16"] <- 74;
    t["4.5"] = 41;
    t[-1] <- 33;
    t[true] = 83;
    t["k1"] <- 32;
    t["k5"] <- 12;
    t["add"] <- 91;
    t[2.0] <- 75;
    t[-1] <- 87;
    t[4] <- 4;
    t["k3"] <- 65;
    t["bba"] = 23;
    t[111] = 3;
    t["3.5"] <- 39;
    t[true] <- 27;
    t["gga"] = 74;
    t[true] = 67;
    t[339] = 61;
    t[312] <- 46;
    t[-1] <- 2;
    try { delete t[true]; } catch (e) { }
    t[6.0] <- 99;
    t[14] = 7;
    t["3.5"] <- 26;
    try { delete t[2]; } catch (e) { }
    t[-1] <- 92;
    t["1.5"] <- 14;
    t[true] = 91;
    t[1] <- 15;
    t[41] <- 78;
    try { delete t[true]; } catch (e) { }
    try { delete t[12]; } catch (e) { }
    t["0.5"] = 92;
    t[11.0] = 58;
    t[-1] <- 61;
    t["5.5"] <- 93;
    try { delete t["5.5"]; } catch (e) { }
    t["che"] <- 70;
    t[299] <- 93;
    try { delete t[36]; } catch (e) { }
    t["k13"] <- 8;
    t[4] = 37;
    t[2] <- 22;
    t[79] <- 69;
    try { delete t[true]; } catch (e) { }
    t["ecc"] <- 59;
    try { delete t["gfc"]; } catch (e) { }
    t[19] <- 49;
    try { delete t["k12"]; } catch (e) { }
    t["2.5"] <- 51;
    t[true] <- 90;
    t[3] = 42;
    t[389] = 92;
    t["gga"] <- 40;
    t["5.5"] <- 22;
    try { delete t[12]; } catch (e) { }
    try { delete t[true]; } catch (e) { }
    try { delete t[199]; } catch (e) { }
    try { delete t["edh"]; } catch (e) { }
    try { delete t["k0"]; } catch (e) { }
    t[true] = 61;
    t["k14"] <- 91;
    t[179] <- 0;
    try { delete t[386]; } catch (e) { }
    t[2] = 34;
    try { delete t["0.5"]; } catch (e) { }
    try { delete t[-3]; } catch (e) { }
    t[true] = 32;
    t["k1"] = 20;
    t["5.5"] <- 37;
    t[17.0] <- 59;
    try { delete t[-5]; } catch (e) { }
    t[true] <- 72;
    t[15.0] <- 83;
    t[2.0] <- 33;
    try { delete t[10.0]; } catch (e) { }
    t[20.0] = 82;
    t[260] <- 81;
    t["k16"] <- 80;
    t[7] <- 58;
    t[1.0] = 14;
    t[true] <- 46;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq3", function() {
    local t = {};
    t["k11"] = 77;
    t[310] <- 1;
    try { delete t["2.5"]; } catch (e) { }
    t["hhg"] = 81;
    try { delete t["cga"]; } catch (e) { }
    t[81] = 97;
    try { delete t[9]; } catch (e) { }
    t[15.0] = 76;
    t[true] = 91;
    t[true] = 50;
    t["1.5"] = 46;
    t["k15"] <- 27;
    t[true] <- 99;
    t[13.0] = 64;
    try { delete t[3]; } catch (e) { }
    t["fae"] = 77;
    t["k10"] = 69;
    try { delete t[365]; } catch (e) { }
    t[9.0] = 15;
    t["0.5"] <- 44;
    try { delete t[true]; } catch (e) { }
    try { delete t[9]; } catch (e) { }
    t[true] <- 15;
    t[12] <- 91;
    t[16.0] = 30;
    try { delete t[0.0]; } catch (e) { }
    t[6] <- 52;
    t[4.0] <- 88;
    t[0] <- 46;
    try { delete t[true]; } catch (e) { }
    t[true] <- 82;
    try { delete t[317]; } catch (e) { }
    try { delete t[13.0]; } catch (e) { }
    t["ege"] = 66;
    t[-5] <- 53;
    try { delete t[-5]; } catch (e) { }
    t["k1"] <- 81;
    t["2.5"] = 86;
    try { delete t[15.0]; } catch (e) { }
    t[0] <- 47;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq4", function() {
    local t = {};
    t[true] <- 61;
    t[10] <- 51;
    t[1.0] <- 28;
    t[-1] <- 99;
    t[134] <- 27;
    try { delete t[20]; } catch (e) { }
    try { delete t[6.0]; } catch (e) { }
    t[20.0] <- 93;
    try { delete t[-4]; } catch (e) { }
    try { delete t[5]; } catch (e) { }
    t["cdh"] <- 35;
    t[0.0] <- 37;
    t[16.0] = 24;
    t[13.0] <- 57;
    t[8.0] <- 5;
    t["5.5"] <- 35;
    t["5.5"] <- 43;
    t["bgd"] <- 81;
    t[5.0] = 45;
    t[5] <- 71;
    t[-4] <- 7;
    t[18.0] = 78;
    try { delete t[169]; } catch (e) { }
    try { delete t[14.0]; } catch (e) { }
    t[-4] <- 36;
    t[-5] = 41;
    t["k20"] <- 52;
    try { delete t[150]; } catch (e) { }
    t["2.5"] = 17;
    t["k10"] <- 73;
    t[14] <- 21;
    t[-1] <- 73;
    t["gdb"] <- 7;
    t["k19"] <- 86;
    try { delete t[17]; } catch (e) { }
    t["fab"] <- 67;
    t[true] <- 83;
    try { delete t["1.5"]; } catch (e) { }
    t[true] <- 62;
    t[true] <- 56;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq5", function() {
    local t = {};
    t[5] <- 67;
    t["1.5"] <- 83;
    t["k3"] <- 47;
    t["gbd"] <- 1;
    t[true] = 35;
    t[true] <- 20;
    t[71] = 79;
    t["k4"] = 0;
    try { delete t[6]; } catch (e) { }
    t["k5"] = 37;
    t["dcd"] <- 49;
    t[1] <- 21;
    try { delete t[2.0]; } catch (e) { }
    t[19] <- 86;
    t[158] = 45;
    try { delete t["5.5"]; } catch (e) { }
    t["3.5"] <- 90;
    t[0.0] <- 95;
    t[true] <- 2;
    t[true] <- 46;
    t[14] <- 5;
    t["bdh"] = 44;
    t[14.0] <- 13;
    t[-1] = 4;
    t[106] <- 43;
    t[-3] <- 43;
    t[159] <- 87;
    t["k2"] <- 80;
    t[15.0] <- 20;
    t[307] = 68;
    try { delete t[7]; } catch (e) { }
    t[-1] = 58;
    t["k1"] = 81;
    t["2.5"] <- 26;
    t["k20"] <- 52;
    t[true] <- 47;
    t[true] <- 37;
    t["k16"] <- 58;
    try { delete t[2]; } catch (e) { }
    t["3.5"] <- 18;
    t["k20"] <- 63;
    try { delete t["k2"]; } catch (e) { }
    t[-4] <- 99;
    t[9] <- 46;
    t[15.0] = 33;
    t[10.0] = 83;
    t[15] <- 70;
    t[5] = 35;
    t[0] <- 35;
    t[true] = 44;
    try { delete t["k14"]; } catch (e) { }
    try { delete t[3]; } catch (e) { }
    t["k6"] <- 46;
    try { delete t["2.5"]; } catch (e) { }
    t[true] = 21;
    t[true] = 38;
    t[0.0] = 25;
    t["4.5"] <- 83;
    t["k20"] <- 91;
    t["1.5"] <- 21;
    t["k3"] <- 40;
    try { delete t["1.5"]; } catch (e) { }
    t[true] <- 59;
    t[302] <- 26;
    t[-5] <- 44;
    t[13.0] <- 14;
    t[-5] = 70;
    t[151] = 69;
    t[11.0] <- 16;
    t[2] <- 18;
    t[true] <- 72;
    t["cbf"] <- 84;
    t[166] <- 72;
    try { delete t["k10"]; } catch (e) { }
    t[true] = 54;
    t["heh"] <- 90;
    t["k19"] <- 76;
    t[15.0] <- 32;
    t[-1] <- 62;
    try { delete t["k15"]; } catch (e) { }
    t["1.5"] <- 37;
    t["k12"] <- 1;
    t[7.0] <- 60;
    t["hec"] = 92;
    try { delete t[15.0]; } catch (e) { }
    t["4.5"] = 82;
    try { delete t[8]; } catch (e) { }
    t[9.0] = 68;
    t[16.0] = 3;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq6", function() {
    local t = {};
    t[248] = 97;
    t[4] <- 84;
    t[0] = 98;
    t["1.5"] <- 93;
    try { delete t[98]; } catch (e) { }
    t[19.0] = 87;
    t[true] <- 42;
    try { delete t[1]; } catch (e) { }
    try { delete t["5.5"]; } catch (e) { }
    t["eba"] <- 75;
    try { delete t[2]; } catch (e) { }
    try { delete t["afd"]; } catch (e) { }
    t[11.0] = 75;
    t["bee"] <- 92;
    t["1.5"] <- 17;
    t[6] = 99;
    t[3] <- 85;
    t[11.0] = 48;
    t[4.0] <- 86;
    try { delete t[5]; } catch (e) { }
    t[true] <- 12;
    try { delete t["bhh"]; } catch (e) { }
    t[true] = 9;
    t["2.5"] <- 90;
    try { delete t[98]; } catch (e) { }
    t[14.0] = 62;
    t["k0"] = 2;
    t[18.0] <- 46;
    t["2.5"] <- 66;
    t[true] <- 53;
    t[19.0] = 60;
    t["1.5"] = 92;
    t[20.0] <- 59;
    t["k19"] <- 48;
    t[10.0] = 82;
    t["5.5"] <- 20;
    try { delete t[17.0]; } catch (e) { }
    t["0.5"] <- 23;
    t[349] <- 91;
    t["4.5"] <- 7;
    try { delete t["k8"]; } catch (e) { }
    t[true] = 67;
    t["1.5"] <- 12;
    t["k20"] <- 31;
    t["k1"] = 80;
    t[23] = 82;
    t[322] <- 3;
    t["k19"] = 10;
    t[20.0] <- 57;
    t[20] <- 17;
    try { delete t["hhh"]; } catch (e) { }
    t[15.0] <- 91;
    t[11.0] = 36;
    t[2] <- 64;
    try { delete t[8.0]; } catch (e) { }
    try { delete t[true]; } catch (e) { }
    t[-5] <- 91;
    t["acf"] <- 47;
    t["gfb"] <- 20;
    t[204] = 70;
    t["5.5"] <- 9;
    t[3] <- 3;
    t["k5"] <- 36;
    t[true] = 28;
    try { delete t["k13"]; } catch (e) { }
    t[-2] = 71;
    t[true] = 24;
    t[true] = 67;
    t[true] <- 67;
    t["k7"] = 82;
    t[-1] <- 85;
    t["k5"] <- 88;
    t[true] = 9;
    t[15] <- 97;
    t[344] <- 47;
    t[17] = 28;
    t["ebh"] = 9;
    t[5] = 84;
    t[359] <- 22;
    t[13] <- 83;
    t[14.0] = 29;
    t["eae"] <- 67;
    t[3] = 11;
    t[11] = 46;
    t["aff"] = 55;
    t[11] = 17;
    t[-4] <- 62;
    t[2.0] <- 86;
    t[7.0] = 16;
    t["k7"] <- 84;
    t[true] <- 72;
    t["2.5"] <- 32;
    t[6] <- 97;
    t["k18"] <- 13;
    try { delete t[16]; } catch (e) { }
    t["k18"] <- 50;
    t["1.5"] <- 82;
    try { delete t["k14"]; } catch (e) { }
    try { delete t[10]; } catch (e) { }
    t[-3] <- 36;
    t[-2] <- 0;
    t["k2"] = 52;
    t["bdg"] = 52;
    t["k16"] <- 6;
    t["2.5"] <- 42;
    try { delete t[true]; } catch (e) { }
    try { delete t[133]; } catch (e) { }
    t[301] <- 77;
    t["k16"] = 64;
    t[44] <- 85;
    t["ffc"] = 93;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq7", function() {
    local t = {};
    try { delete t[true]; } catch (e) { }
    t[420] = 68;
    t[16] <- 27;
    t[true] <- 53;
    t[282] <- 54;
    t[485] <- 28;
    t[18] = 74;
    t["ace"] <- 53;
    t[292] <- 39;
    t["k3"] = 74;
    t["fbb"] = 72;
    t["hgf"] <- 59;
    t["2.5"] = 38;
    t["k7"] <- 10;
    t["2.5"] = 93;
    t[60] <- 65;
    t[-3] <- 62;
    t[391] <- 71;
    t[0] = 88;
    t["4.5"] <- 58;
    t[483] <- 34;
    t[31] <- 93;
    t["2.5"] = 91;
    t[-5] <- 59;
    t[252] <- 7;
    t[4.0] <- 94;
    t[true] <- 63;
    t["3.5"] <- 70;
    t["k13"] <- 70;
    t[true] <- 45;
    t[true] = 29;
    t["k4"] <- 29;
    t[15] = 75;
    t[0.0] <- 18;
    t[4] <- 72;
    t["k16"] <- 79;
    t[14] = 99;
    try { delete t[true]; } catch (e) { }
    t[true] <- 13;
    t[true] <- 7;
    t["hcb"] <- 43;
    t[0] = 72;
    t[485] <- 46;
    t[447] = 26;
    t["k20"] = 32;
    try { delete t[2]; } catch (e) { }
    t["3.5"] <- 61;
    t[73] <- 13;
    t[15.0] = 88;
    t[6] <- 67;
    t[16] <- 38;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq8", function() {
    local t = {};
    t[true] <- 16;
    t[2] <- 17;
    t["gah"] <- 62;
    t["4.5"] <- 24;
    try { delete t[true]; } catch (e) { }
    t["aeg"] <- 60;
    try { delete t[true]; } catch (e) { }
    t[3.0] = 8;
    t[true] <- 13;
    t[-2] = 88;
    t["cba"] <- 62;
    t["k18"] = 58;
    t["2.5"] = 71;
    t[true] <- 17;
    t[272] <- 88;
    t[5] <- 63;
    t["ecf"] <- 67;
    t[413] <- 64;
    try { delete t["fda"]; } catch (e) { }
    t["eag"] <- 32;
    t[true] <- 22;
    t[85] = 84;
    try { delete t[6]; } catch (e) { }
    try { delete t["k9"]; } catch (e) { }
    t[true] <- 43;
    try { delete t[true]; } catch (e) { }
    try { delete t["k13"]; } catch (e) { }
    t[true] = 53;
    t[147] <- 91;
    t[339] <- 13;
    t[-3] = 64;
    t["k19"] <- 8;
    t["k10"] <- 49;
    t[380] = 38;
    t[-5] <- 82;
    try { delete t["aac"]; } catch (e) { }
    t[true] <- 56;
    t["dhh"] <- 19;
    t[297] <- 92;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq9", function() {
    local t = {};
    t[4.0] = 23;
    try { delete t[10]; } catch (e) { }
    t[171] <- 70;
    try { delete t[12]; } catch (e) { }
    t["5.5"] <- 54;
    t["abc"] <- 64;
    try { delete t[396]; } catch (e) { }
    t[470] = 37;
    t["gbe"] <- 26;
    t[-5] <- 25;
    t[13] = 7;
    try { delete t["1.5"]; } catch (e) { }
    t[true] <- 94;
    try { delete t[493]; } catch (e) { }
    t[391] = 74;
    t["fab"] <- 17;
    t[16] <- 10;
    t["gbg"] = 24;
    t[358] = 74;
    t[1] <- 34;
    t["k8"] = 94;
    try { delete t[3]; } catch (e) { }
    try { delete t[true]; } catch (e) { }
    try { delete t["caf"]; } catch (e) { }
    try { delete t[444]; } catch (e) { }
    t[2] <- 26;
    t[12] <- 77;
    t[true] <- 14;
    t["2.5"] <- 41;
    try { delete t[20.0]; } catch (e) { }
    t[0] <- 48;
    t[true] = 63;
    t["bgd"] <- 12;
    try { delete t[true]; } catch (e) { }
    t[-3] <- 85;
    t[0.0] <- 6;
    try { delete t["k8"]; } catch (e) { }
    try { delete t[310]; } catch (e) { }
    t[1] <- 57;
    t[true] <- 37;
    t[-1] = 18;
    t[4.0] <- 11;
    t[3.0] = 8;
    t["k17"] <- 97;
    t["k8"] = 98;
    t["2.5"] = 12;
    try { delete t[16.0]; } catch (e) { }
    t[83] <- 99;
    t[6] = 22;
    t[true] <- 10;
    t["4.5"] <- 33;
    t[18] <- 74;
    try { delete t[16.0]; } catch (e) { }
    t[449] <- 82;
    t[2.0] = 92;
    t["k3"] <- 12;
    t["0.5"] = 62;
    try { delete t[19]; } catch (e) { }
    t["hdf"] <- 93;
    t["1.5"] = 4;
    t[313] = 6;
    t[true] = 75;
    t["acb"] <- 8;
    t["4.5"] <- 68;
    t[170] <- 10;
    t[14] = 57;
    t[true] = 61;
    try { delete t[457]; } catch (e) { }
    t["hcd"] <- 28;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq10", function() {
    local t = {};
    t["4.5"] <- 1;
    t["2.5"] <- 83;
    try { delete t[16]; } catch (e) { }
    t[127] <- 95;
    try { delete t[13]; } catch (e) { }
    try { delete t[1]; } catch (e) { }
    t[14.0] <- 22;
    t[-3] = 58;
    t["hga"] = 74;
    t["k6"] <- 38;
    t["fhg"] <- 60;
    t[3] <- 20;
    try { delete t[true]; } catch (e) { }
    t[15] <- 38;
    try { delete t[273]; } catch (e) { }
    try { delete t[76]; } catch (e) { }
    t[4] <- 19;
    t[226] <- 21;
    try { delete t["fgg"]; } catch (e) { }
    t[4.0] <- 79;
    t[136] <- 58;
    t["k5"] <- 61;
    t[1] = 28;
    t[10] <- 40;
    t[14.0] <- 51;
    t[true] = 63;
    t["ehh"] = 8;
    t["3.5"] <- 17;
    t[3] <- 49;
    t["k17"] <- 56;
    t[true] <- 4;
    try { delete t["4.5"]; } catch (e) { }
    t[-4] <- 19;
    t["3.5"] <- 17;
    t[true] <- 99;
    t["2.5"] <- 92;
    t["3.5"] <- 42;
    t["k7"] = 41;
    t["hab"] = 37;
    t["eba"] <- 76;
    try { delete t[418]; } catch (e) { }
    t["1.5"] <- 46;
    t["fcb"] = 75;
    t["beh"] <- 95;
    t[true] = 57;
    t["efb"] = 80;
    t[6] = 54;
    t[true] = 83;
    t[true] <- 11;
    try { delete t[-1]; } catch (e) { }
    t[true] <- 97;
    t[14.0] <- 88;
    t["4.5"] <- 33;
    t[0] <- 92;
    t["3.5"] <- 75;
    t[1] <- 21;
    try { delete t[true]; } catch (e) { }
    t[304] = 93;
    try { delete t[18]; } catch (e) { }
    t[-4] <- 94;
    t[-1] <- 77;
    t["k9"] <- 12;
    t[7] = 72;
    t[true] = 24;
    t["4.5"] = 2;
    t[4] <- 24;
    t[-5] <- 84;
    t[1] = 72;
    t[true] <- 41;
    t["k5"] = 15;
    t[-3] = 3;
    t[-4] = 50;
    t[-5] <- 18;
    try { delete t["5.5"]; } catch (e) { }
    t["5.5"] <- 56;
    try { delete t["k11"]; } catch (e) { }
    try { delete t[3]; } catch (e) { }
    t[true] <- 24;
    t["k1"] <- 23;
    t["gfg"] = 6;
    t["k9"] = 28;
    t["k3"] <- 81;
    t["k9"] = 5;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq11", function() {
    local t = {};
    try { delete t["3.5"]; } catch (e) { }
    t["chc"] <- 12;
    t["k2"] <- 68;
    try { delete t[19]; } catch (e) { }
    try { delete t["5.5"]; } catch (e) { }
    t["k19"] = 1;
    try { delete t[30]; } catch (e) { }
    t["ahf"] <- 56;
    t["deh"] = 0;
    t["5.5"] = 35;
    t[362] <- 32;
    t["eab"] <- 72;
    t[true] = 13;
    try { delete t[true]; } catch (e) { }
    t[0] <- 27;
    t[15] <- 48;
    try { delete t[true]; } catch (e) { }
    t["efb"] <- 39;
    t[true] <- 97;
    try { delete t["k7"]; } catch (e) { }
    t[1] = 59;
    t["k17"] = 24;
    t["cgg"] <- 14;
    t["aee"] <- 2;
    t[true] <- 77;
    t[21] = 18;
    t[0.0] <- 98;
    t[12.0] = 9;
    t["daf"] <- 47;
    t["k18"] = 61;
    try { delete t["k12"]; } catch (e) { }
    t["k9"] <- 29;
    try { delete t["dcd"]; } catch (e) { }
    t[true] = 61;
    t[true] = 6;
    t[16] <- 32;
    t[true] <- 32;
    t["2.5"] <- 66;
    t[64] <- 29;
    t[143] <- 27;
    try { delete t["abe"]; } catch (e) { }
    t["aac"] <- 36;
    t["k2"] <- 46;
    t["2.5"] <- 84;
    t["k18"] = 4;
    try { delete t["2.5"]; } catch (e) { }
    t[0] = 76;
    t["0.5"] = 93;
    t["k2"] <- 9;
    t[-5] <- 94;
    t["k10"] = 45;
    t["0.5"] <- 53;
    try { delete t[15]; } catch (e) { }
    t[true] = 48;
    t[41] = 11;
    t[13.0] = 93;
    t["3.5"] <- 59;
    try { delete t[265]; } catch (e) { }
    t[9] = 76;
    t[7] <- 89;
    t["2.5"] <- 1;
    t["k19"] <- 25;
    t[5] <- 56;
    t["fge"] <- 25;
    t["dgd"] = 74;
    try { delete t["cch"]; } catch (e) { }
    t[2] <- 35;
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
t("seq12", function() {
    local t = {};
    t[-3] <- 48;
    t[2] <- 35;
    t["5.5"] = 76;
    t[19] <- 18;
    try { delete t[-3]; } catch (e) { }
    t["adb"] <- 65;
    t[5] = 51;
    t[9] = 7;
    try { delete t["bgh"]; } catch (e) { }
    t[true] <- 17;
    t["k1"] <- 71;
    t[205] <- 77;
    t["3.5"] <- 77;
    t[20] <- 92;
    t["eff"] <- 88;
    t[3.0] <- 32;
    t[0] <- 65;
    t["k0"] = 48;
    try { delete t[true]; } catch (e) { }
    t[7] <- 86;
    t[true] <- 26;
    t[385] = 70;
    t[356] <- 35;
    t[15.0] <- 44;
    t[16] = 58;
    t[-1] <- 58;
    t["1.5"] <- 21;
    t[-3] <- 24;
    t["aef"] = 22;
    t[16] <- 5;
    t[184] <- 69;
    t[10] <- 85;
    try { delete t[117]; } catch (e) { }
    t["3.5"] = 80;
    t["5.5"] <- 20;
    t[8] <- 42;
    try { delete t["5.5"]; } catch (e) { }
    t[16.0] = 32;
    t[0] <- 16;
    t[13.0] = 17;
    t[true] <- 60;
    try { delete t[1.0]; } catch (e) { }
    t[461] <- 51;
    t[0] = 81;
    t["k0"] = 88;
    t[1] = 9;
    try { delete t["gbf"]; } catch (e) { }
    t[17.0] <- 62;
    t[108] = 68;
    t[true] = 74;
    t[true] <- 45;
    t[-3] <- 87;
    t[9.0] <- 89;
    t[true] <- 46;
    t["hhh"] <- 22;
    t[2] <- 30;
    try { delete t[483]; } catch (e) { }
    t[0] <- 35;
    try { delete t[true]; } catch (e) { }
    t["k16"] <- 89;
    t["3.5"] <- 77;
    t["hha"] <- 98;
    t[true] <- 66;
    t[3] = 70;
    t[11.0] = 97;
    t[2] = 46;
    t["k4"] = 61;
    t["k8"] <- 63;
    t[-1] <- 52;
    try { delete t[12.0]; } catch (e) { }
    local o = "";
    foreach (kk, vv in t) o += "" + kk + "=" + vv + ";";
    return o + "/" + t.len();
});
print("ITERORDER-DONE\n");

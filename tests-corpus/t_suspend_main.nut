// 主脚本里直接 suspend：主 VM 就此挂起、**后面的语句不再执行**。
// 两侧行为一致（实测）：已经打印的输出会留下（进程是正常 exit，stdio 会 flush），
// 退出码 = suspend 的参数（sq.c 用脚本返回值当退出码，这里是 1）。
// 所以这个文件必须单独放：它会让脚本提前结束，混排会让后面的用例跑不到。
print("SUSPEND_MAIN-BEFORE\n");
suspend(1);
print("SUSPEND_MAIN-AFTER-SHOULD-NOT-PRINT\n");

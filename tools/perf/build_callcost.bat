@echo off
rem Build and run the Rust vs C++ per-opcode cost micro-benchmark.
rem Same machine, same body, both compilers at their strongest settings.
rem ASCII only: cmd.exe parses this file under code page 936.
rem Binaries go to tmp-c2\ (gitignored).
call "C:\Program Files\Microsoft Visual Studio\18\Enterprise\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d %~dp0\..\..
echo === rustc (opt-level=3, lto=fat, codegen-units=1, target-cpu=native) ===
rustc -O -C opt-level=3 -C codegen-units=1 -C lto=fat -C target-cpu=native -o tmp-c2\callcost_rs.exe tools\perf\callcost.rs || exit /b 1
echo === cl (O2 /GL /arch:AVX2 /GS- /Oi /Ob3) ===
cl /nologo /O2 /GL /arch:AVX2 /GS- /Oi /Ob3 /EHsc /Fe:tmp-c2\callcost_cpp.exe /Fo:tmp-c2\ tools\perf\callcost.cpp || exit /b 1
echo === run ===
tmp-c2\callcost_rs.exe
tmp-c2\callcost_cpp.exe
exit /b 0

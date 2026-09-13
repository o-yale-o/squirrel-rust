@echo off
rem Dump the release sq.exe disassembly for dispatch-shape analysis.
rem Output goes to tmp-c2\disasm2.txt (gitignored, ~16 MB).
rem ASCII only: cmd.exe parses this file under code page 936.
call "C:\Program Files\Microsoft Visual Studio\18\Enterprise\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d %~dp0\..\..
dumpbin /nologo /disasm target\release\sq.exe > tmp-c2\disasm2.txt 2>nul
exit /b %ERRORLEVEL%

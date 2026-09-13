@echo off
rem Build the host-API differential probe (runs against the C++ reference)
call "C:\Program Files\Microsoft Visual Studio\18\Enterprise\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d D:\OpenSource\squirrel-rust\tools\hostapi
cl /nologo /W3 /MD /D_SQ64 /I D:\OpenSource\squirrel\include /I D:\OpenSource\squirrel\squirrel /I D:\OpenSource\squirrel\sqstdlib ref_host.c /Fe:ref_host.exe /Fo:obj\ /link /LIBPATH:D:\OpenSource\squirrel-rust\tmp-c2\lib squirrel.lib sqstdlib.lib ucrt.lib vcruntime.lib legacy_stdio_definitions.lib
exit /b %ERRORLEVEL%

@echo off
set /a BLIT_LAUNCH_WAIT=0
:wait_for_launch_ok
if exist "D:/blit-test/ldt4-sessions/ldt4-20260721T224319Z-96a4e3b03caf/logs/ldt4-079/launch.ok" goto launch_ready
set /a BLIT_LAUNCH_WAIT+=1
if %BLIT_LAUNCH_WAIT% GEQ 15 exit /b 111
>nul 2>&1 ping -n 2 127.0.0.1
goto wait_for_launch_ok
:launch_ready
set BLIT_TRACE_SESSION_PHASES=1
set BLIT_TRACE_RUN_ID=ldt4-079
"D:/blit-test/bins/active/blit-daemon.exe" --config "D:/blit-test/ldt4-sessions/ldt4-20260721T224319Z-96a4e3b03caf/logs/ldt4-079/daemon.toml" >> "D:/blit-test/ldt4-sessions/ldt4-20260721T224319Z-96a4e3b03caf/logs/ldt4-079/daemon.out" 2>> "D:/blit-test/ldt4-sessions/ldt4-20260721T224319Z-96a4e3b03caf/logs/ldt4-079/daemon.err"

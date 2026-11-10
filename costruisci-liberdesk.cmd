@echo off
rem Scarica (o aggiorna) e compila LiberDesk. Metti questo file in AmbienteRust.
title Costruisci LiberDesk
set "ROOT=%~dp0"
if "%ROOT:~-1%"=="\" set "ROOT=%ROOT:~0,-1%"
set "CARGO_HOME=%ROOT%\rust\cargo"
set "RUSTUP_HOME=%ROOT%\rust\rustup"
set "PATH=%ROOT%\winlibs\mingw64\bin;%ROOT%\rust\cargo\bin;%ROOT%\git\cmd;%ROOT%\git\bin;%PATH%"
set "PROG=%ROOT%\Progetti"
if not exist "%PROG%" mkdir "%PROG%"
cd /d "%PROG%"
taskkill /f /im liberdesk.exe >nul 2>&1

echo === Scarico o aggiorno LiberDesk ===
if exist "%PROG%\LiberDesk\.git" (
  git -c "safe.directory=*" -C "%PROG%\LiberDesk" fetch origin
  if errorlevel 1 goto fail
  git -c "safe.directory=*" -C "%PROG%\LiberDesk" reset --hard origin/main
) else (
  git clone https://github.com/lorenzodapolito-oss/LiberDesk "%PROG%\LiberDesk"
)
if errorlevel 1 goto fail

echo === Compilo ===
cd /d "%PROG%\LiberDesk"
cargo build --release
if errorlevel 1 goto fail

set "OUT=%PROG%\LiberDesk\dist\LiberDesk-portable"
if exist "%OUT%" rmdir /s /q "%OUT%"
mkdir "%OUT%"
copy /y "target\release\liberdesk.exe" "%OUT%\LiberDesk.exe" >nul
if errorlevel 1 goto fail

echo.
echo FATTO: %OUT%\LiberDesk.exe
pause
exit /b 0

:fail
echo.
echo ERRORE: operazione non riuscita. Controlla i messaggi sopra.
pause
exit /b 1

@echo off
echo ========================================
echo Syncing all configs to WSL
echo ========================================
echo.

echo [1/3] Copying .claude directory...
wsl bash -c "mkdir -p ~/.claude && cp -rf /mnt/c/Users/%USERNAME%/.claude/* ~/.claude/ 2>/dev/null"
if %ERRORLEVEL% EQU 0 (echo OK: .claude copied) else (echo WARN: .claude failed)

echo.
echo [2/3] Copying .codex configs...
wsl bash -c "mkdir -p ~/.codex && cp -f /mnt/c/Users/%USERNAME%/.codex/auth.json ~/.codex/auth.json 2>/dev/null && cp -f /mnt/c/Users/%USERNAME%/.codex/config.toml ~/.codex/config.toml 2>/dev/null"
if %ERRORLEVEL% EQU 0 (echo OK: .codex configs copied) else (echo WARN: .codex configs failed)

echo.
echo [3/3] Copying Claude settings.json...
wsl bash -c "cp -f /mnt/c/Users/%USERNAME%/.claude/settings.json ~/.claude/settings.json 2>/dev/null"
if %ERRORLEVEL% EQU 0 (echo OK: settings.json copied) else (echo WARN: settings.json failed)

echo.
echo ========================================
echo Sync complete! Verification:
echo ========================================
echo.

echo ~/.claude directory:
wsl ls -lah ~/.claude/ 2>nul
echo.

echo ~/.codex directory:
wsl ls -lah ~/.codex/ 2>nul
echo.

echo Claude settings preview:
wsl head -10 ~/.claude/settings.json 2>nul

pause

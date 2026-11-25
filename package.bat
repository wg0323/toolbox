@echo off
setlocal
chcp 65001 >nul
cd /d "%~dp0"

set "MODE=%~1"
if "%MODE%"=="" set "MODE=bundle"

set "VERSION=unknown"
for /f "delims=" %%v in ('node -p "require('./package.json').version" 2^>nul') do set "VERSION=%%v"

echo.
echo ==========================================
echo   Toolbox 打包
echo   版本: %VERSION%
echo   模式: %MODE%   (bundle / exe / debug)
echo ==========================================
echo.

where node >nul 2>nul
if errorlevel 1 (
  echo [错误] PATH 中找不到 node.exe
  exit /b 1
)

where cargo >nul 2>nul
if errorlevel 1 (
  echo [错误] PATH 中找不到 cargo，请确认 Rust 工具链已安装
  exit /b 1
)

if not exist "node_modules" (
  echo [1/3] 安装前端依赖...
  call npm install
  if errorlevel 1 (
    echo [错误] npm install 失败
    exit /b 1
  )
) else (
  echo [1/3] 前端依赖已存在，跳过安装
)

echo [2/3] 清理旧安装包...
if exist "src-tauri\target\release\bundle" rmdir /s /q "src-tauri\target\release\bundle"
if exist "src-tauri\target\debug\bundle" rmdir /s /q "src-tauri\target\debug\bundle"

echo [3/3] 开始构建...
if /i "%MODE%"=="exe" (
  call npx tauri build --no-bundle
) else if /i "%MODE%"=="debug" (
  call npx tauri build --debug --no-bundle
) else (
  call npx tauri build
)
if errorlevel 1 (
  echo.
  echo [错误] 构建失败，请查看上方输出
  echo        完整打包需要 NSIS，首次会从 GitHub 下载，网络不通时可改用: package.bat exe
  exit /b 1
)

echo.
echo ==========================================
echo   构建完成，产物：
if /i "%MODE%"=="debug" (
  if exist "src-tauri\target\debug\toolbox.exe" echo   %CD%\src-tauri\target\debug\toolbox.exe
) else (
  if exist "src-tauri\target\release\toolbox.exe" echo   %CD%\src-tauri\target\release\toolbox.exe
  if exist "src-tauri\target\release\bundle" (
    for /r "src-tauri\target\release\bundle" %%f in (*.exe *.msi) do echo   %%f
  )
)
echo ==========================================
echo.

endlocal

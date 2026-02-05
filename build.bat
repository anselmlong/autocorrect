@echo off
REM Build script for Windows Autocorrect

echo ================================
echo Building Windows Autocorrect
echo ================================
echo.

REM Check if Rust is installed
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Cargo not found. Please install Rust from https://rustup.rs/
    echo.
    pause
    exit /b 1
)

echo Checking dictionary...
if not exist "dictionary\words.txt" (
    echo.
    echo WARNING: dictionary/words.txt not found!
    echo The app will use a small fallback dictionary.
    echo.
    echo For best results, download a word list:
    echo   curl -o dictionary/words.txt https://raw.githubusercontent.com/dwyl/english-words/master/words_alpha.txt
    echo.
    echo Or press any key to continue with fallback dictionary...
    pause >nul
)

echo.
echo Building release version...
cargo build --release

if %ERRORLEVEL% EQU 0 (
    echo.
    echo ================================
    echo Build successful!
    echo ================================
    echo.
    echo Executable location: target\release\autocorrect.exe
    echo.
    echo To run: target\release\autocorrect.exe
    echo.
) else (
    echo.
    echo Build failed! Check error messages above.
    echo.
)

pause

# Quick Start Guide

Get up and running in 5 minutes!

## Step 1: Install Rust

If you don't have Rust installed:

```powershell
# Option 1: Using winget (Windows Package Manager)
winget install Rustlang.Rustup

# Option 2: Download installer
# Visit: https://rustup.rs/
```

## Step 2: Download Dictionary (Optional but Recommended)

Open PowerShell in the project directory and run:

```powershell
# Download ~370k English words (~4MB file)
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/dwyl/english-words/master/words_alpha.txt" -OutFile "dictionary/words.txt"
```

Or skip this step to use the built-in fallback dictionary (~100 words).

## Step 3: Build

```powershell
# Option 1: Use the build script
.\build.bat

# Option 2: Build directly with cargo
cargo build --release
```

## Step 4: Run

```powershell
.\target\release\autocorrect.exe
```

Or just double-click `autocorrect.exe` in the `target/release/` folder.

## Step 5: Test It Out

1. Open Notepad (or any text editor)
2. Type: `teh` and press space
3. It should automatically change to: `the`

## Managing Autocorrect

**System Tray Icon**: Look for a green dot in your system tray (near the clock)

**Right-click the icon**:
- Disable Autocorrect - Turn off
- Enable Autocorrect - Turn back on  
- Quit - Exit the application

## Undo a Correction

Type something → It gets autocorrected → Press **Ctrl+Z** immediately → Original text restored!

## Add Personal Words

Your personal dictionary is at:
```
%APPDATA%\Autocorrect\personal_dictionary.txt
```

To open it:
```powershell
notepad $env:APPDATA\Autocorrect\personal_dictionary.txt
```

Add words (one per line):
```
myname
github
rustlang
```

Save and restart autocorrect.

## Troubleshooting

### "Cargo not found"
- Install Rust: https://rustup.rs/
- Restart your terminal after installation

### "Dictionary not found" warning
- That's okay! The app will use a fallback dictionary
- For better corrections, download a dictionary (see Step 2)

### Corrections not working
- Check the system tray icon - is autocorrect enabled?
- Try restarting the app
- Some apps (like Visual Studio) may block keyboard hooks

### How to stop it?
- Right-click tray icon → Quit
- Or press Ctrl+C in the terminal if running from command line

## Common Corrections

With the fallback dictionary, these work out of the box:
- `teh` → `the`
- `adn` → `and`
- `hte` → `the`
- `taht` → `that`
- `recieve` → `receive` (with full dictionary)

## Next Steps

- Add your frequently mistyped words to the personal dictionary
- Download a full dictionary for comprehensive corrections
- Set up autocorrect to run on Windows startup (optional)

## Run on Startup (Optional)

1. Press `Win+R`
2. Type: `shell:startup` and press Enter
3. Create a shortcut to `autocorrect.exe` in this folder
4. Autocorrect will now start when you log in

---

**Need help?** Check the full README.md for detailed documentation.

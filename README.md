# Windows Autocorrect

A privacy-first, system-wide autocorrect tool for Windows written in Rust. Uses the SymSpell algorithm for fast spell correction (<10ms).

## Features

- ✅ **System-wide**: Works in any application (text editors, browsers, chat apps, etc.)
- ✅ **Fast**: SymSpell algorithm provides corrections in under 10ms
- ✅ **Privacy-first**: Fully offline, no network calls, all processing local
- ✅ **Smart correction**: Tracks typed words and corrects on space/punctuation
- ✅ **Easy undo**: Press Ctrl+Z immediately after a correction to revert it
- ✅ **Tray icon**: Easy enable/disable toggle from system tray
- ✅ **Personal dictionary**: Add your own words (names, technical terms, etc.)
- ✅ **Low-level hook**: Uses Windows keyboard hook for reliable operation

## Requirements

- Windows 10 or later (64-bit)
- Rust toolchain (only for building from source)

## Installation

### Option 1: MSI Installer (Recommended)

Download the latest `.msi` installer from [GitHub Releases](https://github.com/anselmlong/autocorrect/releases).

1. Run the installer
2. The application will start automatically and add itself to Windows Startup
3. A green icon appears in your system tray

### Option 2: Portable ZIP

1. Download `autocorrect-portable-x86_64-pc-windows-msvc.zip` from [GitHub Releases](https://github.com/anselmlong/autocorrect/releases)
2. Extract to any folder
3. Run `autocorrect.exe`

### Option 3: Build from Source

#### Prerequisites
- [Rust](https://rustup.rs/) (latest stable)
- Git

#### Build Steps

1. **Clone the repository**:
   ```bash
   git clone https://github.com/anselmlong/autocorrect.git
   cd autocorrect
   ```

2. **Build the application**:
   ```bash
   cargo build --release
   ```

3. **Run**:
   ```bash
   cargo run --release
   ```

The compiled executable will be in `target/release/autocorrect.exe`.

## CLI Arguments

The application supports several command-line options:

```
autocorrect [OPTIONS]

Options:
      --disabled          Start with autocorrect disabled
  -d, --dictionary <PATH>  Custom dictionary file path
      --console           Run in console mode (don't hide console window)
      --check-update      Check for updates and exit
  -h, --help              Print help
  -V, --version           Print version
```

### Examples

```bash
# Start disabled
autocorrect --disabled

# Use custom dictionary
autocorrect --dictionary C:\path\to\my\words.txt

# Check for updates
autocorrect --check-update
```

## Configuration

Autocorrect can be configured via a TOML configuration file.

### Config File Location

- **Windows**: `%APPDATA%\autocorrect\config.toml`

### Configuration Options

Create or edit `config.toml`:

```toml
max_edit_distance = 2
enabled_by_default = true
undo_timeout_seconds = 5
hotkey_toggle = "Ctrl+Shift+A"
auto_check_updates = true
```

### Options Explained

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_edit_distance` | integer | 2 | Maximum character edits allowed (1-3) |
| `enabled_by_default` | boolean | true | Start with autocorrect enabled |
| `undo_timeout_seconds` | integer | 5 | Seconds to allow undo after correction |
| `hotkey_toggle` | string | "Ctrl+Shift+A" | Hotkey to toggle autocorrect |
| `auto_check_updates` | boolean | true | Check for updates on startup |

## Auto-Updates

The application can automatically check for and install updates from GitHub Releases.

### Check for Updates

Run with the `--check-update` flag:
```bash
autocorrect --check-update
```

If an update is available, it will be downloaded and installed automatically.

### Enable/Disable Auto-Check

Set `auto_check_updates = false` in your config file to disable automatic update checks on startup.

## Building

1. **Install Rust** (if not already installed):
   ```bash
   # Download from https://rustup.rs/
   # Or use:
   winget install Rustlang.Rustup
   ```

2. **Download a dictionary** (required):
   - Download a word list (e.g., from https://github.com/dwyl/english-words)
   - Place it as `dictionary/words.txt` (one word per line)
   - Or the app will use a built-in fallback dictionary (~100 common words)

3. **Build the application**:
   ```bash
   cargo build --release
   ```

4. **Run**:
   ```bash
   cargo run --release
   ```

The compiled executable will be in `target/release/autocorrect.exe`.

## Usage

### Starting the Application

Run `autocorrect.exe`. A green icon will appear in your system tray.

### Basic Operation

1. Type normally in any application
2. When you type a misspelled word and press space (or punctuation), it will be automatically corrected
3. The correction happens instantly by simulating backspaces and retyping

### Undo a Correction

If autocorrect changes a word you didn't want changed:
- Press **Ctrl+Z** immediately after the correction
- The original word will be restored

### Enable/Disable

Right-click the tray icon and select:
- "Disable Autocorrect" to turn off
- "Enable Autocorrect" to turn back on

### Personal Dictionary

Add your own words (names, technical terms, slang):

1. Find your personal dictionary file:
   - Location: `%APPDATA%\Autocorrect\personal_dictionary.txt`
   - Or: In the same folder as the executable

2. Edit the file and add words (one per line):
   ```
   # Personal Dictionary
   myname
   github
   rustlang
   ```

3. Restart the application to load new words

## Dictionary Format

### Built-in Dictionary (`dictionary/words.txt`)

Format: One word per line, optionally with frequency:
```
the 1000000
hello 15000
world 14000
```

Or just:
```
the
hello
world
```

### Personal Dictionary

Simply list words (one per line):
```
# Comments start with #
myname
companyname
techterm
```

## Architecture

- **main.rs**: Entry point, tray icon, and keyboard hook setup
- **symspell.rs**: SymSpell algorithm implementation (edit distance up to 2)
- **dictionary.rs**: Dictionary loading (built-in + personal)
- **corrector.rs**: Word tracking, correction logic, and undo buffer

## How It Works

1. **Keyboard Hook**: Installs a low-level Windows keyboard hook (`WH_KEYBOARD_LL`) that sees every keystroke
2. **Word Building**: Tracks letters as you type to build the current word
3. **Application Detection**: Detects the type of focused window (Notepad vs Notion vs Chrome)
4. **Input Method Selection**: Chooses the best input method based on the application:
   - **SendInput** with thread attachment for standard Win32 apps (Notepad, Word)
   - **SendMessage** fallback for Electron apps (Notion, VS Code, Slack)
   - **SendMessage** fallback for browsers (Chrome, Edge, Firefox)
5. **Trigger Points**: When you press space/punctuation/enter, checks if the word needs correction
6. **SymSpell Lookup**: Fast dictionary lookup using the SymSpell algorithm (<10ms)
7. **Auto-replace**: Deletes the misspelled word using backspaces, then types the correction
8. **Undo Buffer**: Stores the last correction for 5 seconds, allowing Ctrl+Z to revert

## Performance

- **Lookup time**: <10ms for most corrections
- **Memory**: ~20MB for 80k word dictionary
- **CPU**: Minimal impact, only processes when you type

## Privacy & Security

- ✅ **No network**: Never connects to the internet
- ✅ **No logging**: Doesn't store what you type
- ✅ **No telemetry**: No data collection
- ✅ **Local processing**: All corrections happen on your machine
- ✅ **Open source**: Audit the code yourself

## Application Compatibility

Autocorrect now supports multiple input methods to work with different types of applications:

### ✅ Fully Supported
- **Notepad, WordPad** - Standard Windows text editors
- **Microsoft Office** - Word, Excel, PowerPoint
- **Visual Studio Code** - Using SendMessage fallback for Electron compatibility
- **Notion** - Using SendMessage fallback for Electron compatibility
- **Browsers** - Chrome, Edge, Firefox (using SendMessage fallback)
- **Chat Apps** - Slack, Discord, Microsoft Teams

### How It Works

The application automatically detects the type of window you're typing in and switches input methods:

1. **Standard Apps** (Notepad, Word): Uses `SendInput` with thread attachment for proper focus management
2. **Electron Apps** (Notion, VS Code): Uses `SendMessage` fallback for Chromium compatibility
3. **Browsers** (Chrome, Edge): Uses `SendMessage` fallback for web content

The detection happens automatically and transparently - you don't need to configure anything.

### Technical Details

- **Input Method Selection**: Based on window class name detection
- **Key Delays**: Standard apps use 5ms delays, Electron/Chromium apps use 10ms for React/Virtual DOM synchronization
- **Thread Attachment**: Ensures proper focus management across different applications
- **Fallback Mechanism**: Automatically falls back to SendMessage if SendInput fails

## Limitations

- Only works on Windows (uses Windows-specific APIs)
- Cannot correct inside password fields (by design, for security)
- May not work in some applications with custom input handling
- Only supports English dictionary by default (add your own for other languages)
- Games with DirectInput may not work (different input system)

## Troubleshooting

### App doesn't start
- Run from command line to see error messages
- Check that `dictionary/words.txt` exists or let it use fallback

### Corrections not working
- Check that autocorrect is enabled (tray icon right-click menu)
- Some applications block keyboard hooks
- Try restarting the application
- For Electron apps (Notion, VS Code), ensure the app window has focus

### Corrections work in Notepad but not in Notion/VS Code
This should be fixed in the latest version. The app now automatically:
1. Detects Electron/Chromium-based applications
2. Uses SendMessage instead of SendInput for these apps
3. Adjusts timing delays for React/Virtual DOM synchronization

If you're still having issues:
- Make sure you're running the latest version
- Check that the application window has proper focus
- Try clicking in the text area before typing

### Wrong corrections
- Add correct words to your personal dictionary
- File location: `%APPDATA%\Autocorrect\personal_dictionary.txt`

### High CPU usage
- This shouldn't happen normally
- Check dictionary size (very large dictionaries use more memory)

## License

MIT License - feel free to use, modify, and distribute.

## Credits

- Built with Rust
- SymSpell algorithm by Wolf Garbe
- Uses winapi for Windows keyboard hook
- Uses tray-icon for system tray functionality

## Contributing

This is a personal project, but suggestions and improvements are welcome!

## Roadmap

Possible future enhancements:
- [ ] Multi-language support
- [ ] Case-sensitive corrections
- [ ] Configurable shortcut keys
- [ ] Statistics (words corrected, etc.)
- [ ] Auto-update dictionary
- [ ] Machine learning-based corrections
- [ ] Context-aware corrections

---

**Note**: This tool requires administrator privileges on some systems to install the keyboard hook. This is a Windows security requirement for system-wide keyboard monitoring.

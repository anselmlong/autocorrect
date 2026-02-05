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
- Rust toolchain (for building from source)

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

1. **Keyboard Hook**: Installs a low-level Windows keyboard hook that sees every keystroke
2. **Word Building**: Tracks letters as you type to build the current word
3. **Trigger Points**: When you press space/punctuation/enter, checks if the word needs correction
4. **SymSpell Lookup**: Fast dictionary lookup using the SymSpell algorithm
5. **Auto-replace**: Simulates backspaces to delete the misspelled word, then types the correction
6. **Undo Buffer**: Stores the last correction for 5 seconds, allowing Ctrl+Z to revert

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

## Limitations

- Only works on Windows (uses Windows-specific APIs)
- Cannot correct inside password fields (by design, for security)
- May not work in some applications with custom input handling
- Only supports English dictionary by default (add your own for other languages)

## Troubleshooting

### App doesn't start
- Run from command line to see error messages
- Check that `dictionary/words.txt` exists or let it use fallback

### Corrections not working
- Check that autocorrect is enabled (tray icon right-click menu)
- Some applications block keyboard hooks
- Try restarting the application

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

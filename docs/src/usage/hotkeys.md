# Hotkeys

Configure custom key combinations for triggering voice transcription.

**Default:** `Ctrl+Option+Z`

## Basic Configuration

Edit `~/.whisper-hotkey/config.toml`:

```toml
[[profiles]]
model_type = "base.en"
modifiers = ["Control", "Option"]  # Modifier keys
key = "Z"                          # Main key
preload = true
threads = 4
beam_size = 1
language = "en"
```

Restart app to apply changes.

## Multiple Hotkeys

Different hotkeys for different profiles:

```toml
# Fast mode
[[profiles]]
name = "fast"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true

# Accurate mode
[[profiles]]
name = "accurate"
model_type = "small"
modifiers = ["Command", "Shift"]
key = "V"
preload = true
threads = 4
beam_size = 5
```

See [Multi-Profile Support](./profiles.md) for details.

## Available Options

### Modifiers

- `"Control"` - Ctrl key
- `"Option"` - Option/Alt key
- `"Command"` - Command/Cmd key (⌘)
- `"Shift"` - Shift key

**Combine multiple:**
```toml
modifiers = ["Control", "Option"]
modifiers = ["Command", "Shift"]
modifiers = ["Control", "Option", "Shift"]
```

**Tip:** Use 2+ modifiers to avoid conflicts with app shortcuts.

### Keys

**Letters:** `"A"` through `"Z"`
**Numbers:** `"0"` through `"9"`
**Function:** `"F1"` through `"F12"`
**Special:** `"Space"`, `"Tab"`, `"Return"`, `"Escape"`

## Recommended Combinations

**Safe choices (rarely conflict):**

- `Control+Option+Z` (default)
- `Control+Option+V`
- `Command+Shift+Space`
- `Control+Shift+R`

**Avoid these (common macOS shortcuts):**

- `Cmd+C`, `Cmd+V`, `Cmd+Z` (Copy, Paste, Undo)
- `Cmd+Q`, `Cmd+W` (Quit, Close)
- `Ctrl+C` (Terminal interrupt)

## Testing

After changing config:

1. Save `config.toml`
2. Restart app
3. Open text editor
4. Press and hold hotkey → speak → release
5. Verify text appears

## Troubleshooting

**Hotkey not working:**

1. Grant **Accessibility** permission:
   - System Settings → Privacy & Security → Accessibility
   - Add Terminal/iTerm or WhisperHotkey.app
   - Restart app

2. Check for conflicts:
   - Try different key combination
   - Disable conflicting shortcut in System Settings → Keyboard

3. Check logs:
   ```bash
   tail -f ~/.whisper-hotkey/crash.log
   ```

**Duplicate hotkey error:**

Each profile needs unique hotkey. Change one:

```toml
[[profiles]]
key = "Z"

[[profiles]]
key = "V"  # Different key
```

## Next Steps

- [Multi-Profile Support](./profiles.md) - Multiple hotkeys with different settings
- [Alias Matching](./alias-matching.md) - Auto-expand phrases

# Hotkeys

Configure custom key combinations for triggering voice transcription.

## Default Hotkey

The default hotkey is: **`Ctrl+Option+Z`**

This was chosen to:
- Avoid conflicts with common app shortcuts
- Be easily reachable with one hand
- Work consistently across macOS applications

## Configuring Hotkeys

### Single Profile Configuration

Edit `~/.whisper-hotkey/config.toml`:

```toml
[[profiles]]
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 1
language = "en"
```

### Multiple Profiles

Configure different hotkeys for different profiles:

```toml
# Fast transcription with base.en model
[[profiles]]
name = "fast"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 1
language = "en"

# Accurate transcription with small model
[[profiles]]
name = "accurate"
model_type = "small"
modifiers = ["Command", "Shift"]
key = "V"
preload = true
threads = 4
beam_size = 5
language = "en"
```

See [Multi-Profile Support](./profiles.md) for details.

## Available Modifiers

### Supported Modifiers

- `"Control"` - Ctrl key
- `"Option"` - Option/Alt key
- `"Command"` - Command/Cmd key (⌘)
- `"Shift"` - Shift key

### Modifier Combinations

You can combine multiple modifiers:

```toml
modifiers = ["Control", "Option"]        # Ctrl+Option
modifiers = ["Command", "Shift"]         # Cmd+Shift
modifiers = ["Control", "Command"]       # Ctrl+Cmd
modifiers = ["Control", "Option", "Shift"] # Ctrl+Option+Shift
```

**Recommendation:** Use at least 2 modifiers to avoid conflicts with app shortcuts.

## Available Keys

### Letter Keys

Any letter A-Z (case-insensitive):

```toml
key = "Z"  # or "A", "B", "C", etc.
```

### Number Keys

Number keys 0-9:

```toml
key = "1"  # or "0", "2", "3", etc.
```

### Function Keys

Function keys F1-F12:

```toml
key = "F1"  # or "F2", "F3", etc.
```

### Special Keys

Common special keys:

```toml
key = "Space"
key = "Tab"
key = "Return"
key = "Escape"
```

## Choosing Good Hotkeys

### Avoid Conflicts

**Check these common shortcuts before choosing:**

- **`Cmd+C`** - Copy
- **`Cmd+V`** - Paste
- **`Cmd+Z`** - Undo
- **`Cmd+Q`** - Quit
- **`Cmd+W`** - Close window
- **`Cmd+S`** - Save
- **`Ctrl+C`** - Terminal interrupt

### Recommended Combinations

**Safe choices that rarely conflict:**

- `Control+Option+Z` (default)
- `Control+Option+V`
- `Command+Shift+Space`
- `Control+Shift+R`
- `Option+Shift+D`

**For multiple profiles:**

- Fast: `Control+Option+Z`
- Accurate: `Command+Shift+V`
- Alternative: `Control+Shift+Space`

### Accessibility Considerations

**One-handed operation:**
- Left hand: `Control+Shift+A` (Ctrl and Shift with pinky, A with ring finger)
- Right hand: `Control+Option+;` (modifiers with thumb, ; with pinky)

**Two-handed operation:**
- `Command+Shift+Space` (easier to hold while speaking)

## Hotkey Priority

When using multiple profiles, hotkey registration order matters:

1. Profiles are registered in order they appear in config
2. If registration fails (conflict), error is logged
3. Successfully registered hotkeys remain active

**Example:**

```toml
# This registers first
[[profiles]]
modifiers = ["Control", "Option"]
key = "Z"
# ...

# This registers second
[[profiles]]
modifiers = ["Command", "Shift"]
key = "V"
# ...
```

## Testing Hotkeys

After changing hotkey configuration:

1. **Save config** file
2. **Restart app** (Ctrl+C, then rerun)
3. **Test new hotkey**:
   - Open text editor
   - Press and hold new hotkey
   - Speak test phrase
   - Release hotkey
   - Verify text appears

## Troubleshooting

### Hotkey Not Working

**Symptom:** Pressing hotkey does nothing

**Solutions:**

1. **Grant Accessibility permission:**
   ```bash
   # Check System Settings → Privacy & Security → Accessibility
   # Add Terminal/iTerm or WhisperHotkey.app
   ```

2. **Check for conflicts:**
   ```bash
   # Test if another app is using the same hotkey
   # Try different key combination
   ```

3. **Check logs:**
   ```bash
   tail -f ~/.whisper-hotkey/crash.log
   # Look for "Failed to register hotkey" errors
   ```

### Hotkey Conflicts

**Symptom:** Hotkey triggers another app's function

**Solutions:**

1. **Change hotkey in Whisper Hotkey config**
2. **OR disable conflicting app's shortcut:**
   - System Settings → Keyboard → Keyboard Shortcuts
   - Find conflicting shortcut and disable it

### Multiple Profiles - Same Hotkey

**Symptom:** Two profiles have same hotkey

**Error:**
```
Failed to register hotkey for profile 'accurate': already registered
```

**Solution:** Assign unique hotkeys to each profile:

```toml
[[profiles]]
name = "fast"
modifiers = ["Control", "Option"]
key = "Z"
# ...

[[profiles]]
name = "accurate"
modifiers = ["Command", "Shift"]
key = "V"  # Different hotkey
# ...
```

## Security Considerations

### Why Accessibility Permission Required

Registering global hotkeys requires macOS Accessibility permission because:
- Global hotkeys monitor all keyboard input
- macOS protects against keylogging via permission system
- Whisper Hotkey only monitors configured hotkey combinations

### Privacy

Whisper Hotkey:
- ✅ Only monitors configured hotkey combinations
- ✅ Does not log other keyboard input
- ✅ Does not send hotkey data anywhere
- ❌ Cannot see passwords or secure input

## Next Steps

- Set up [Multi-Profile Support](./profiles.md) with multiple hotkeys
- Learn about [Alias Matching](./alias-matching.md) for auto-expansion
- Optimize [Performance Settings](../configuration/performance.md)

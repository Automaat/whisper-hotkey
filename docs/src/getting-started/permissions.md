# macOS Permissions

Whisper Hotkey requires three macOS permissions to function:

## Required Permissions

### 1. Microphone Access

**Why needed:** Capture audio for transcription

**How to grant:**
1. System Settings → Privacy & Security → Microphone
2. Enable for Terminal/iTerm (if running from command line)
3. OR enable for WhisperHotkey.app (if running .app bundle)
4. Restart app after granting

**Reset permission:**
```bash
tccutil reset Microphone
```

### 2. Accessibility

**Why needed:**
- Register global hotkeys
- Insert transcribed text at cursor position

**How to grant:**
1. System Settings → Privacy & Security → Accessibility
2. Click the lock icon to make changes
3. Add Terminal/iTerm or WhisperHotkey.app to the list
4. Restart app after granting

**Reset permission:**
```bash
tccutil reset Accessibility
```

### 3. Input Monitoring

**Why needed:** Monitor keyboard input for hotkey detection

**How to grant:**
1. System Settings → Privacy & Security → Input Monitoring
2. Enable for Terminal/iTerm or WhisperHotkey.app
3. Restart app after granting

**Reset permission:**
```bash
tccutil reset ListenEvent
```

## Troubleshooting Permissions

### Permissions Granted But Not Working

**Symptom:** You've granted all permissions, but app still can't access them

**Cause:** macOS quarantine attribute (applied to downloaded .app bundles)

**Solution:**
```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine /Applications/WhisperHotkey.app

# Restart the app
```

The app automatically detects quarantine on startup and shows this command.

### Checking Current Permissions

**Microphone:**
```bash
sqlite3 ~/Library/Application\ Support/com.apple.TCC/TCC.db \
  "SELECT service, client, allowed FROM access WHERE service='kTCCServiceMicrophone';"
```

**Accessibility:**
```bash
sqlite3 /Library/Application\ Support/com.apple.TCC/TCC.db \
  "SELECT service, client, allowed FROM access WHERE service='kTCCServiceAccessibility';"
```

### Permission Prompts Not Appearing

If macOS doesn't prompt for permissions:

1. **Quit the app** completely
2. **Reset all permissions** (see commands above)
3. **Restart your Mac** (sometimes required)
4. **Launch app again** - prompts should appear

### Terminal Secure Input Mode

Some apps enable "Secure Input Mode" which blocks text insertion:

**Affected apps:**
- Terminal.app (when secure input is active)
- iTerm2 (with secure input enabled)
- Password managers
- Some security tools

**Check Terminal secure input:**
```bash
ioreg -l -w 0 | grep SecureInput
```

**Workaround:** Use Whisper Hotkey in other apps, or disable secure input in affected app's preferences.

## Why No Sandbox?

macOS App Store requires sandboxing, but sandboxed apps cannot:
- Use Accessibility APIs (required for hotkeys and text insertion)
- Access global input monitoring

This is why Whisper Hotkey is distributed outside the App Store.

## Privacy Considerations

### What Whisper Hotkey Accesses

✅ **Does access:**
- Microphone (only when hotkey is pressed)
- Keyboard input (only to detect hotkey)
- Active text cursor position (only when inserting text)

❌ **Never accesses:**
- Your existing files or documents
- Internet (after model download)
- Other apps' data
- System passwords or keychain

### Data Storage

All data stays local:
- **Audio**: Discarded immediately after transcription
- **Config**: `~/.whisper-hotkey/config.toml`
- **Model**: `~/.whisper-hotkey/models/ggml-*.bin`
- **Logs**: `~/.whisper-hotkey/crash.log` (local only, no telemetry)

## Security Best Practices

1. **Download from official sources only:**
   - GitHub releases: https://github.com/Automaat/whisper-hotkey/releases
   - Homebrew: `brew install Automaat/whisper-hotkey/whisper-hotkey`

2. **Verify checksums** (for DMG downloads):
   ```bash
   shasum -a 256 WhisperHotkey-*.dmg
   # Compare with SHA256SUMS.txt from release
   ```

3. **Build from source** if security-critical:
   - Audit code: https://github.com/Automaat/whisper-hotkey
   - Build yourself: `cargo build --release`

## Next Steps

After granting permissions, return to [Quick Start](./quick-start.md) to test your setup.

# Permissions Problems

Detailed troubleshooting for macOS permission issues.

## Required Permissions

Whisper Hotkey requires three macOS permissions:

1. **Microphone** - Audio recording
2. **Accessibility** - Hotkeys + text insertion
3. **Input Monitoring** - Hotkey detection

## Permission Symptoms

### Microphone Permission Missing

**Symptoms:**
- "No input device available" error
- 0 samples captured
- No audio recording

**Grant permission:**
1. System Settings → Privacy & Security → Microphone
2. Enable for Terminal/iTerm or WhisperHotkey.app
3. Restart app

**Reset if needed:**
```bash
tccutil reset Microphone
```

### Accessibility Permission Missing

**Symptoms:**
- Hotkey doesn't trigger
- Text doesn't insert
- "Failed to register hotkey" error

**Grant permission:**
1. System Settings → Privacy & Security → Accessibility
2. Click lock icon to make changes
3. Add Terminal/iTerm or WhisperHotkey.app
4. Restart app

**Reset if needed:**
```bash
tccutil reset Accessibility
```

### Input Monitoring Permission Missing

**Symptoms:**
- Hotkey doesn't respond
- No hotkey events in logs

**Grant permission:**
1. System Settings → Privacy & Security → Input Monitoring
2. Enable for Terminal/iTerm or WhisperHotkey.app
3. Restart app

**Reset if needed:**
```bash
tccutil reset ListenEvent
```

## Common Permission Problems

### Permissions Granted But Not Working

**Symptom:** All permissions granted but app still can't access

**Cause:** macOS quarantine attribute

**Solution:**
```bash
# Remove quarantine
xattr -d com.apple.quarantine /Applications/WhisperHotkey.app

# Restart app
```

**Why:** Downloaded apps have quarantine attribute which blocks permission recognition.

### Permission Prompts Not Appearing

**Symptom:** macOS doesn't prompt for permissions

**Solutions:**

1. **Restart app:**
   ```bash
   # Quit app
   # Start again
   ```

2. **Reset all permissions:**
   ```bash
   tccutil reset Microphone
   tccutil reset Accessibility
   tccutil reset ListenEvent
   ```

3. **Restart Mac:**
   - Sometimes required for permission system to work
   - Log out and back in may be sufficient

4. **Start app again:**
   - Prompts should appear now

### Accessibility Permission Keeps Resetting

**Symptom:** Permission granted but reverts after restart

**Causes:**

1. **App binary changed:**
   - Recompiling changes code signature
   - macOS treats as new app

2. **Quarantine attribute:**
   - Remove quarantine (see above)

3. **Code signing issues:**
   - Unsigned apps may have permission issues
   - Use official DMG or Homebrew version

**Solutions:**

1. **Use stable installation:**
   ```bash
   brew install Automaat/whisper-hotkey/whisper-hotkey
   ```

2. **Remove quarantine:**
   ```bash
   xattr -d com.apple.quarantine /Applications/WhisperHotkey.app
   ```

3. **Grant permission permanently:**
   - After removing quarantine
   - Restart app
   - Grant permissions

### Terminal Secure Input Mode

**Symptom:** Hotkey works in other apps but not Terminal

**Cause:** Terminal has Secure Keyboard Entry enabled

**Check:**
```bash
ioreg -l -w 0 | grep SecureInput
```

If shows `"SecureInputPID" = [number]`, secure input is active.

**Solution:**

1. **Disable in Terminal:**
   - Terminal.app → Preferences
   - Uncheck "Secure Keyboard Entry"

2. **Disable in iTerm2:**
   - iTerm2 → Preferences → Security
   - Uncheck "Secure Keyboard Entry"

3. **Use in other apps:**
   - Text insertion won't work in secure input mode
   - Use Whisper Hotkey in other apps

### Permission Denied in Logs

**Symptom:** Logs show "Permission denied" or "Access denied"

**Solutions:**

1. **Check all three permissions:**
   - Microphone
   - Accessibility
   - Input Monitoring

2. **Restart app after granting**

3. **Check app in permission list:**
   - System Settings → Privacy & Security
   - Look for Terminal/iTerm or WhisperHotkey.app
   - If multiple entries, remove all and re-add

4. **Check for SIP restrictions:**
   ```bash
   csrutil status
   # Should show "enabled" (normal)
   # If disabled, re-enable for security
   ```

## Checking Current Permissions

### Via TCC Database (Microphone)

```bash
sqlite3 ~/Library/Application\ Support/com.apple.TCC/TCC.db \
  "SELECT service, client, allowed FROM access WHERE service='kTCCServiceMicrophone';"
```

Look for your app in output. `allowed = 1` means granted.

### Via TCC Database (Accessibility)

```bash
sqlite3 /Library/Application\ Support/com.apple.TCC/TCC.db \
  "SELECT service, client, allowed FROM access WHERE service='kTCCServiceAccessibility';"
```

**Note:** Requires sudo for system database.

### Via System Settings

**Visual check:**
1. System Settings → Privacy & Security
2. Click each permission type:
   - Microphone
   - Accessibility
   - Input Monitoring
3. Verify your app is listed and enabled

## Permission Best Practices

### For End Users

1. **Grant all permissions immediately** when prompted
2. **Restart app** after granting
3. **Don't revoke** unless testing
4. **Use official releases** (Homebrew or DMG)
5. **Remove quarantine** if from DMG

### For Developers

1. **Build in release mode:**
   ```bash
   cargo build --release
   ```

2. **Run from fixed location:**
   - Don't run from different directories
   - Binary path changes = new app to macOS

3. **Test permissions early:**
   - First thing after building

4. **Reset permissions between tests:**
   ```bash
   tccutil reset Microphone
   tccutil reset Accessibility
   tccutil reset ListenEvent
   ```

## macOS Version Differences

### macOS 14 (Sonoma)

- Stricter permission enforcement
- May require explicit Input Monitoring
- Remove quarantine mandatory for DMG

### macOS 13 (Ventura)

- Similar to Sonoma
- Input Monitoring usually required

### macOS 12 (Monterey)

- Less strict
- Input Monitoring may be optional

### macOS 11 (Big Sur)

- Older permission system
- May not have Input Monitoring

**Recommendation:** Use macOS 13 or newer.

## Troubleshooting Steps

### Complete Reset

If nothing works, full reset:

```bash
# 1. Quit app
killall whisper-hotkey

# 2. Remove from Applications (if .app)
rm -rf /Applications/WhisperHotkey.app

# 3. Remove binary (if installed)
rm /usr/local/bin/whisper-hotkey

# 4. Reset permissions
tccutil reset Microphone
tccutil reset Accessibility
tccutil reset ListenEvent

# 5. Restart Mac
sudo reboot

# 6. Reinstall
brew install Automaat/whisper-hotkey/whisper-hotkey

# 7. Start app, grant all permissions

# 8. Test
```

### If Still Failing

1. **Check Console app:**
   - Applications → Utilities → Console
   - Filter for "whisper-hotkey"
   - Look for permission errors

2. **Check for conflicting apps:**
   - Other apps using same permissions
   - Disable temporarily to test

3. **Report issue:**
   - GitHub: https://github.com/Automaat/whisper-hotkey/issues
   - Include macOS version, app version, steps taken

## Prevention

### Avoid Permission Issues

1. **Use Homebrew:**
   ```bash
   brew install Automaat/whisper-hotkey/whisper-hotkey
   ```
   - Code signed
   - No quarantine
   - Automatic updates

2. **Don't move app after granting:**
   - Moving changes path
   - macOS treats as new app
   - Need to re-grant

3. **Grant all at once:**
   - Don't skip any prompts
   - Restart after granting all

4. **Update macOS:**
   - Keep system updated
   - Fixes permission bugs

## Next Steps

- Return to [Common Issues](./common-issues.md)
- See [Performance Issues](./performance.md)
- Read [Quick Start](../getting-started/quick-start.md) guide

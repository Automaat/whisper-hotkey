# Common Issues

Solutions to frequently encountered problems.

## Installation Issues

### DMG Won't Open

**Error:** "WhisperHotkey.dmg cannot be opened"

**Solution:**
```bash
xattr -r -d com.apple.quarantine ~/Downloads/WhisperHotkey-*.dmg
```

### App Won't Launch

**Error:** "WhisperHotkey.app is damaged and can't be opened"

**Solution:**
```bash
xattr -d com.apple.quarantine /Applications/WhisperHotkey.app
```

### Homebrew Installation Fails

**Error:** `brew install` fails

**Solution:**
```bash
# Update Homebrew
brew update

# Try again
brew install Automaat/whisper-hotkey/whisper-hotkey

# If still fails, install from DMG
```

## Permission Issues

See dedicated [Permissions Problems](./permissions.md) page.

## Audio Issues

### No Audio Captured

**Symptom:** 0 samples captured, no recording

**Solutions:**

1. **Grant Microphone permission:**
   - System Settings → Privacy & Security → Microphone
   - Enable for Terminal/iTerm or WhisperHotkey.app
   - Restart app

2. **Check microphone:**
   ```bash
   # Test in Voice Memos app
   # If Voice Memos works, check permissions again
   ```

3. **Reset microphone permission:**
   ```bash
   tccutil reset Microphone
   # Restart app, grant permission when prompted
   ```

### Audio Cutting Off

**Symptom:** Recording stops before releasing hotkey

**Solution:**
```toml
# Increase buffer size in config
[audio]
buffer_size = 2048  # From 1024
```

### Audio Glitches/Crackling

**Symptom:** Pops or clicks in recordings

**Causes & Solutions:**

1. **CPU overload:**
   - Close CPU-intensive apps
   - Check Activity Monitor

2. **Buffer too small:**
   ```toml
   [audio]
   buffer_size = 2048
   ```

3. **Microphone too loud:**
   - System Settings → Sound → Input
   - Reduce input volume

### Microphone Not Found

**Error:** "No input device available"

**Solutions:**

1. **Check system default:**
   - System Settings → Sound → Input
   - Select microphone

2. **Grant permission** (see above)

3. **Restart app** after changing input device

## Hotkey Issues

### Hotkey Not Working

**Symptom:** Pressing hotkey does nothing

**Solutions:**

1. **Grant Accessibility permission:**
   - System Settings → Privacy & Security → Accessibility
   - Add Terminal/iTerm or WhisperHotkey.app
   - Restart app

2. **Grant Input Monitoring permission:**
   - System Settings → Privacy & Security → Input Monitoring
   - Add Terminal/iTerm or WhisperHotkey.app
   - Restart app

3. **Check for conflicts:**
   - Try different hotkey combination
   - System Settings → Keyboard → Keyboard Shortcuts
   - Disable conflicting shortcuts

4. **Check logs:**
   ```bash
   tail -f ~/.whisper-hotkey/crash.log
   # Look for "Failed to register hotkey"
   ```

### Wrong App Triggers

**Symptom:** Hotkey triggers another app's shortcut

**Solution:** Change hotkey in config:
```toml
[[profiles]]
modifiers = ["Control", "Shift"]  # Different combination
key = "V"  # Different key
```

### Hotkey Registers But Doesn't Record

**Symptom:** Console shows "Hotkey pressed" but no recording

**Solutions:**

1. **Check microphone permission** (see Audio Issues above)

2. **Check logs for errors:**
   ```bash
   RUST_LOG=debug cargo run --release
   # Look for error messages
   ```

## Text Insertion Issues

### Text Not Inserting

**Symptom:** Transcription completes but text doesn't appear

**Solutions:**

1. **Grant Accessibility permission** (see Hotkey Issues above)

2. **Grant Input Monitoring permission** (see Hotkey Issues above)

3. **Try different app:**
   - Some apps block text insertion
   - Test in TextEdit first

4. **Check secure input mode (Terminal):**
   ```bash
   ioreg -l -w 0 | grep SecureInput
   # If shows "SecureInput" = true, disable it:
   # Terminal → Preferences → Uncheck "Secure Keyboard Entry"
   ```

### Partial Text Insertion

**Symptom:** Only first few words inserted

**Solution:**
- May be app-specific limitation
- Try different app
- Report issue with app name

### Wrong Cursor Position

**Symptom:** Text inserted at wrong location

**Solution:**
- Click to place cursor before using hotkey
- Some apps don't report cursor position correctly
- Report issue with app name

## Transcription Issues

### Model Download Fails

**Error:** "Failed to download model"

**Solutions:**

1. **Check internet connection**

2. **Manual download:**
   ```bash
   cd ~/.whisper-hotkey/models/
   curl -L -o ggml-base.en.bin \
     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
   ```

3. **Check disk space:**
   ```bash
   df -h ~
   # Need 500MB-3GB depending on model
   ```

### Model Load Fails

**Error:** "Failed to load Whisper model"

**Solutions:**

1. **Check file exists:**
   ```bash
   ls -lh ~/.whisper-hotkey/models/
   ```

2. **Delete and redownload:**
   ```bash
   rm ~/.whisper-hotkey/models/ggml-base.en.bin
   # Restart app - redownloads
   ```

3. **Check file size:**
   ```bash
   ls -lh ~/.whisper-hotkey/models/ggml-base.en.bin
   # base.en should be ~142MB
   # If smaller, file corrupted
   ```

### Slow Transcription

**Symptom:** Takes >5s for 10s audio (base.en)

**Solutions:**

1. **Use faster model:**
   ```toml
   [[profiles]]
   model_type = "tiny.en"  # Faster
   ```

2. **Increase threads:**
   ```toml
   [[profiles]]
   threads = 8  # From 4
   ```

3. **Reduce beam_size:**
   ```toml
   [[profiles]]
   beam_size = 1  # From 5
   ```

4. **Close other apps:**
   - Check Activity Monitor for CPU usage

### Poor Accuracy

**Symptom:** Transcription has many errors

**Solutions:**

1. **Use better model:**
   ```toml
   [[profiles]]
   model_type = "small.en"  # More accurate
   ```

2. **Increase beam_size:**
   ```toml
   [[profiles]]
   beam_size = 5  # From 1
   ```

3. **Improve audio quality:**
   - Speak clearly and at normal pace
   - Reduce background noise
   - Use better microphone
   - Increase microphone volume (System Settings)

4. **Specify language:**
   ```toml
   [[profiles]]
   language = "en"  # Don't auto-detect
   ```

## Performance Issues

See dedicated [Performance Issues](./performance.md) page.

## Configuration Issues

### Config Not Loading

**Error:** "Failed to load config"

**Solutions:**

1. **Check syntax:**
   ```bash
   # Try to parse manually
   cargo run --release
   # Look for parse errors in output
   ```

2. **Reset to defaults:**
   ```bash
   mv ~/.whisper-hotkey/config.toml ~/.whisper-hotkey/config.toml.backup
   # Restart app - creates default config
   ```

3. **Check file permissions:**
   ```bash
   ls -l ~/.whisper-hotkey/config.toml
   # Should be readable
   ```

### Config Changes Not Applied

**Symptom:** Changed config but app behavior unchanged

**Solution:**
- **Restart app** (config loaded only at startup)
- Ctrl+C, then restart

### Invalid Config Values

**Error:** "Invalid value for ..."

**Solutions:**

1. **Check documentation:**
   - See [Configuration Reference](../configuration/reference.md)
   - Verify valid values

2. **Use defaults:**
   - Remove problematic field
   - App uses default value

## Crash Issues

### App Crashes on Startup

**Solutions:**

1. **Check logs:**
   ```bash
   cat ~/.whisper-hotkey/crash.log
   ```

2. **Reset config:**
   ```bash
   mv ~/.whisper-hotkey/config.toml ~/.whisper-hotkey/config.toml.backup
   ```

3. **Delete models and redownload:**
   ```bash
   rm -rf ~/.whisper-hotkey/models/
   ```

4. **Reinstall:**
   ```bash
   brew reinstall whisper-hotkey
   # OR
   ./scripts/install.sh
   ```

### App Crashes During Transcription

**Solutions:**

1. **Check logs** (see above)

2. **Try smaller model:**
   ```toml
   [[profiles]]
   model_type = "base.en"  # From small/medium
   ```

3. **Reduce memory usage:**
   ```toml
   [[profiles]]
   preload = false  # Lazy load
   ```

4. **Report bug** with logs

### App Hangs

**Symptom:** App becomes unresponsive

**Solutions:**

1. **Force quit:**
   ```bash
   killall whisper-hotkey
   ```

2. **Check CPU usage:**
   - Activity Monitor
   - If 100% CPU, wait for transcription to finish

3. **Reduce load:**
   - Close other apps
   - Use smaller model

## Getting Help

### Check Logs

```bash
# View crash log
tail -f ~/.whisper-hotkey/crash.log

# Run with debug logging
RUST_LOG=debug cargo run --release

# Run with trace logging
RUST_LOG=trace cargo run --release
```

### Report Issue

Include in bug report:
1. **macOS version:** `sw_vers`
2. **CPU:** Apple Silicon or Intel
3. **App version:** Check release tag
4. **Config:** Share relevant config snippet
5. **Logs:** Include error messages
6. **Steps to reproduce**

**GitHub Issues:** https://github.com/Automaat/whisper-hotkey/issues

## Still Need Help?

- **Documentation:** Full docs at [GitHub Pages](https://automaat.github.io/whisper-hotkey/)
- **Issues:** [GitHub Issues](https://github.com/Automaat/whisper-hotkey/issues)
- **Discussions:** [GitHub Discussions](https://github.com/Automaat/whisper-hotkey/discussions)

# Quick Start

After installation, follow these steps to test Whisper Hotkey.

## First Launch

When you run Whisper Hotkey for the first time, you'll see console output like this:

```
✓ Config loaded from ~/.whisper-hotkey/config.toml
✓ Telemetry initialized
✓ Permissions OK
✓ Model found at /Users/you/.whisper-hotkey/models/ggml-small.bin
Loading Whisper model (this may take a few seconds)...
  Optimization: 4 threads, beam_size=5
✓ Whisper model loaded and ready
✓ Audio capture initialized
✓ Hotkey registered: ["Control", "Option"] + Z

Whisper Hotkey is running. Press the hotkey to record and transcribe.
✓ Full pipeline ready: hotkey → audio → transcription → text insertion
Press Ctrl+C to exit.
```

## Test Your Setup

1. **Open any text editor** (TextEdit, VS Code, Notes, Chrome, etc.)
2. **Click into a text field** to place your cursor
3. **Press and hold** `Ctrl+Option+Z` (or your configured hotkey)
4. **Speak clearly**: "Hello, this is a test"
5. **Release** the hotkey
6. **Text appears** at cursor in ~2 seconds

## Expected Console Output

During transcription:

```
🎤 Hotkey pressed - recording started
⏹️  Hotkey released - processing audio
📼 Captured 3.5s audio (56000 samples)
✨ Transcription: "Hello, this is a test"
✅ Inserted 22 chars
✓ Ready for next recording
```

## Common First-Time Issues

### No Audio Recording

**Symptom:** No audio captured, or "No input device available" error

**Solution:** Grant **Microphone** permission
- Go to: System Settings → Privacy & Security → Microphone
- Enable for Terminal/iTerm or WhisperHotkey.app
- Restart app

### Hotkey Not Working

**Symptom:** Pressing hotkey does nothing

**Solution:** Grant **Accessibility** permission
- Go to: System Settings → Privacy & Security → Accessibility
- Enable for Terminal/iTerm or WhisperHotkey.app
- Restart app

### Text Not Inserting

**Symptom:** Transcription works, but text doesn't appear

**Solution:** Grant **Input Monitoring** permission
- Go to: System Settings → Privacy & Security → Input Monitoring
- Enable for Terminal/iTerm or WhisperHotkey.app
- Some apps block insertion (Terminal secure input mode)
- Restart app

### Quarantine Issues (DMG Installation)

**Symptom:** Permissions granted but app still can't access them

**Solution:** Remove macOS quarantine attribute
```bash
xattr -d com.apple.quarantine /Applications/WhisperHotkey.app
```

See [macOS Permissions](./permissions.md) for detailed permission setup.

## Tips for Best Results

- **Speak clearly** and at normal pace
- **Wait 0.5s** after pressing hotkey before speaking
- **Keep recordings under 30s** for faster processing
- **Use in quiet environment** for better accuracy
- **Position mic properly** (built-in or external)

## Performance Expectations

| Metric | Expected Performance |
|--------|---------------------|
| Audio start | ~5-10ms |
| Transcription (10s audio) | ~1.5-2s |
| Text insertion | ~20-50ms |
| Idle CPU | ~0.5% |
| Idle RAM | ~1.3GB |

## Next Steps

- Learn about [Hotkeys](../usage/hotkeys.md) configuration
- Explore [Multi-Profile Support](../usage/profiles.md)
- Optimize [Performance Settings](../configuration/performance.md)
- Set up [Alias Matching](../usage/alias-matching.md) for common phrases

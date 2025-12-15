# Basic Usage

Whisper Hotkey provides system-wide voice-to-text transcription via hotkey activation.

## How It Works

1. **Press and hold** the configured hotkey
2. **Speak** your text clearly
3. **Release** the hotkey when done
4. **Text appears** at your cursor position

## Recording Process

### Starting Recording

When you **press and hold** the hotkey:
- Audio buffer is cleared
- Microphone starts capturing at 16kHz mono
- Visual feedback (if enabled) shows recording status

### During Recording

While **holding** the hotkey:
- Audio samples accumulate in memory
- No processing happens yet (pure capture mode)
- Maximum practical recording length: ~30 seconds

### Stopping Recording

When you **release** the hotkey:
- Audio capture stops immediately
- Audio is sent to Whisper for transcription
- Processing typically takes 1-2 seconds for 10 seconds of audio

### Text Insertion

After transcription completes:
- Text is inserted at current cursor position
- Uses macOS CGEvent (simulates keyboard)
- Preserves cursor position in most apps

## Best Practices

### For Accuracy

- **Speak clearly** at normal conversational pace
- **Use quiet environment** to minimize background noise
- **Wait 0.5s after pressing hotkey** before speaking (audio buffer initialization)
- **Position microphone properly**:
  - Built-in Mac mic: speak facing keyboard
  - External mic: position 6-12 inches from mouth

### For Performance

- **Keep recordings under 30s** for faster processing
- **Use English-only models** (tiny.en, base.en, small.en) for English speech
- **Enable model preload** for instant readiness (default: true)
- **Adjust beam_size**:
  - `beam_size = 1`: Fastest, good accuracy (default)
  - `beam_size = 5`: Balanced
  - `beam_size = 10`: Best accuracy, slower

### For Reliability

- **Grant all required permissions** before using (see [Permissions](../getting-started/permissions.md))
- **Test in multiple apps** (some apps block text insertion)
- **Check logs if issues occur**: `tail -f ~/.whisper-hotkey/crash.log`

## Supported Applications

Whisper Hotkey works with **most macOS applications**:

### Fully Supported

✅ Text editors (TextEdit, VS Code, Sublime, Vim/Neovim)
✅ Web browsers (Chrome, Safari, Firefox, Arc)
✅ Note-taking apps (Notes.app, Obsidian, Notion)
✅ Communication apps (Slack, Discord, Messages)
✅ Email clients (Mail.app, Spark, Outlook)
✅ Office apps (Pages, Word, Google Docs in browser)

### Partially Supported

⚠️ **Terminal apps** (Terminal.app, iTerm2):
- Works when secure input is disabled
- Check: `ioreg -l -w 0 | grep SecureInput`
- Disable in Terminal: Preferences → Secure Keyboard Entry

⚠️ **Password managers** (1Password, LastPass):
- May block text insertion in password fields
- Works in other fields

### Not Supported

❌ Apps with custom text rendering (some games, proprietary apps)
❌ Secure input fields that block CGEvent
❌ Apps running with higher privileges than Whisper Hotkey

## Console Output

### Normal Operation

```
🎤 Hotkey pressed - recording started
⏹️  Hotkey released - processing audio
📼 Captured 3.5s audio (56000 samples)
✨ Transcription: "Hello, this is a test"
✅ Inserted 22 chars
✓ Ready for next recording
```

### With Debug Logging

Enable debug logging to see detailed timing:

```bash
RUST_LOG=debug cargo run --release
```

Output includes:
- Audio capture latency
- Transcription inference time
- Text insertion duration
- Memory usage metrics

### With Trace Logging

Enable trace logging for low-level operations:

```bash
RUST_LOG=whisper_hotkey=trace cargo run --release
```

Output includes:
- Audio buffer operations
- Whisper model internals
- CGEvent details
- Thread synchronization

## Common Usage Patterns

### Dictating Long Text

For transcribing multiple sentences:

1. Break into 10-15 second chunks
2. Press hotkey, speak first chunk, release
3. Wait for text insertion (~2s)
4. Repeat for next chunk

**Tip:** Use punctuation words: "period", "comma", "question mark" (requires alias matching)

### Correcting Mistakes

If transcription is wrong:

1. Select incorrect text
2. Press backspace/delete
3. Re-record with clearer pronunciation

### Code Dictation

For dictating code:

1. Use [Alias Matching](./alias-matching.md) for common patterns:
   - "dot" → "."
   - "semicolon" → ";"
   - "equals" → "="
2. Speak slowly and clearly
3. Use descriptive variable names (easier to transcribe)

## Limitations

- **No real-time streaming**: Must hold hotkey for entire phrase (Whisper design limitation)
- **No punctuation inference**: Whisper outputs words only (use alias matching for punctuation)
- **macOS only**: Uses Core Graphics and Accessibility APIs
- **Some apps resist insertion**: Terminal secure input, password fields, etc.

## Next Steps

- Configure [Hotkeys](./hotkeys.md) to customize key combinations
- Set up [Multi-Profile Support](./profiles.md) for different use cases
- Enable [Alias Matching](./alias-matching.md) for common phrases

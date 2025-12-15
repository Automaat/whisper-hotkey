# Multi-Profile Support

Configure multiple transcription profiles with different hotkeys, models, and settings for different use cases.

## What Are Profiles?

A **profile** combines:
- **Hotkey** (unique key combination)
- **Model** (e.g., base.en, small, medium)
- **Performance settings** (threads, beam_size)
- **Language** (auto-detect or specific language)

Each profile runs independently, allowing you to optimize for different scenarios.

## Use Cases

### Speed vs. Accuracy

```toml
# Fast mode - quick dictation
[[profiles]]
name = "fast"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 1
language = "en"

# Accurate mode - important documents
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

### Multi-Language

```toml
# English transcription
[[profiles]]
name = "english"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "E"
preload = true
threads = 4
beam_size = 1
language = "en"

# Spanish transcription
[[profiles]]
name = "spanish"
model_type = "base"
modifiers = ["Control", "Option"]
key = "S"
preload = true
threads = 4
beam_size = 1
language = "es"

# French transcription
[[profiles]]
name = "french"
model_type = "base"
modifiers = ["Control", "Option"]
key = "F"
preload = true
threads = 4
beam_size = 1
language = "fr"
```

### Application-Specific

```toml
# Code dictation - tiny model, fast
[[profiles]]
name = "code"
model_type = "tiny.en"
modifiers = ["Control", "Shift"]
key = "C"
preload = true
threads = 8
beam_size = 1
language = "en"

# Documentation - better accuracy
[[profiles]]
name = "docs"
model_type = "small.en"
modifiers = ["Control", "Shift"]
key = "D"
preload = true
threads = 4
beam_size = 5
language = "en"

# Email - balanced
[[profiles]]
name = "email"
model_type = "base.en"
modifiers = ["Control", "Shift"]
key = "M"
preload = true
threads = 4
beam_size = 1
language = "en"
```

## Configuration

### Profile Structure

Each profile is defined in `~/.whisper-hotkey/config.toml` as:

```toml
[[profiles]]
name = "profile-name"        # Optional: auto-generated if not specified
model_type = "base.en"       # Required: model to use
modifiers = ["Control", "Option"]  # Required: modifier keys
key = "Z"                    # Required: main key
preload = true               # Optional: preload model at startup (default: true)
threads = 4                  # Optional: CPU threads (default: 4)
beam_size = 1                # Optional: beam search width (default: 1)
language = "en"              # Optional: language code (default: "en")
```

### Profile Names

**Auto-generated names:**
- If `name` is not specified, uses `model_type` as name
- Example: `model_type = "base.en"` → profile name is "base.en"

**Explicit names:**
- Use when multiple profiles share same `model_type`
- Must be unique across all profiles

```toml
# These two need explicit names (both use base.en)
[[profiles]]
name = "fast-english"
model_type = "base.en"
# ...

[[profiles]]
name = "slow-english"
model_type = "base.en"
beam_size = 5  # Slower, more accurate
# ...
```

### Required Fields

Every profile **must** have:
- `model_type` - Which Whisper model to use
- `modifiers` - Array of modifier keys
- `key` - Main key to trigger

### Optional Fields

**Defaults if not specified:**
- `name` - Auto-generated from `model_type`
- `preload = true` - Load model at startup
- `threads = 4` - CPU threads for inference
- `beam_size = 1` - Greedy decoding (fastest)
- `language = "en"` - English (skips auto-detect)

## Model Preloading

### What Is Preloading?

`preload = true` (default) means:
- Model loads at app startup (takes 2-3 seconds per model)
- First transcription is instant (no load delay)
- Model stays in memory (~1.3GB per model)

`preload = false` means:
- Model loads on first hotkey press
- 2-3 second delay before first transcription
- Model unloads after use (saves memory)

### Preloading Strategy

**Preload profiles you use frequently:**

```toml
# Daily use - preload
[[profiles]]
name = "primary"
model_type = "base.en"
preload = true
# ...

# Occasional use - lazy load
[[profiles]]
name = "secondary"
model_type = "medium"
preload = false
# ...
```

**Memory considerations:**

| Model | RAM Usage | Preload Recommendation |
|-------|-----------|------------------------|
| tiny.en | ~75MB | ✅ Always |
| base.en | ~142MB | ✅ Always |
| small.en | ~466MB | ✅ If used daily |
| medium.en | ~1.5GB | ⚠️ If RAM available |
| large | ~3GB | ❌ Usually lazy load |

**Total RAM:** Sum of all preloaded models + ~500MB overhead

## Profile Management

### Listing Active Profiles

Check app startup logs:

```
✓ Profiles loaded:
  - fast (base.en): Control+Option+Z [preloaded]
  - accurate (small): Command+Shift+V [preloaded]
  - spanish (base): Control+Option+S [lazy]
```

### Switching Between Profiles

Simply use different hotkeys - each profile is independent:

1. **Fast transcription:** Press `Ctrl+Option+Z`
2. **Accurate transcription:** Press `Cmd+Shift+V`
3. **Spanish transcription:** Press `Ctrl+Option+S`

### Adding New Profile

1. **Edit config:**
   ```toml
   [[profiles]]
   name = "new-profile"
   model_type = "small.en"
   modifiers = ["Option", "Shift"]
   key = "N"
   preload = true
   threads = 4
   beam_size = 1
   language = "en"
   ```

2. **Restart app:** `Ctrl+C`, then restart

3. **Model downloads automatically** if not present

4. **Test new hotkey**

### Removing Profile

1. **Delete profile section** from config
2. **Restart app**
3. **Optional:** Delete unused model:
   ```bash
   rm ~/.whisper-hotkey/models/ggml-{model-name}.bin
   ```

### Reordering Profiles

Profile order in config affects:
- Startup loading order (preloaded models)
- Log output order

**Recommendation:** Put most-used profile first

```toml
# Primary profile (loads first)
[[profiles]]
name = "primary"
# ...

# Secondary profiles
[[profiles]]
name = "secondary"
# ...
```

## Performance Impact

### Startup Time

Each preloaded model adds ~2-3 seconds to startup:

- 1 profile: ~2-3s startup
- 2 profiles: ~4-6s startup
- 3 profiles: ~6-9s startup

### Memory Usage

Example configurations:

**Lightweight (1 profile):**
```toml
[[profiles]]
model_type = "base.en"  # 142MB
preload = true
```
**Total RAM:** ~650MB (142MB model + 500MB overhead)

**Balanced (2 profiles):**
```toml
[[profiles]]
model_type = "base.en"  # 142MB
preload = true

[[profiles]]
model_type = "small"    # 466MB
preload = true
```
**Total RAM:** ~1.1GB (608MB models + 500MB overhead)

**Power user (3 profiles, selective preload):**
```toml
[[profiles]]
model_type = "base.en"  # 142MB
preload = true

[[profiles]]
model_type = "small"    # 466MB
preload = true

[[profiles]]
model_type = "medium"   # 1.5GB
preload = false         # Lazy load
```
**Total RAM:** ~1.1GB (only preloaded models)

### CPU Usage

Idle CPU per profile: negligible (~0.1% per profile)

Active transcription: Uses configured `threads` (isolated per profile)

## Troubleshooting

### Hotkey Conflict Between Profiles

**Error:**
```
Failed to register hotkey for profile 'profile2': already registered
```

**Cause:** Two profiles have same `modifiers` + `key`

**Solution:** Assign unique hotkeys to each profile

### Model Download Timeout

**Error:**
```
Failed to download model for profile 'profile-name': timeout
```

**Solution:**
1. **Check internet connection**
2. **Manual download:**
   ```bash
   cd ~/.whisper-hotkey/models/
   curl -L -o ggml-base.en.bin \
     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
   ```

### Out of Memory

**Symptom:** App crashes with "Out of memory" error

**Cause:** Too many preloaded models

**Solution:**
1. **Reduce preloaded models:**
   ```toml
   [[profiles]]
   preload = false  # Change to lazy load
   ```
2. **Use smaller models:**
   ```toml
   model_type = "base.en"  # Instead of "medium"
   ```

### Profile Not Working

**Symptom:** Hotkey for specific profile doesn't trigger

**Solutions:**
1. **Check config syntax:**
   ```bash
   # Validate TOML syntax
   mise exec -- cargo run --release
   # Look for parse errors in startup logs
   ```

2. **Check hotkey registration:**
   ```bash
   # Look for "Hotkey registered" message for each profile
   ```

3. **Check permissions** (Accessibility, Microphone)

## Best Practices

1. **Limit preloaded profiles** to 2-3 most-used
2. **Use meaningful names** for profiles with explicit `name` field
3. **Assign ergonomic hotkeys** (easy to reach while speaking)
4. **Test each profile** after configuration changes
5. **Document your setup** (comment config file)

Example commented config:

```toml
# Fast, everyday transcription
[[profiles]]
name = "fast"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 1
language = "en"

# Accurate transcription for important documents
# Slower but better accuracy with beam search
[[profiles]]
name = "accurate"
model_type = "small"
modifiers = ["Command", "Shift"]
key = "V"
preload = true
threads = 4
beam_size = 5  # 5x slower, better accuracy
language = "en"
```

## Next Steps

- Learn about [Alias Matching](./alias-matching.md) for auto-expansion
- Optimize [Performance Settings](../configuration/performance.md)
- See [Model Selection](../configuration/models.md) for model comparison

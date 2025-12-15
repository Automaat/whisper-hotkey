# Model Selection

Choose the optimal Whisper model for your speed/accuracy requirements.

## Available Models

### Comparison Table

| Model | Size | RAM | Speed (10s audio) | Accuracy | Languages |
|-------|------|-----|-------------------|----------|-----------|
| tiny | 75MB | ~350MB | ~0.5s | ⭐⭐ | 99 |
| tiny.en | 75MB | ~350MB | ~0.5s | ⭐⭐ | English |
| base | 142MB | ~500MB | ~1s | ⭐⭐⭐ | 99 |
| **base.en** | 142MB | ~500MB | ~1s | ⭐⭐⭐ | English |
| small | 466MB | ~1.3GB | ~2s | ⭐⭐⭐⭐ | 99 |
| small.en | 466MB | ~1.3GB | ~2s | ⭐⭐⭐⭐ | English |
| medium | 1.5GB | ~3.5GB | ~6s | ⭐⭐⭐⭐⭐ | 99 |
| medium.en | 1.5GB | ~3.5GB | ~6s | ⭐⭐⭐⭐⭐ | English |
| large | 3GB | ~7GB | ~12s | ⭐⭐⭐⭐⭐ | 99 |
| large-v1 | 3GB | ~7GB | ~12s | ⭐⭐⭐⭐⭐ | 99 |
| large-v2 | 3GB | ~7GB | ~12s | ⭐⭐⭐⭐⭐ | 99 |
| large-v3 | 3GB | ~7GB | ~12s | ⭐⭐⭐⭐⭐ | 99 |

**Bold = Recommended default**

*Benchmarks on M1 Pro with beam_size=1, threads=4*

## Model Recommendations

### For Most Users: `base.en`

**Best balance of speed and accuracy for English:**

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

**Pros:**
- ✅ Fast (~1s for 10s audio)
- ✅ Small download (142MB)
- ✅ Good accuracy for everyday use
- ✅ Low memory (~500MB)

**Cons:**
- ⚠️ English only
- ⚠️ May miss technical terms

### For Speed: `tiny.en`

**Fastest transcription, acceptable accuracy:**

```toml
[[profiles]]
model_type = "tiny.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 8
beam_size = 1
language = "en"
```

**Pros:**
- ✅ Very fast (~0.5s for 10s audio)
- ✅ Minimal download (75MB)
- ✅ Lowest memory (~350MB)

**Cons:**
- ⚠️ Lower accuracy (may need corrections)
- ⚠️ Worse with accents or technical terms
- ⚠️ English only

### For Accuracy: `small.en`

**Better accuracy for important documents:**

```toml
[[profiles]]
model_type = "small.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 5
language = "en"
```

**Pros:**
- ✅ Excellent accuracy
- ✅ Better with technical terms
- ✅ Better with accents

**Cons:**
- ⚠️ Slower (~2s for 10s audio, ~10s with beam_size=5)
- ⚠️ Larger download (466MB)
- ⚠️ More memory (~1.3GB)
- ⚠️ English only

### For Multi-Language: `base` or `small`

**Non-English transcription:**

```toml
[[profiles]]
model_type = "base"  # or "small" for better accuracy
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 1
language = "es"  # Spanish (or "fr", "de", "ja", etc.)
```

**Note:** Multilingual models (without `.en`) support 99 languages but are slightly slower.

### For Maximum Accuracy: `medium` or `large`

**Critical use cases only:**

```toml
[[profiles]]
model_type = "medium"
modifiers = ["Control", "Option"]
key = "Z"
preload = false  # Lazy load to save memory
threads = 4
beam_size = 10
language = "en"
```

**Pros:**
- ✅ Best possible accuracy
- ✅ Excellent with accents, technical terms
- ✅ Best for non-English languages

**Cons:**
- ❌ Very slow (6-12s for 10s audio)
- ❌ Large download (1.5-3GB)
- ❌ High memory (3.5-7GB)
- ❌ Not suitable for real-time use

## English-Only vs Multilingual

### English-Only Models (`.en`)

**Models:** `tiny.en`, `base.en`, `small.en`, `medium.en`

**Pros:**
- ✅ 10-20% faster than multilingual
- ✅ Better English accuracy
- ✅ Skips language detection overhead

**Cons:**
- ❌ English only

**Use when:** Only transcribing English

### Multilingual Models

**Models:** `tiny`, `base`, `small`, `medium`, `large`, `large-v1/v2/v3`

**Pros:**
- ✅ 99 languages supported
- ✅ Auto-detects language (if `language` not specified)
- ✅ Same model for all languages

**Cons:**
- ⚠️ 10-20% slower than English-only
- ⚠️ Slightly lower English accuracy

**Use when:** Transcribing multiple languages

## Supported Languages

Multilingual models support 99 languages:

**Most common:**
- `"en"` - English
- `"es"` - Spanish
- `"fr"` - French
- `"de"` - German
- `"it"` - Italian
- `"pt"` - Portuguese
- `"nl"` - Dutch
- `"ru"` - Russian
- `"zh"` - Chinese
- `"ja"` - Japanese
- `"ko"` - Korean
- `"ar"` - Arabic
- `"hi"` - Hindi

See [full language list](https://github.com/openai/whisper/blob/main/whisper/tokenizer.py#L10) for all 99 supported languages.

## Model Download

### Automatic Download

Models are downloaded automatically on first use:

1. App starts, checks if model exists
2. If not found, downloads from Hugging Face
3. Saves to `~/.whisper-hotkey/models/ggml-{name}.bin`
4. Loads model into memory

**Download time:**
- `base.en` (142MB): ~30s on fast connection
- `small` (466MB): ~2min on fast connection
- `medium` (1.5GB): ~5min on fast connection

### Manual Download

If automatic download fails:

```bash
cd ~/.whisper-hotkey/models/

# Download base.en model
curl -L -o ggml-base.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin

# Download small model
curl -L -o ggml-small.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin
```

**Hugging Face URLs:**
```
https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{MODEL}.bin
```

Replace `{MODEL}` with: `tiny`, `tiny.en`, `base`, `base.en`, `small`, `small.en`, `medium`, `medium.en`, `large`, `large-v1`, `large-v2`, `large-v3`

### Verify Download

```bash
ls -lh ~/.whisper-hotkey/models/

# Should show:
# -rw-r--r--  142M  ggml-base.en.bin
```

## Model Storage

### Location

Models are stored in:
```
~/.whisper-hotkey/models/ggml-{name}.bin
```

### Disk Space

| Model | Disk Space |
|-------|-----------|
| tiny.en | 75MB |
| base.en | 142MB |
| small.en | 466MB |
| medium.en | 1.5GB |
| large | 3GB |

**Total:** Sum of all models you use

### Cleanup

Remove unused models:

```bash
# List models
ls -lh ~/.whisper-hotkey/models/

# Remove specific model
rm ~/.whisper-hotkey/models/ggml-medium.bin
```

## Switching Models

### Change Model

1. **Edit config:**
   ```toml
   [[profiles]]
   model_type = "small.en"  # Changed from "base.en"
   ```

2. **Restart app:** Ctrl+C, then restart

3. **Model downloads** if not present

4. **Test** with hotkey

### Multiple Models

Use multiple profiles with different models:

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
model_type = "small.en"
modifiers = ["Command", "Shift"]
key = "V"
preload = true
```

See [Multi-Profile Support](../usage/profiles.md).

## Performance Tuning

### Speed vs Accuracy Trade-offs

**Maximize speed:**
```toml
model_type = "tiny.en"
threads = 8
beam_size = 1
```

**Balanced:**
```toml
model_type = "base.en"
threads = 4
beam_size = 1
```

**Maximize accuracy:**
```toml
model_type = "small.en"
threads = 4
beam_size = 5
```

See [Performance Tuning](./performance.md) for details.

## Troubleshooting

### Model Download Fails

**Error:**
```
Failed to download model: connection timeout
```

**Solutions:**
1. **Check internet connection**
2. **Try manual download** (see above)
3. **Check Hugging Face status:** https://status.huggingface.co/
4. **Use VPN** if region-blocked

### Model Not Found

**Error:**
```
Model not found at ~/.whisper-hotkey/models/ggml-base.en.bin
```

**Solutions:**
1. **Check file exists:**
   ```bash
   ls ~/.whisper-hotkey/models/
   ```

2. **Download manually** (see above)

3. **Check permissions:**
   ```bash
   ls -l ~/.whisper-hotkey/models/
   # Should be readable
   ```

### Model Load Fails

**Error:**
```
Failed to load Whisper model: invalid file format
```

**Causes:**
1. **Corrupted download**
2. **Wrong file format**

**Solution:**
```bash
# Delete corrupted file
rm ~/.whisper-hotkey/models/ggml-base.en.bin

# Restart app - redownloads
```

### Out of Memory

**Error:**
```
Out of memory loading model
```

**Solution:**
1. **Close other apps** (free RAM)
2. **Use smaller model:**
   ```toml
   model_type = "base.en"  # Instead of "small" or "medium"
   ```
3. **Disable preload:**
   ```toml
   preload = false  # Lazy load
   ```

## Next Steps

- Tune [Performance Settings](./performance.md)
- Set up [Multi-Profile Support](../usage/profiles.md) with different models
- See [Configuration Reference](./reference.md) for all options

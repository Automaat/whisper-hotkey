# Configuration Reference

Complete reference for `~/.whisper-hotkey/config.toml` configuration file.

## File Location

Configuration file: `~/.whisper-hotkey/config.toml`

Created automatically on first run with sensible defaults.

## File Format

TOML format (Tom's Obvious, Minimal Language).

**Basic structure:**

```toml
# Profiles (each with hotkey + model config)
[[profiles]]
# ... profile settings ...

# Audio capture settings
[audio]
# ... audio settings ...

# Telemetry settings
[telemetry]
# ... telemetry settings ...

# Recording settings
[recording]
# ... recording settings ...

# Aliases settings
[aliases]
# ... aliases settings ...
```

## Complete Example

```toml
# === Transcription Profiles ===

# Fast profile with base.en model
[[profiles]]
name = "fast"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 1
language = "en"

# Accurate profile with small model
[[profiles]]
name = "accurate"
model_type = "small"
modifiers = ["Command", "Shift"]
key = "V"
preload = true
threads = 4
beam_size = 5
language = "en"

# === Audio Settings ===

[audio]
buffer_size = 1024
sample_rate = 16000

# === Telemetry Settings ===

[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"

# === Recording Settings ===

[recording]
enabled = true
retention_days = 7
max_count = 100
cleanup_interval_hours = 1

# === Aliases Settings ===

[aliases]
enabled = true
threshold = 0.8

[aliases.entries]
"period" = "."
"comma" = ","
"dot com" = ".com"
"at sign" = "@"
```

## Profiles

### `[[profiles]]`

Array of transcription profiles. Each profile is independent.

**Required fields:**
- `model_type` (string) - Whisper model
- `modifiers` (array of strings) - Modifier keys
- `key` (string) - Main key

**Optional fields:**
- `name` (string) - Profile name (default: auto-generated from `model_type`)
- `preload` (boolean) - Preload model at startup (default: `true`)
- `threads` (integer) - CPU threads for inference (default: `4`)
- `beam_size` (integer) - Beam search width (default: `1`)
- `language` (string) - Language code (default: `"en"`)

### `model_type`

**Type:** String

**Valid values:**
- `"tiny"` - Multilingual tiny (~75MB)
- `"tiny.en"` - English-only tiny (~75MB)
- `"base"` - Multilingual base (~142MB)
- `"base.en"` - English-only base (~142MB) **[Recommended for single profile]**
- `"small"` - Multilingual small (~466MB)
- `"small.en"` - English-only small (~466MB) **[Recommended for accuracy]**
- `"medium"` - Multilingual medium (~1.5GB)
- `"medium.en"` - English-only medium (~1.5GB)
- `"large"` - Multilingual large (~3GB)
- `"large-v1"` - Multilingual large v1 (~3GB)
- `"large-v2"` - Multilingual large v2 (~3GB)
- `"large-v3"` - Multilingual large v3 (~3GB)

**Example:**
```toml
[[profiles]]
model_type = "base.en"
# ...
```

See [Model Selection](./models.md) for comparison.

### `modifiers`

**Type:** Array of strings

**Valid values:**
- `"Control"` - Ctrl key
- `"Option"` - Option/Alt key
- `"Command"` - Command/Cmd key
- `"Shift"` - Shift key

**Example:**
```toml
[[profiles]]
modifiers = ["Control", "Option"]
# ...
```

### `key`

**Type:** String

**Valid values:**
- Letters: `"A"` through `"Z"` (case-insensitive)
- Numbers: `"0"` through `"9"`
- Function keys: `"F1"` through `"F12"`
- Special: `"Space"`, `"Tab"`, `"Return"`, `"Escape"`

**Example:**
```toml
[[profiles]]
key = "Z"
# ...
```

### `name`

**Type:** String (optional)

**Default:** Auto-generated from `model_type`

**Example:**
```toml
[[profiles]]
name = "fast-english"
model_type = "base.en"
# ...
```

Use explicit names when multiple profiles share the same `model_type`.

### `preload`

**Type:** Boolean

**Default:** `true`

**Values:**
- `true` - Load model at startup (instant first transcription)
- `false` - Load model on first use (saves memory)

**Example:**
```toml
[[profiles]]
preload = true
# ...
```

### `threads`

**Type:** Integer

**Default:** `4`

**Valid range:** `1` to CPU core count

**Recommendations:**
- M1/M2/M3 (8 cores): `4` or `8`
- Intel (4 cores): `2` or `4`
- Intel (2 cores): `2`

**Example:**
```toml
[[profiles]]
threads = 4
# ...
```

### `beam_size`

**Type:** Integer

**Default:** `1`

**Valid range:** `1` to `10`

**Values:**
- `1` - Greedy decoding (fastest, good accuracy)
- `5` - Beam search (balanced, better accuracy)
- `10` - Wide beam search (slowest, best accuracy)

**Performance impact:**
- `beam_size = 5` is ~5x slower than `beam_size = 1`
- `beam_size = 10` is ~10x slower than `beam_size = 1`

**Example:**
```toml
[[profiles]]
beam_size = 1
# ...
```

### `language`

**Type:** String (optional)

**Default:** `"en"`

**Values:**
- `"en"` - English (skips auto-detect)
- `"es"` - Spanish
- `"fr"` - French
- `"de"` - German
- `"it"` - Italian
- `"ja"` - Japanese
- `"zh"` - Chinese
- `null` or omit - Auto-detect (adds ~200ms overhead)

**Example:**
```toml
[[profiles]]
language = "en"
# ...
```

See [Whisper language codes](https://github.com/openai/whisper/blob/main/whisper/tokenizer.py) for full list.

## Audio

### `[audio]`

Audio capture configuration.

**Fields:**
- `buffer_size` (integer) - Ring buffer size in samples (default: `1024`)
- `sample_rate` (integer) - Sample rate in Hz (default: `16000`)

**Example:**
```toml
[audio]
buffer_size = 1024
sample_rate = 16000
```

### `buffer_size`

**Type:** Integer

**Default:** `1024`

**Valid range:** `256` to `4096`

**Description:** Ring buffer size for audio capture.

**Recommendations:**
- Leave at `1024` for most use cases
- Increase to `2048` if audio glitches occur
- Decrease to `512` for lower latency (may cause glitches)

### `sample_rate`

**Type:** Integer

**Default:** `16000`

**Valid values:** `16000` (Whisper requirement)

**Description:** Audio sample rate in Hz. Whisper requires 16kHz.

**Do not change** unless you know what you're doing.

## Telemetry

### `[telemetry]`

Local crash logging configuration (no cloud telemetry).

**Fields:**
- `enabled` (boolean) - Enable local crash logs (default: `true`)
- `log_path` (string) - Log file path (default: `"~/.whisper-hotkey/crash.log"`)

**Example:**
```toml
[telemetry]
enabled = true
log_path = "~/.whisper-hotkey/crash.log"
```

### `enabled`

**Type:** Boolean

**Default:** `true`

**Values:**
- `true` - Write crash logs locally (recommended)
- `false` - Disable logging (not recommended)

**Privacy:** Logs are 100% local, never sent anywhere.

### `log_path`

**Type:** String

**Default:** `"~/.whisper-hotkey/crash.log"`

**Description:** Path to log file (supports `~` expansion).

**Example:**
```toml
log_path = "~/.whisper-hotkey/logs/crash.log"
```

## Recording

### `[recording]`

Debug recording configuration (saves audio files for debugging).

**Fields:**
- `enabled` (boolean) - Enable debug recordings (default: `true`)
- `retention_days` (integer) - Delete recordings older than N days (default: `7`)
- `max_count` (integer) - Keep only N most recent recordings (default: `100`)
- `cleanup_interval_hours` (integer) - Hours between cleanup runs (default: `1`)

**Example:**
```toml
[recording]
enabled = true
retention_days = 7
max_count = 100
cleanup_interval_hours = 1
```

### `enabled`

**Type:** Boolean

**Default:** `true`

**Values:**
- `true` - Save WAV files for debugging
- `false` - Don't save recordings

**Recordings saved to:** `~/.whisper-hotkey/recordings/`

### `retention_days`

**Type:** Integer

**Default:** `7`

**Valid range:** `0` to any positive integer

**Values:**
- `0` - Keep all recordings forever
- `7` - Delete recordings older than 7 days (default)
- `1` - Delete recordings older than 1 day

### `max_count`

**Type:** Integer

**Default:** `100`

**Valid range:** `0` to any positive integer

**Values:**
- `0` - Unlimited recordings
- `100` - Keep only 100 most recent (default)

### `cleanup_interval_hours`

**Type:** Integer

**Default:** `1`

**Valid range:** `0` to any positive integer

**Values:**
- `0` - Cleanup only at startup
- `1` - Cleanup every hour (default)
- `24` - Cleanup once per day

## Aliases

### `[aliases]`

Alias matching configuration for auto-expansion.

**Fields:**
- `enabled` (boolean) - Enable alias matching (default: `true`)
- `threshold` (float) - Similarity threshold 0.0-1.0 (default: `0.8`)

**Example:**
```toml
[aliases]
enabled = true
threshold = 0.8

[aliases.entries]
"period" = "."
"comma" = ","
"dot com" = ".com"
```

### `enabled`

**Type:** Boolean

**Default:** `true`

**Values:**
- `true` - Enable alias matching
- `false` - Disable alias matching

### `threshold`

**Type:** Float

**Default:** `0.8`

**Valid range:** `0.0` to `1.0`

**Values:**
- `1.0` - Exact match only
- `0.9` - Very strict
- `0.8` - Balanced (default, recommended)
- `0.7` - Lenient
- `0.6` - Very lenient

### `[aliases.entries]`

**Type:** Key-value pairs (trigger → output)

**Format:**
```toml
[aliases.entries]
"trigger phrase" = "output text"
```

**Example:**
```toml
[aliases.entries]
"period" = "."
"comma" = ","
"semicolon" = ";"
"dot com" = ".com"
"at sign" = "@"
"my email" = "user@example.com"
```

See [Alias Matching](../usage/alias-matching.md) for details.

## Legacy Fields

These fields are deprecated but still supported for backward compatibility:

### `[hotkey]`, `[model]`

**Deprecated:** Use `[[profiles]]` instead.

**Old format:**
```toml
[hotkey]
modifiers = ["Control", "Option"]
key = "Z"

[model]
model_type = "base.en"
preload = true
threads = 4
beam_size = 1
```

**New format:**
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

## Validation

### Automatic Validation

Config is validated on startup:
- Missing required fields → Use defaults
- Invalid values → Error message + use defaults
- Invalid TOML syntax → Error message + exit

### Manual Validation

Test config without running app:

```bash
# Dry run (validates config only)
mise exec -- cargo run --release -- --validate-config
```

## Reloading Configuration

Config is loaded at startup only. To apply changes:

1. **Edit** `~/.whisper-hotkey/config.toml`
2. **Restart** app (Ctrl+C, then restart)
3. **Test** changes

## Backup and Restore

### Backup

```bash
cp ~/.whisper-hotkey/config.toml ~/.whisper-hotkey/config.toml.backup
```

### Restore

```bash
cp ~/.whisper-hotkey/config.toml.backup ~/.whisper-hotkey/config.toml
```

### Reset to Defaults

```bash
rm ~/.whisper-hotkey/config.toml
# Restart app - creates default config
```

## Next Steps

- Learn about [Audio Settings](./audio.md) for capture configuration
- See [Model Selection](./models.md) for model comparison
- Optimize [Performance Settings](./performance.md)

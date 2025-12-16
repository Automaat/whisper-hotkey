# Multi-Profile Support

Multiple hotkeys with different models and settings.

## Common Setups

### Fast vs. Accurate

```toml
# Fast - daily use
[[profiles]]
name = "fast"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true
threads = 4
beam_size = 1
language = "en"

# Accurate - important docs
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

**Usage:**
- `Ctrl+Option+Z` → Fast (base.en, ~1s)
- `Cmd+Shift+V` → Accurate (small, ~10s with beam_size=5)

### Multi-Language

```toml
# English
[[profiles]]
name = "english"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "E"
language = "en"

# Spanish
[[profiles]]
name = "spanish"
model_type = "base"
modifiers = ["Control", "Option"]
key = "S"
language = "es"
```

**Usage:**
- `Ctrl+Option+E` → English
- `Ctrl+Option+S` → Spanish

## Configuration Fields

### Required

```toml
[[profiles]]
model_type = "base.en"              # Which model
modifiers = ["Control", "Option"]   # Modifier keys
key = "Z"                           # Main key
```

### Optional (with defaults)

```toml
name = "my-profile"    # Default: uses model_type
preload = true         # Default: true (load at startup)
threads = 4            # Default: 4
beam_size = 1          # Default: 1 (fastest)
language = "en"        # Default: "en"
```

## Model Preloading

**`preload = true`** (recommended):
- Loads at startup (2-3s per model)
- Instant first transcription
- Stays in memory

**`preload = false`**:
- Loads on first use
- 2-3s delay before first transcription
- Saves memory

**Memory usage:**
- base.en: ~142MB
- small: ~466MB
- medium: ~1.5GB

**Tip:** Preload daily-use profiles, lazy load occasional ones.

## Managing Profiles

### Add Profile

1. Edit `~/.whisper-hotkey/config.toml`
2. Add new `[[profiles]]` section
3. Restart app
4. Model downloads automatically if needed

### Remove Profile

1. Delete `[[profiles]]` section from config
2. Restart app
3. Optional: Delete model file:
   ```bash
   rm ~/.whisper-hotkey/models/ggml-{model-name}.bin
   ```

### Switch Between Profiles

Use different hotkeys - they're independent:
- `Ctrl+Option+Z` → Profile 1
- `Cmd+Shift+V` → Profile 2

## Complete Example

```toml
# Fast mode - everyday use
[[profiles]]
name = "fast"
model_type = "base.en"
modifiers = ["Control", "Option"]
key = "Z"
preload = true

# Accurate mode - important documents
[[profiles]]
name = "accurate"
model_type = "small"
modifiers = ["Command", "Shift"]
key = "V"
preload = true
beam_size = 5  # Slower but more accurate

# Spanish - occasional use
[[profiles]]
name = "spanish"
model_type = "base"
modifiers = ["Control", "Option"]
key = "S"
preload = false  # Lazy load (saves memory)
language = "es"
```

## Troubleshooting

**Duplicate hotkey error:**

Each profile needs unique hotkey:
```toml
[[profiles]]
key = "Z"

[[profiles]]
key = "V"  # Different key
```

**Out of memory:**

Too many preloaded models. Set `preload = false` or use smaller models:
```toml
[[profiles]]
preload = false
# OR
model_type = "base.en"  # Instead of small/medium
```

**Profile not working:**

1. Check config syntax (valid TOML)
2. Restart app after config changes
3. Check logs: `tail -f ~/.whisper-hotkey/crash.log`

## Tips

- Limit to 2-3 preloaded profiles
- Use meaningful names
- Comment your config
- Put most-used profile first (loads first)

## Next Steps

- [Alias Matching](./alias-matching.md) - Auto-expand phrases
- [Model Selection](../configuration/models.md) - Compare all models
- [Performance Tuning](../configuration/performance.md) - Optimize settings

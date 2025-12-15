# Alias Matching

Auto-expand common phrases and words using fuzzy matching.

## What Is Alias Matching?

Alias matching automatically replaces transcribed phrases with predefined outputs:

- **Input:** "dot com" (spoken)
- **Transcribed:** "dot com"
- **Output:** ".com" (inserted)

Uses fuzzy string matching to handle Whisper transcription variations.

## Use Cases

### Punctuation

```toml
[aliases]
enabled = true
threshold = 0.8

[aliases.entries]
"period" = "."
"comma" = ","
"semicolon" = ";"
"colon" = ":"
"question mark" = "?"
"exclamation mark" = "!"
"exclamation point" = "!"
"dash" = "-"
"hyphen" = "-"
"underscore" = "_"
```

### Common Phrases

```toml
[aliases.entries]
"dot com" = ".com"
"dot org" = ".org"
"dot net" = ".net"
"at sign" = "@"
"hashtag" = "#"
"dollar sign" = "$"
"percent sign" = "%"
"ampersand" = "&"
```

### Code Symbols

```toml
[aliases.entries]
"left paren" = "("
"right paren" = ")"
"left bracket" = "["
"right bracket" = "]"
"left brace" = "{"
"right brace" = "}"
"equals" = "="
"plus" = "+"
"minus" = "-"
"asterisk" = "*"
"slash" = "/"
"backslash" = "\\"
"pipe" = "|"
```

### Email/URL Shortcuts

```toml
[aliases.entries]
"my email" = "user@example.com"
"work email" = "user@company.com"
"personal website" = "https://example.com"
"github profile" = "https://github.com/username"
```

### Frequently Used Text

```toml
[aliases.entries]
"my phone" = "+1-555-123-4567"
"my address" = "123 Main St, City, ST 12345"
"meeting link" = "https://zoom.us/j/123456789"
"signature" = "Best regards,\nYour Name"
```

## Configuration

### Basic Setup

Edit `~/.whisper-hotkey/config.toml`:

```toml
[aliases]
enabled = true       # Enable alias matching (default: true)
threshold = 0.8      # Similarity threshold 0.0-1.0 (default: 0.8)

[aliases.entries]
# Add your aliases here
"dot com" = ".com"
"at sign" = "@"
```

### Threshold

The `threshold` controls fuzzy matching sensitivity:

- **`1.0`** - Exact match only (no typos allowed)
- **`0.9`** - Very strict (1-2 char difference)
- **`0.8`** - Balanced (default, handles most Whisper variations)
- **`0.7`** - Lenient (allows more variation)
- **`0.6`** - Very lenient (may match unintended phrases)

**Recommendation:** Start with `0.8`, adjust if needed.

### Alias Format

```toml
[aliases.entries]
"trigger phrase" = "output text"
```

- **Trigger phrase** (left): What Whisper transcribes
- **Output text** (right): What gets inserted

**Rules:**
- Case-insensitive matching
- Whitespace normalized
- Uses Levenshtein distance for fuzzy matching

## Examples

### Simple Punctuation

**Config:**
```toml
[aliases.entries]
"period" = "."
"comma" = ","
```

**Usage:**
- Say: "Hello world period"
- Transcribed: "Hello world period"
- Inserted: "Hello world."

### Multi-Word Aliases

**Config:**
```toml
[aliases.entries]
"new paragraph" = "\n\n"
"line break" = "\n"
```

**Usage:**
- Say: "First paragraph new paragraph second paragraph"
- Inserted: "First paragraph\n\nSecond paragraph"

### Special Characters

**Config:**
```toml
[aliases.entries]
"left arrow" = "←"
"right arrow" = "→"
"check mark" = "✓"
"cross mark" = "✗"
```

### Code Templates

**Config:**
```toml
[aliases.entries]
"function def" = "function () {\n  \n}"
"if statement" = "if () {\n  \n}"
"for loop" = "for (let i = 0; i < ; i++) {\n  \n}"
```

## Fuzzy Matching Behavior

### Why Fuzzy Matching?

Whisper may transcribe trigger phrases with slight variations:

- "period" → "Period" (case difference)
- "dot com" → "dotcom" (spacing difference)
- "at sign" → "at sine" (transcription error)

Fuzzy matching handles these automatically.

### Matching Examples

With `threshold = 0.8`:

| Spoken | Transcribed | Alias | Matches? |
|--------|-------------|-------|----------|
| "period" | "period" | "period" = "." | ✅ Yes (exact) |
| "period" | "Period" | "period" = "." | ✅ Yes (case) |
| "dot com" | "dotcom" | "dot com" = ".com" | ✅ Yes (spacing) |
| "at sign" | "at sine" | "at sign" = "@" | ✅ Yes (1 char) |
| "comma" | "coma" | "comma" = "," | ✅ Yes (1 char) |
| "semicolon" | "semi colon" | "semicolon" = ";" | ✅ Yes (spacing) |
| "period" | "paragraph" | "period" = "." | ❌ No (too different) |

### Multiple Matches

If multiple aliases match, **best match wins** (highest similarity).

**Example:**

```toml
[aliases.entries]
"at" = "@"
"at sign" = "@"
```

- Transcribed: "at sign"
- "at sign" matches better than "at"
- Uses "at sign" mapping

## Performance

### Matching Speed

Alias matching adds minimal overhead:
- ~1-2ms for 10 aliases
- ~5-10ms for 100 aliases
- ~20-30ms for 1000 aliases

**Recommendation:** Keep under 100 aliases for best performance.

### Memory Usage

Negligible (~1KB per alias).

## Advanced Usage

### Context-Aware Aliases

Use profile-specific aliases by creating separate configs (future feature).

Current workaround: Include all aliases, use unique trigger phrases.

```toml
[aliases.entries]
# General aliases
"period" = "."

# Code-specific (unique phrases)
"rust function" = "fn () {\n  \n}"
"python function" = "def ():\n    "

# Email-specific
"email sign off" = "Best regards,\nYour Name"
```

### Multi-Line Output

Use `\n` for newlines:

```toml
[aliases.entries]
"my signature" = "Best regards,\nJohn Doe\nSenior Engineer\ncompany@example.com"
```

### Escape Characters

TOML string escaping:

```toml
[aliases.entries]
"quote" = "\""        # Double quote
"single quote" = "'"  # Single quote
"backslash" = "\\"    # Backslash
"tab" = "\t"          # Tab character
```

### Unicode Characters

Full Unicode support:

```toml
[aliases.entries]
"smiley face" = "😊"
"check mark" = "✓"
"arrow right" = "→"
"lambda" = "λ"
```

## Troubleshooting

### Alias Not Matching

**Symptom:** Spoke trigger phrase but didn't expand

**Solutions:**

1. **Check transcription:**
   ```bash
   # Look at console output for actual transcription
   ✨ Transcription: "actual text"
   ```

2. **Lower threshold:**
   ```toml
   threshold = 0.7  # More lenient
   ```

3. **Add alternative triggers:**
   ```toml
   "period" = "."
   "full stop" = "."  # British English alternative
   ```

4. **Check alias is enabled:**
   ```toml
   [aliases]
   enabled = true
   ```

### Wrong Alias Matched

**Symptom:** Different alias matched than intended

**Cause:** Multiple similar aliases, wrong one had better score

**Solution:** Make trigger phrases more distinct:

**Before:**
```toml
"at" = "@"
"at sign" = "@gmail.com"  # Too similar
```

**After:**
```toml
"at symbol" = "@"
"gmail address" = "@gmail.com"  # More distinct
```

### Alias Matching Too Aggressive

**Symptom:** Unintended words getting replaced

**Solution:** Increase threshold:

```toml
threshold = 0.9  # More strict
```

### Special Characters Not Working

**Symptom:** Backslashes or quotes not inserting correctly

**Solution:** Use proper TOML escaping:

```toml
"backslash" = "\\\\"  # Double escape for TOML
"quote" = "\""        # Escape double quote
"newline" = "\\n"     # Literal \n (not newline)
```

## Best Practices

1. **Start small** - Add 5-10 common aliases first
2. **Test each alias** - Verify trigger phrase matches consistently
3. **Use distinct phrases** - Avoid overlapping trigger phrases
4. **Document your aliases** - Comment config for future reference
5. **Keep threshold at 0.8** - Unless you have specific needs
6. **Group related aliases** - Use comments for organization

**Example organized config:**

```toml
[aliases]
enabled = true
threshold = 0.8

[aliases.entries]
# === Punctuation ===
"period" = "."
"comma" = ","
"semicolon" = ";"
"colon" = ":"

# === Common Phrases ===
"dot com" = ".com"
"at sign" = "@"

# === Personal Info ===
"my email" = "user@example.com"
"my phone" = "+1-555-123-4567"

# === Code Symbols ===
"left paren" = "("
"right paren" = ")"
"equals" = "="
```

## Disabling Alias Matching

To disable temporarily:

```toml
[aliases]
enabled = false
```

To disable completely, remove or comment out `[aliases]` section.

## Next Steps

- Optimize [Performance Settings](../configuration/performance.md)
- Learn about [Configuration Reference](../configuration/reference.md)
- See [Common Issues](../troubleshooting/common-issues.md)

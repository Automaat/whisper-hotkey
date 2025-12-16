# Alias Matching

Auto-expand spoken phrases into text. Say "period" → inserts "."

## Quick Start

Edit `~/.whisper-hotkey/config.toml`:

```toml
[aliases]
enabled = true
threshold = 0.8  # Default: handles Whisper transcription variations

[aliases.entries]
"period" = "."
"comma" = ","
"dot com" = ".com"
```

Restart app, then say: "Hello world period" → inserts "Hello world."

## Common Examples

### Punctuation

```toml
[aliases.entries]
"period" = "."
"comma" = ","
"question mark" = "?"
"exclamation mark" = "!"
"semicolon" = ";"
"colon" = ":"
"new line" = "\n"
"new paragraph" = "\n\n"
```

**Usage:** "Send email to John comma ask about meeting period"
→ "Send email to John, ask about meeting."

### Email & Web

```toml
[aliases.entries]
"at sign" = "@"
"dot com" = ".com"
"dot org" = ".org"
"my email" = "john@example.com"
"work email" = "john@company.com"
"github" = "https://github.com/username"
```

**Usage:** "Contact me at my email"
→ "Contact me at john@example.com"

### Code Symbols

```toml
[aliases.entries]
"equals" = "="
"plus" = "+"
"left paren" = "("
"right paren" = ")"
"left brace" = "{"
"right brace" = "}"
"arrow" = "=>"
```

**Usage:** "function getName left paren right paren left brace"
→ "function getName() {"

### Personal Snippets

```toml
[aliases.entries]
"my phone" = "+1-555-123-4567"
"my address" = "123 Main St, Anytown, CA 12345"
"meeting link" = "https://zoom.us/j/123456789"
"signature" = "Best regards,\nJohn Doe\njohn@example.com"
```

**Usage:** "Call me at my phone"
→ "Call me at +1-555-123-4567"

## Complete Example Config

```toml
[aliases]
enabled = true
threshold = 0.8

[aliases.entries]
# Punctuation
"period" = "."
"comma" = ","
"question mark" = "?"
"new line" = "\n"

# Common
"at sign" = "@"
"dot com" = ".com"
"dollar sign" = "$"

# Personal
"my email" = "john@example.com"
"my phone" = "+1-555-1234"
"my address" = "123 Main St, City, ST 12345"
```

## Tips

**Threshold:** Default `0.8` works for most cases. Handles minor transcription variations.
- Too many false matches? Increase to `0.9`
- Not matching? Lower to `0.7`

**Testing:** Say the trigger phrase in normal speech to verify it works.

**Organizing:** Group related aliases with comments for easier management.

## Troubleshooting

**Alias not working:**
1. Check alias enabled: `enabled = true`
2. Verify trigger phrase is what Whisper actually transcribes
3. Try lowering threshold to `0.7`

**Disable aliases:**
```toml
[aliases]
enabled = false
```

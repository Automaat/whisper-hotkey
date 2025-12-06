# Homebrew Distribution Setup

## Automated Release → Homebrew Update

This setup enables automatic Homebrew tap updates when a new release is created.

## One-Time Setup

### 1. Create Personal Access Token (PAT)

1. Go to: https://github.com/settings/tokens/new
2. Name: `HOMEBREW_TAP_TOKEN`
3. Expiration: 90 days (or custom)
4. Repository access: **Selected repositories**
   - Select: `Automaat/homebrew-whisper-hotkey`
5. Permissions:
   - **Contents**: Read and Write
   - **Metadata**: Read (automatic)
   - **Actions**: Write (to trigger workflows)
6. Click "Generate token"
7. Copy the token (starts with `ghp_`)

### 2. Add Token to Repository Secrets

1. Go to: https://github.com/Automaat/whisper-hotkey/settings/secrets/actions
2. Click "New repository secret"
3. Name: `HOMEBREW_TAP_TOKEN`
4. Value: Paste the token from step 1
5. Click "Add secret"

## How It Works

```mermaid
graph LR
    A[Create Release] --> B[Release Workflow]
    B --> C[Build DMG]
    C --> D[Create SHA256]
    D --> E[Publish Release]
    E --> F[Trigger Tap Update]
    F --> G[Update Cask Formula]
    G --> H[Users: brew upgrade]
```

1. **Release Creation**: Run `./scripts/create-release.sh`
2. **Build & Package**: GitHub Actions builds DMG
3. **Calculate SHA256**: Checksum generated
4. **Publish Release**: Assets uploaded to GitHub
5. **Trigger Update**: Dispatches event to tap repo
6. **Update Formula**: Tap workflow updates version/SHA256
7. **User Updates**: Available via `brew upgrade`

## Release Commands

```bash
# Create new release (auto-increments version)
./scripts/create-release.sh

# Create specific version
./scripts/create-release.sh 0.11.0

# Monitor release
gh run watch

# Verify tap updated (wait 2-3 min after release)
brew update
brew info whisper-hotkey
```

## Manual Trigger (Backup)

If automatic trigger fails:

```bash
# Option 1: Use script
./scripts/update-homebrew.sh

# Option 2: Trigger workflow manually
gh workflow run update-cask \
  --repo Automaat/homebrew-whisper-hotkey \
  -f version=0.11.0 \
  -f sha256=<SHA256_FROM_RELEASE>
```

## Troubleshooting

### Token Issues

If tap update fails with "Bad credentials":
1. Token expired → Create new token
2. Update secret in repository settings

### Workflow Not Triggering

Check:
- Token has `actions:write` permission
- Token has access to tap repository
- Release workflow completed successfully

### Formula Not Updating

Check tap workflow logs:
```bash
gh run list --repo Automaat/homebrew-whisper-hotkey
gh run view <RUN_ID> --repo Automaat/homebrew-whisper-hotkey
```

## Files Overview

**Main Repo** (`whisper-hotkey`):
- `.github/workflows/release.yml` - Triggers tap update after release
- `scripts/create-release.sh` - Creates release
- `scripts/update-homebrew.sh` - Manual backup script

**Tap Repo** (`homebrew-whisper-hotkey`):
- `.github/workflows/update-cask.yml` - Receives trigger, updates formula
- `Casks/whisper-hotkey.rb` - Formula that gets updated

## Security Notes

- PAT only needs access to tap repository
- Token expires (set reasonable expiration)
- Use repository secrets (never commit tokens)
- Minimal permissions (contents + actions)
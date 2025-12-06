# GitHub App Setup for Auto-Renewing Tokens

Use GitHub App instead of Personal Access Tokens for never-expiring automation.

## Why GitHub App?

| PAT | GitHub App |
|-----|------------|
| ❌ Expires (90 days - 1 year) | ✅ Never expires |
| ❌ Manual refresh required | ✅ Auto-refreshes hourly |
| ❌ User-scoped | ✅ App-scoped (more secure) |
| ❌ Full user permissions | ✅ Minimal permissions |

## Setup Steps

### 1. Create GitHub App

Go to: https://github.com/settings/apps/new

**Required Settings:**
```yaml
Name: WhisperHotkey Automation
Homepage URL: https://github.com/Automaat/whisper-hotkey
Webhook: ☐ Active (uncheck)

Repository permissions:
  Contents: Write
  Actions: Write
  Metadata: Read (automatic)

Where can this GitHub App be installed?:
  ◉ Only on this account
```

Click "Create GitHub App"

### 2. Generate Private Key

After creation:
1. Scroll to "Private keys" section
2. Click "Generate a private key"
3. Save the downloaded `.pem` file securely

### 3. Note App ID

At top of app settings page:
- **App ID**: `123456` (example)
- You'll need this number

### 4. Install App on Repositories

1. In app settings, click "Install App"
2. Choose your account
3. Select repositories:
   - ✓ `Automaat/whisper-hotkey`
   - ✓ `Automaat/homebrew-whisper-hotkey`
4. Click "Install"

### 5. Add Repository Secrets

Go to: https://github.com/Automaat/whisper-hotkey/settings/secrets/actions

**Secret 1: APP_ID**
- Name: `APP_ID`
- Value: Your app ID from step 3 (e.g., `123456`)

**Secret 2: APP_PRIVATE_KEY**
- Name: `APP_PRIVATE_KEY`
- Value: Entire contents of `.pem` file:
  ```
  -----BEGIN RSA PRIVATE KEY-----
  MIIEowIBAAKCAQEA...
  [full key content]
  ...XkK2tRfz
  -----END RSA PRIVATE KEY-----
  ```

### 6. Update Release Workflow

Edit `.github/workflows/release.yml`:

**Replace this:**
```yaml
- name: Trigger Homebrew Tap Update
  uses: peter-evans/repository-dispatch@v3
  with:
    token: ${{ secrets.HOMEBREW_TAP_TOKEN }}
    repository: Automaat/homebrew-whisper-hotkey
    event-type: update-cask
    client-payload: '{"version": "${{ steps.version.outputs.VERSION }}", "sha256": "${{ steps.sha256.outputs.sha256 }}"}'
```

**With this:**
```yaml
- name: Generate token
  id: generate_token
  uses: tibdex/github-app-token@v2
  with:
    app_id: ${{ secrets.APP_ID }}
    private_key: ${{ secrets.APP_PRIVATE_KEY }}

- name: Trigger Homebrew Tap Update
  uses: peter-evans/repository-dispatch@v3
  with:
    token: ${{ steps.generate_token.outputs.token }}
    repository: Automaat/homebrew-whisper-hotkey
    event-type: update-cask
    client-payload: '{"version": "${{ steps.version.outputs.VERSION }}", "sha256": "${{ steps.sha256.outputs.sha256 }}"}'
```

## Verification

Test the setup:
```bash
# Trigger a test workflow run
gh workflow run release.yml --ref feat/homebrew-distribution

# Watch the run
gh run watch
```

Check for:
- ✓ "Generate token" step succeeds
- ✓ "Trigger Homebrew Tap Update" succeeds
- ✓ Tap repo receives dispatch event

## Maintenance

**Never expires!** But if you need to:

- **Rotate key**: Generate new private key in app settings
- **Change permissions**: Update in app settings, reinstall on repos
- **Revoke access**: Uninstall app from repositories
- **Delete app**: Settings → Developer settings → GitHub Apps → Delete

## Troubleshooting

### "Error: HttpError: A GitHub App installation access token"

App not installed on target repository. Install on both:
- Main repo (whisper-hotkey)
- Tap repo (homebrew-whisper-hotkey)

### "Error: HttpError: Bad credentials"

Check:
1. APP_ID is correct (number only)
2. APP_PRIVATE_KEY includes full PEM content
3. App has correct permissions

### Token generation fails

Verify private key format:
```bash
# Should start with:
-----BEGIN RSA PRIVATE KEY-----
# Should end with:
-----END RSA PRIVATE KEY-----
```

## Benefits Over PAT

1. **Zero Maintenance**: No expiration dates to track
2. **Better Security**: App-specific, not user-wide access
3. **Audit Trail**: All actions logged to app
4. **Easy Revocation**: Just uninstall the app
5. **Free**: No cost for private repos

## Alternative: Keep Using PAT

If you prefer PAT:
1. Set expiration to 1 year (maximum)
2. Calendar reminder 1 week before expiry
3. Regenerate and update secret
4. Continue with `HOMEBREW_TAP_TOKEN`

## References

- [GitHub Apps Documentation](https://docs.github.com/en/apps)
- [tibdex/github-app-token Action](https://github.com/tibdex/github-app-token)
- [Authentication in GitHub Actions](https://docs.github.com/en/actions/security-guides/automatic-token-authentication)
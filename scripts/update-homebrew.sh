#!/bin/bash
set -e

# Update Homebrew tap after release
# Usage: ./scripts/update-homebrew.sh [version]

VERSION=${1:-$(gh release view --json tagName --jq '.tagName' | sed 's/^v//')}

if [ -z "$VERSION" ]; then
    echo "❌ No version specified and couldn't detect from latest release"
    exit 1
fi

echo "📦 Updating Homebrew tap for version $VERSION..."

# Download SHA256
SHA256=$(curl -sL "https://github.com/Automaat/whisper-hotkey/releases/download/v$VERSION/WhisperHotkey-$VERSION.dmg.sha256" | awk '{print $1}')

if [ -z "$SHA256" ]; then
    echo "❌ Failed to get SHA256 for version $VERSION"
    exit 1
fi

echo "✓ Version: $VERSION"
echo "✓ SHA256: $SHA256"

# Clone or update tap
TMPDIR=$(mktemp -d)
cd "$TMPDIR"
git clone git@github.com:Automaat/homebrew-whisper-hotkey.git
cd homebrew-whisper-hotkey

# Update cask
cat > Casks/whisper-hotkey.rb << EOF
cask "whisper-hotkey" do
  version "$VERSION"
  sha256 "$SHA256"

  url "https://github.com/Automaat/whisper-hotkey/releases/download/v#{version}/WhisperHotkey-#{version}.dmg"
  name "WhisperHotkey"
  desc "macOS voice-to-text via hotkey using local Whisper"
  homepage "https://github.com/Automaat/whisper-hotkey"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "WhisperHotkey.app"

  zap trash: [
    "~/.whisper-hotkey",
    "~/Library/LaunchAgents/com.whisper-hotkey.plist",
  ]

  caveats <<~EOS
    WhisperHotkey requires permissions to function:

    1. Accessibility: System Settings → Privacy & Security → Accessibility
    2. Input Monitoring: System Settings → Privacy & Security → Input Monitoring
    3. Microphone: System Settings → Privacy & Security → Microphone

    Add "WhisperHotkey" to each and enable the checkboxes.

    Default hotkey: Ctrl+Option+Z
    Config: ~/.whisper-hotkey/config.toml

    For more info: https://github.com/Automaat/whisper-hotkey#readme
  EOS
end
EOF

# Commit and push
git add Casks/whisper-hotkey.rb
git commit -s -S -m "bump: whisper-hotkey $VERSION

SHA256: $SHA256"
git push

# Cleanup
cd ~
rm -rf "$TMPDIR"

echo ""
echo "✅ Homebrew tap updated!"
echo ""
echo "Users can now update with:"
echo "  brew update && brew upgrade whisper-hotkey"
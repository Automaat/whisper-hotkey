#!/usr/bin/env bash
set -euo pipefail

# Helper script to trigger GitHub release workflow

VERSION="${1:-}"

if [ -z "$VERSION" ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Create GitHub Release"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "Usage:"
    echo "  $0 [VERSION]"
    echo ""
    echo "Examples:"
    echo "  $0              # Auto-increment minor (0.0.0 → 0.1.0)"
    echo "  $0 0.1.0        # Specific version"
    echo "  $0 1.0.0        # Major release"
    echo ""
    echo "This triggers the GitHub Actions release workflow which:"
    echo "  1. Creates and pushes git tag"
    echo "  2. Builds release binary"
    echo "  3. Creates .app bundle and DMG"
    echo "  4. Publishes GitHub release with DMG"
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    read -p "Proceed with auto-increment? [y/N] " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Cancelled."
        exit 0
    fi
fi

# Check gh CLI
if ! command -v gh &> /dev/null; then
    echo "❌ GitHub CLI (gh) not found"
    echo "Install: brew install gh"
    exit 1
fi

# Check auth
if ! gh auth status &> /dev/null; then
    echo "❌ Not authenticated with GitHub"
    echo "Run: gh auth login"
    exit 1
fi

# Trigger workflow
echo "🚀 Triggering release workflow..."
if [ -n "$VERSION" ]; then
    echo "   Version: $VERSION (manual)"
    gh workflow run release.yml -f version="$VERSION"
else
    echo "   Version: auto-increment minor"
    gh workflow run release.yml
fi

echo ""
echo "✅ Workflow triggered!"
echo ""
echo "Monitor progress:"
echo "  gh run watch"
echo "  # OR"
echo "  gh run list --workflow=release.yml"
echo ""
echo "Once complete, release will be available at:"
echo "  https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/releases"

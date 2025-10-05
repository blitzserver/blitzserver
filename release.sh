#!/bin/bash

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    print_error "Not in a git repository"
    exit 1
fi

# Check if we have uncommitted changes
if ! git diff-index --quiet HEAD --; then
    print_error "You have uncommitted changes. Please commit or stash them first."
    git status --porcelain
    exit 1
fi

# Get version from Cargo.toml
VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
if [ -z "$VERSION" ]; then
    print_error "Could not extract version from Cargo.toml"
    exit 1
fi

print_status "Building release v$VERSION"

# Check if tag already exists
if git tag | grep -q "^v$VERSION$"; then
    print_error "Tag v$VERSION already exists"
    exit 1
fi

# Create release directory
RELEASE_DIR="releases/v$VERSION"
mkdir -p "$RELEASE_DIR"

print_status "Installing required targets..."

# Install targets if not already installed
rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
rustup target add x86_64-unknown-linux-musl 2>/dev/null || true

print_status "Building Linux musl binary..."
cargo build --release --target x86_64-unknown-linux-musl
if [ $? -eq 0 ]; then
    print_success "Linux musl build completed"
    cp target/x86_64-unknown-linux-musl/release/blitzserver "$RELEASE_DIR/blitzserver-linux-x86_64"
else
    print_error "Linux musl build failed"
    exit 1
fi

print_status "Building Windows binary..."
cargo build --release --target x86_64-pc-windows-gnu
if [ $? -eq 0 ]; then
    print_success "Windows build completed"
    cp target/x86_64-pc-windows-gnu/release/blitzserver.exe "$RELEASE_DIR/blitzserver-windows-x86_64.exe"
else
    print_error "Windows build failed"
    exit 1
fi

print_status "Creating release archive..."
cd "$RELEASE_DIR"
tar -czf "blitzserver-v$VERSION-linux-x86_64.tar.gz" "blitzserver-linux-x86_64"
zip "blitzserver-v$VERSION-windows-x86_64.zip" "blitzserver-windows-x86_64.exe"
cd - > /dev/null

print_status "Creating git tag..."
git tag "v$VERSION" -m "Release v$VERSION"

print_status "Pushing tag to origin..."
git push origin "v$VERSION"

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    print_warning "GitHub CLI (gh) not found. Please install it to create releases automatically."
    print_status "Manual steps needed:"
    echo "1. Go to GitHub and create a new release for tag v$VERSION"
    echo "2. Upload the following files:"
    echo "   - $RELEASE_DIR/blitzserver-v$VERSION-linux-x86_64.tar.gz"
    echo "   - $RELEASE_DIR/blitzserver-v$VERSION-windows-x86_64.zip"
    exit 0
fi

print_status "Creating GitHub release..."

# Generate release notes
RELEASE_NOTES="## Release v$VERSION

### Downloads
- **Linux x86_64**: \`blitzserver-v$VERSION-linux-x86_64.tar.gz\`
- **Windows x86_64**: \`blitzserver-v$VERSION-windows-x86_64.zip\`

### Changes
$(git log --oneline $(git describe --tags --abbrev=0 HEAD^)..HEAD | sed 's/^/- /')"

# Create the release
gh release create "v$VERSION" \
    "$RELEASE_DIR/blitzserver-v$VERSION-linux-x86_64.tar.gz" \
    "$RELEASE_DIR/blitzserver-v$VERSION-windows-x86_64.zip" \
    --title "v$VERSION" \
    --notes "$RELEASE_NOTES"

if [ $? -eq 0 ]; then
    print_success "GitHub release created successfully!"
    print_status "Release URL: https://github.com/$(gh repo view --json owner,name -q '.owner.login + "/" + .name')/releases/tag/v$VERSION"
else
    print_error "Failed to create GitHub release"
    exit 1
fi

print_success "Release process completed successfully!"
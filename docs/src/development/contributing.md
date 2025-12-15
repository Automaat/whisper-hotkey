# Contributing

Guidelines for contributing to Whisper Hotkey.

## Welcome!

We welcome contributions of all kinds:
- Bug reports
- Feature requests
- Code improvements
- Documentation updates
- Performance optimizations

## Code of Conduct

Be respectful, constructive, and professional.

## Getting Started

### 1. Fork and Clone

```bash
# Fork on GitHub
# Then clone your fork
git clone https://github.com/YOUR_USERNAME/whisper-hotkey.git
cd whisper-hotkey
```

### 2. Set Up Development Environment

```bash
# Install mise (optional but recommended)
curl https://mise.run | sh

# Install tools
mise install

# Build
mise exec -- cargo build
```

### 3. Create Branch

```bash
git checkout -b feat/your-feature-name
# or
git checkout -b fix/bug-description
```

## Development Workflow

### Before You Code

1. **Check existing issues** - Avoid duplicate work
2. **Create issue** - Discuss approach for large changes
3. **Read code** - Understand existing patterns

### While Coding

1. **Write tests** - Add tests for new features
2. **Follow style** - Use `cargo fmt`
3. **Add docs** - Document public APIs
4. **Keep commits small** - One logical change per commit

### Before Submitting

```bash
# Format code
cargo fmt --all

# Check lints
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test

# Check all (via mise)
mise run check
```

## Code Style

### Rust Conventions

- **Style:** `cargo fmt` (default settings)
- **Naming:** `snake_case` for functions/variables, `PascalCase` for types
- **Line length:** 100 characters max
- **Indentation:** 4 spaces

### Error Handling

**Application code:**
```rust
use anyhow::{Context, Result};

fn setup() -> Result<()> {
    do_something()
        .context("failed to setup")?;
    Ok(())
}
```

**Library modules:**
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("something failed")]
    Failed(#[from] std::io::Error),
}
```

### Async Boundaries

- **Main thread:** tokio (event loop)
- **Audio thread:** OS real-time thread (NOT tokio, NO allocations)
- **Transcription:** tokio blocking pool (CPU-bound)

### Unsafe Code

**Only for:**
- FFI (CoreAudio, CGEvent)
- Lock-free data structures
- Profiled hot paths

**Requirements:**
- Document safety invariants
- Minimize unsafe scope
- Add safety comments

```rust
/// # Safety: `ptr` must be valid for duration of call
unsafe fn do_ffi(ptr: *mut T) {
    // Minimal unsafe block
}
```

## Testing Requirements

### For New Features

1. **Add unit tests** for pure logic
2. **Add integration tests** for hardware (mark `#[ignore]`)
3. **Test manually** - Full pipeline test

### For Bug Fixes

1. **Add regression test** demonstrating bug
2. **Fix bug**
3. **Verify test passes**

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test -- --ignored

# Coverage
cargo llvm-cov
```

## Documentation

### Code Documentation

```rust
/// Brief description of function
///
/// # Arguments
///
/// * `input` - Description of input
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When function fails and why
///
/// # Examples
///
/// ```
/// let result = my_function("test");
/// assert_eq!(result, expected);
/// ```
pub fn my_function(input: &str) -> Result<String> {
    // ...
}
```

### User Documentation

Update relevant docs in `docs/src/` if:
- Adding user-facing feature
- Changing configuration
- Updating performance characteristics

## Commit Guidelines

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, no code change
- `refactor`: Code restructure, no behavior change
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Maintenance (deps, build, etc.)

**Examples:**
```
feat(audio): add voice activity detection

Implement VAD to detect speech start/end automatically.
Reduces need to hold hotkey for full duration.

fix(hotkey): handle modifier key edge cases

Some modifier combinations weren't registering correctly.
Add tests for all modifier permutations.

docs(config): add examples for multi-profile setup
```

### Signing Commits

```bash
# Configure git
git config user.name "Your Name"
git config user.email "your@email.com"

# Sign commits
git commit -s -S -m "feat: add new feature"
```

## Pull Request Process

### 1. Push Branch

```bash
git push origin feat/your-feature-name
```

### 2. Create Pull Request

- Use descriptive title
- Reference related issues (#123)
- Describe changes clearly
- Add screenshots/videos if UI changes

### 3. PR Template

```markdown
## Motivation
Why are we doing this change?

## Implementation
How was this implemented? Any alternatives considered?

## Testing
- [ ] Unit tests added
- [ ] Integration tests added (if applicable)
- [ ] Manual testing performed

## Checklist
- [ ] Code formatted (`cargo fmt`)
- [ ] Lints pass (`cargo clippy`)
- [ ] Tests pass (`cargo test`)
- [ ] Documentation updated
```

### 4. Review Process

- Maintainers will review within 1-3 days
- Address review comments
- Push updates to same branch
- Request re-review

### 5. Merge

- Squash and merge (default)
- Maintainer will merge when approved

## Areas for Contribution

### High Priority

- **Performance optimizations** - Reduce latency, memory usage
- **Test coverage** - Increase unit test coverage
- **Documentation** - Improve user docs, API docs
- **Bug fixes** - See [Issues](https://github.com/Automaat/whisper-hotkey/issues)

### Medium Priority

- **Voice activity detection** - Auto-detect speech start/end
- **Additional models** - Support more Whisper variants
- **Configuration UI** - GUI for config editing
- **Linux/Windows ports** - Cross-platform support

### Low Priority

- **Cloud sync** - Sync config across devices
- **Alternative models** - Non-Whisper models
- **Plugin system** - Extensibility

## Getting Help

### Questions

- **GitHub Discussions** - For questions and discussions
- **Issues** - For bug reports and feature requests

### Debugging

See [Testing](./testing.md) for profiling and debugging guides.

## Release Process

**For maintainers only:**

### Version Bump

Follow [SemVer](https://semver.org/):
- MAJOR: Breaking changes
- MINOR: New features (backward compatible)
- PATCH: Bug fixes

### Creating Release

```bash
# Auto-increment minor version
./scripts/create-release.sh

# Specific version
./scripts/create-release.sh 0.2.0
```

This:
1. Creates git tag
2. Triggers GitHub Actions
3. Builds binaries
4. Creates GitHub release
5. Uploads artifacts

## Recognition

Contributors are recognized in:
- GitHub contributors page
- Release notes (significant contributions)
- README (major features)

## Thank You!

Every contribution helps make Whisper Hotkey better. Thank you for your time and effort!

# Taskwarrior with Turso Backend

This fork adds Turso (libsql) backend support to Taskwarrior, allowing your tasks to be stored in a Turso cloud database with automatic cross-device synchronization.

## What is Turso Remote Mode?

This implementation uses **Turso Remote mode**, which means:
- Tasks are stored directly in Turso cloud database
- All operations require internet connectivity
- Changes are immediately visible across all devices
- No local SQLite files (cloud-native architecture)

## Prerequisites

### All Platforms
- **Rust toolchain**: Install from [rustup.rs](https://rustup.rs)
- **CMake** 3.22 or higher
- **C++ compiler** with C++17 support
- **Git**

### macOS Specific
```bash
brew install cmake
```

### Ubuntu/Debian Specific
```bash
sudo apt update
sudo apt install cmake build-essential uuid-dev
```

## Turso Setup

1. **Create Turso account** at [turso.tech](https://turso.tech)

2. **Install Turso CLI**:
   ```bash
   curl -sSfL https://get.tur.so/install.sh | bash
   ```

3. **Login and create database**:
   ```bash
   turso auth login
   turso db create taskwarrior
   ```

4. **Get your database URL and token**:
   ```bash
   turso db show taskwarrior --url
   turso db tokens create taskwarrior
   ```

   Save these values - you'll need them for configuration.

## Compilation

### 1. Clone the Repository
```bash
git clone https://github.com/ivishalgandhi/taskwarrior.git
cd taskwarrior
git checkout feature/turso-backend
```

### 2. Build
```bash
mkdir build
cd build
cmake ..
make -j$(nproc)  # Linux
# or
make -j$(sysctl -n hw.ncpu)  # macOS
```

The binary will be at: `build/src/task`

### 3. Install (Optional)
```bash
sudo make install
```

Or use the binary directly from `build/src/task`

## Configuration

Edit `~/.taskrc` and add your Turso credentials:

```ini
turso.url=libsql://your-database-name.turso.io
turso.token=your_turso_auth_token_here
```

**Important**: Do not set `turso.file`. Turso Remote only accepts `turso.url` and `turso.token`. If `turso.file` is present, Taskwarrior fails at open with an error telling you to remove it. Both `turso.url` and `turso.token` are required.

### Example Configuration
```ini
# Turso Backend Configuration
turso.url=libsql://taskwarrior-myusername.aws-us-east-1.turso.io
turso.token=eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9...

# Optional: Color theme
include dark-256.theme

# UDAs and other standard Taskwarrior config...
```

## Usage

Once configured, use Taskwarrior normally:

```bash
# Add tasks
task add "Buy groceries"
task add "Write documentation" project:work

# List tasks
task ls

# Complete tasks
task 1 done

# Modify tasks
task 2 modify priority:H
```

All operations immediately sync to Turso and are visible on other devices.

## How It Works

### Architecture

```
┌─────────────┐
│   macOS     │
│  Taskwarrior│──┐
└─────────────┘  │
                 │
┌─────────────┐  │    ┌──────────────┐
│   Ubuntu    │  ├───▶│    Turso     │
│  Taskwarrior│──┘    │   Database   │
└─────────────┘       └──────────────┘
                             ▲
┌─────────────┐              │
│  Windows    │──────────────┘
│  Taskwarrior│
└─────────────┘
```

### Technical Details

1. **Direct Cloud Connection**: Each Taskwarrior instance connects directly to Turso using `libsql` Remote mode
2. **No Local Storage**: Unlike standard Taskwarrior, no local SQLite files are created
3. **Immediate Sync**: Changes are written to Turso immediately and visible to all devices
4. **TaskChampion Integration**: Uses TaskChampion's Storage trait with a custom Turso backend implementation

### Code Structure
```
src/
├── Context.cpp              # Validates Turso credentials, opens Turso or on-disk Replica
├── turso.cpp / turso.h      # Turso Remote credential checks
├── TDB2.cpp                 # open_replica_turso(url, token) → Rust FFI
└── taskchampion-cpp/
    ├── Cargo.toml           # Rust dependencies (libsql, tokio, cxx)
    └── src/
        ├── lib.rs           # FFI bridge to C++
        └── turso.rs         # Turso storage implementation
```

## Troubleshooting

### "Turso Remote only: remove turso.file..."
**Solution**: Delete `turso.file` from `~/.taskrc`. Only `turso.url` and `turso.token` are supported.

### "Turso Remote requires turso.token..."
**Solution**: Set `turso.token` whenever `turso.url` is set.

### Build fails with "cxx-build" error
**Solution**: Ensure Rust is installed (`rustup --version`)

### "Connection refused" or network errors
**Solution**: 
- Check internet connectivity
- Verify Turso URL is correct: `turso db show taskwarrior --url`
- Test token: `turso db shell taskwarrior` (should connect)

### Tasks not syncing between devices
**Solution**:
- Ensure all devices use the same `turso.url` and `turso.token`
- Verify each instance has internet connectivity
- Check Turso dashboard for database activity

### macOS linker warnings
**Effect**: Cosmetic warnings about version mismatch, doesn't affect functionality
**Solution**: Safe to ignore, or update Xcode command line tools

## Limitations

- **Requires Internet**: All operations need internet connectivity (no offline mode)
- **Latency**: Network round-trip adds slight delay compared to local storage
- **Turso Limits**: Free tier has usage limits (check [turso.tech/pricing](https://turso.tech/pricing))

## Migrating from Standard Taskwarrior

If you have existing tasks in standard Taskwarrior:

1. **Export existing tasks**:
   ```bash
   task export > tasks.json
   ```

2. **Build and configure Turso backend** (follow steps above)

3. **Import tasks**:
   ```bash
   task import tasks.json
   ```

Your tasks are now in Turso and will sync across devices.

## Keeping Updated with Upstream Taskwarrior Releases

As Taskwarrior releases new versions (e.g., 3.4.3, 3.5.0), you can incorporate those updates into your Turso fork.

### One-Time Setup

Add the official Taskwarrior repository as an upstream remote:

```bash
cd /path/to/your/taskwarrior
git remote add upstream https://github.com/GothenburgBitFactory/taskwarrior.git

# Verify remotes
git remote -v
# origin    https://github.com/ivishalgandhi/taskwarrior.git (your fork)
# upstream  https://github.com/GothenburgBitFactory/taskwarrior.git (official)
```

### Updating to a New Taskwarrior Version

```bash
# 1. Fetch upstream changes
git fetch upstream

# 2. Switch to your Turso feature branch
git checkout feature/turso-backend

# 3. Rebase your changes on top of the new release
git rebase upstream/master
# For a specific version tag:
# git rebase v3.4.3

# 4. If conflicts occur, resolve them (see below)

# 5. Force push to your fork (only if you've already pushed this branch)
git push origin feature/turso-backend --force-with-lease
```

### Files Modified by Turso Integration

Your Turso changes are minimal and localized:

**C++ Changes** (~30 lines total):
- `src/Context.cpp` - Turso config detection
- `src/TDB2.h` - Method declaration
- `src/TDB2_turso_snippet.cpp` - FFI call

**Rust Code** (entirely new):
- `src/taskchampion-cpp/` - All Rust files are new

**Build Config**:
- `Cargo.toml` - Workspace configuration

**Conflict Likelihood**: Very low - your changes don't overlap with Taskwarrior core functionality.

### Handling Merge Conflicts

If you encounter conflicts during rebase:

```bash
# Git will pause and show conflicting files
git status

# Edit the conflicting file(s) - look for <<<<<<< markers
# Keep both your Turso changes AND upstream changes when possible

# After fixing conflicts:
git add <filename>
git rebase --continue

# To abort and try a different approach:
git rebase --abort
```

### Testing After Update

Always rebuild and test after incorporating upstream changes:

```bash
cd build
rm -rf *        # Clean build recommended
cmake ..
make -j$(nproc)  # or $(sysctl -n hw.ncpu) on macOS

# Test Turso integration
./src/task ls
./src/task add "Test after update"
```

### Alternative: Merge Instead of Rebase

If rebasing is problematic, you can merge upstream changes:

```bash
git checkout feature/turso-backend
git merge upstream/master
# or: git merge v3.4.3

# Resolve any conflicts, then:
git commit
```

**Rebase vs Merge**:
- **Rebase** (recommended): Cleaner history, your commits appear on top of upstream
- **Merge**: Preserves exact history, creates merge commits

### Automation Script

Create `update-from-upstream.sh` in your repo:

```bash
#!/bin/bash
set -e

echo "Fetching upstream Taskwarrior changes..."
git fetch upstream

CURRENT_BRANCH=$(git branch --show-current)
echo "Current branch: $CURRENT_BRANCH"

if [ "$CURRENT_BRANCH" != "feature/turso-backend" ]; then
    echo "⚠️  Not on feature/turso-backend branch"
    read -p "Switch to it? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git checkout feature/turso-backend
    else
        exit 1
    fi
fi

read -p "Rebase onto upstream/master? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git rebase upstream/master
    echo "✅ Rebased successfully"
    echo "⚠️  Next steps:"
    echo "   1. Rebuild: cd build && rm -rf * && cmake .. && make"
    echo "   2. Test: ./src/task ls"
    echo "   3. Push: git push origin feature/turso-backend --force-with-lease"
fi
```

Make it executable:
```bash
chmod +x update-from-upstream.sh
```

### Long-Term Maintenance Options

1. **Keep Feature Branch** (Current approach)
   - ✅ Easy to maintain
   - ✅ Clear separation of code
   - ⚠️ Need to rebase on each upstream update

2. **Contribute Upstream**
   - Submit a PR to official Taskwarrior
   - If accepted, no maintenance needed
   - Turso becomes an official backend option

3. **Maintain as Plugin**
   - Extract Turso code into a plugin architecture
   - Harder initially, but independent of upstream

### Recommended Workflow

For most users, the **feature branch + periodic rebase** approach works well. Your changes are minimal and well-isolated, so conflicts should be rare. If you find yourself rebasing frequently or want Turso support for the broader Taskwarrior community, consider proposing it as an upstream feature!

## Support

- **GitHub Issues**: [github.com/ivishalgandhi/taskwarrior/issues](https://github.com/ivishalgandhi/taskwarrior/issues)
- **Original Taskwarrior**: [taskwarrior.org](https://taskwarrior.org)
- **Turso Docs**: [docs.turso.tech](https://docs.turso.tech)

## License

Same as Taskwarrior (MIT License)

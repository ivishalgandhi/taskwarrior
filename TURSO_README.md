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

**Important**: Do NOT add `turso.file` - Remote mode doesn't use local files.

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
├── Context.cpp              # Checks for turso.url in config, initializes Turso storage
├── TDB2_turso_snippet.cpp   # Calls Rust FFI to create Turso replica
└── taskchampion-cpp/
    ├── Cargo.toml           # Rust dependencies (libsql, tokio, cxx)
    └── src/
        ├── lib.rs           # FFI bridge to C++
        └── turso.rs         # Turso storage implementation
```

## Troubleshooting

### "Task Database Error: No turso.url configured"
**Solution**: Add `turso.url` and `turso.token` to `~/.taskrc`

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

## Support

- **GitHub Issues**: [github.com/ivishalgandhi/taskwarrior/issues](https://github.com/ivishalgandhi/taskwarrior/issues)
- **Original Taskwarrior**: [taskwarrior.org](https://taskwarrior.org)
- **Turso Docs**: [docs.turso.tech](https://docs.turso.tech)

## License

Same as Taskwarrior (MIT License)

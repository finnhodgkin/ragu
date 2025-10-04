# Spago Rust CLI Reference

## Quick Start

```bash
# Build the project
cargo build --release

# Run commands
./target/release/spago-rust <command>
```

## Global Options

Available for all commands:

```bash
-t, --tag <TAG>       Use a specific package set version
-f, --force-refresh   Bypass cache and fetch fresh data
-v, --verbose         Show detailed output
-h, --help            Show help information
-V, --version         Show version
```

## Commands

### `list` - List Package Set Versions

List available package set versions from the PureScript registry.

```bash
spago list              # Show 20 most recent versions
spago list --all        # Show all available versions
spago list -a           # Short form
```

**Example output:**

```
📋 Available package sets:

  → psc-0.15.15-20251004 (latest)
  · psc-0.15.15-20250925
  · psc-0.15.15-20250924
  ...

✓ Total: 100 package sets
```

### `info` - Package Information

Get detailed information about a specific package.

```bash
spago info <package>              # Basic info
spago info <package> -d           # Show direct dependencies
spago info <package> -T           # Show transitive dependencies
spago info <package> -r           # Show reverse dependencies
spago info <package> -d -r        # Combine flags
```

**Options:**

- `-d, --deps` - Show direct dependencies
- `-T, --transitive` - Show all transitive dependencies
- `-r, --reverse` - Show which packages depend on this one

**Examples:**

```bash
spago info prelude                # Package details only
spago info effect -d              # With direct dependencies
spago info halogen -T             # With full dependency tree
spago info prelude -r             # See who uses prelude
```

### `search` - Find Packages

Search for packages by name (partial match, case-insensitive).

```bash
spago search <query>              # List matching packages
spago search <query> --details    # Show full package details
spago search <query> -d           # Short form
```

**Examples:**

```bash
spago search halogen              # Find halogen-related packages
spago search effect --details     # Detailed search results
```

**Example output:**

```
🔍 Found 22 package(s) matching 'halogen'

  → halogen v7.0.0 (22 deps)
  → halogen-hooks v0.6.3 (25 deps)
  → halogen-vdom v8.0.0 (10 deps)
  ...
```

### `stats` - Package Set Statistics

Show comprehensive statistics about the package set.

```bash
spago stats                       # Latest package set
spago --tag <version> stats       # Specific version
```

**Shows:**

- Total packages and dependencies
- Average dependencies per package
- Min/max dependency counts
- Packages with no dependencies
- Top packages by dependency count

**Example output:**

```
📊 Package Set Statistics

  Tag: psc-0.15.15-20251004

  Total packages: 543
  Total dependencies: 5156
  Average dependencies: 9.50
  Max dependencies: 55
  Min dependencies: 0
  Packages with no deps: 9

📈 Top packages by dependencies:

  55 → lumi-components
  45 → whine-core
  43 → httpurple
  ...
```

### `cache` - Cache Management

Manage the local package set cache.

```bash
spago cache info                  # Show cache info
spago cache clear                 # Clear entire cache
spago cache remove <tag>          # Remove specific version
```

**Example output (info):**

```
📁 Cache Information

  Location: ~/.cache/spago-rust/package-sets
  Cached package sets: 3
  Total size: 417.86 KB
```

### `install` - Install Packages _(Coming Soon)_

Install packages and resolve dependencies.

```bash
spago install <packages...>       # Install packages
spago install <pkg> --no-deps     # Skip dependency resolution
spago i <pkg>                     # Short form
```

### `build` - Build Project _(Coming Soon)_

Build the PureScript project.

```bash
spago build                       # Build once
spago build --watch               # Watch for changes
spago build --clear               # Clear output first
```

### `init` - Initialize Project _(Coming Soon)_

Create a new Spago project.

```bash
spago init                        # Interactive
spago init --name my-project      # With name
```

## Usage Patterns

### Working with Specific Versions

```bash
# Use a specific package set version for all commands
spago --tag psc-0.15.15-20250925 stats
spago --tag psc-0.15.15-20250925 info prelude
spago --tag psc-0.15.15-20250925 search effect
```

### Force Refresh

```bash
# Bypass cache and fetch fresh data
spago --force-refresh stats
spago --force-refresh list
spago -f search halogen
```

### Verbose Mode

```bash
# See detailed operation logs
spago --verbose stats
spago -v info prelude
```

### Combining Options

```bash
# Use multiple global options together
spago --tag psc-0.15.15-20250925 --force-refresh --verbose info effect -d
```

## Performance Notes

### Operation Times (Release Build)

- **Tag list (cached)**: ~5ms (60x faster than API)
- **Tag list (API)**: ~300ms (only when cache is stale/bypassed)
- **Cache load**: ~370µs (binary deserialization)
- **Package lookup**: ~170ns (O(1) HashMap)
- **Search**: ~50µs (543 packages)
- **Dependency resolution**: ~2µs (transitive)

All operations are highly optimized for speed!

### Smart Caching

Spago Rust uses a two-tier caching system:

1. **Tag List Cache**

   - **TTL**: 24 hours
   - **Benefit**: Avoid GitHub API calls (~60x speedup)
   - **First run**: Fetches from GitHub (~300ms)
   - **Subsequent runs**: Loads from cache (~5ms)

2. **Package Set Cache**
   - **TTL**: Indefinite (immutable data)
   - **Format**: Binary (bincode)
   - **Benefit**: Instant package queries

This means:

- ✅ **First command** of the day: One API call
- ✅ **Rest of the day**: Zero network requests
- ✅ **Works offline**: After first fetch

See [CACHING.md](CACHING.md) for detailed information.

## Tips

1. **Cache is your friend**: First run fetches from network, subsequent runs use cache
2. **Use search**: Don't remember exact names? `spago search <partial-name>`
3. **Check dependencies**: Use `-d` and `-T` flags to understand package relationships
4. **Clear cache when needed**: `spago cache clear` if you suspect stale data
5. **Specific versions**: Use `--tag` to work with older package sets

## Exit Codes

- `0` - Success
- `101` - Error (with descriptive message)

## Environment

- **Cache location**:
  - macOS/Linux: `~/.cache/spago-rust/package-sets/`
  - Windows: `%LOCALAPPDATA%\spago-rust\package-sets\`
- **Config location**: _(Coming soon)_
- **GitHub API**: Rate limit 60 req/hour (unauthenticated)

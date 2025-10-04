# Spago Rust 🚀

A blazingly fast Rust implementation of [Spago](https://github.com/purescript/spago), the PureScript package manager and build tool.

## Why Rust?

The current JavaScript implementation of Spago (`spago@next`) becomes unreliable and slow on large projects. This Rust implementation aims to be:

- ⚡ **Blazingly fast** - Binary serialization for package sets (~3ms cache load time)
- 🔒 **Reliable** - Strong typing and memory safety
- 📦 **Minimal** - Focused on package sets and `spago.yaml` format
- 🎯 **Simple** - Clear dependency model without complex offline modes

## Features Implemented

### Package Registry

- ✅ Fetch package sets from [purescript/package-sets](https://github.com/purescript/package-sets)
- ✅ Binary caching with `bincode` for ultra-fast loading
- ✅ List available package set versions from GitHub API
- ✅ Get latest package set version
- ✅ Smart cache management with SHA-256 keys

### Fast Package Queries

- ✅ O(1) package lookup by name
- ✅ Batch package queries
- ✅ Full-text package search
- ✅ Direct dependency resolution
- ✅ Transitive dependency calculation (BFS)
- ✅ Reverse dependency lookup
- ✅ Package set validation
- ✅ Statistical analysis

### Performance (Release Build)

```
Tag list (cached):         ~5ms    (60x faster than GitHub API)
Tag list (GitHub API):     ~300ms  (with 24-hour cache TTL)
Cache load:                ~370µs  (binary deserialization)
Single package lookup:     ~170ns  (O(1) HashMap)
Multi lookup (5 packages): ~460ns
Package search:            ~50µs   (543 packages)
Direct dependencies:       ~830ns
Transitive dependencies:   ~2µs
```

**Smart Caching:**

- Package sets: Binary cached indefinitely (until cleared)
- Tag list: JSON cached for 24 hours
- `--force-refresh` bypasses all caches

## Usage

```rust
use spago_rust::registry::{
    get_package_set,
    list_available_tags,
    get_latest_tag,
    PackageQuery
};

// List available versions
let tags = list_available_tags()?;
println!("Available tags: {:?}", tags);

// Get the latest package set
let latest = get_latest_tag()?;
let packages = get_package_set(&latest, false)?;

// Or use a specific version
let packages = get_package_set("psc-0.15.15-20251004", false)?;

// Force refresh (bypass cache)
let packages = get_package_set(&latest, true)?;

// Fast package queries
let query = PackageQuery::new(&packages);

// O(1) lookup
if let Some(pkg) = query.get("prelude") {
    println!("{} v{}", pkg.name, pkg.package.version);
}

// Get dependencies
let deps = query.get_dependencies("effect")?;
let all_deps = query.get_transitive_dependencies("console")?;

// Search packages
let results = query.search("halogen");

// Get stats
let stats = query.stats();
println!("Avg deps: {:.2}", stats.avg_dependencies);
```

## CLI Usage

### Build and Run

```bash
# Development build
cargo build
cargo run -- <command>

# Release build (optimized)
cargo build --release
./target/release/spago-rust <command>
```

### Commands

```bash
# List available package set versions
spago list
spago list --all        # Show all available tags

# Get package information
spago info prelude
spago info effect -d     # Show direct dependencies
spago info console -T    # Show transitive dependencies
spago info prelude -r    # Show reverse dependencies

# Search for packages
spago search halogen
spago search effect --details

# Show package set statistics
spago stats

# Cache management
spago cache info         # Show cache location and size
spago cache clear        # Clear all cached package sets
spago cache remove <tag> # Remove specific cached version

# Global options
spago --tag psc-0.15.15-20250925 <command>  # Use specific tag
spago --force-refresh <command>              # Bypass cache
spago --verbose <command>                    # Verbose output
```

### Examples

```bash
# Find all halogen-related packages
spago search halogen

# Get detailed info about a package with all its dependencies
spago info halogen -d

# Check statistics for a specific package set version
spago --tag psc-0.15.15-20250925 stats

# Clear cache and force fresh download
spago cache clear
spago --force-refresh stats
```

## Cache Location

Spago Rust uses intelligent caching to minimize network requests:

### Package Sets

- **Location**:
  - macOS: `~/Library/Caches/spago-rust/package-sets/`
  - Linux: `~/.cache/spago-rust/package-sets/`
  - Windows: `%LOCALAPPDATA%\spago-rust\package-sets\`
- **Format**: Binary (bincode) for ultra-fast deserialization
- **Key**: SHA-256 hash of package set tag
- **TTL**: Indefinite (until manually cleared)

### Tag List

- **Location**: `[cache-dir]/metadata/tags.json`
- **Format**: JSON with timestamp
- **TTL**: 24 hours (configurable)
- **Benefit**: ~60x faster than GitHub API calls (5ms vs 300ms)

This two-tier caching strategy ensures:

- **Zero network** requests for common operations
- **Minimal GitHub API** usage (respects rate limits)
- **Blazingly fast** startup times

## Architecture

The codebase is organized into modular components for maintainability and performance:

```
src/
├── main.rs           # CLI entry point
├── cli/
│   ├── mod.rs        # Command definitions (clap)
│   └── commands/     # Command implementations
│       ├── list.rs   # List package sets
│       ├── info.rs   # Package information
│       ├── search.rs # Package search
│       ├── stats.rs  # Statistics
│       └── cache.rs  # Cache management
└── registry/
    ├── mod.rs        # Public API exports
    ├── types.rs      # Core data structures
    ├── cache.rs      # Binary caching
    ├── package_sets.rs  # GitHub fetching
    └── packages.rs   # Fast query interface
```

### Module Responsibilities

- **cli**: Command-line interface using clap (commands separate from implementation)
- **registry/types**: Core data structures with zero-copy where possible
- **registry/cache**: SHA-256 keyed binary cache management
- **registry/package_sets**: GitHub integration for fetching package sets
- **registry/packages**: High-performance query operations with O(1) lookups

## Roadmap

### Completed ✅

- [x] Package set fetching and caching
- [x] Fast package queries (O(1) lookups)
- [x] Dependency resolution (direct & transitive)
- [x] CLI interface with commands
- [x] Package search
- [x] Package statistics
- [x] Cache management

### In Progress 🚧

- [ ] Parse `spago.yaml` configuration files
- [ ] Install command implementation
- [ ] Build command integration

### Planned 📋

- [ ] Workspace support (multi-package projects)
- [ ] Local package handling
- [ ] Integration with PureScript compiler
- [ ] Watch mode for builds
- [ ] Project initialization (init command)

## Design Principles

1. **Package Sets First** - Use curated package sets as the source of truth
2. **No Complex Lockfiles** - Package sets provide version consistency
3. **Fast by Default** - Binary caching, parallel downloads, optimized algorithms
4. **Transparent Caching** - Clear cache locations and management
5. **Minimal Configuration** - Support `spago.yaml` format only

## API Notes

- GitHub API rate limit: 60 requests/hour (unauthenticated)
- Returns up to 100 most recent package set tags
- Package sets use the `packages.json` format from the registry

## License

BSD-3-Clause (matching the PureScript ecosystem)

# Caching Strategy

Spago Rust implements a sophisticated two-tier caching system to maximize performance and minimize network requests.

## Why Cache?

- **GitHub API Rate Limits**: 60 requests/hour for unauthenticated requests
- **Network Latency**: API calls take ~300ms, cache reads take ~5ms
- **Offline Support**: Work without network connection once cached
- **Battery Life**: Fewer network requests = longer battery on laptops

## Cache Architecture

### Tier 1: Package Sets (Binary Cache)

**What**: Complete package set data (all packages + dependencies)

**Location**:

- macOS: `~/Library/Caches/spago-rust/package-sets/`
- Linux: `~/.cache/spago-rust/package-sets/`
- Windows: `%LOCALAPPDATA%\spago-rust\package-sets\`

**Format**: Binary (bincode serialization)

- Faster to load than JSON (~370µs vs ~2ms)
- Smaller file size (~140KB vs ~700KB)
- Type-safe deserialization

**Key Strategy**: SHA-256 hash of package set tag

- Avoids filename conflicts
- Content-addressable storage
- Automatic deduplication

**TTL**: Indefinite

- Package sets are immutable
- Never stale or outdated
- Cleared only manually

**Performance**:

```
First fetch:  ~500ms  (network + JSON parse + bincode encode + save)
Cache load:   ~370µs  (read + bincode decode)
Speedup:      ~1300x
```

### Tier 2: Tag List (Metadata Cache)

**What**: List of available package set versions

**Location**: `[cache-dir]/metadata/tags.json`

**Format**: JSON with timestamp

```json
{
  "tags": ["psc-0.15.15-20251004", "psc-0.15.15-20250925", ...],
  "fetched_at": "2025-10-04T10:38:30.763849Z"
}
```

**TTL**: 24 hours (default)

- New tags are published infrequently
- 24 hours is a good balance
- Can be customized or bypassed

**Performance**:

```
GitHub API:   ~300ms
Cache load:   ~5ms
Speedup:      ~60x
```

**Freshness Check**:

- Timestamp stored with data
- Age calculated on load
- Stale cache automatically refetched
- User sees "fresh for Xh Ym" message

## Cache Invalidation

### Manual Clearing

```bash
# Clear everything (package sets + tags)
spago cache clear

# Clear specific package set
spago cache remove psc-0.15.15-20251004

# View cache info
spago cache info
```

### Force Refresh

```bash
# Bypass all caches for one command
spago --force-refresh list
spago --force-refresh search effect
spago --force-refresh stats
```

**When to use**:

- Testing with latest package set
- Debugging cache issues
- After manual package-sets updates

### Automatic Staleness

Tag cache automatically refreshes after 24 hours:

```
Run 1 (0h):    "Loaded tags from cache (fresh for 23h 59m)"
Run 2 (24h):   "Fetching available tags from GitHub API..."
Run 3 (24h+1): "Loaded tags from cache (fresh for 23h 59m)"
```

## Network Request Minimization

### First-Time Usage

```
User runs: spago search effect

Network requests:
1. GET github.com/purescript/package-sets/tags  (~300ms)
2. GET raw.githubusercontent.com/.../packages.json (~200ms)

Total: ~500ms
```

### Subsequent Usage

```
User runs: spago search halogen

Network requests: ZERO

Time: ~5ms (all from cache)
```

### Daily Usage

```
Day 1: spago list     -> API call (300ms)
Day 1: spago search   -> Cached (5ms)
Day 1: spago info     -> Cached (5ms)
Day 2: spago list     -> Cached (5ms)
Day 3: spago stats    -> API call (300ms, cache was stale)
```

Result: **1-2 API calls per day** instead of dozens!

## Cache Efficiency

### Storage Space

Typical cache after regular use:

```
package-sets/
  ├── abc123...bin  (139 KB)   # psc-0.15.15-20251004
  └── def456...bin  (141 KB)   # psc-0.15.15-20250925

metadata/
  └── tags.json     (2 KB)

Total: ~282 KB for 2 package sets
```

### Memory Usage

- Package set loaded on demand
- Not kept in memory after command
- Typical peak: ~500KB

### Disk I/O

Binary format enables memory-mapped loading:

- Sequential read pattern
- Efficient for SSDs
- No parsing overhead

## Configuration (Future)

Future versions may support:

```yaml
# spago.yaml
cache:
  tag_ttl_hours: 12 # Default: 24
  package_ttl_days: 30 # Default: infinite
  max_size_mb: 100 # Default: unlimited
```

## Best Practices

1. **Let it cache**: Don't use `--force-refresh` unless needed
2. **Monitor size**: Run `spago cache info` occasionally
3. **Clean periodically**: `spago cache clear` if cache grows large
4. **Offline mode**: Works offline after first fetch!

## Troubleshooting

### Cache Miss When Expected Hit

```bash
# Check if cache exists
spago cache info

# View cache timestamp
cat ~/Library/Caches/spago-rust/metadata/tags.json | jq '.fetched_at'

# Force refresh and recreate
spago --force-refresh list
```

### Stale Data

```bash
# Clear and refresh
spago cache clear
spago list
```

### Disk Space Issues

```bash
# Check cache size
du -sh ~/Library/Caches/spago-rust

# Remove old package sets manually
rm ~/Library/Caches/spago-rust/package-sets/*.bin

# Keep only recent tags
spago --force-refresh list
```

## Implementation Details

### Bincode Format

Package sets use bincode for maximum performance:

- Zero-copy deserialization where possible
- Native endianness (platform-specific)
- Version-agnostic (handles struct changes)

### Timestamp Handling

Uses chrono with UTC:

- No timezone issues
- Serializable to JSON
- Duration math for TTL

### Error Handling

Cache misses are silent:

1. Try cache load
2. If error/miss -> fetch from network
3. Save to cache for next time

Users only see messages for:

- Successful cache loads (with freshness)
- Network fetches (when necessary)

## Future Enhancements

- [ ] Parallel cache warming
- [ ] Incremental tag updates
- [ ] Compressed cache format (zstd)
- [ ] Cache preloading daemon
- [ ] Shared cache for multiple projects

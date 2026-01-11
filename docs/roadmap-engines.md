# Engine Management Roadmap

## Overview

A unified interface for chess engine analysis, supporting both cloud-based evaluation services and local UCI engines.

## Requirements

### Cloud Engines (Priority Order)
1. **Lichess Cloud Eval** - Crowd-sourced position evaluations
2. **Lichess Tablebase** - Syzygy endgame tablebases via API
3. **ChessDB** - Opening book and game database queries
4. **Syzygy Online** - Alternative tablebase API

### Local Engines
- **UCI protocol only** - Modern standard, covers Stockfish, Leela, Komodo, etc.

### Features
- **Caching** - Cache cloud API responses (positions are deterministic)
- **Rate limiting** - Respect API limits, queue requests
- **Unified interface** - Same API for cloud and local engines
- **Engine profiles** - Presets for different analysis depths/times

### Nice to Have (Later)
- Multiple simultaneous engines for comparison
- Additional cloud sources as available

## Architecture

```
src/
├── engine/
│   ├── mod.rs
│   ├── traits.rs        # EngineProvider trait
│   ├── manager.rs       # Engine lifecycle, selection
│   ├── cache.rs         # Response caching layer
│   │
│   ├── uci/
│   │   ├── mod.rs
│   │   ├── protocol.rs  # UCI command/response parsing
│   │   ├── process.rs   # Subprocess management
│   │   └── engine.rs    # UciEngine impl
│   │
│   └── cloud/
│       ├── mod.rs
│       ├── lichess.rs   # Lichess cloud eval + tablebase
│       ├── chessdb.rs   # ChessDB API
│       └── syzygy.rs    # Syzygy online API
```

## Core Traits

```rust
#[async_trait]
trait EngineProvider {
    /// Get evaluation for a position
    async fn evaluate(&self, fen: &str, options: EvalOptions) -> Result<Evaluation>;
    
    /// Check if this engine can handle the position
    fn supports(&self, fen: &str) -> bool;
    
    /// Engine metadata
    fn info(&self) -> EngineInfo;
}

struct EvalOptions {
    depth: Option<u8>,
    time_ms: Option<u32>,
    multipv: u8,  // Number of lines to return
}

struct Evaluation {
    score: Score,
    pv: Vec<Move>,      // Principal variation
    depth: u8,
    nodes: Option<u64>,
    source: EngineSource,
}

enum Score {
    Centipawns(i32),
    Mate(i8),  // Positive = white mates in N, negative = black
}
```

## Implementation Phases

### Phase 1: UCI Engine Support
- [ ] UCI protocol parser (info, bestmove, options)
- [ ] Subprocess spawning and lifecycle management
- [ ] Basic `go depth N` / `go movetime N` commands
- [ ] Position setup via FEN
- [ ] Multi-PV support

### Phase 2: Lichess Cloud Integration
- [ ] Lichess cloud eval API client
- [ ] Lichess tablebase API client
- [ ] Response parsing and normalization
- [ ] Handle "position not in cloud" gracefully

### Phase 3: Caching Layer
- [ ] In-memory LRU cache
- [ ] Persistent cache (SQLite or file-based)
- [ ] Cache key: FEN + eval options
- [ ] TTL for cloud results (or infinite for tablebases)

### Phase 4: Additional Cloud Engines
- [ ] ChessDB integration
- [ ] Syzygy online API
- [ ] Unified fallback chain (try cloud → fall back to local)

### Phase 5: Rate Limiting & Robustness
- [ ] Request queue with rate limiting per provider
- [ ] Retry logic with exponential backoff
- [ ] Graceful degradation when APIs unavailable
- [ ] Request batching where APIs support it

### Phase 6: Engine Manager
- [ ] Engine discovery (scan common paths)
- [ ] Engine configuration UI integration
- [ ] Engine presets/profiles
- [ ] Quick-switch between engines

## API Reference

### Lichess Cloud Eval
- Endpoint: `https://lichess.org/api/cloud-eval`
- Params: `fen`, `multiPv`
- Rate limit: ~1 req/sec without auth
- Docs: https://lichess.org/api#tag/Analysis

### Lichess Tablebase
- Endpoint: `https://tablebase.lichess.ovh/standard`
- Params: `fen`
- Returns: DTZ, best moves, category
- Docs: https://github.com/lichess-org/lila-tablebase

### ChessDB
- Endpoint: `https://www.chessdb.cn/cdb.php`
- Params: `action=queryall`, `board=FEN`
- Returns: Move suggestions with scores
- Docs: https://www.chessdb.cn/cloudbookc_api_en.html

## Caching Strategy

```rust
struct CacheKey {
    fen: String,      // Normalized FEN (remove move clocks for cloud)
    provider: String, // "lichess", "chessdb", etc.
    options_hash: u64,
}

// Cache durations
const TABLEBASE_TTL: Duration = Duration::MAX;  // Never expires
const CLOUD_EVAL_TTL: Duration = Duration::days(30);
const LOCAL_EVAL_TTL: Duration = Duration::days(7);  // May want to re-eval
```

## Testing Strategy

- Mock UCI engine for protocol tests
- Recorded API responses for cloud engine tests
- Integration tests with real Stockfish binary
- Rate limit testing (don't hammer real APIs in CI)

## Dependencies

- `tokio` for async subprocess and HTTP
- `reqwest` for HTTP client
- `serde` for API response parsing
- `lru` for in-memory cache

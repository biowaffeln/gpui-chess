# Custom Database Format Roadmap

## Overview

A high-performance chess game database optimized for extremely fast position lookups and tree generation. Uses memory-mapped storage with multi-layer filtering to achieve sub-100ms queries across 10 million games.

## Design Goals

| Constraint | Target |
|------------|--------|
| Game capacity | 10 million games |
| Storage size | < 10 GB |
| Query latency | < 100ms (target), < 1s (hard limit) |
| Primary use case | Position search → tree generation with statistics |
| Optimization priority | Read performance (writes can be slow) |

## Architecture

```
src/
├── database/
│   ├── mod.rs
│   ├── format.rs         # File format definitions, headers
│   ├── writer.rs         # Database builder/writer
│   ├── reader.rs         # mmap-based reader
│   ├── query.rs          # Query engine
│   │
│   ├── index/
│   │   ├── mod.rs
│   │   ├── game_index.rs    # Fixed-size game index entries
│   │   ├── xor_filter.rs    # XOR filter for position hashes
│   │   ├── pawn_mask.rs     # Pawn movement tracking
│   │   └── opening_tree.rs  # Pre-built opening tree
│   │
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── mmap.rs       # Memory-mapped file handling
│   │   ├── moves.rs      # Move data encoding/decoding
│   │   └── metadata.rs   # Player names, dates, etc.
│   │
│   └── stats/
│       ├── mod.rs
│       └── tree.rs       # Result tree with statistics
```

## File Format

### Layout

```
┌─────────────────────────────────────────────────────────────┐
│ HEADER (64 bytes)                                           │
│   magic: [u8; 4]        = "CXDB"                           │
│   version: u16          = 1                                 │
│   game_count: u32                                           │
│   index_offset: u64                                         │
│   opening_tree_offset: u64                                  │
│   moves_offset: u64                                         │
│   metadata_offset: u64                                      │
│   reserved: [u8; 22]                                        │
├─────────────────────────────────────────────────────────────┤
│ GAME INDEX (64 bytes × game_count)                ~640 MB   │
│   Per game:                                                 │
│   ├─ moves_offset: u32       → pointer into MOVES section  │
│   ├─ moves_len: u16          → number of half-moves        │
│   ├─ xor_filter: [u8; 32]    → position hash filter        │
│   ├─ pawn_mask_white: u8     → which white pawns moved     │
│   ├─ pawn_mask_black: u8     → which black pawns moved     │
│   ├─ min_piece_count: u8     → minimum pieces during game  │
│   ├─ max_ply: u8             → game length                 │
│   ├─ white_elo: u16                                        │
│   ├─ black_elo: u16                                        │
│   ├─ result: u8              → 0=draw, 1=white, 2=black    │
│   ├─ date: u32               → days since epoch            │
│   ├─ metadata_offset: u32    → pointer into METADATA       │
│   ├─ metadata_len: u16                                     │
│   └─ reserved: [u8; 6]                                     │
├─────────────────────────────────────────────────────────────┤
│ OPENING TREE                                       ~200 MB  │
│   Pre-computed tree for positions reachable in ≤12 moves   │
│   ┌───────────────────────────────────────────────────────┐ │
│   │ Node:                                                 │ │
│   │   zobrist: u64                                        │ │
│   │   children: [(move_idx, node_offset); N]             │ │
│   │   stats: AggregateStats                               │ │
│   │   game_ids: [u32; M]  (sample, not exhaustive)       │ │
│   └───────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│ MOVE DATA (packed)                                 ~800 MB  │
│   Game 0: [move_idx, move_idx, move_idx, ...]             │
│   Game 1: [move_idx, move_idx, ...]                        │
│   ...                                                       │
│   (Each move_idx is index into legal moves list, 1 byte)   │
├─────────────────────────────────────────────────────────────┤
│ METADATA (packed, variable length)                   ~1 GB  │
│   Game 0: "Carlsen\0Nepomniachtchi\0WCC 2021\0Event\0"    │
│   Game 1: ...                                               │
│   (Null-terminated strings: white, black, event, site)     │
└─────────────────────────────────────────────────────────────┘

Total: ~2.6 GB for 10M games
```

### Move Encoding

Moves are stored as indices into the legal move list at each position:

```rust
// To encode a game:
let mut position = Chess::default();
let mut encoded_moves = Vec::new();

for san_move in game.moves {
    let legal_moves: Vec<Move> = position.legal_moves().collect();
    let move_obj = parse_san(&position, &san_move);
    let index = legal_moves.iter().position(|m| m == &move_obj).unwrap();
    encoded_moves.push(index as u8);  // Max legal moves is ~218, fits in u8
    position.play(&move_obj);
}
```

This requires `shakmaty` to decode (must regenerate legal moves), but is maximally compact.

## Filtering Strategies

### XOR Filter (Primary)

32-byte XOR filter per game containing Zobrist hashes of all positions reached:

```rust
struct XorFilter {
    fingerprints: [u8; 32],
    // ~80 positions per game, 32 bytes → <1% false positive rate
}

impl XorFilter {
    fn may_contain(&self, zobrist: u64) -> bool {
        // O(1) lookup, 2-3 cache line accesses
    }
}
```

- **True negative**: Position definitely not in game (skip game)
- **True positive**: Position might be in game (need to verify)
- **Expected hit rate**: ~99% of games filtered out

### Pawn Mask (Secondary)

8-bit mask per color tracking which pawns ever moved:

```rust
// Bit i = 1 means pawn on file i moved at some point
pawn_mask_white: u8  // e.g., 0b00011000 = d,e pawns moved
pawn_mask_black: u8
```

**Pruning logic**: If querying a position where white's g-pawn is on g4:
- Check if bit 6 (g-file) is set in `pawn_mask_white`
- If not set → pawn never moved → position impossible → skip game

### Piece Count (Tertiary)

```rust
min_piece_count: u8  // Minimum pieces on board during game
```

If querying a position with 28 pieces and game's `min_piece_count` is 24, the position might exist. But if `min_piece_count` is 30, skip (impossible).

### Castling Rights (Future)

Could add 4 bits tracking if each castling right was ever lost.

## Opening Tree

Pre-built tree covering positions reachable within first 12 moves (~24 ply).

```rust
struct OpeningTreeNode {
    zobrist: u64,
    stats: AggregateStats,
    children: Vec<(MoveIndex, NodeOffset)>,
    sample_game_ids: Vec<u32>,  // Representative games, not all
}

struct AggregateStats {
    total_games: u32,
    white_wins: u32,
    black_wins: u32,
    draws: u32,
    avg_white_elo: u16,
    avg_black_elo: u16,
    performance: i16,  // Performance rating
}
```

**Why 12 moves?**
- Covers ~99% of opening queries
- Tree size manageable (~1-5M nodes)
- Positions beyond this are rare enough that filtering is fast

## Query Engine

### Position Query Flow

```rust
pub struct PositionQuery {
    target_fen: String,
    filters: QueryFilters,
}

pub struct QueryFilters {
    min_elo: Option<u16>,
    max_elo: Option<u16>,
    min_date: Option<Date>,
    max_date: Option<Date>,
    player: Option<String>,
    result: Option<GameResult>,
}

pub struct QueryResult {
    matching_games: Vec<GameMatch>,
    move_tree: MoveTree,  // Tree of what happened next
    stats: AggregateStats,
}
```

### Query Algorithm

```rust
fn query_position(&self, query: PositionQuery) -> QueryResult {
    let zobrist = compute_zobrist(&query.target_fen);
    let pawn_config = extract_pawn_config(&query.target_fen);
    let piece_count = count_pieces(&query.target_fen);
    
    // 1. Check opening tree first (O(1) for common positions)
    if let Some(node) = self.opening_tree.lookup(zobrist) {
        return self.build_result_from_tree_node(node, &query.filters);
    }
    
    // 2. Scan game index with filters
    let candidates: Vec<u32> = self.game_index
        .par_iter()  // Parallel iteration
        .enumerate()
        .filter(|(_, game)| {
            // XOR filter: quick rejection
            if !game.xor_filter.may_contain(zobrist) {
                return false;
            }
            
            // Pawn mask: check compatibility
            if !pawn_config.compatible_with(game.pawn_mask_white, game.pawn_mask_black) {
                return false;
            }
            
            // Piece count bounds
            if piece_count < game.min_piece_count {
                return false;
            }
            
            // User filters (ELO, date, etc.)
            if !query.filters.matches(game) {
                return false;
            }
            
            true  // Candidate for verification
        })
        .map(|(idx, _)| idx as u32)
        .collect();
    
    // 3. Verify candidates by replaying games
    let matches: Vec<GameMatch> = candidates
        .par_iter()
        .filter_map(|&game_id| {
            let moves = self.read_moves(game_id);
            self.replay_and_find_position(game_id, &moves, zobrist)
        })
        .collect();
    
    // 4. Build result tree
    self.build_move_tree(matches)
}
```

### Parallelization

- Game index scan: embarrassingly parallel, use `rayon`
- Game replay: also parallel, CPU-bound
- Target: saturate all cores during query

## Statistics & Tree Building

When building the result tree for "what happened next":

```rust
struct MoveTree {
    position: Zobrist,
    continuations: Vec<Continuation>,
}

struct Continuation {
    move_san: String,
    count: u32,
    white_wins: u32,
    draws: u32,
    black_wins: u32,
    avg_elo: u16,
    performance: i16,
    children: Option<Box<MoveTree>>,  // Lazy load deeper levels
}
```

## Implementation Phases

### Phase 1: Core Format & Writer
- [ ] Define file format structures
- [ ] Implement database writer/builder
- [ ] Move encoding using shakmaty
- [ ] Basic metadata storage
- [ ] XOR filter generation per game
- [ ] Pawn mask computation

### Phase 2: Reader & mmap
- [ ] Memory-mapped file reader
- [ ] Game index parsing
- [ ] Move data decoding
- [ ] Metadata retrieval

### Phase 3: Basic Query Engine
- [ ] Zobrist hash computation (use shakmaty)
- [ ] XOR filter lookups
- [ ] Pawn mask filtering
- [ ] Piece count filtering
- [ ] Sequential game index scan
- [ ] Game replay and position verification

### Phase 4: Opening Tree
- [ ] Tree data structure
- [ ] Tree builder (process all games, build first 12 moves)
- [ ] Tree serialization to file format
- [ ] Tree lookup in query engine
- [ ] Statistics aggregation

### Phase 5: Parallel Query Engine
- [ ] Parallel game index scan with rayon
- [ ] Parallel game replay
- [ ] Benchmark and optimize hot paths
- [ ] Target: <100ms for 10M games

### Phase 6: Advanced Filters & Statistics
- [ ] ELO range filtering
- [ ] Date range filtering
- [ ] Player name filtering (requires metadata index)
- [ ] Result tree building with statistics
- [ ] Performance rating calculation

### Phase 7: Database Management
- [ ] Append games to existing database
- [ ] Delete games (mark as deleted, compact later)
- [ ] Database compaction/rebuild
- [ ] Import from PGN (integration with PGN parser)

## Testing Strategy

### Unit Tests
- XOR filter correctness and false positive rate
- Pawn mask computation for various games
- Move encoding/decoding round-trip
- Zobrist hash consistency

### Integration Tests
- Build database from 1000 games, verify queries
- Round-trip: build → query → verify against naive scan
- Edge cases: stalemate, insufficient material, long games

### Performance Tests
- Benchmark with 1M games subset
- Measure filter effectiveness (% games eliminated)
- Profile hot paths (where is time spent?)
- Memory usage under mmap

### Stress Tests
- Query performance with 10M games
- Concurrent query load
- Large result sets (popular positions)

## Dependencies

- `shakmaty` — Move generation, Zobrist hashing
- `memmap2` — Memory-mapped file I/O
- `rayon` — Parallel iteration
- `xorf` or custom — XOR filter implementation
- `bytemuck` — Zero-copy struct casting

## Performance Notes

### Expected Query Performance (10M games)

| Stage | Time | Notes |
|-------|------|-------|
| Opening tree lookup | <1ms | O(1) hash lookup |
| Game index scan | ~50ms | 640MB sequential read |
| XOR filter checks | ~20ms | 10M × 3 cache accesses |
| Pawn/piece filtering | ~5ms | Simple bit ops |
| Candidate replay | ~20ms | ~100K games × parallel |
| Tree building | ~5ms | Aggregate matches |
| **Total** | **~100ms** | Target achieved |

### Why mmap?

1. **No explicit I/O** — OS handles paging transparently
2. **Warm cache** — Repeated queries hit page cache
3. **Zero-copy** — Read structs directly from mapped memory
4. **Lazy loading** — Only touched pages loaded from disk

## Open Questions

- Should opening tree store exhaustive game IDs or just samples?
- How deep should opening tree go? (12 moves = sweet spot?)
- Should we support multiple database files? (e.g., one per year)
- Index for player name search? (would add complexity)

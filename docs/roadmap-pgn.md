# PGN Loader & Parser Roadmap

## Overview

A full-featured PGN (Portable Game Notation) parser and writer with streaming support for handling large multi-game files.

**Core parsing powered by [`pgn-reader`](https://crates.io/crates/pgn-reader)** — a fast, non-allocating, streaming PGN parser from the shakmaty ecosystem.

## Why pgn-reader?

- **Blazing fast**: ~500k games/sec, ~1 GiB/s throughput
- **Streaming**: Non-allocating, processes games one at a time
- **Visitor pattern**: Flexible callbacks for headers, moves, comments, NAGs, variations
- **Battle-tested**: Used by Lichess, handles quirky real-world PGN files
- **Safe**: Linear time complexity, no panics on malformed input
- **Already shakmaty-based**: Integrates perfectly with our move generation

## Requirements

### Must Have
- **Streaming parser** - Handle large files without loading everything into memory ✓ (pgn-reader)
- **Multi-game PGN files** - Parse files containing thousands of games ✓ (pgn-reader)
- **Full annotation support**: ✓ (pgn-reader)
  - Comments (both `{ }` and `;` styles)
  - NAGs (Numeric Annotation Glyphs: `$1`, `$2`, etc.)
  - RAV (Recursive Annotation Variations) - nested move variations
- **PGN writing** - Export games back to valid PGN format (we build this)
- **Standard headers** - Parse all seven tag roster (STR) tags + common extensions ✓ (pgn-reader)

### Nice to Have (Later)
- ChessBase-specific extensions
- Fuzzy/tolerant parsing for malformed PGN files (pgn-reader handles common quirks)

## Architecture

```
src/
├── pgn/
│   ├── mod.rs
│   ├── visitor.rs     # Custom Visitor impl to build our game model
│   ├── writer.rs      # PGN serialization (we implement this)
│   ├── types.rs       # PgnGame, MoveTree, AnnotatedMove, etc.
│   └── loader.rs      # High-level API: load file, iterate games
```

## How pgn-reader Works

The crate uses a **visitor pattern** — you implement a `Visitor` trait and the reader calls your methods as it parses:

```rust
use pgn_reader::{Visitor, Reader, SanPlus, Nag, RawComment, Skip};

struct GameBuilder {
    headers: HashMap<String, String>,
    moves: MoveTree,
    current_variation: Vec<AnnotatedMove>,
}

impl Visitor for GameBuilder {
    type Result = PgnGame;

    fn header(&mut self, key: &[u8], value: &[u8]) {
        // Called for each header: [White "Carlsen"], [Date "2024.01.15"]
        let key = String::from_utf8_lossy(key);
        let value = String::from_utf8_lossy(value);
        self.headers.insert(key.into(), value.into());
    }

    fn san(&mut self, san: SanPlus) {
        // Called for each move: e4, Nf3, O-O
        self.current_variation.push(AnnotatedMove::from_san(san));
    }

    fn nag(&mut self, nag: Nag) {
        // Called for NAGs: $1 (!), $2 (?), $3 (!!), etc.
        if let Some(last) = self.current_variation.last_mut() {
            last.nags.push(nag);
        }
    }

    fn comment(&mut self, comment: RawComment<'_>) {
        // Called for comments: { this is a comment }
        if let Some(last) = self.current_variation.last_mut() {
            last.comment = Some(comment.as_bytes().to_vec());
        }
    }

    fn begin_variation(&mut self) -> Skip {
        // Called when entering a RAV: (1. d4 d5 2. c4)
        // Push current state, start new variation
        Skip(false)  // Return Skip(true) to skip this variation
    }

    fn end_variation(&mut self) {
        // Called when exiting a RAV
        // Pop variation, attach to parent move
    }

    fn end_game(&mut self) -> Self::Result {
        // Called at end of game, return the built game
        std::mem::take(&mut self.game)
    }
}
```

## Data Model

```rust
struct PgnGame {
    headers: HashMap<String, String>,
    moves: MoveTree,  // Tree structure for RAV support
}

struct MoveTree {
    main_line: Vec<AnnotatedMove>,
    // Each move can have variations branching off
}

struct AnnotatedMove {
    san: SanPlus,           // From pgn-reader, includes check/mate suffixes
    nags: Vec<Nag>,         // From pgn-reader
    comment: Option<String>,
    variations: Vec<MoveTree>,  // RAV: alternative lines
}
```

## Implementation Phases

### Phase 1: Basic Visitor & Game Model
- [ ] Define `PgnGame`, `MoveTree`, `AnnotatedMove` types
- [ ] Implement basic `Visitor` for headers + mainline moves
- [ ] High-level `load_pgn_file()` function returning iterator
- [ ] Integration with existing app (display loaded game)

### Phase 2: Full Annotation Support
- [ ] NAG handling in visitor (attach to moves)
- [ ] Comment parsing in visitor
- [ ] RAV handling: `begin_variation` / `end_variation` with stack
- [ ] Tree data structure construction

### Phase 3: Advanced Loading Features
- [ ] Progress reporting for large files (count bytes processed)
- [ ] Selective loading (parse headers only, skip to specific game)
- [ ] Error handling and recovery (skip malformed games, continue)
- [ ] Memory-efficient mode for huge files (process & discard)

### Phase 4: PGN Writer
- [ ] Serialize `PgnGame` back to PGN string
- [ ] Configurable formatting (line width, variation style)
- [ ] Multi-game file writing
- [ ] Round-trip fidelity (parse → write → parse = same result)

### Phase 5: Integration with Database
- [ ] Bulk import: stream PGN directly into database writer
- [ ] Progress UI for large imports
- [ ] Validation mode: check move legality during import

## Testing Strategy

- Unit tests with PGN snippets for each visitor callback
- Integration tests with real PGN files from:
  - TWIC (The Week in Chess)
  - Lichess game exports
  - ChessBase sample files
- Round-trip tests (parse → write → parse)
- Performance benchmarks vs raw pgn-reader (measure our overhead)

## Dependencies

```toml
[dependencies]
pgn-reader = "0.29"   # Streaming PGN parser
shakmaty = "0.30"     # Already using for move generation
```

## Performance Expectations

Based on pgn-reader benchmarks:
- **Parsing only**: ~500k games/sec
- **With validation**: ~200k games/sec  
- **Our overhead** (building tree model): Target <50% slowdown → ~250k games/sec

For a 10M game file: ~40 seconds to parse with full model construction.

## Open Questions

- Should we validate move legality during parsing or defer? (pgn-reader doesn't validate)
- How to handle encoding issues? (pgn-reader works with bytes, we need UTF-8 strings)
- Should `PgnGame` own a `Chess` position for each node, or reconstruct on demand?

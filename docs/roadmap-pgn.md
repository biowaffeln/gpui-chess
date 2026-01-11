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
- **Full annotation support**: ✓ (pgn-reader provides callbacks)
  - Comments (both `{ }` and `;` styles)
  - NAGs (Numeric Annotation Glyphs: `$1`, `$2`, etc.)
  - RAV (Recursive Annotation Variations) - nested move variations
- **PGN writing** - Export games back to valid PGN format (we build this)
- **Standard headers** - Parse all seven tag roster (STR) tags + common extensions ✓ (pgn-reader)

### Nice to Have (Later)
- ChessBase-specific extensions
- Fuzzy/tolerant parsing for malformed PGN files (pgn-reader handles common quirks)

## Current Architecture

```
src/domain/pgn/
├── mod.rs          # Module exports
├── types.rs        # PgnGame, PgnHeaders
└── visitor.rs      # GameBuilder visitor, parse_pgn, load_pgn_file

src/models/
└── pgn_library.rs  # PgnLibraryModel, GameSummary (UI state)

src/ui/views/
└── pgn_panel.rs    # Tabbed panel with Moves/Library, Table, file picker
```

## Implementation Phases

### Phase 1: Basic Visitor & Game Model ✅ COMPLETE

- [x] Define `PgnGame`, `PgnHeaders` types (`src/domain/pgn/types.rs`)
- [x] Implement basic `Visitor` for headers + mainline moves (`GameBuilder` in `visitor.rs`)
- [x] High-level `load_pgn_file()` function with byte-based reading (handles non-UTF-8)
- [x] Integration with existing app:
  - [x] `PgnLibraryModel` to manage loaded games
  - [x] `GameSummary` with White, WhiteElo, Black, BlackElo, Date, Event, Result, MoveCount
  - [x] Tabbed PGN panel (Moves | Library tabs)
  - [x] Table view with all game metadata
  - [x] Native file picker dialog for loading PGN files
  - [x] Click game to load into board view
- [x] Consistent SAN handling with `SanPlus` (check/mate suffixes)
- [x] Move validation during parsing (via shakmaty)

### Phase 2: Full Annotation Support

- [ ] NAG handling in visitor (attach to moves)
  - Store NAGs on MoveNode
  - Display NAG symbols (!!, !, !?, ?!, ?, ??) in move list
- [ ] Comment parsing in visitor
  - Store comments on MoveNode  
  - Display comments in move list (expandable?)
- [ ] RAV handling: `begin_variation` / `end_variation` with stack
  - Currently we skip variations with `Skip(true)`
  - Need to track position stack and attach variations to correct parent move
- [ ] Update MoveNode/MoveTree to store annotations

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

## How pgn-reader Works

The crate uses a **visitor pattern** — you implement a `Visitor` trait and the reader calls your methods as it parses:

```rust
use pgn_reader::{Visitor, Reader, SanPlus, Nag, RawComment, Skip};

impl Visitor for GameBuilder {
    type Tags = TagsState;
    type Movetext = MovetextState;
    type Output = Option<PgnGame>;

    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> { ... }
    fn tag(&mut self, tags: &mut Self::Tags, name: &[u8], value: RawTag<'_>) { ... }
    fn begin_movetext(&mut self, tags: Self::Tags) -> ControlFlow<Self::Output, Self::Movetext> { ... }
    fn san(&mut self, movetext: &mut Self::Movetext, san: SanPlus) { ... }
    fn nag(&mut self, movetext: &mut Self::Movetext, nag: Nag) { ... }      // TODO
    fn comment(&mut self, movetext: &mut Self::Movetext, comment: RawComment<'_>) { ... }  // TODO
    fn begin_variation(&mut self, movetext: &mut Self::Movetext) -> Skip { ... }  // TODO: don't skip
    fn end_variation(&mut self, movetext: &mut Self::Movetext) { ... }  // TODO
    fn end_game(&mut self, movetext: Self::Movetext) -> Self::Output { ... }
}
```

## Testing Strategy

- [x] Unit tests with PGN snippets for visitor callbacks (5 tests in `visitor.rs`)
- [ ] Integration tests with real PGN files from:
  - [x] TWIC (The Week in Chess) - tested with `twic1602.pgn`
  - [ ] Lichess game exports
  - [ ] ChessBase sample files
- [ ] Round-trip tests (parse → write → parse)
- [ ] Performance benchmarks vs raw pgn-reader (measure our overhead)

## Dependencies

```toml
[dependencies]
pgn-reader = "0.29"   # Streaming PGN parser
shakmaty = "0.30"     # Move generation and validation
```

## Resolved Questions

- **Encoding issues**: Solved by reading files as bytes (`read_to_end`) and passing to pgn-reader directly. Non-UTF-8 player names (Latin-1) are handled gracefully.
- **Position storage**: Each `MoveNode` owns a `Chess` position for instant access without reconstruction.
- **Move legality**: We validate during parsing via `san.to_move(&position)` - invalid moves cause the game to be skipped.

## Open Questions

- How to display comments in the move list? Inline? Expandable panel?
- Should NAGs be shown as symbols (!!, ?) or kept as $n codes?
- For RAV: how deep should we support nested variations?

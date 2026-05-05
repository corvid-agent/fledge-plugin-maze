# fledge-plugin-maze

Terminal maze generator and solver for [fledge](https://github.com/CorvidLabs/fledge) -- recursive backtracker with BFS solver.

Generates perfect mazes using a randomized depth-first (recursive backtracker) algorithm and solves them with breadth-first search. Renders with Unicode box-drawing characters by default, with an optional ASCII mode and animated TUI mode.

## Install

```bash
fledge plugins install corvid-agent/fledge-plugin-maze
```

## Usage

```bash
# Generate a default 20x10 maze
fledge maze

# Custom size
fledge maze --width 30 --height 15

# Reproducible maze with seed
fledge maze --seed 42

# Show the solution path (BFS shortest path)
fledge maze --solve

# Animated generation and solve in alternate screen
fledge maze --animate

# ASCII rendering (no Unicode box-drawing)
fledge maze --ascii
```

## Options

| Flag | Short | Description |
|------|-------|-------------|
| `--width <n>` | `-w` | Maze width in cells (default: 20, max: 100) |
| `--height <n>` | `-h` | Maze height in cells (default: 10, max: 50) |
| `--seed <n>` | `-s` | Random seed for reproducible mazes |
| `--solve` | | Show the BFS solution path from top-left to bottom-right |
| `--animate` | `-a` | Animated generation and solve (implies `--solve`) |
| `--ascii` | | Use ASCII `+`, `-`, `|` instead of Unicode box-drawing |
| `--help` | `-h` | Show help |

## How it works

1. **Generation** -- Recursive backtracker (randomized DFS). Starts at (0,0), carves passages by removing walls between the current cell and a random unvisited neighbor. Backtracks when stuck. Produces a perfect maze (every cell reachable, exactly one path between any two cells).

2. **Solving** -- Breadth-first search from the top-left corner to the bottom-right corner, guaranteeing the shortest path.

3. **Rendering** -- Unicode box-drawing characters with color via `crossterm`. The animated mode uses the alternate screen with per-step wall carving and BFS exploration visualization.

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt --check
```

## License

MIT

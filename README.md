# Boulder Dash Survival

A first Rust / Bevy prototype: explore an endless, progressively generated grid,
dig through dirt, and avoid falling stones. Graphics are built from simple shapes;
no external assets are needed.

## Run

Requires Rust 1.89 or newer, a graphical desktop, and a GPU/driver supported by Bevy.

```sh
cargo run
```

Pass a numeric seed to replay a world:

```sh
cargo run -- 42
```

The first build downloads and compiles Bevy 0.18.1. On Linux, install the development
libraries for X11, Wayland, and xkbcommon if your system does not already have them.
See [Bevy's Linux dependencies](https://github.com/bevyengine/bevy/blob/v0.18.1/docs/linux_dependencies.md).

## Controls

| Key | Action |
| --- | --- |
| WASD / arrow keys | Move one cell at a time; hold to continue; moving into dirt digs it |
| R | Restart with the same seed |
| N | Start a fresh world with a new seed |
| Escape | Quit |

## Current rules

- Each new cell is randomly dirt (82%) or stone (18%). A small empty starting pocket is provided.
- Cells are generated in 16 × 16 chunks before they come into view. Coordinate hashing
  makes terrain independent of exploration order, including at negative coordinates.
- Stones block movement; dirt supports stones. Stones fall vertically into empty
  cells, one cell per simulation step. The player safely supports a resting stone
  when digging directly beneath it. A stone crushes the player only if it was
  already falling, having moved through at least one empty cell before impact.
- Moving left or right into a stone pushes it one cell if the cell beyond is empty.
  Dirt or another stone blocks the push; stones cannot be pushed vertically or in chains.
  The player steps into the stone's old cell. The pushed stone can fall or slip starting
  on the next simulation step.
- A stone resting on another stone slips sideways if the side cell and the cell
  below that side are empty and clear of the player. When both sides are available,
  it chooses randomly (reproducibly from the seed and simulation tick). It slides
  one cell horizontally, then can fall on the next step. Dirt below prevents slipping.
- Player movement and falling stones animate smoothly over a 0.12-second step.
  A departing player reserves their old cell until the step finishes.
- A harmless purple triangular creature starts in the starting pocket. Its point
  shows its heading. It follows obstacles with its right hand: try right, forward,
  left, then backward; wait if surrounded. It moves smoothly through empty cells
  without digging. Dirt, stones, the player, and other creatures block it.
- Creatures are solid and cannot be crushed or pushed. They block falling and
  slipping stones, stone pushes, and player movement without causing damage.
  Moving entities reserve both their old and new cells for the step, preventing
  creatures from overlapping or exchanging cells through each other.
- Gravity runs within 24 cells of the player, beyond the visible area. Distant areas
  freeze until approached again. The map expands in all directions without predefined bounds.
- Explored chunks and changes remain in memory throughout the session. Memory use
  grows with exploration; disk saves and unloading are future work.
- Creature spawning beyond the starter creature, stats, inventory, and magic are future work.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

The simulation is independent of Bevy and can also be tested without downloading dependencies:

```sh
rustc --edition=2024 --test src/world.rs -o /tmp/bbsurvival-world-tests
/tmp/bbsurvival-world-tests
```

`src/world.rs` owns generation, digging, and gravity. `src/main.rs` connects keyboard
input, fixed simulation steps, rendering, and the HUD. See [DESIGN.md](DESIGN.md)
for the broader design notes.

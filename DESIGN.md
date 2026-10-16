# Boulder Dash Survival — Design Notes

These notes capture the design discussion. The first prototype implements the basic
map, player movement, digging, and falling stones; see README.md for its current rules.

## Agreed foundations

- **Language:** Rust.
- **Engine:** Bevy.
- **Game:** A 2D survival game inspired by Boulder Dash.
- **World:** An infinite matrix (grid) of cells. The map is randomly generated progressively as the player travels.
- **Movement:** Stones, the player, and creatures move from one cell to an adjacent cell. Movement is visually smooth rather than jumping between cells as in the classic Atari game.

## Player and creatures

The player has these properties:

- HP (health).
- Strength.
- Mana (magic power).

Creatures have properties including:

- Strength.
- Speed.

Exact stat values, combat rules, magic, and creature behaviors remain to be designed.

## Inventory and items

- The player has an inventory.
- Items can be found in the world.
- Example items include swords and food.

Inventory capacity, item stacking, equipment slots, item effects, and how items are collected or used remain to be decided.

## First implementation scope

- Generate random stones and dirt as the player explores previously unknown terrain.
- Provide empty cells for movement and falling stones.
- The player clears dirt by moving into it, as in classic Boulder Dash.
- Stones fall through empty cells and stop on dirt or other stones.
- Stones on top of stones slip left or right when the side and the cell beneath
  that side are free; if both sides are free, choose randomly. Movement is horizontal
  first, followed by falling, to keep transitions between adjacent cells.
- The player can push a stone left or right into an empty cell. Dirt or another
  stone beyond it blocks pushing. The player and stone each move one adjacent cell.
- Keep player and stone movement smooth between adjacent cells.

The prototype uses seeded chunks, a local gravity simulation, four-direction input,
and a safe starting pocket. These are initial implementation choices that can be tuned.
The player supports resting stones when digging directly beneath them. Only stones
already falling from a greater height crush the player; restart is available.
Stats and inventory are deferred.

## Proposed approaches — not yet agreed

- Generate the world in chunks using a world seed for consistent generation.
- Preserve changes to explored areas when the player returns.
- Simulate an area around the player, with a margin beyond the screen, and leave distant areas dormant.
- Keep logical grid positions authoritative and animate visual movement between cells.
- Use a fixed simulation timestep, with movement duration determined by entity speed.
- Reserve destination cells during movement to resolve competing moves.
- Use four-direction movement without diagonals.
- Include falling rocks, crushing, and rocks rolling off rounded obstacles.
- Separate carried inventory from equipped items; equipment can modify stats.
- Allow consumable items, potentially including food for HP and potions for mana.

## Open design questions

- Is the game single-player or multiplayer, and which platforms should it target?
- What is the main survival loop, and what motivates continued exploration?
- Does moving into a creature initiate combat, or does the player attack explicitly?
- How do strength, equipment, damage, and magic interact?
- Does food restore HP, satisfy hunger, or both?
- How much activity continues outside the player's immediate area?
- What rules determine terrain, caves, resources, creature placement, and difficulty across the infinite world?
- What are the consequences of death?
- What belongs in the first playable version?

## Suggested first playable scope — not yet agreed

A generated cave that expands as the player explores, smooth movement on the grid, digging and falling stones, one creature type, player stats, and a basic inventory with a sword and food.

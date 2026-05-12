# Solarance Movement Prototype (Archived)

> **This repository is archived.** Development has moved to **[GalaxyCr8r/solarance-beginnings](https://github.com/GalaxyCr8r/solarance-beginnings)**.

This was a standalone prototype used to explore ship movement, dead reckoning, and bullet mechanics against a SpacetimeDB backend before merging the work into the main Solarance game.

## What's here

- `spacetimedb/` — SpacetimeDB Rust module (tables + reducers for ships, bullets, sectors)
- `src/` — Macroquad-based Rust client (rendering, input, connection)
- `solarance-shared/` — Shared dead-reckoning / position prediction code used by both client and server
- `docs/` — MVP design doc and handoff notes for integration into `solarance-beginnings`

## Status

The smooth movement, angular dampening, and bullet systems prototyped here were merged into `solarance-beginnings`. The final commit on this branch switched from degrees to radians to match the main repo's conventions.

For anything new, go to **[solarance-beginnings](https://github.com/GalaxyCr8r/solarance-beginnings)**.

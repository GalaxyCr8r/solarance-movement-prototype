# Solarance Movement Prototype: Objectives and Thresholds

> Plans never survive contact with the enemy, but you won't survive contact with the enemy without plans.
>
> - Anonymous

I, Karl Nyborg, created this repo to get away from the already monolithic Solarance: Beginnings codebase, and all the baggage therein. I want to try to make a more modular and maintainable codebase, but that comes secondary to developing, testing, and iterating on the movement system.

Game developement is not the same as commerical software development. Games live and die base off of FEEL. If the basic every day motion of the game, the thing that ALL other gameplay element hang off of, does not feel good, then the game will be a failure before it even begins.

## Overview

There are _thresholds_ and _objectives_, _thresholds_ are the minimum requirements to consider the movement system good, _objectives_ are the things that will make the movement system great. The primary _threshold_ is to attempt to make asteroids-like movement feel good within the constraints of a SpacetimeDB backend and MMO architecture. The secondary _objective_ is to make inter-sector and inter-stellar movement feel good as well.

## Defintions

- Ship: A vehicle that can move through space. Used by players and NPCs. Within sectors and star systems, ships can move freely. However interstellar travel requires the use of gates, warpholes, or warpdrives.
- Sector: A discrete area of space within a star system. Space station and ship-to-ship combat are exclusively inside sectors.
- Lagrange: A collection of sectors. Traditionally a lagrange point is a point in space where the gravitational pull of two objects cancel each other out, but in this context it is a collection of nearby sectors usually placed near those points.
- Star System: A collection of stars, planets, asteroid belts, nebulae, and sectors.

# Thresholds

Things required to compelete before continuing project on Solarance: Beginnings.

## 1. Asteroids-like movement

The first rung, implement a simple client and server - a minimum viable product where ships can move around in space. This should feel like asteroids, but with the added complexity of a SpacetimeDB backend and MMO architecture.

## 2. Find the lowest bar of performance to bandwidth

We need to find the slowest amount of updates per second that will allow the movement system to feel good. This will be the lowest bar. We may even deploy to STDB main cloud to use the free tier and their built-in measurement tools.

# Objectives

Listed in no particular order. These would be good things to investigate before continuing on Solarance: Beginnings.

## 1. Prototype inter-sector movement

## 2. Prototype gate and warphole usage for inter-stellar movement

## 3. Prototype warpdrive usage for inter-stellar movement

## 4. Prototype automatic sector creation

Could be useful for interdiction combat and exploration of new sectors.

Sectors will be created automatically at various lagrange points in planet and moon orbits. However, players and corporations will want to create their own sectors for their own purposes. Potentially also controlling who knows about the sector by creating it at orbits that aren't nearby to a planet or other point of interest.

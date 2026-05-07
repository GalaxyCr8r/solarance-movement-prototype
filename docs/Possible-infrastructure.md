# Implementation Handoff: Game Backend Orchestration System

## Context

This document captures architecture decisions from a design conversation about building a scalable game-logic backend for a "Star Sector"-style 2D Space MMO. The system runs on a homelab (Lenovo Yoga 730, 8GB RAM, Ubuntu Server 24.04) and uses SpacetimeDB (STDB) as the primary game-state backend. The goal is to minimize STDB cloud compute costs by running heavy game logic (NPC AI, background simulation) in local Docker containers that connect to STDB as clients.

## Component 1: World Worker (`worker/`)

### What It Is

A single Rust binary that can handle **any assignment** — one sector, multiple sectors, an entire solar system, or multiple solar systems. The same Docker image is used for every worker instance; only the configuration differs.

In the future we will want it to generate client STDB module binding at build time. But for now you can retrieve the client schema from the local file system here: `..\solarance-movement-prototype\src\module_bindings`

The exact calculations done inside the worker will mostly be NPC AI - making decisions of when to attack someone, when to run away, etc. - most background simulation will happen via timers which SpacetimeDB itself can handle.

### Key Design Decisions

1. **Assignment is application-level state, not container config.** The worker reads an initial assignment from an env var (`INITIAL_ASSIGNMENT`) on startup as a fallback, but in steady state, the orchestrator pushes assignment changes via HTTP at runtime. This avoids container restarts (which would kill STDB connections and drop in-memory state).

2. **One tokio task per sector.** The worker spawns an async task for each assigned sector. Each task subscribes to the relevant STDB tables, runs the tick loop (target: 2 ticks/sec), and processes NPC AI / background simulation for that sector.

3. **Built-in HTTP API** (use `axum` or `warp`). Endpoints:

| Endpoint                     | Method | Purpose                                                                                                               |
| ---------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------- |
| `GET /status`                | GET    | Returns per-sector player counts, CPU/memory usage, uptime. Orchestrator polls this.                                  |
| `POST /assignments/add`      | POST   | Orchestrator tells worker to take on a new sector. Worker spins up a new tokio task, subscribes to STDB.              |
| `POST /assignments/remove`   | POST   | Orchestrator tells worker to release a sector. Worker finishes current tick, unsubscribes from STDB, confirms.        |
| `POST /assignments/activate` | POST   | Orchestrator tells worker to begin active processing on a sector it has already subscribed to (used during handoffs). |
| `GET /health`                | GET    | Simple liveness check for Docker/Swarm health checks.                                                                 |

4. **STDB connection.** The worker connects to STDB using the shared auth token (read from Docker Swarm secret at `/run/secrets/stdb_token`). Test stack points at a local STDB instance; prod stack points at STDB MainCloud.

### Initial Assignment Format (env var)

```json
{
  "systems": [
    {
      "system_id": "sol",
      "sectors": ["sector_1", "sector_2", "sector_3"]
    },
    {
      "system_id": "alpha_centauri",
      "sectors": ["all"]
    }
  ]
}
```

`"all"` means the worker handles every sector in that system. Specific sector IDs mean it only handles those.

---

## Component 2: Orchestrator (`orchestrator/`)

### What It Is

A single container that runs alongside the workers in the same Swarm stack. It has two communication channels:

1. **Docker Engine API** (via `/var/run/docker.sock` mounted into the container) — used to create/destroy/update Swarm services (i.e., spin up new worker containers or shut down idle ones).
2. **HTTP to workers** — used to push runtime assignment changes and poll load metrics.

The orchestrator does **not** need SpacetimeDB bindings. It gets all game-world load information from the workers themselves.

### Rust Crates

- `bollard` — async Docker Engine API client
- `reqwest` or `hyper` — HTTP client for polling workers
- `tokio` — async runtime
- `serde` / `serde_json` — config and metrics serialization

### Decision Loop (runs every 10–30 seconds)

```
1. Discover all running worker services via Docker API
2. Poll each worker's GET /status endpoint
3. Collect per-sector player counts and resource usage
4. Evaluate scaling rules:
   - IF a sector has > SPLIT_THRESHOLD players (e.g., 150)
     → Split: spin up a dedicated worker for that sector
   - IF a worker's total load < MERGE_THRESHOLD (e.g., 20 players across all its sectors)
     → Merge: redistribute its sectors to other underloaded workers, shut it down
   - IF a worker's CPU/memory exceeds OVERLOAD_THRESHOLD
     → Shed: move some of its sectors to a new or underloaded worker
5. Execute changes (see Handoff Protocol below)
6. Sleep until next cycle
```

### Handoff Protocol (Sector Transfer: Worker A → Worker B)

This is the most critical piece to get right. Two workers must never actively process the same sector simultaneously, and there should be no gap where nobody processes it.

```
Step 1: Orchestrator → Worker B:  POST /assignments/add    {"sector": "sector_47"}
        Worker B subscribes to STDB tables for sector_47, loads state, reports ready.

Step 2: Orchestrator polls Worker B's /status until sector_47 shows status: "ready"

Step 3: Orchestrator → Worker A:  POST /assignments/remove  {"sector": "sector_47"}
        Worker A finishes current tick for sector_47, unsubscribes, confirms removal.

Step 4: Orchestrator → Worker B:  POST /assignments/activate {"sector": "sector_47"}
        Worker B begins active tick processing.
```

During the brief overlap (Step 1–3), Worker B is subscribed and receiving STDB updates but not writing. Worker A is still the sole writer. Once Worker A confirms release, Worker B activates. This ensures exactly-one-writer semantics.

---

## Component 3: Docker Stacks

### Shared Concepts

- **Swarm secrets** for the STDB auth token. Created once via `echo "your-token" | docker secret create stdb_token -`. All workers and the orchestrator read it from `/run/secrets/stdb_token`.
- **Swarm internal DNS** handles service discovery. The orchestrator finds workers via Docker API service listing, not hardcoded addresses.
- **Portainer** is deployed as part of the stack for GUI-based monitoring, log viewing, and manual intervention. It does not make orchestration decisions.

### Test Stack (`stacks/docker-compose.test.yml`)

```yaml
# Key differences from prod:
# - Includes a local SpacetimeDB container
# - Workers connect to the local STDB instance
# - Orchestrator runs with relaxed thresholds for testing
# - Fewer initial worker replicas

services:
  spacetimedb:
    image: clockworklabs/spacetimedb:latest
    ports:
      - "3000:3000"
    volumes:
      - stdb_test_data:/var/lib/spacetimedb

  orchestrator:
    build:
      context: .
      dockerfile: Dockerfile.orchestrator
    environment:
      - STDB_HOST=spacetimedb:3000
      - POLL_INTERVAL_SECS=15
      - SPLIT_THRESHOLD=50 # Lower for testing
      - MERGE_THRESHOLD=5
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    secrets:
      - stdb_token
    deploy:
      replicas: 1

  worker:
    build:
      context: .
      dockerfile: Dockerfile.worker
    environment:
      - STDB_HOST=spacetimedb:3000
      - INITIAL_ASSIGNMENT={"systems":[{"system_id":"test_system","sectors":["all"]}]}
    secrets:
      - stdb_token
    deploy:
      replicas: 2

  portainer:
    image: portainer/portainer-ce:latest
    ports:
      - "9443:9443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - portainer_data:/data

secrets:
  stdb_token:
    external: true

volumes:
  stdb_test_data:
  portainer_data:
```

### Prod Stack (`stacks/docker-compose.prod.yml`)

```yaml
# Key differences from test:
# - NO local SpacetimeDB container (uses STDB MainCloud)
# - Workers connect to the official MainCloud endpoint
# - Production thresholds tuned for real player load
# - More initial worker replicas to cover all solar systems

services:
  orchestrator:
    image: your-registry/orchestrator:latest
    environment:
      - STDB_HOST=maincloud.spacetimedb.com # Official MainCloud endpoint
      - POLL_INTERVAL_SECS=10
      - SPLIT_THRESHOLD=150
      - MERGE_THRESHOLD=20
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    secrets:
      - stdb_token
    deploy:
      replicas: 1

  worker:
    image: your-registry/worker:latest
    environment:
      - STDB_HOST=maincloud.spacetimedb.com
      - INITIAL_ASSIGNMENT={} # Orchestrator assigns on startup
    secrets:
      - stdb_token
    deploy:
      replicas: 5
      resources:
        limits:
          memory: 1G # Tune based on profiling
        reservations:
          memory: 256M

  portainer:
    image: portainer/portainer-ce:latest
    ports:
      - "9443:9443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - portainer_data:/data

secrets:
  stdb_token:
    external: true

volumes:
  portainer_data:
```

---

## Implementation Order

### Phase 1: Basic Worker (Start Here)

1. Scaffold the `worker/` Rust project with `spacetimedb-sdk`, `axum`, and `tokio`.
2. Implement a hardcoded single-sector tick loop that connects to STDB and subscribes to tables.
3. Add the `/health` and `/status` HTTP endpoints.
4. Dockerize it (`Dockerfile.worker`).
5. Deploy it via the test compose stack with a local STDB instance.
6. Verify it connects, subscribes, and ticks correctly.

### Phase 2: Dynamic Assignments

1. Add the `/assignments/add`, `/assignments/remove`, and `/assignments/activate` endpoints.
2. Implement the internal assignment registry (a `HashMap<SectorId, SectorTask>` behind an `Arc<RwLock<...>>`).
3. Parse `INITIAL_ASSIGNMENT` env var on startup.
4. Test manually: `curl` the endpoints to add/remove sectors while the worker is running.

### Phase 3: Orchestrator

1. Scaffold the `orchestrator/` Rust project with `bollard`, `reqwest`, and `tokio`.
2. Implement worker discovery via Docker API.
3. Implement the metrics polling loop.
4. Implement basic scaling rules (split/merge based on player count thresholds).
5. Implement the handoff protocol.
6. Deploy via test stack, test with simulated load.

### Phase 4: Production Readiness

1. Profile memory/CPU per sector on the Lenovo to tune thresholds.
2. Switch STDB connection to MainCloud, deploy via prod stack.
3. Add logging and alerting (Portainer logs + Uptime Kuma on the Raspberry Pi).
4. Stress test with simulated NPC load (target: 10,000 NPC ships at 2 ticks/sec).

---

## Key Architecture Principles

- **Same image, different config.** Every worker runs the same binary. Assignment differences are runtime state, not build-time config.
- **Never restart to reconfigure.** Assignment changes happen via HTTP at runtime. Container restarts kill STDB connections and lose in-memory state.
- **Orchestrator is the source of truth.** The env var `INITIAL_ASSIGNMENT` is a startup fallback. In steady state, the orchestrator owns the mapping of sectors to workers.
- **Workers self-report.** The orchestrator never connects to STDB. All game-world load data comes from worker HTTP endpoints.
- **Docker manages infrastructure; the app manages game logic.** Swarm handles secrets, health checks, restarts, networking. The Rust processes handle sector assignment, ticking, and STDB communication.

---

## Hardware Constraints to Keep in Mind

- **8GB RAM total** on the Lenovo. Budget ~2GB for OS + Docker + Portainer. That leaves ~6GB for workers + orchestrator. Profile early to know your per-sector memory cost.
- **4 cores (8 threads)** on the i5-8250U. Don't over-subscribe: if each worker spawns many tokio tasks, they'll fight for CPU time.
- **512GB NVMe** is generous for storage but irrelevant for this workload (state lives in STDB, not locally).
- **Raspberry Pi** is monitoring-only. Don't run game logic on it.

# Components

**Components for Large-Scale Structural Graph System:**

| Layer       | Components                                                                        |
| ----------- | --------------------------------------------------------------------------------- |
| **Storage** | Merkle DAG Store, Symbol Index (trie+hashmap), Reverse Reference Index            |
| **Engines** | Subgraph Extraction, View Projection (L0-L4), Delta Algebra, Snapshot/Time Replay |
| **Caching** | Hot Region LRU, View Cache, Snapshot Cache                                        |
| **Access**  | Lazy Hydration API, Intent Translation (UI→Delta)                                 |

# Mapping

Below is a **storage-engine mapping** aligned with the [COA invariants](../01-BluePrint/01-intro.md) (typed artifacts, Merkle immutability, staged construction, time scrubbing)  

Assumptions:

* Immutable Merkle DAG core
* No in-place mutation
* Versioned structural graphs
* Multi-view projections
* Time scrub replay
* Deterministic reconstruction

---

## 1. Core Data Domains → Storage Mapping

| Domain                 | Nature                           | Best Storage              | Why                                        | Cache?              |
| ---------------------- | -------------------------------- | ------------------------- | ------------------------------------------ | ------------------- |
| Merkle AST Nodes       | Immutable content-addressed tree | LMDB / RocksDB (KV)       | O(1) hash lookup, memory-mapped, no upsert | Yes (hot nodes)     |
| Artifact Metadata      | Small structured records         | LMDB                      | Fast read, append-only                     | Yes                 |
| Symbol Index           | Large lookup map                 | LMDB or embedded RocksDB  | Fast key→node resolution                   | Yes                 |
| Reverse Ref Index      | Graph-like adjacency             | Graph DB (Nebula/TuGraph) | Traversals, impact queries                 | Partial             |
| Dependency Graph       | Highly connected                 | Graph DB                  | Native traversal engine                    | Partial             |
| Derivation Chains      | DAG with version edges           | Graph DB                  | Efficient ancestor queries                 | No (rarely mutated) |
| View Projections       | Derived graphs                   | Graph DB                  | Natural fit for multi-view                 | Yes                 |
| Structural Deltas      | Append-only event log            | Parquet + DuckDB          | Analytical, columnar scan                  | No                  |
| Execution Logs         | Append-only, audit               | Parquet                   | Compression + time filtering               | No                  |
| Snapshot Index         | Root hash registry               | LMDB                      | Fast version pointer                       | Yes                 |
| Large Binary Artifacts | Immutable blobs                  | Object store (S3/MinIO)   | Content addressed                          | No                  |

---

## 2. Graph Databases Role (NebulaGraph / TuGraph)

Use graph DB **only for connected graph queries**, not as artifact store.

| Use Case                     | Store in Graph DB? | Reason               |
| ---------------------------- | ------------------ | -------------------- |
| Symbol dependency traversal  | Yes                | Native BFS/DFS       |
| Cross-module impact analysis | Yes                | Multi-hop queries    |
| View projection graph        | Yes                | Natural mapping      |
| Merkle AST storage           | No                 | KV better            |
| Delta history                | No                 | Columnar better      |
| Raw artifact storage         | No                 | Graph DB inefficient |

Implication:
Graph DB is **index + relationship engine**, not artifact storage.

---

## 3. KV Store (LMDB / RocksDB) Role

Ideal for immutable Merkle-based system.

| Data Type          | Why KV Works                  |
| ------------------ | ----------------------------- |
| AST node blobs     | Key = content hash            |
| Snapshot root      | Key = version id              |
| Symbol registry    | Key = SymbolRef               |
| Artifact headers   | Small structured record       |
| View cache entries | Key = (view_type + root_hash) |

Advanced advantages due to immutability:

* No compaction stress
* No update contention
* Memory-mapped zero-copy reads
* Perfect for time-travel lookup
* Cheap branch creation

LMDB advantage:

* Memory-mapped B+tree
* No background compaction
* Extremely fast read
* Good for read-heavy immutable workload

RocksDB advantage:

* Better for high write concurrency

For this system: **LMDB fits very well.**

---

## 4. Columnar Storage (Parquet + DuckDB / Embedded ClickHouse)

Use for:

| Domain                    | Why Columnar         |
| ------------------------- | -------------------- |
| Delta history analytics   | Scan many events     |
| Performance metrics       | Aggregation          |
| Autonomy analytics        | Time-series queries  |
| Experiment sandbox output | Analytical workloads |
| Replay indexing           | Time-window queries  |

Comparison:

| Option              | Pros                        | Cons                |
| ------------------- | --------------------------- | ------------------- |
| DuckDB              | Embedded, simple, fast OLAP | Not distributed     |
| Embedded ClickHouse | High throughput             | Operational heavier |
| Parquet only        | Cheap archival              | Needs query engine  |

Recommended:
Parquet + DuckDB embedded.

Not suitable for:

* Low latency node lookup
* Traversal queries

---

## 5. Graph DB vs KV vs Columnar — Clear Separation

| Concern                | KV  | Graph DB       | Columnar |
| ---------------------- | --- | -------------- | -------- |
| Hash → node lookup     | ✓   | ✗              | ✗        |
| Symbol → definition    | ✓   | ✓ (index edge) | ✗        |
| Multi-hop dependency   | ✗   | ✓              | ✗        |
| Version pointer lookup | ✓   | ✗              | ✗        |
| Time analytics         | ✗   | ✗              | ✓        |
| Artifact blob storage  | ✓   | ✗              | ✗        |
| View projection graph  | ✗   | ✓              | ✗        |
| Execution logs         | ✗   | ✗              | ✓        |

---

## 6. Caching Layer (Redis / DragonFly)

Because data is immutable:

| Cache Target          | Why Cache                 |
| --------------------- | ------------------------- |
| Hot subtrees          | UI responsiveness         |
| View projections      | Avoid recomputation       |
| Dependency closures   | Expensive graph traversal |
| Snapshot diff results | Frequent UI use           |
| Symbol resolution     | Repeated lookups          |
| Replay frame chunks   | Smooth scrubbing          |

Advanced opportunity due to immutability:

* No invalidation logic needed
* Cache key = root_hash
* Infinite safe caching
* Perfect for distributed caching

DragonFly advantage:

* Better memory efficiency than Redis
* High throughput

---

## 7. View Projection Storage Strategy

Views are derived.

Two options:

| Strategy                | Pros            | Cons             |
| ----------------------- | --------------- | ---------------- |
| On-demand projection    | No storage cost | CPU heavy        |
| Materialized view cache | Fast UI         | Storage overhead |

Given immutability:
Materialized views can be keyed by root_hash.
No invalidation required.

Best hybrid:

* Compute once
* Store in KV (small)
* Heavy relationship view stored in Graph DB

---

## 8. Time Scrubbing Support

Needs:

| Component              | Storage  |
| ---------------------- | -------- |
| Snapshot root registry | LMDB     |
| Delta chain            | Parquet  |
| Merkle DAG lookup      | LMDB     |
| Dependency diff        | Graph DB |
| Replay frame cache     | Redis    |

Advanced opportunity:
Because Merkle DAG is immutable:

* Diff = hash comparison
* Time travel = pointer swap
* Branching = cheap pointer creation

No snapshot duplication required.

---

## 9. What Immutability Enables

| Feature                  | Why Now Easy          |
| ------------------------ | --------------------- |
| Infinite branching       | Just new root pointer |
| Lock-free reads          | No write conflict     |
| Aggressive caching       | No invalidation       |
| Deterministic replay     | Hash anchored         |
| Structural deduplication | Merkle sharing        |
| Cheap snapshot cloning   | Copy root reference   |
| Parallel analysis        | Same root read-only   |
| Event sourcing alignment | Native                |

---

## 10. What Not To Do

| Anti Pattern                         | Why            |
| ------------------------------------ | -------------- |
| Store AST fully inside graph DB      | Too heavy      |
| Use columnar DB for real-time lookup | Slow           |
| Use graph DB for blob storage        | Inefficient    |
| Use Redis as source of truth         | Volatile       |
| Over-materialize every view          | Storage blowup |

---

## 11. Full System Storage Topology

| Layer                         | Engine            |
| ----------------------------- | ----------------- |
| Artifact Blob + Merkle        | LMDB              |
| Symbol Index                  | LMDB              |
| Snapshot Registry             | LMDB              |
| Dependency / Derivation Graph | Nebula / TuGraph  |
| View Projection Graph         | Nebula / TuGraph  |
| Delta Event Log               | Parquet           |
| Analytical Queries            | DuckDB            |
| Runtime Cache                 | Redis / DragonFly |
| Binary Storage                | Object Store      |

---

## 12. Performance Characteristics

| Concern                    | Impact                    |
| -------------------------- | ------------------------- |
| Large monorepo             | LMDB memory map efficient |
| Multi-hop dependency query | Graph DB optimized        |
| Massive delta log          | Columnar compresses well  |
| Time scrubbing             | O(1) root switch          |
| UI latency                 | Cache critical            |
| Parallel branch creation   | Trivial                   |

---

## 13. Critical Engineering Risks

| Risk                          | Mitigation                             |
| ----------------------------- | -------------------------------------- |
| Graph DB write amplification  | Batch writes                           |
| LMDB file size growth         | Periodic compaction via snapshot prune |
| Cross-engine consistency      | Transaction orchestration layer        |
| High cardinality symbol index | Sharding by namespace                  |
| View explosion                | Lazy materialization                   |
| Replay latency                | Frame prefetching                      |

---

## 14. High-Level Architectural Decision

This **polyglot storage architecture** optimized by data shape:

* KV for immutable structural truth
* Graph DB for relationships
* Columnar for analytics
* Memory cache for latency
* Object store for large blobs

Since this system is:

* Immutable
* Deterministic
* Append-only
* Hash-addressed

one can exploit:

* Zero invalidation caching
* Lock-free read scaling
* Cheap branching
* Hash-based structural dedupe
* Deterministic time replay

---
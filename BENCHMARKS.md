# Benchmarks

All results measured on Apple M-series (arm64), writing to `io::sink()`, using
`cargo bench` (optimized release profile). Numbers are criterion mean estimates.

## How to run

```bash
# Run all benchmarks
cargo bench

# Save a named baseline at a milestone
cargo bench --bench logging -- --save-baseline <name>

# Compare a future run against a saved baseline
cargo bench --bench logging -- --baseline <name>

# Open the HTML report
open target/criterion/report/index.html
```

Criterion saves named baselines under `target/criterion/<bench>/<name>/`. The
`target/` directory is gitignored, so baselines are local only. Record milestone
snapshots here manually when they represent a meaningful state change.

---

## Principles

**Save a baseline at every meaningful milestone.** A milestone is any commit that
completes a phase, introduces a new optimization, or deliberately accepts a
tradeoff. Use a descriptive name tied to what changed, not to a date:
`phase3-buffer-reuse`, `phase5-nonblocking`, not `2026-06-03`.

**Record the snapshot here, not just in criterion.** Baselines live in `target/`
and are not committed. This file is the durable record. Include the before/after
table, the delta, and — most importantly — the explanation of *why* the number
moved. A number without context is hard to act on later.

**Explain regressions instead of hiding them.** If a change makes one benchmark
worse while improving another, document both and explain the tradeoff. The
disabled-event regression in Phase 3 is a good example: it is flagged, explained,
and judged acceptable. Future contributors should be able to read this file and
understand what was known and decided, not just what the numbers were.

**Treat "within noise" honestly.** Criterion's confidence intervals tell you when
a delta is statistically meaningful. Do not claim an improvement that falls within
noise — note it as noise and move on.

**Compare against the right baseline.** When working on a specific path (e.g. the
disabled fast-path in Phase 4), compare against the baseline that represents the
state just before that work started. Do not compare against an older baseline that
included unrelated changes.

**Keep the benchmark code representative.** The benchmarks use `io::sink()` to
isolate encoding cost from I/O cost. If a future change makes the sink choice
material (e.g. a batching writer), add a separate benchmark that reflects real
sink behavior rather than modifying the existing ones.

---

## Milestone snapshots

### Phase 2 complete — pre-optimization baseline

Recorded after Phase 2 (subloggers and context) was finished, before any Phase 3
performance work. This is the starting point for measuring optimization impact.

| benchmark | wirelog static | wirelog dynamic | tracing |
|---|---|---|---|
| disabled event | 2.0 ns | 2.0 ns | 0.3 ns |
| single field | 174 ns | 173 ns | 693 ns |
| ten fields | 290 ns | 296 ns | 1,181 ns |

**wirelog vs tracing (active logging):** ~4× faster across the board.

**wirelog static vs dynamic dispatch:** indistinguishable — vtable overhead is
unmeasurable next to the `Mutex` acquisition cost.

---

### Phase 3 — `thread_local!` buffer reuse

Criterion baseline saved as: `phase3-buffer-reuse`

Eliminated the per-event `Vec::with_capacity(256)` allocation by pooling a
reusable buffer per thread. On `Event::new()` the buffer is moved out of TLS via
`mem::take`; `Drop` returns it after the event is flushed or discarded.

| benchmark | before | after | delta |
|---|---|---|---|
| wirelog/disabled_event | 2.0 ns | 2.6 ns | **+30% regression** |
| wirelog/single_field | 174 ns | 166 ns | -5% |
| wirelog/ten_fields | 290 ns | 285 ns | -2% |
| wirelog_dyn/single_field | 173 ns | 171 ns | within noise |
| wirelog_dyn/ten_fields | 296 ns | 294 ns | within noise |

**Disabled event regression (+0.6 ns):** adding a `Drop` impl causes the compiler
to insert drop glue at every drop point. The disabled path exits the `Drop` impl
immediately (`capacity() == 0`), but the call itself is no longer free. In
absolute terms 0.6 ns is negligible; it is noted here because criterion flagged it.

**Active path improvement (~7–8 ns):** real but modest. The `Vec` allocation was
only ~7 ns of the original 174 ns hot path. The `Mutex` acquisition dominates and
cannot be eliminated without a lock-free writer (Phase 5 `NonBlocking<W>`).

**Key finding:** the allocator was not the bottleneck. Further meaningful gains on
the active path require reducing or eliminating the mutex — not the buffer.

---

---

### Phase 5 — `NonBlocking<W>`

Criterion baseline saved as: `phase5-nonblocking`

`NonBlocking<W>` wraps any `Write` sink and drains log lines via a bounded `mpsc`
channel on a dedicated background thread. Log calls on the hot path become a
channel send and never block on the underlying writer.

```
cargo bench --bench non_blocking --features non-blocking
```

| benchmark | blocking (`io::sink()`) | non-blocking (`io::sink()`) | delta |
|---|---|---|---|
| single field | 168 ns | 274 ns | **+106 ns** |
| ten fields | 289 ns | 458 ns | **+169 ns** |

**The non-blocking path is slower per-call in this benchmark — that is expected.**

In a single-threaded benchmark against `io::sink()`, the mutex is uncontested and
nearly free (~20 ns). The non-blocking path pays extra for:

1. `buf.to_vec()` — a heap allocation per log line to move the buffer across the
   channel boundary (~30–50 ns depending on line size)
2. `SyncSender::try_send` — atomic channel bookkeeping (~20–30 ns)

The non-blocking advantage only materialises in scenarios this benchmark does not
exercise:

- **Mutex contention:** with many threads logging concurrently, the blocking path
  serialises completely through a `Mutex`. The non-blocking path degrades gracefully
  — writers contend only on the channel atomics, not on the underlying writer.
- **Slow sinks:** when the underlying writer is a file, network socket, or anything
  with meaningful I/O latency, the blocking path stalls the caller for the duration
  of every write. The non-blocking path returns as soon as the channel send completes.

**Key finding:** `NonBlocking<W>` trades raw single-threaded throughput (one extra
allocation and a channel send) for bounded, predictable caller latency. The
right comparison is not wall-clock ns/call but tail latency under concurrent load —
a benchmark that is not captured here.

**Future optimisation:** the per-call allocation from `buf.to_vec()` is the
dominant new cost. This could be eliminated by passing an `Arc<[u8]>` or a
pre-pooled buffer across the channel instead of an owned `Vec<u8>`, at the cost
of added complexity.

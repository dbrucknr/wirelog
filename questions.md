# Open questions

## Colored output

Should wirelog support a pretty-print console writer with colored output?
This is tracked as an optional `pretty` feature in Phase 6.

---

## `NonBlocking<W>` — keep, redesign, or remove?

Phase 5 implemented `NonBlocking<W>`: a bounded `mpsc` channel + background writer
thread, gated behind `--features non-blocking`. The implementation works correctly,
but several design issues surfaced that make it feel premature for a v0.1.0 release.

### What the benchmarks showed

Single-threaded against `io::sink()`, the non-blocking path is **slower**:

| benchmark | blocking | non-blocking | overhead |
|---|---|---|---|
| single field | 168 ns | 274 ns | +106 ns |
| ten fields | 289 ns | 458 ns | +169 ns |

The overhead comes from `buf.to_vec()` (a heap allocation per log line to cross
the channel boundary) plus `SyncSender::try_send` atomics. In single-threaded use
the mutex is uncontested and nearly free — the non-blocking path adds cost without
removing any.

The benefit only materialises under **mutex contention** (many threads competing to
log concurrently) or **slow sinks** (file I/O, network). Neither is exercised by
the current benchmarks.

### API design gaps

**`dropped()` is inaccessible after construction.** Once `NonBlocking<W>` is moved
into `Logger::new()`, there is no way to call `dropped()` on it. The only way to
observe the counter is to clone the internal `Arc<AtomicU64>` before handing
ownership to the logger — which is exactly what the tests do by reaching into a
private field. The public API has no clean answer to "how many lines were dropped?"

A `WorkerGuard`-style pattern (as in `tracing-appender`) would fix this: the
builder returns both a `NonBlocking<W>` (the sink) and a `WorkerGuard` (holds the
channel handle, exposes `dropped()`, blocks on drop to drain). The user holds the
guard for the lifetime of the program.

**Drop blocks.** `drop(logger)` blocks until the background thread drains and
exits. This is the right behaviour for correctness, but "non-blocking" writers
that block on drop are surprising. The `WorkerGuard` pattern also solves this by
making the blocking shutdown explicit.

**Silent drops are a bad default for a logging library.** A counter that requires
proactive polling is easy to miss. Dropped log lines are precisely the kind of
silent data loss that causes invisible production incidents.

### The async connection

The strongest argument for `NonBlocking<W>` is async code: when wirelog is used
inside a Tokio runtime, acquiring a `Mutex` or blocking on file I/O blocks the
executor thread and starves other tasks. A channel send is non-blocking from the
executor's perspective.

But `NonBlocking<W>` does not fully solve this either — the background thread uses
`std::thread`, which is orthogonal to Tokio's runtime. The correct solution for
async code is `TokioNonBlocking<W>` (see stretch goals in TODO): a `tokio::sync::mpsc`
channel + `tokio::spawn` task + `tokio::io::AsyncWrite`, where the write path is
genuinely non-blocking from Tokio's perspective.

**The synchronous `NonBlocking<W>` may be solving the wrong problem.** In
synchronous code, the mutex overhead is acceptable; in async code, the correct
solution is the Tokio variant. The middle ground — synchronous code with a slow
sink — is real but niche, and probably better served by `BufWriter<W>` (simple
buffering, no background thread, no dropped messages, no surprising shutdown) for
most use cases.

### Options

1. **Remove before v0.1.0.** The API is not ready to publish. Reintroduce with
   a `WorkerGuard` pattern and a benchmark that actually demonstrates the benefit
   (concurrent writers, slow sink). The implementation can be preserved in a branch.

2. **Mark as `#[doc(hidden)]` / unstable.** Keep the code but do not advertise the
   feature in docs or README. Let it mature before making API stability guarantees.

3. **Keep as-is with explicit caveats.** Document the design limitations, the
   benchmark results, and the intended use case clearly. Accept that v0.1.0 ships
   a feature that is experimental.

4. **Redesign before publish.** Implement the `WorkerGuard` pattern, add a
   concurrent benchmark, and ship a well-designed API. This is the most work but
   the cleanest outcome.

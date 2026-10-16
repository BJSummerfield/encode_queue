# Notes: The definitive guide to error handling in Rust

Source article: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

Rules below are derived from the article's code and surrounding prose. Snippet
IDs (S1..S23) refer to `snippets.md` in this directory. Every prose quote is
from the source article (link above); quotes are kept under 100 words.

## Prescriptive rules

### 1. Design only `'static` error types
State errors using only owned fields, or references/trait objects explicitly
annotated `'static`. A reference-typed field (e.g. `&'a str`) makes the error
only `'static` if that particular reference happens to be `'static`, so it is
`QuestionablyStatic` (S2), whereas ownership (`String`) or an explicit
`&'static` makes it always `'static` (S3).
Reasoning: errors are often handled long after the code that produced them
returns, sometimes on a different thread; a non-`'static` error could be
dropped before it is handled ("Good luck handling an error that's been dropped
unexpectedly! Rust protects us from this scenario."). Designing `'static`
errors also enables downcasting, which requires the underlying type to be
`'static` (S6).
Snippets: S2, S3, S6.

### 2. Implementing `Error` is low-cost; keep it light
Any type implementing both `Debug` and `Display` can implement `Error` (S1).
Most of the trait's methods have default implementations, and `Error::cause` /
`Error::description` are deprecated in favor of `Error::source` and the
`Display` implementation, so a concrete error often needs only a one-line
`impl Error for DateError {}` (S12). If a custom error should expose the error
that caused it, provide your own `source()`.
Snippets: S1, S12.

### 3. Only put `Error`-implementing types in `Result::Err`, even when not required
The `Err` of `std::result::Result` (S4) and associated error types such as
`FromStr::Err` (S5) are not bounded by `Error` — any type is allowed. The
article still "strongly encourages" using only `Error` implementations, so
callers are not surprised. The one notable exception it calls out is that
`Box<dyn Error>` itself does *not* implement `Error`, so it must be wrapped in
a newtype that does, or replaced by a helper like `anyhow`.
Snippets: S4, S5.

### 4. Use `Box<dyn Error>` only to tell a human something went wrong
`Box<dyn Error>` is "Rust's vaguest error type" — great for quickly
communicating a failure to a human debugger or end user, but "god-awful at
providing structured data for an error handler to act on". If you expect
consumers to change program behavior based on the error's internals, do not
use it: parsing details out of messages is fragile and makes the message part
of the public API.
Reasoning: it boxes the error because a dynamic trait object has no known size
at compile time.
Snippets: S1 (the trait it erases), S6 (how to recover a concrete type).

### 5. Do not force callers to downcast to discover the failure
Returning a dynamic error communicates that "the internal structure of the
error shouldn't matter to callers". Requiring them to dig into your crate's
error types, downcast, and react is a "leaky implementation detail".
Downcasting (S6) is appropriate when the *same* team both produces and consumes
the error — e.g. Actix's `as_error` lets the user get back the error type they
originally wrapped (S9) — and it is legitimate in tests, where you should be
able to assert the concrete underlying type.
Snippets: S6, S7, S8, S9.

### 6. Codify all possible error states in the public API (use an error enum)
Start by listing every way a caller can lose their mind (S11 shows the
constructor whose error type is the open question `???`), then express them as
an enum of variants carrying the invalid fields (S12 `DateError`). An enum
makes the entire universe of errors obvious from the signature and lets callers
react programmatically with structured data.
Reasoning: dynamic errors and string messages invite Hyrum's Law — "with a
sufficient number of users of an API, it does not matter what you promise in
the contract: all observable behaviors of your system will be depended on by
somebody." The Go `MaxBytesError` shows the consequence: its message text "cannot
be changed" because real programs depend on it (S10).
Snippets: S10, S11, S12.

### 7. Avoid module-scale umbrella errors; keep errors relevant
Do not lump every failure in a module into one giant public error type — a
module with ten differently-failing functions should not define one
`Fn1Error..Fn10Error` enum, or callers must "filter out the noise of nine,
unrelated error cases" (S13 shows this anti-pattern). Instead compose only the
errors relevant to a given operation (S15 `UtcDateTimeError` wraps `DateError`,
`UtcTimestampError`, plus its own `InvalidLeapSecond`), and decide per-function
errors on judgment — S17 shows the bespoke-per-function variant (`InvalidLeapSecondDateError`),
which the article flags as potentially excessive.
Guiding rule: "Design error types that prioritize _relevant_ information.
Minimize unrelated noise."
Snippets: S13, S14, S15, S17.

### 8. Use `thiserror` to remove error boilerplate
`thiserror` replaces the hand-written `Display` and `Error` impls (S18):
`#[error(transparent)]` forwards a wrapped error's message, `#[from]` generates
`From` so the `?` operator transparently converts a `DateError`/
`UtcTimestampError` into `UtcDateTimeError` (S19), and inline
`#[error("...")]` supplies a message for variants that have no `Display` of
their own. It is the structured counterpart to `anyhow`'s dynamic handling,
and both can coexist.
Snippets: S18, S19, S21.

### 9. Return only your own or stdlib error types across crate boundaries
Returning a concrete `anyhow::Error` (or similar) across a crate boundary
forces the caller to depend on that crate and — if it is part of your public
interface — locks you to its major version. The rule of thumb: "Return only
your own or standard library error types across crate boundaries." A public
library like `wgpu` therefore exposes its own structured, thiserror-derived
error types (S21).
Snippets: S21.

### 10. Mark public error enums `non_exhaustive` to reserve the right to grow
Marking an error enum `non_exhaustive` (S21 `CreateComputePipelineError`, S23
`ErrorKind`) means rustc forces matchers to add a catch-all pattern. This lets
you add new error variants later without making it a breaking change — the
article notes that without it, "adding a new error variant would be a breaking
change."
Snippets: S21, S23.

### 11. Represent a single failure mode minimally
When there is exactly one way something can fail, an empty/fieldless struct is
the cleanest error — `tracing`'s `SetGlobalDefaultError` uses a
no-construct `()` field because the global dispatcher can only fail if it is
already set (S20).
Snippets: S20.

### 12. Understand `std::io::Error`'s trade-offs before relying on it
`std::io::Error` packs four representations behind one type (S22): OS codes, a
`Simple` `ErrorKind`, a `SimpleMessage`, and a `Custom` catch-all that pairs an
`ErrorKind` with a `Box<dyn Error + Send + Sync>`. Because `Read`/`Write`
require `std::io::Error`, they must live in `std` (not `core`), so "`no_std`
programs have to reinvent `Read` and `Write` without `std::io::Error`". Its
specialized users are "forced to represent their custom errors dynamically", so
"the more specialized the use case, the more vague `std::io::Error` becomes."
`ErrorKind` (S23) mixes network, filesystem, and process complaints, includes
write-only cases (e.g. `ReadOnlyFilesystem`) shared by read operations, and
treats `Other` as an unstable catch-all — hence the newer, `unstable` and
doc-hidden `Uncategorized` variant.
Snippets: S22, S23.

## Anti-patterns the article warns against

- **Letting error message strings become a de-facto public API.** Callers will
  `if`/`else` on the strings and depend on them (Hyrum's Law); changing the
  message then breaks their code. Shown by Go's `MaxBytesError`, whose message
  "cannot be changed" (S10); the fix is an explicit enum (S12).
- **Using `Box<dyn Error>` when callers need structured data to act on**, or
  when program behavior must depend on the error's internals. No dedicated
  "bad" snippet; the trade-off is illustrated by the trait it erases (S1) and
  the downcast methods it forces you into (S6).
- **Module-scale umbrella error enums.** The `Fn1Error..Fn10Error` single-enum
  design makes every caller filter out unrelated variants (S13).
- **Forcing callers to downcast** to figure out what went wrong, leaking
  implementation details (S6).
- **Bespoke error types per function** — the article calls this potentially
  excessive and leaves it to your judgment (S17).
- **Depending on `std::io::ErrorKind::Other` as a stable contract.** It was
  an unstable catch-all that std itself used, and real code depended on it
  anyway, which is why `Uncategorized` (unstable + doc-hidden) replaced it
  (S23).

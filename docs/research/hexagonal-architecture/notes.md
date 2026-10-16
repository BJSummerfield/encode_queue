# Notes: prescriptive rules from "Master hexagonal architecture in Rust"

Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

These rules are derived from the article's code (see `snippets.md`, S1–S29) and its
surrounding prose. Every rule cites the snippet(s) that demonstrate it. Short prose
quotes (each under 100 words) are attributed to the article link above.

## Rules

### R1. Keep third-party crates out of `main` and out of request handlers
`main` must not configure HTTP middleware, open a database pool, or bind a listener
itself. In the "Very Bad Application", `main` builds axum tracing layers, connects a
Sqlite pool, and hands the concrete pool to `axum::Router` (S1); the handler then
drives a real `sqlx` transaction (S2). The article's reasoning: the problem is not
where setup lives, but the "failure to encapsulate and abstract dependencies"
(article link) — if you change the HTTP server, or a major axum version lands, or
you swap the database client, `main` and every handler must change. The fixed
`main` only constructs adapters, assembles a `Service`, injects it into a first-party
`HttpServer` and runs it (S28), importing nothing but its own crate (S29).
Demonstrated anti-pattern: S1, S2.

### R2. Never let a concrete third-party type flow through the whole application
The bad app passes an imported `SqlitePool` into `AppState` so it is reachable from
every handler (S1). The article: to change the database client "you'd have to rip
out this hard dependency from every corner of your application... This isn't a
leaky abstraction, it's a broken dam." (article link). Fix: depend only on traits —
`AppState` is generic over `AR: AuthorRepository` (S16) and later over
`AS: AuthorService` (S25).

### R3. Put data access behind a repository port
Handlers should ask for "some store of data", not for a specific SQL database; an
HTTP handler "shouldn't even know what a database is", yet the bad one "knows SQL"
(article link). The port is a domain trait (`AuthorRepository`, S5); its SQLite
adapter (S8, S9) owns all SQL (S3) and all vendor error handling (S4). The
article warns: "Hard-wiring a database into your system guarantees pain when you
need to scale." (article link) Mongo, CQRS or event stores are all valid
implementations of the same port.

### R4. Keep SQL, transactions, and database error codes inside the adapter
The bad handler starts/commits transactions inline (S2), writes raw
`sqlx::query!` SQL (S3), and matches the sqlite error code `"2067"` to detect
duplicates (S4). In the hexagonal version, transaction management and error-code
translation live entirely inside `impl AuthorRepository for Sqlite` (S9); it
translates the vendor "duplicate" error into the domain's `Duplicate` variant, and
wraps anything the domain doesn't know about in the catch-all error (S9). The
domain is explicitly "oblivious" to transactions, retries and caches (article link).

### R5. Define domain models with validating constructors; validation belongs to the domain
`AuthorName` trims and rejects empty names; `CreateAuthorRequest` can only be built
from a valid `AuthorName`, so an invalid request simply cannot be passed to the
repository (S6). The article: "Validations should be defined by the domain, not
request handlers." (article link). Anti-pattern: the bad handler does inline
validation of the JSON body before touching the database (S2).

### R6. Don't add Serde derives to validated domain models ("Don't be abSerde")
S6 deliberately has no `serde::Serialize`/`Deserialize` on `Author`, `AuthorName`
or `CreateAuthorError`. The article warns that Serde annotations on domain models
only make sense if the model performs no field validation *and* you want port
implementors to serialize models directly; otherwise "you give your adapters the
power to create invalid domain models from raw data, since they can use Serde to
bypass your constructors" (article link). Serde derives instead appear on the
transport-side types (S18, S20).

### R7. Separate the request model from the persistent entity model
`CreateAuthorRequest` (what's needed to create) is a different type from `Author`
(the saved entity) (S6). The rejected alternative is one struct with
`id: Option<Uuid>` (S7). Reasoning: "we'd be mistaken in assuming that the data required to create an
`Author` and the representation of an existing `Author` will never diverge. This
is not at all true of real applications." (article link) Keeping them apart means
domain changes don't force API changes and vice versa — "Hexagonal architecture
is about building for change." (article link).

### R8. Domain error types must completely describe the expected failures, with a catch-all for the rest
`CreateAuthorError::Duplicate` covers the business-rule failure; the adapter adds
an `Unknown`-style variant wrapping `anyhow::Error` for anything the domain can't
classify (S6, S9). A caller of the port gets "a complete description of every
error scenario it's expected to handle, and the compiler will make sure that it
does" (article link), so handling is exhaustive at compile time. The article advises *not*
marking application-internal error enums `non_exhaustive` (you *want* new variants
to break the build so they get handled), reserving `non_exhaustive` for libraries
(article link).

### R9. Wrap third-party library types and expose only what the application needs
`Sqlite` wraps `sqlx::SqlitePool` (S8); axum is hidden behind a first-party
`HttpServer`/`HttpServerConfig` (S28), so `main`'s imports contain only the
application's own modules (S29). The article: "Wrap external libraries and expose
only the functionality your application requires." (article link). Note that
constructor code *outside* the trait may return `anyhow::Result` freely (S8), and
should add context to opaque errors (S8).

### R10. Write async domain traits with explicit `impl Future + Send` and the full supertrait stack
The naive `async fn` trait method (S10) compiles but its future isn't `Send`, so
it can't be used from a multi-threaded web server. The article walks the
compulsions one by one: make the future `Send` (S11), bound the trait `Send` (S12),
then `Send + Sync` because `Arc` requires both (S13), then `+ 'static` because the
repo is shared across threads and must not be dropped early (S14), and finally
`Clone` because axum requires injected state to be `Clone` (S15). Final shape:
`pub trait AuthorRepository: Clone + Send + Sync + 'static` with
`-> impl Future<...> + Send` (S15).

### R11. Share application state via `Arc` behind a generic, trait-based `AppState`
`AppState<AR: AuthorRepository>` holds `Arc<AR>` (S16); once a `Service` exists it
becomes `AppState<AS: AuthorService>` (S25). Handlers then depend only on the port,
which is what makes mock injection possible (R12) and keeps implementation details
out of the handler (R1–R2).

### R12. Unit test handlers by injecting mock repositories
Because handlers depend only on `AuthorRepository`, a `MockAuthorRepository`
(S21) can stand in for a real database, letting you test the handler's mapping of
repository results to HTTP responses (S22) — impossible with the hard-wired
version (R1). The article notes the `Clone` bound forces the mock to share state
through `Arc<Mutex<Result<...>>>` since `anyhow::Error` is not `Clone` (S21), and
recommends the Mockall crate to avoid hand-crafted mocks (article link).

### R13. Translate between transport types and domain types at the edges
The HTTP body type converts into the domain request at the handler boundary
(`try_into_domain`, S18); the domain result converts into the public response shape
via `From<&Author> for CreateAuthorResponseData` (S20). Reasoning: "These two
things can now change independently... Changes to transport concerns or business
logic no longer spread through your program like wildfire." (article link)

### R14. Keep public (transport) errors separate from domain errors
`ApiError` maps `CreateAuthorError::Duplicate` to a hand-built
`422 Unprocessable Entity` message and the unknown variant to a generic
`500 Internal Server Error`, logging the real cause and backtrace internally
(S19). The article: "Always separate your public errors from their domain
representations." (article link) — returning raw domain errors "is an easy way to
leak private details of your application" and causes unexpected response changes
when internals change (article link).

### R15. Don't panic in response to unexpected errors; propagate a catch-all domain error
The `Sqlite` adapter wraps unexpected sqlx errors in the domain's catch-all variant
rather than panicking (S9). Two reasons from the article: "1. Panicking poisons
held mutexes... 2. Other Rust devs won't expect you to panic." (article link)
Return errors, follow established error-handling conventions.

### R16. Put business logic (orchestrating multiple dependencies) in a `Service`, not in handlers
A `Service` port declares the business API (S26); its canonical implementation
composes `AuthorRepository` + `AuthorMetrics` + `AuthorNotifier` and encodes the
conditional flow: persist, then record success/failure metrics, then notify (S27).
Reasoning: if that logic lived in the handler, every combination of dependency
success/failure would need *integration* testing — "The `Service` trait keeps your
unit test surface area sane." (article link) Handlers just call the service
(S17, S25). The article also warns against making the side dependencies
fallible when they needn't be: infallible metric/notification ports keep the test
matrix small (S24, S27).

### R17. `main` is only for bootstrapping
`main` brings the application online and cleans up: it picks the adapters
(SQLite, Prometheus, email client), combines them via the domain's `Service`
constructor, injects them into the HTTP server and runs it (S28). The article:
"The less code you put in `main`, the smaller your testing dead zone." (article
link) — `main` composes unmockable dependencies and exits on error, so keep it
small; a first-party `HttpServer` type also lets integration tests reuse the same
bootstrap (S28, S29).

### R18. Draw domain boundaries around real business capabilities, and start with one large domain
A domain is "some tangible arm of your business" (article link), and "a domain
should include all entities that must change together as part of a single, atomic
operation" (article link). Cross-domain operations are never atomic: if you find
yourself leaking transactions into business logic to make a cross-domain operation
atomic, the boundaries are wrong (article link). Start with a single, large domain
and decompose as observed friction demands — "Battle-testing your app reveals
entities and relations you can't predict." (article link) Domains communicate
through each other's `Service` ports (S26, S27), which is what later makes
extracting a stable domain into a microservice a simple matter (article link).

## Anti-patterns the article warns against

| Anti-pattern | Where shown |
|---|---|
| `main` configuring middleware and wiring concrete pools into the router | S1 (R1) |
| Handlers knowing SQL and orchestrating DB transactions | S2, S3 (R4) |
| Database vendor error codes leaking into handler logic | S4 (R4) |
| Inline request-body validation in the handler instead of the domain | S2 (R5) |
| One entity/request struct with `Option<Uuid>` | S7 (R7) |
| Serde derives bypassing validating constructors on domain models | S6 (R6) |
| Naive `async fn` domain trait without `Send` futures | S10 (R10) |
| Returning full-fat domain error strings to end users | S19 (R14) |
| Panicking on errors the domain can't handle | S9 (R15) |
| Hard-wiring one data store, guaranteeing pain at scale | S1, S2 (R3) |
| Business orchestration logic embedded in handlers (combinatorial test explosion) | S2 contrasted with S26/S27 (R16) |
| Launching an MVP as microservices with guessed domain boundaries | prose only — the `Service` port (S26, S27) is the stable seam the article says to extract instead |

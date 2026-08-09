# Conventions

This document defines conventions for writing robust, maintainable applications. The rules are language-agnostic; Rust examples are used for concreteness.

**Core philosophy:** Validate at the boundary, carry only valid data inward, and let the domain define every interface. If a type exists in your system, it should be impossible for it to be invalid.

## Type Conventions

| Rule | Rationale |
|------|-----------|
| Every distinct concept gets its own type | Prevents argument-swap bugs; encodes invariants at compile time |
| Constructors are fallible and are the only way to create a type | Guarantees every instance is valid; validation lives in one place |
| Derive standard traits: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash` | Free ergonomics; enables use in collections, comparisons, logging |
| Prefer `#[derive_more]` or equivalent to reduce boilerplate | Less surface area to maintain; fewer places for bugs |
| Never expose the wrapped field as `pub` | Forces all construction through the validated constructor |
| Use `unsafe` only for bypass constructors, named `_unchecked` | Signals to reviewers: invariant is the caller's responsibility |

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EmailAddress(String);

impl EmailAddress {
    pub fn new(raw: &str) -> Result<Self, InvalidEmailError> {
        // validate...
        Ok(Self(raw.to_lowercase()))
    }

    pub unsafe fn new_unchecked(raw: &str) -> Self {
        Self(raw.to_string())
    }
}
```

## Boundary Crossing Conventions

| Rule | Rationale |
|------|-----------|
| All external input is untrusted until parsed into a domain type | Raw data can be malformed, malicious, or out of range |
| Use `TryFrom` (or language equivalent) for fallible conversions | Standard, discoverable, composable with `?` operator |
| Convert at the edge, never carry raw data past the boundary | Invalid data cannot leak into business logic |
| Adapters convert their native types → domain types before calling domain logic | Domain stays ignorant of adapter internals |
| Domain types never depend on adapter types | Reversing the dependency inverts the architecture |
| One converter per boundary crossing | Each conversion is independently testable and traceable |

```rust
// At the boundary: adapter → domain
impl TryFrom<sqlx::types::Text> for AuthorName {
    type Error = InvalidAuthorNameError;
    fn try_from(value: sqlx::types::Text) -> Result<Self, Self::Error> {
        AuthorName::new(&value)
    }
}

// At the boundary: HTTP input → domain
impl TryFrom<CreateAuthorHttpRequestBody> for CreateAuthorRequest {
    type Error = RequestValidationError;
    fn try_from(body: CreateAuthorHttpRequestBody) -> Result<Self, Self::Error> {
        Ok(Self {
            name: AuthorName::try_from(body.name)?,
        })
    }
}
```

## Error Conventions

| Rule | Rationale |
|------|-----------|
| Codify every failure mode as an enum variant | Caller knows the complete error space from the signature |
| Never leak implementation errors across boundaries | Database errors, HTTP codes, etc. are adapter concerns |
| Use a catch-all variant only for truly unknown failures | `Unknown(anyhow::Error)` for things the domain can't classify |
| Use `thiserror` (or equivalent) to eliminate boilerplate | `#[from]`, `#[transparent]`, display strings as annotations |
| Domain errors carry structured data, not just messages | Callers can react programmatically, not parse strings |
| Validation errors belong on the type that failed validation, not the operation | `EmailAddress::new` returns `InvalidEmailError`, not `CreateUserError` |

```rust
#[derive(thiserror::Error, Debug)]
pub enum CreateAuthorError {
    #[error("author already exists: {0}")]
    AlreadyExists(AuthorId),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

// Adapter translates its errors at the boundary
impl AuthorRepository for Sqlite {
    fn create_author(&self, req: &CreateAuthorRequest) -> Result<Author, CreateAuthorError> {
        // ...
        .map_err(|e| match e.code() {
            SQLITE_UNIQUE_CONSTRAINT => CreateAuthorError::AlreadyExists(id),
            _ => CreateAuthorError::Unknown(e.into()),
        })
    }
}
```

## Architecture Conventions

| Rule | Rationale |
|------|-----------|
| Domain defines interfaces; adapters implement them | Business logic is portable, testable, and independent of infrastructure |
| `main` does only bootstrapping | Wiring and configuration are boring; behavior is interesting |
| Prefer enum dispatch over dynamic dispatch | You know your type space; exhaustiveness checking beats runtime casts |
| Organize by feature (vertical slices), not by layer | Colocates related code; changing a feature touches one directory |
| Wrap third-party types in your own types | Prevents external APIs from leaking through your codebase |
| Dependencies point inward: adapters → domain → nothing | Domain has zero external dependencies |

**Vertical slices layout:**
```
src/
  main.rs              # bootstrapping only
  domain/
    author.rs          # Author, AuthorName, CreateAuthorRequest, CreateAuthorError
    command.rs         # QueueCommand, CommandText, etc.
  adapters/
    file_queue.rs      # FileQueue: implements QueueRepository
    editor.rs          # EditorAdapter: implements EditorRepository
  features/
    add_command.rs     # uses domain types + adapters
    list_commands.rs   # uses domain types + adapters
    process_queue.rs   # uses domain types + adapters
```

## Testing Conventions

| Rule | Rationale |
|------|-----------|
| Test constructors exhaustively | If the constructor is the only way in, it must be bulletproof |
| Test each adapter independently from the domain | Domain tests use mocks; adapter tests use real I/O |
| Test boundary conversions in both directions | `TryFrom` forward and back; nothing silently drops data |
| Prefer unit tests over integration tests | Unit tests are fast, exhaustive, and isolate failure modes |
| Integration tests only for cross-adapter coordination | Verify the wiring works, not the individual pieces |
| Test error paths as often as success paths | Failure cases outnumber success cases in production |

```rust
#[test]
fn email_rejects_missing_at() {
    assert!(EmailAddress::new("invalid").is_err());
}

#[test]
fn email_lowercases_on_construction() {
    let email = EmailAddress::new("User@Example.COM").unwrap();
    assert_eq!(email.as_str(), "user@example.com");
}
```

## Code Review Checklist

### Types
- [ ] Every distinct concept has its own type (no bare `String`, `&str`, `i32` for domain concepts)
- [ ] Wrapped fields are private; construction goes through a fallible constructor
- [ ] Standard traits derived: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`

### Boundaries
- [ ] All external input is parsed into domain types before entering business logic
- [ ] Conversions use `TryFrom`/`From`, not ad-hoc functions
- [ ] No raw or adapter types leak past their boundary

### Errors
- [ ] Errors are enums with codified variants (no `Box<dyn Error>` in public APIs)
- [ ] Implementation errors are translated at the boundary, never propagated raw
- [ ] Validation errors live on the type being validated, not the operation

### Architecture
- [ ] Domain has zero external dependencies
- [ ] `main` contains only bootstrapping (no business logic)
- [ ] Code is organized by feature (vertical slices), not by layer
- [ ] Enum dispatch used instead of `dyn` where the type space is known

### Tests
- [ ] Constructors tested exhaustively (valid, invalid, edge cases)
- [ ] Error paths tested as often as success paths
- [ ] Domain tests use mocks, not real adapters
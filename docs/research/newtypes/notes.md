# Notes: prescriptive rules and anti-patterns from "The Ultimate Guide to Rust Newtypes"

Article: <https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes>

These rules are derived from the article's code (see `snippets.md`, blocks `S1`–`S31`) and the
prose that surrounds them. Snippet IDs (`S#`) refer to `snippets.md`. Where the article quotes
prose directly, it is kept short (under 100 words) and linked to the article per its license.

---

## Prescriptive rules

### R1. Give distinct concepts distinct types (newtype wrappers) to make argument-swap errors impossible

**Statement:** Wrap each domain concept (e.g. an email address, a password) in its own
tuple-struct newtype, and have functions accept the newtypes rather than raw `&str`s. The compiler
then rejects passing one where the other is expected.

**Why:** With two `&str` parameters the caller *can* swap them (a password as the email, etc.), and
logging the plaintext password "will bring great shame upon your family". Distinct types remove
this whole class of bug. ([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S5` (typed `create_user(email: EmailAddress, password: Password)`). Contrast the
problem it fixes in `S2`.

---

### R2. "Parse, don't validate": validate in the constructor; business logic accepts only valid values

**Statement:** Pull validation out of business functions and into a `new` constructor that returns a
`Result`. Once a value has passed `new`, any code that receives the newtype can trust it and skip
re-checking.

**Why:** The article frames this as the practice of "investing extra time upfront to design
datatypes that are always valid". A value that exists is a valid value, so `create_user` becomes
"all business logic". This is the "Parse, don't validate" idea popularized by Alexis King's post
(referenced in the article). ([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S8` (`EmailAddress::new` validates; `create_user` no longer calls `validate_email`), and
the resulting focused tests in `S9`. Contrast the clutter it removes in `S2` and the shrunk error
type vs `S3`.

---

### R3. Keep the wrapped field private; construct only through the validating constructor

**Statement:** Do not make the inner field `pub`. The only way to obtain an instance should be the
constructor that enforces validity.

**Why:** If the field is `pub`, "there's absolutely nothing to stop you" from building an instance
with the wrong underlying value — e.g. wrapping a raw password as an `EmailAddress` and a raw email
as a `Password`. ([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S5` and `S8` show the private `String` field; the anti-pattern this rule prevents is
shown in `S6`/`S7` (see A2).

---

### R4. Preserve invariants in every mutating method of a mutable newtype (which makes other ops infallible)

**Statement:** When a newtype has `&mut self` methods, each one must keep the type's invariant true.
As a payoff, operations the invariant guarantees become total/infallible.

**Why:** `NonEmptyVec` must always hold at least one element, so `pop` refuses to pop the last item;
consequently `last` needs no `Option`/`unwrap` of an empty vec.

**Snippet:** `S10` (`NonEmptyVec::pop` guards the last element; `NonEmptyVec::last` is infallible).

---

### R5. Derive the standard traits whenever they make sense — especially in libraries

**Statement:** `derive` the usual traits (`Debug`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`,
`Hash`, etc.) for any type where they are meaningful, even before you see a use.

**Why:** Deriving them is a good habit, and for a library it is *required* to be kind to users: due
to the Orphan Rule, your users can't add these traits themselves and would otherwise have to wrap
your newtype again. ("Kiss those GitHub stars goodbye.") ([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S11` (`EmailAddress` derives the standard set). Note the article also says *not* to
derive `Default` (no sensible "default email") or `Copy` (because `String` isn't `Copy`).

---

### R6. Implement `Display` manually; you get `ToString` for free

**Statement:** There is no `derive` for `Display`, so write the `fmt` impl by hand. Implementing
`Display` gives you a free `std::string::ToString`.

**Why:** It makes the newtype print like its underlying value, and unlocks the blanket `ToString`.

**Snippet:** `S12` (manual `impl Display for EmailAddress`).

---

### R7. Hand-write `Eq`/`Ord`/`PartialOrd` when the underlying type lacks a total order, and keep them consistent

**Statement:** If the wrapped type can't be `Eq`/`Ord` (e.g. `f64`, because `NAN`), but the *restricted*
range of the newtype can, derive `PartialEq` but implement `Ord` and `PartialOrd` yourself. `Eq` is
just a marker on top of `PartialEq`; `PartialOrd::partial_cmp` should simply wrap `Ord::cmp`.

**Why:** Deriving `PartialOrd` would delegate to the underlying `f64`'s (inconsistent) `partial_cmp`
and disagree with a manual `Ord`. Clippy's `derive_ord_xor_partial_ord` will flag a mismatch, so you
should define both yourself and make `partial_cmp` call `cmp`.

**Snippet:** `S13` (`Subsecond`, `f64` wrapper over `0.0..1.0`) and `S14` (manual `Eq`, `PartialOrd`,
`Ord`; `partial_cmp` returns `Some(self.cmp(other))`, `cmp` falls back to `unreachable!()`).

---

### R8. Use `From` for infallible conversions and `TryFrom` for fallible ones

**Statement:** Choose `From` when the conversion can't fail (and get `Into` for free via the std
blanket impl); choose `TryFrom` when it can.

**Why:** These are the classic, unsurprising patterns for moving between your newtypes and
"Other People's Code" that only knows primitives like `&str`/`f64`.

**Snippet:** `S15` (`From<i32>` → `Into` for free) and `S16` (`TryFrom<f64>` for `Subsecond`).

---

### R9. Make the canonical constructor the single source of truth; don't duplicate it

**Statement:** Implement conversion traits (`TryFrom`, etc.) as a thin call to your one canonical
constructor. Avoid competing constructors for the same purpose (e.g. both a `new(raw: f64)` and a
`try_from(raw: f64)` with independent logic).

**Why:** "A newtype's constructor serves as its source of truth – never have multiple constructors
for the same use case." Two competing implementations "double[] the code you need to maintain and
test for no benefit." ([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S16` (`TryFrom::try_from` just delegates to `Subsecond::new`).

---

### R10. Provide user-friendly getters (and, for libraries, the third-party conversion traits your users need)

**Statement:** Add getters with conventional `to_x`/`into_x`/`as_x` names to return the inner value,
and — if you're a library author — implement the conversion traits your audience almost certainly
wants.

**Why:** Consumers (e.g. a DB client) want a `&str`/`String`, not an `EmailAddress`. For a
numeric-heavy library the article cites implementing `num::ToPrimitive` from the `num` crate as a
reasonable convenience.

**Snippet:** `S17` (`EmailAddress::into_string`).

---

### R11. Use `AsRef<T>` to expose a reference to the wrapped type

**Statement:** Implement `AsRef<T>` to hand out a reference to the inner value when library code
expects the primitive type.

**Why:** "Library code you interface with will expect `&str`, not `&EmailAddress`." `AsRef` is the
light-weight, reference-returning way to bridge that.

**Snippet:** `S18` (`impl AsRef<str> for EmailAddress`).

---

### R12. On generic `Deref` wrappers, prefer associated functions to inherent methods

**Statement:** For a newtype that `Deref`s to a user-supplied type generically, expose functionality
as associated functions (no `self`) rather than inherent methods.

**Why:** An inherent method that shares a name with a method on the deref'd target shadows it, and
the newtype's method wins — so a user relying on `Deref` "is out of luck". Associated functions can't
intercept method calls. This advice is attributed by the article to *Rust for Rustaceans*.

**Snippet:** `S21`/`S22` (`SmartBox` uses an associated `print_best_rust_resource` so it doesn't clash
with `ConfusedUnitStruct`'s inherent method of the same name; both calls work in `S22`).

---

### R13. If you implement `Borrow<T>`, make the newtype hash/compare identically to `T` (normalize in the constructor)

**Statement:** `Borrow<T>` claims the newtype and `T` are interchangeable; honor that by making
`Hash`/`PartialEq`/`Ord` agree exactly with the wrapped type. Do the normalization (e.g. lowercasing)
in the constructor so manual `PartialEq`/`Hash` aren't needed.

**Why:** `HashMap` assumes a `Borrow<T>` key hashes and compares *identically* to `T`; if they differ,
lookups with the borrowed type silently miss. Fixing it in the constructor makes the newtype
"equivalent to `&str` for the sake of hashing, equality and ordering, so it's safe to implement
`Borrow`." ([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S26` (`new` lowercases in the constructor). Contrast the failing arrangement in `S24`/`S25`
(see A5).

---

### R14. Provide an `unsafe …_unchecked` constructor to bypass costly revalidation when validity is already known

**Statement:** For the cases where you already know a value is valid (e.g. pulled from a validated
database), offer an `unsafe` constructor that skips re-validation, following std's convention of
suffixing it with `_unchecked`.

**Why:** `unsafe` is "a signal to other developers that some invariant exists that is not enforced by
the compiler". std models this with e.g. `String::from_utf8_unchecked`; the article applies the same
pattern to `EmailAddress::new_unchecked` for values coming out of the database.

**Snippet:** `S27` (std's `from_utf8_unchecked`) and `S28` (`EmailAddress::new` +
`unsafe fn new_unchecked`).

---

### R15. Cut boilerplate deliberately: start with `derive_more`, treat `nutype` as a commitment

**Statement:** After you can write newtypes by hand, use `derive_more` to `derive` common trait impls
(`From`, `AsRef`, `Deref`, …) with minimal magic. Only reach for `nutype` — which generates
sanitizers, `regex` validation, error types, and `new_unchecked` — when you accept its strong
footprint.

**Why:** "Before introducing magic from external crates, you should understand what you're
automating and why." `nutype` "is a commitment"; its error types pervade the codebase and are hard to
migrate away from. ([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S29` (`derive_more`) and `S30` (`nutype`).

---

## Anti-patterns the article warns against

### A1. Same-typed positional arguments + validation buried in business logic (and the error/test sprawl it causes)

**What:** Accepting two `&str`s (`email`, `password`) and validating inside the business function,
which breeds a big multi-variant error enum and a combinatorial test matrix.

**Why:** Swap-prone arguments, log exposure of secrets, cluttered business logic, and — if validations
become parallel or partially independent — testing *permutations* of failures.

**Snippet:** `S2` (swap-prone `create_user`), `S3` (the sprawling `CreateUserError` enum),
`S4` (the many test cases it forces).

### A2. Making the wrapped field `pub`

**What:** Declaring `pub struct EmailAddress(pub String)` so instances can be built with arbitrary
values.

**Why:** It "unleashes … evil" — you can now construct `EmailAddress(raw_password)` and
`Password(raw_email_address)` and call `create_user` with swapped data.

**Snippet:** `S6` (the `pub` field) and `S7` (the resulting swapped construction).

### A3. Writing (or copy-pasting) your own email-validation regex

**What:** Hand-rolling or Stack-Overflow-pasting an email regex.

**Why:** "Never write your own email validation regex. Don't copy-paste regexes you find on Stack
Overflow. Use a well maintained library … and proceed on the assumption that they've also got it
wrong." Valid email addresses are surprising (the article shows a multi-relay example from an RFC).
([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S8` is where the (library-supplied) `email_regex()` call appears; the rule is to source
that regex from a well-maintained library, not to invent it.

### A4. Adding an unnecessary `Deref` impl

**What:** Implementing `Deref` just to make the newtype "feel like" its underlying type.

**Why:** "Deref is a powerful trait that you should approach like you would disarming a very small
bomb." It bolts every `&self` method of the wrapped type onto your newtype, vastly expanding the
public surface (e.g. an `EmailAddress` that can never be empty suddenly exposes `str::is_empty`), and
it interacts badly with generic wrappers (see R12). It's "a judgement call".
([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S19` (the `Deref` impl) and `S20` (the seductive coercion it enables); the clash hazard it
can cause is the motivation for `S21`/`S22`.

### A5. `Borrow` that disagrees with the wrapped type's `Hash`/`Eq`

**What:** Implementing `Borrow<str>` on a newtype whose `PartialEq`/`Hash` are case-insensitive, so the
newtype and `&str` don't hash/compare identically.

**Why:** `HashMap` trusts `Borrow` keys to match `T`; when they don't, a lookup with `&str` returns
`None` even though the entry was just inserted. The article calls such impls "unofficially unsafe"
and says to "scrutinize any `Borrow` implementation you see in code review."
([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S24` (case-insensitive `PartialEq`/`Hash` + `Borrow<str>`) and `S25` (the failing
assertion output). Fix shown in `S26`.

### A6. Shadowing `to_string` with an inherent method

**What:** Defining your own `to_string` on a type that already implements `Display`.

**Why:** `Display` already provides `ToString`; shadowing it is "bad news that Clippy considers an
error" (`inherent_to_string_shadow_display`). So the article deliberately does *not* define
`EmailAddress::to_string`. ([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S17` marks the spot (`:coderef[18]`) where a `to_string` would have gone but was omitted.

### A7. Competing `new` / `try_from` constructors for the same purpose

**What:** Maintaining both a `new(raw)` and a `try_from(raw)` that each carry their own validation
logic.

**Why:** It "doubles the code you need to maintain and test for no benefit." Define conversion traits
in terms of the one canonical constructor instead (see R9).

**Snippet:** `S16` shows the preferred shape (`TryFrom` delegating to `new`), i.e. what you should
*not* replace with a duplicate.

### A8. Blindly adopting `nutype` (and tolerating its vague errors)

**What:** Reaching for `nutype` without weighing the cost, or shipping its auto-generated error
messages as-is.

**Why:** `nutype` is a hard-to-remove dependency, and "its generated error messages are quite vague,
and there's no way to override them or include additional detail", which "hamper[]s debugging". The
article advises thinking hard before adopting it and wrapping its errors with more context.
([article](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes))

**Snippet:** `S30` (the `nutype` macro) and `S31` (its terse `EmailAddress violated the regular
expression.` message).

---

## Scope note

This article covers newtype wrappers, constructors/validation, trait implementations
(`Display`, `Eq`/`Ord`/`PartialOrd`, `From`/`TryFrom`, `AsRef`, `Deref`, `Borrow`), `unsafe
_unchecked` constructors, and boilerplate-reducing crates (`derive_more`, `nutype`). It explicitly
defers *marker types / provenance* to a future article, so no rules here concern that topic. The
"Orphan Rule" is explained in the article but is background, not a newtype rule (see `S1` for the
`impl Display` workaround it enables).

# Notes: Writing production Rust macros with `macro_rules!`

Article: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules
Source markdown: see `SOURCE.md` in this directory. Snippet IDs (S1..S27) refer to
`snippets.md`. Prose quotes below are attributed to the article link above (license terms).

The article walks through three production `macro_rules!` macros in increasing order of
complexity — the stdlib's `assert_eq!` (S1–S6), axum's `IntoResponse` tuple machinery
(S7–S17), and Tokio's `join!` (S18–S27) — and derives its guidance from that code.

## Rules

1. **Write one rule per call form the macro should support.**
   `assert_eq!` supports a plain two-argument call and a call with a format string plus
   args, and each call form gets its own dedicated pattern (S2); the plain rule even
   absorbs an optional trailing comma with `$(,)?`.
   Reasoning: keeping each rule minimal makes the macro's behavior predictable — one
   pattern explains one caller convention (S1 shows both caller forms).

2. **Evaluate captured fragments once by binding them to locals before use.**
   Both `assert_eq!` rules start from `match (&$left, &$right) { (left_val, right_val) => ... }`,
   so `$left` and `$right` are evaluated a single time (S3).
   Reasoning: the fragments may be expensive expressions, not just literals; the article
   notes the primitives in the docs example hide that "$left and $right could be
   expensive function calls". Macro hygiene also makes the local-binding pattern safe:
   the macro-local variables the rules introduce (e.g. `assert_eq!`'s `kind`) "won't
   conflict with any variable named `kind` where `assert_eq!` is called – a feature
   known as macro hygiene", so a macro's locals can never collide with caller variables
   of the same name (prose explanation attached to the S3 pattern; the article link
   above).

3. **Reference every dependency from an exported macro through `$crate::` (fully
   qualified) or import it inside the rule body.**
   The expansions use `$crate::panicking::assert_failed`, `$crate::option::Option::{None,
   Some}`, `$crate::format_args!` (S4), and `join!`'s body does `use $crate::macros::support::...` (S26).
   Reasoning: with `#[macro_export]` there are no guarantees about what is in scope at the
   call site; users shouldn't be forced to import implementation details. The article's own
   summary: "Use a fully qualified path to declarative macro dependencies, or import them
   in the body of the macro." (https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules)

4. **Hide the plumbing from the caller; keep the macro's public surface simple.**
   `assert_eq!` conceals the `Option<fmt::Arguments>` that `assert_failed` requires — the
   no-args rule passes `None`, the formatted rule wraps `format_args!` in `Some` (S4, S5, S6).
   Reasoning: it "saves us from writing our own call to `PartialEq::eq`, and constructing
   optional format arguments manually", and hides `None` boilerplate from every call site.

5. **Reduce every rule's expansion to a handful of fundamental calls.**
   `assert_eq!` collapses to one `assert_failed` call (S4, S5); each generated
   `IntoResponse` impl is a reducer over `into_response`/`into_response_parts` (S10–S12);
   `join!` collapses to one `poll_fn` loop over a tuple of futures (S26).
   Reasoning: "Reducing complexity to handful of fundamental function calls is a hallmark
   of a well-written macro."
   (https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules)

6. **Generate repeated boilerplate with a helper macro that enumerates the cases,
   instead of writing them out by hand.**
   `all_the_tuples_no_last_special_case!(impl_into_response);` (S16) expands to 16
   invocations of `impl_into_response!` with arities 1..=16 (S17), producing 4 impls per
   arity (S10).
   Reasoning: Rust has no variadic parameters, so "axum _appears_ to implement
   `IntoResponse` for tuples of arbitrary length" by statically implementing a bounded set
   of arities — and the enumeration lives in exactly one place.

7. **Pass macro names around as plain tokens; a macro's identifier is just an `ident`
   fragment.**
   `all_the_tuples_no_last_special_case!` takes `$name:ident` and re-invokes it with
   growing argument lists (S17); the call site hands it `impl_into_response` (S16).
   Reasoning: "Macros operate on tokens, and the name of a macro is just another token!"

8. **Use unique `ident` placeholders per invocation (and `ident`, not `ty`, when the
   concrete type is only known at the call site).**
   `T1..=T16` are arbitrary but "must be unique within each `impl_into_response!`
   invocation, or we'd force the tuple items with matching idents to have the same type";
   each placeholder becomes an independent generic parameter (S17).
   Reasoning: the generator doesn't know the concrete types in advance — `ident` fragments
   stay unresolved until the caller's code instantiates them (S9).

9. **Mark internal (private) rules with a leading `@` token, not with a separate exported
   macro.**
   `join!`'s normalize and final rules are only reachable through the `@{ ... }` token
   structure that its own expansions produce (S19–S20, S25).
   Reasoning: no sensible user writes inputs shaped like `@{ ... }`, so the rule is
   effectively private; this avoids exporting a helper macro and making it "an explicit
   part of Tokio's public API". The article adds that `@` "is a generally accepted
   community standard" for this (and is not special-cased by the compiler).

10. **Delimit the accumulator structure with `{ }` so a recursive rule can tell
    processed tokens from unprocessed ones.**
    The `@{ ($($s)*) ($($n)*) $($t)* }` grammar puts normalized data inside the braces and
    leaves unprocessed expressions outside, which the normalize rule consumes one at a time
    (S20); the traced expansions show the boundary moving as each item is absorbed
    (S21–S24).
    Reasoning: "The curly braces give the normalize rule a way to distinguish processed
    from unprocessed inputs as it recurses."

11. **Normalize variadic input into a structured intermediate form with a recursive
    `tt`-muncher, and keep a public entry-point rule separate.**
    The entry-point rule `( $($e:expr),+ $(,)?)` re-invokes the macro with `@{ () (0) }`
    and the raw expressions (S19); each normalize pass peels off one expression, prefixes
    it with its index-underscore group, and appends it to the accumulator (S20); when no
    unprocessed expressions remain, the final rule matches (S25).
    Reasoning: `macro_rules!` has no loops, so structural recursion over the token stream
    is the mechanism ("incremental `tt` muncher"), and it mirrors the same reductive
    pattern the smaller macros use.

12. **Accumulate counts as token lists and evaluate them once, at the end.**
    The normalizer appends the literal tokens `+ 1` to `$n` each pass (S20) and one `_` to
    `$s` per branch (S25); the sum is only evaluated as `const COUNT: u32 = $($total)*;`
    in the final expansion (S26).
    Reasoning: `macro_rules!` cannot do arithmetic on captured fragments, so the only
    "computation" available during recursion is token concatenation; the compiler evaluates
    the `0+1+1+...` expression to a numeric constant at the end.

13. **Select a tuple element by position with underscore-skip patterns, not by numeric
    indexing.**
    `let ( $($skip,)* fut, .. ) = &mut *futures;` skips exactly as many elements as there
    are underscores in `$skip`, zero underscores for the first branch, one for the second,
    etc. (S26, S27).
    Reasoning: underscore is the pattern for "match anything", so N underscores + a
    binding picks out the (N+1)th element; this works for heterogeneous tuples, which an
    array-based approach cannot (see anti-pattern 4).

14. **Give the empty-input case its own explicit rule.**
    `join!()` with no arguments expands to `async {}.await` via a dedicated rule (S18);
    the article treats it as a first-class, intentional branch, not an accident.
    Reasoning: edge cases that a variadic matcher (`$($e:expr),+`) cannot cover still need
    defined, sensible behavior.

## Anti-patterns the article warns against

1. **Treating macros as "magic".** The article opens by insisting readers stop
   "describing macros as 'magic'", because the label "entrenches the superstition that
   macros are too arcane" and creates "a false distinction between 'Rust' and 'Rust with
   macros'".
   Demonstrated by the whole piece: every case study (S2, S10, S20) shows ordinary token
   pattern matching doing the work.

2. **Writing exported macro expansions with unqualified paths that depend on the caller's
   scope.** Using bare `panicking::assert_failed` or relying on the user to import an
   implementation detail breaks whenever the call site lacks the name.
   Demonstrated (as the good form) by S4, where everything goes through `$crate::`.

3. **Evaluating a captured expression fragment more than once.** Substituting `$left`
   (or `$right`) twice into the expansion would re-run possibly expensive input code.
   Demonstrated (as the good form) by S3, which binds `left_val`/`right_val` up front.

4. **Storing per-branch data in a homogeneous array when outputs differ.** "Unlike
   tuples, arrays must be homogeneous", so an array of futures would "force the `Output`
   type of all `Future`s to be the same"; `join!`'s tuple manipulation "avoids this
   limitation entirely".
   Demonstrated by S26 (tuple + skip patterns instead of array indexing).

5. **Reusing the same ident name for multiple items of a generated tuple.** Matching
   items with the same ident placeholder would force those items to share one type.
   Demonstrated (as the good form) by S17's distinct `T1..=T16`.

6. **Recursive token counting for large collections.** "Macros are limited to 128
   recursions by default, so this approach is unsuitable for counting large collections."
   Demonstrated by S20/S26, the recursive counting the warning applies to.

7. **Capturing what are conceptually types as `ty` fragments when they must stay
   uncommitted.** A `ty` fragment would force concrete types at generation time; `ident`
   placeholders let each item keep its own type until the caller's code instantiates them.
   Demonstrated (as the good form) by S17, using `T1..=T16` as generic parameters.

## Recommended practices (learning advice)

1. **Read standard-library code regularly.** The article's "Light reading" aside asks
   "Do you regularly read standard library code? I highly recommend it", reasoning that
   "a strong understanding of stdlib internals makes debugging almost _any_ Rust
   easier". Supported by S1–S6, the stdlib `assert_eq!` case study the article opens
   with.

2. **Study the original upstream source of the code you are adapting, not just the
   article's excerpt.** When walking through `join!`, the article asks: "please do study
   [the original] when we're done – it's informative on the broader topics of async,
   safety and Tokio internals" (link to the Tokio `join.rs` source). Supported by
   S18–S27.

3. **Write your own macros, and study code outside your comfort zone.** The article
   closes: "The best way to continue dispelling the aura that surrounds `macro_rules!`
   is to write your own, and supplement by studying code that takes you outside your
   comfort zone." No snippet — conclusion prose (article link above).

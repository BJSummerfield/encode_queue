# Snippets extracted from writing-production-rust-macros-with-macro-rules.md

Total code blocks (n): 27

## S1

- Nearest preceding heading: ## A standard Rust macro: `assert_eq!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
let a = 3;
let b = 1 + 2;
assert_eq!(a, b);

assert_eq!(a, b, "we are testing addition with {} and {}", a, b);
```

## S2

- Nearest preceding heading: ## A standard Rust macro: `assert_eq!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
#[macro_export]
macro_rules! assert_eq {
	($left:expr, $right:expr $(,)?) => { ... }; :coderef[1]
	($left:expr, $right:expr, $($arg:tt)+) => { ... }; :coderef[2]
}
```

## S3

- Nearest preceding heading: ## A standard Rust macro: `assert_eq!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) {
            (left_val, right_val) => {  :coderef[3]
                if !(*left_val == *right_val) {  :coderef[4]
                    let kind = $crate::panicking::AssertKind::Eq;  :coderef[5]
					// ...
                }
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = $crate::panicking::AssertKind::Eq;
                    // ...
                }
            }
        }
    };
}
```

## S4

- Nearest preceding heading: ## A standard Rust macro: `assert_eq!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = $crate::panicking::AssertKind::Eq;
					$crate::panicking::assert_failed(
						kind,
						&*left_val,
						&*right_val,
						$crate::option::Option::None,
					);
                }
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = $crate::panicking::AssertKind::Eq;
					$crate::panicking::assert_failed(
						kind,
						&*left_val,
						&*right_val,
						$crate::option::Option::Some(
							$crate::format_args!($($arg)+)
						),
					);  :coderef[6]
                }
            }
        }
    };
}
```

## S5

- Nearest preceding heading: ## A standard Rust macro: `assert_eq!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
pub fn assert_failed<T, U>(
    kind: AssertKind,  :coderef[7]
    left: &T,
    right: &U,
    args: Option<fmt::Arguments<'_>>,  :coderef[8]
) -> !
where
    T: fmt::Debug + ?Sized,  :coderef[9]
    U: fmt::Debug + ?Sized,
{
    assert_failed_inner(kind, &left, &right, args)
}
```

## S6

- Nearest preceding heading: ## A standard Rust macro: `assert_eq!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
macro_rules! format_args {
    ($fmt:expr) => {{ /* compiler built-in */ }};
    ($fmt:expr, $($args:tt)*) => {{ /* compiler built-in */ }};
}
```

## S7

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
pub trait IntoResponse {
    /// Create a response.
    fn into_response(self) -> Response;
}
```

## S8

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
(a).into_response()
(a, b).into_response()
(a, b, c).into_response()
(a, b, c, d).into_response()
...
```

## S9

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
macro_rules! impl_into_response {
	( $($ty:ident),* $(,)? ) => { ... } :coderef[10]
}
```

## S10

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
macro_rules! impl_into_response {
	( $($ty:ident),* $(,)? ) => {
		impl<R, $($ty,)*> IntoResponse for ($($ty),*, R)  :coderef[11]
		where
		    $( $ty: IntoResponseParts, )*
		    R: IntoResponse,
		{ ... }

		impl<R, $($ty,)*> IntoResponse for (StatusCode, $($ty),*, R)  :coderef[12]
		where
		    $( $ty: IntoResponseParts, )*
		    R: IntoResponse,
		{ ... }

		impl<R, $($ty,)*> IntoResponse for (http::response::Parts, $($ty),*, R)  :coderef[13]
		where
		    $( $ty: IntoResponseParts, )*
		    R: IntoResponse,
		{ ... }

		impl<R, $($ty,)*> IntoResponse for (http::response::Response<()>, $($ty),*, R)  :coderef[14]
		where
		    $( $ty: IntoResponseParts, )*
		    R: IntoResponse,
		{ ... }
	}
}
```

## S11

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
impl<R, $($ty,)*> IntoResponse for ($($ty),*, R)
where
    $( $ty: IntoResponseParts, )*
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let ($($ty),*, res) = self;  :coderef[15]

        let res = res.into_response();
        let parts = ResponseParts { res };  :coderef[16]

        $(
            let parts = match $ty.into_response_parts(parts) {
                Ok(parts) => parts,
                Err(err) => {
                    return err.into_response(); :coderef[17]
                }
            };
        )*  :coderef[18]

        parts.res  :coderef[19]
    }
}
```

## S12

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
impl<R, $($ty,)*> IntoResponse for (StatusCode, $($ty),*, R)
where
    $( $ty: IntoResponseParts, )*
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let (status, $($ty),*, res) = self;

        let res = res.into_response();
        let parts = ResponseParts { res };

        $(
            let parts = match $ty.into_response_parts(parts) {
                Ok(parts) => parts,
                Err(err) => {
                    return err.into_response();
                }
            };
        )*

        (status, parts.res).into_response()
    }
}
```

## S13

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
impl<R> IntoResponse for (StatusCode, R)
where
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let mut res = self.1.into_response();
        *res.status_mut() = self.0;
        res
    }
}
```

## S14

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
impl<R> IntoResponse for (http::response::Parts, R)
where
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let (parts, res) = self;
        (parts.status, parts.headers, parts.extensions, res).into_response()  :coderef[20]
    }
}
```

## S15

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
impl<R, $($ty,)*> IntoResponse for (http::response::Response<()>, $($ty),*, R)
where
    $( $ty: IntoResponseParts, )*
    R: IntoResponse,
{
    fn into_response(self) -> Response {
        let (template, $($ty),*, res) = self;
        let (parts, ()) = template.into_parts();
        (parts, $($ty),*, res).into_response()
    }
}
```

## S16

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust crate=axum path=axum-core/src/response/into_response.rs
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust crate=axum path=axum-core/src/response/into_response.rs
all_the_tuples_no_last_special_case!(impl_into_response);
```

## S17

- Nearest preceding heading: ## Write dynamic Rust with intermediate `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
macro_rules! all_the_tuples_no_last_special_case {
    ($name:ident) => {
        $name!(T1);
        $name!(T1, T2);
        $name!(T1, T2, T3);
        $name!(T1, T2, T3, T4);
        $name!(T1, T2, T3, T4, T5);
        $name!(T1, T2, T3, T4, T5, T6);
        $name!(T1, T2, T3, T4, T5, T6, T7);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
    };
}
```

## S18

- Nearest preceding heading: ## Advanced Rust `macro_rules!` for normalization, counting and concurrency
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
#[macro_export]
macro_rules! join {
	() => { async {}.await }
}
```

## S19

- Nearest preceding heading: ## Advanced Rust `macro_rules!` for normalization, counting and concurrency
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
#[macro_export]
macro_rules! join {
	// ===== Entry point =====

	( $($e:expr),+ $(,)?) => {  :coderef[21]
		$crate::join!(@{ () (0) } $($e,)*)  :coderef[22]
	};

	() => { async {}.await }
}
```

## S20

- Nearest preceding heading: ## Advanced Rust `macro_rules!` for normalization, counting and concurrency
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
#[macro_export]
macro_rules! join {
	// ===== Normalize =====

	(@ { ( $($s:tt)* ) ( $($n:tt)* ) $($t:tt)* } $e:expr, $($r:tt)* ) => { :coderef[23]
        $crate::join!(@{ ($($s)* _) ($($n)* + 1) $($t)* ($($s)*) $e, } $($r)*) :coderef[a]
    };

	// ===== Entry point =====

	( $($e:expr),+ $(,)?) => {
		$crate::join!(@{ () (0) } $($e,)*)
	};

	() => { async {}.await }
}
```

## S21

- Nearest preceding heading: ## Advanced Rust `macro_rules!` for normalization, counting and concurrency
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
let (first, second) = tokio::join!(
	do_stuff_async(),
	more_async_work(),
);
```

## S22

- Nearest preceding heading: ## Advanced Rust `macro_rules!` for normalization, counting and concurrency
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
join!(@{ () (0) }, do_stuff_async(), more_async_work(),)
```

## S23

- Nearest preceding heading: ## Advanced Rust `macro_rules!` for normalization, counting and concurrency
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
join!(@{ (_) (0+1) () do_stuff_async(), } more_async_work(),)
```

## S24

- Nearest preceding heading: ## Advanced Rust `macro_rules!` for normalization, counting and concurrency
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
join!(@{ (__) (0+1+1) () do_stuff_async(), (_) more_async_work(), })
```

## S25

- Nearest preceding heading: ## Advanced Rust `macro_rules!` for normalization, counting and concurrency
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
(@ {
    // One `_` for each branch in the `join!` macro. This is not used once
    // normalization is complete.
	( $($count:tt)* )  :coderef[24]

    // The expression `0+1+1+ ... +1` equal to the number of branches.
    ( $($total:tt)* )  :coderef[25]

    // Normalized join! branches
    $( ( $($skip:tt)* ) $e:expr, )*  :coderef[26]

}) => {...}
```

## S26

- Nearest preceding heading: ### Counting with `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
use $crate::macros::support::{maybe_done, poll_fn, Future, Pin};
use $crate::macros::support::Poll::{Ready, Pending};

let mut futures = ( $( maybe_done($e), )* );  :coderef[27]
let mut futures = &mut futures;
let mut skip_next_time: u32 = 0;

poll_fn(move |cx| {  :coderef[28]
    const COUNT: u32 = $($total)*;  :coderef[29]

    let mut is_pending = false;

    let mut to_run = COUNT;

    let mut skip = skip_next_time;

    skip_next_time = if skip + 1 == COUNT { 0 } else { skip + 1 };

	loop {
	    $(  :coderef[30]
	        if skip == 0 {
	            if to_run == 0 {
	                break;
	            }
	            to_run -= 1;

	            // Extract the future for this branch from the tuple.
	            let ( $($skip,)* fut, .. ) = &mut *futures;  :coderef[31]

				let mut fut = unsafe { Pin::new_unchecked(fut) };

	            if fut.poll(cx).is_pending() {
	                is_pending = true;
	            }
	        } else {
	            skip -= 1;
	        }
	    )*
    }

    if is_pending {
        Pending
    } else {
        Ready(($({
            // Extract the future for this branch from the tuple.
            let ( $($skip,)* fut, .. ) = &mut futures;  :coderef[32]

			let mut fut = unsafe { Pin::new_unchecked(fut) };

            fut.take_output().expect("expected completed future")
        },)*))
    }
}).await
```

## S27

- Nearest preceding heading: ### Counting with `macro_rules!`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/writing-production-rust-macros-with-macro-rules

```rust
let ( $($skip,)* fut, .. ) = &mut *futures;
```

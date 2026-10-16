# Snippets: the-definitive-guide-to-rust-error-handling

n = 23

Source article: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

---

## S1

Heading: ### What is an error in Rust?
Fence info string: `rust path=src/core/error.rs`
Raw lines: 50-58
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust path=src/core/error.rs
pub trait Error: Debug + Display {
    // Provided methods
    fn source(&self) -> Option<&(dyn Error + 'static)> { ... } :coderef[1]
    fn description(&self) -> &str { ... }
    fn cause(&self) -> Option<&dyn Error> { ... }
    fn provide<'a>(&'a self, request: &mut Request<'a>) { ... }
}
```

---

## S2

Heading: ### What is an error in Rust?
Fence info string: `rust`
Raw lines: 112-116
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
pub struct QuestionablyStatic<'a> {
	field: &'a str,
}
```

---

## S3

Heading: ### What is an error in Rust?
Fence info string: `rust`
Raw lines: 120-128
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
pub struct StaticByOwnership {
	field: String
}

pub struct ExplicitlyStatic {
	field: &'static str
}
```

---

## S4

Heading: ### Errors in the context of `Result`
Fence info string: `rust`
Raw lines: 134-139
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

---

## S5

Heading: ### Errors in the context of `Result`
Fence info string: `rust`
Raw lines: 145-151
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
pub trait FromStr: Sized {
    type Err; :coderef[2]

    fn from_str(s: &str) -> Result<Self, Self::Err>;
}
```

---

## S6

Heading: ### Downcasting errors in Rust
Fence info string: `rust`
Raw lines: 233-237
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
pub fn downcast<T: Error + 'static>(self: Box<Self>) -> Result<Box<T>, Box<Self>>
pub fn downcast_mut<T: Error + 'static>(&mut self) -> Option<&mut T>
pub fn downcast_ref<T: Error + 'static>(&self) -> Option<&T>
```

---

## S7

Heading: ### Downcasting errors in Rust
Fence info string: `rust crate=actix-web path=src/error/error.rs`
Raw lines: 267-271
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust crate=actix-web path=src/error/error.rs
pub struct Error {
    cause: Box<dyn ResponseError>,
}
```

---

## S8

Heading: ### Downcasting errors in Rust
Fence info string: `rust crate=actix-web path=src/error/response_error.rs`
Raw lines: 275-280
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust crate=actix-web path=src/error/response_error.rs
pub trait ResponseError: fmt::Debug + fmt::Display {
	fn status_code(&self) -> StatusCode
	fn error_response(&self) -> HttpResponse<BoxBody>
}
```

---

## S9

Heading: ### Downcasting errors in Rust
Fence info string: `rust crate=actix-web path=src/error/error.rs`
Raw lines: 294-301
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust crate=actix-web path=src/error/error.rs

impl Error {
	pub fn as_error<T: ResponseError + 'static>(&self) -> Option<&T> {
	    <dyn ResponseError>::downcast_ref(self.cause.as_ref())
	}
}
```

---

## S10

Heading: ### Sane APIs support programmatic error handling
Fence info string: `go crate=go path=src/net/http/request.go`
Raw lines: 381-391
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```go crate=go path=src/net/http/request.go
// MaxBytesError is returned by [MaxBytesReader] when its read limit is exceeded.
type MaxBytesError struct {
	Limit int64
}

func (e *MaxBytesError) Error() string {
	// Due to Hyrum's law, this text cannot be changed. :coderef[3]
	return "http: request body too large"
}
```

---

## S11

Heading: ### Build expressive Rust errors with enums
Fence info string: `rust`
Raw lines: 415-428
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

impl Date {
	pub fn new(year: i32, month: u8, day: u8) -> Result<Self, ???> { :coderef[4]
		todo!()
	}
}
```

---

## S12

Heading: ### Build expressive Rust errors with enums
Fence info string: `rust`
Raw lines: 442-464
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateError {
    InvalidMonth(u8),
    InvalidDay { month: u8, day: u8 },
    NonLeapYear(i32),
}

impl Display for DateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use DateError::*;
        match self {
            InvalidMonth(month) => write!(f, "{} is not a valid month", month),
            InvalidDay{ month, day } => {
                write!(f, "{} is not a valid day for month {}", day, month)
            },
            NonLeapYear(year) => write!(f, "{} is not a leap year", year),
        }
    }
}

impl Error for DateError {}
```

---

## S13

Heading: ### Composing structured error types
Fence info string: `rust`
Raw lines: 490-502
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
pub enum Error {
	Fn1Error,
	Fn2Error,
	// ...,
	Fn10Error,
}

pub fn fn1() -> Result<(), Error> {}
pub fn fn2() -> Result<(), Error> {}
// ...
pub fn fn10() -> Result<(), Error> {}
```

---

## S14

Heading: ### Composing structured error types
Fence info string: `rust`
Raw lines: 511-559
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct UtcTimestamp {
    hour: u8,
    minute: u8,
    second: u8,
}

impl UtcTimestamp {
    pub fn new(hour: u8, minute: u8, second: u8)
    -> Result<UtcTimestamp, UtcTimestampError> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtcTimestampError {
    InvalidHour(u8),
    InvalidMinute(u8),
    InvalidSecond(u8),
    InvalidLeapSecond { hour: u8, minute: u8, second: u8 },
}

impl Display for UtcTimestampError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use UtcTimestampError::*;
        match self {
            InvalidHour(hour) => write!(f, "{} is not a valid hour", hour),
            InvalidMinute(minute) => {
                write!(f, "{} is not a valid minute", minute)
            }
            InvalidSecond(second) => {
                write!(f, "{} is not a valid second", second)
            }
            InvalidLeapSecond {
                hour,
                minute,
                second,
            } => write!(
                f,
                "{}:{}:{} is not a valid leap second",
                hour, minute, second
            ),
        }
    }
}

impl Error for UtcTimestampError {}
```

---

## S15

Heading: ### Composing structured error types
Fence info string: `rust`
Raw lines: 575-610
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct UtcDateTime {
    date: Date,
    time: UtcTimestamp,
}

impl UtcDateTime {
    fn new(year: i32, month: u8, day: u8, hour: u8, minute: u8)
    -> Result<Self, UtcDateTimeError> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtcDateTimeError {
    Date(DateError),
    Time(UtcTimestampError),
    InvalidLeapSecond(Date),
}

impl Display for UtcDateTimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use UtcDateTimeError::*;
        match self {
            Date(err) => write!(f, "invalid date: {}", err),
            Time(err) => write!(f, "invalid time: {}", err),
            InvalidLeapSecond(date) => {
                write!(f, "no leap second occurs on {}", date)
            },
        }
    }
}

impl Error for UtcDateTimeError {}
```

---

## S16

Heading: ### Composing structured error types
Fence info string: `rust`
Raw lines: 618-625
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
impl UtcDateTime {
    fn from_date_and_time(date: Date, time: UtcTimestamp)
    -> Result<Self, ???> {
        todo!()
    }
}
```

---

## S17

Heading: ### Composing structured error types
Fence info string: `rust`
Raw lines: 631-655
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidLeapSecondDateError(Date);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtcDateTimeError {
    Date(DateError),
    Time(UtcTimestampError),
    InvalidLeapSecond(InvalidLeapSecondDateError),
}

// Error implementations omitted

impl UtcDateTime {
    fn new(year: i32, month: u8, day: u8, hour: u8, minute: u8)
    -> Result<Self, UtcDateTimeError> {
        todo!()
    }

    fn from_date_and_time(date: Date, time: UtcTimestamp)
    -> Result<Self, InvalidLeapSecondDateError> {
        todo!()
    }
}
```

---

## S18

Heading: ### How to improve the ergonomics of your Rust errors
Fence info string: `rust`
Raw lines: 677-687
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)] :coderef[5]
pub enum UtcDateTimeError {
    #[error(transparent)] :coderef[6]
    Date(#[from] DateError), :coderef[7]
    #[error(transparent)]
    Time(#[from] UtcTimestampError),
    #[error("no leap second occurs on {0}")] :coderef[8]
    InvalidLeapSecond(Date),
}
```

---

## S19

Heading: ### How to improve the ergonomics of your Rust errors
Fence info string: `rust`
Raw lines: 697-701
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust
fn some_utc_datetime_func() -> Result<(), UtcDateTimeError> {
    Err(DateError::InvalidMonth(13))?
}
```

---

## S20

Heading: ### Structured error handling examples from the Rust ecosystem
Fence info string: `rust crate=tracing path=tracing-core/src/dispatch.rs`
Raw lines: 717-744
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust crate=tracing path=tracing-core/src/dispatch.rs
/// Returned if setting the global dispatcher fails.
pub struct SetGlobalDefaultError { :coderef[9]
    _no_construct: (),
}

impl SetGlobalDefaultError {
    const MESSAGE: &'static str = "a global default trace dispatcher has already been set";
}

impl fmt::Debug for SetGlobalDefaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SetGlobalDefaultError")
            .field(&Self::MESSAGE)
            .finish()
    }
}

impl fmt::Display for SetGlobalDefaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(Self::MESSAGE)
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl error::Error for SetGlobalDefaultError {}
```

---

## S21

Heading: ### Structured error handling examples from the Rust ecosystem
Fence info string: `rust crate=wgpu path=wgpu-core/src/pipeline.rs`
Raw lines: 752-771
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust crate=wgpu path=wgpu-core/src/pipeline.rs
#[derive(Clone, Debug, Error)]
#[non_exhaustive] :coderef[10]
pub enum CreateComputePipelineError {
    #[error(transparent)]
    Device(#[from] DeviceError),
    #[error("Unable to derive an implicit layout")]
    Implicit(#[from] ImplicitLayoutError), :coderef[11]
    #[error("Error matching shader requirements against the pipeline")]
    Stage(#[from] validation::StageError),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Pipeline constant error: {0}")]
    PipelineConstants(String), :coderef[12]
    #[error(transparent)]
    MissingDownlevelFlags(#[from] MissingDownlevelFlags),
    #[error(transparent)]
    InvalidResource(#[from] InvalidResourceError),
}
```

---

## S22

Heading: ### `std::io::Error`, Rust's most challenging error type
Fence info string: `rust crate=rust path=library/std/src/io/error.rs|repr_unpacked.rs`
Raw lines: 795-815
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust crate=rust path=library/std/src/io/error.rs|repr_unpacked.rs
pub struct Error {
    repr: Repr,
}

struct Repr(Inner);

type Inner = ErrorData<Box<Custom>>;

struct Custom {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
}

enum ErrorData<C> { :coderef[13]
    Os(RawOsError),
    Simple(ErrorKind),
    SimpleMessage(&'static SimpleMessage),
    Custom(C),
}
```

---

## S23

Heading: ### `std::io::Error`, Rust's most challenging error type
Fence info string: `rust crate=rust path=library/std/src/io/error.rs`
Raw lines: 828-861
Source: https://www.howtocodeit.com/guides/the-definitive-guide-to-rust-error-handling

```rust crate=rust path=library/std/src/io/error.rs
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ErrorKind {
	// ...

    #[stable(feature = "rust1", since = "1.0.0")]
    ConnectionRefused, :coderef[14]
    #[stable(feature = "rust1", since = "1.0.0")]
    ConnectionReset,

    // ...

    #[unstable(feature = "io_error_more", issue = "86442")]
    FilesystemQuotaExceeded, :coderef[15]
    #[stable(feature = "io_error_a_bit_more", since = "1.83.0")]
    FileTooLarge,

	// ...

    #[stable(feature = "io_error_a_bit_more", since = "1.83.0")]
    ArgumentListTooLong, :coderef[16]
    #[stable(feature = "rust1", since = "1.0.0")]
    Interrupted,

	// ...

    #[stable(feature = "rust1", since = "1.0.0")]
    Other, :coderef[17]
    #[unstable(feature = "io_error_uncategorized", issue = "none")] :coderef[18]
    #[doc(hidden)] :coderef[19]
    Uncategorized,
}
```

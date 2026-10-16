# Snippets extracted from the-ultimate-guide-to-rust-newtypes.md

Total code blocks (n): 31


## S1

- Nearest preceding heading: ## What are newtype wrappers in Rust?
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
struct Wrapper(Vec<String>);

impl fmt::Display for Wrapper {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "[{}]", self.0.join(", ")) }
	}
}
```

## S2

- Nearest preceding heading: ## Why newtype-driven design will change your life
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
pub fn create_user(email: &str, password: &str) -> Result<User, CreateUserError> { :coderef[1]
    validate_email(email)?;
    validate_password(password)?; :coderef[2]
    let password_hash = hash_password(password)?;
    // Save user to database
    // Trigger welcome emails :coderef[3]
    // ...
    Ok(User)
}
```

## S3

- Nearest preceding heading: ## Why newtype-driven design will change your life
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
#[derive(Error, Clone, Debug, PartialEq)] :coderef[4]
pub enum CreateUserError {
    #[error("invalid email address: {0}")]
    InvalidEmail(String),
    #[error("invalid password: {reason}")]
    InvalidPassword {
        reason: String,
    },
    #[error("failed to hash password: {0}")]
    PasswordHashError(#[from] BcryptError),
    #[error("user with email address {email} already exists")]
    UserAlreadyExists {
        email: String,
    },
    // ...
}
```

## S4

- Nearest preceding heading: ## Why newtype-driven design will change your life
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user_invalid_email() {
        let email = "invalid-email";
        let password = "password";
        let result = create_user(email, password);
        let expected = Err(CreateUserError::InvalidEmail(email.to_string()));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_create_user_invalid_password() { unimplemented!() }

    #[test]
    fn test_create_user_password_hash_error() { unimplemented!() }

    #[test]
    fn test_create_user_user_already_exists() { unimplemented!() }

    #[test]
    fn test_create_user_success() { unimplemented!() }
}
```

## S5

- Nearest preceding heading: ## Newtype essentials
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct EmailAddress(String);

#[derive(Debug, Clone, PartialEq)]
pub struct Password(String); :coderef[5]

pub fn create_user(email: EmailAddress, password: Password) -> Result<User, CreateUserError> { :coderef[6]
    validate_email(&email)?;
    validate_password(&password)?;
    let password_hash = hash_password(&password)?;
    // ...
    Ok(User)
}
```

## S6

- Nearest preceding heading: ## Newtype essentials
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
pub struct EmailAddress(pub String)
```

## S7

- Nearest preceding heading: ## Newtype essentials
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
let email_address = EmailAddress(raw_password);
let password = Password(raw_email_address);
create_user(email_address, password)?;
```

## S8

- Nearest preceding heading: ### Constructors as the source of truth
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct EmailAddress(String);

#[derive(Error, Debug, Clone, PartialEq)]
#[error("{0} is not a valid email address")]
pub struct EmailAddressError(String); :coderef[7]

impl EmailAddress {
    pub fn new(raw_email: &str) -> Result<Self, EmailAddressError> {
        if email_regex().is_match(raw_email) { :coderef[8]
            Ok(Self(raw_email.into()))
        } else {
            Err(EmailAddressError(raw_email))
        }
    }
}

#[derive(Error, Clone, Debug, PartialEq)]
#[error("a user with email address {0} already exists")]
pub struct UserAlreadyExistsError(EmailAddress); :coderef[9]

fn create_user(email: EmailAddress, password: Password) -> Result<User, UserAlreadyExistsError> {
    // Save user to database
    // Trigger welcome emails
    // ... :coderef[10]
    Ok(User)
}
```

## S9

- Nearest preceding heading: ### Constructors as the source of truth
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
#[cfg(test)]
mod email_address_tests {
    use super::*;

    #[test]
    fn test_new_invalid_email() {
        let raw_email = "invalid-email";
        let result = EmailAddress::new(raw_email);
        let expected = Err(EmailAddressError(raw_email.to_string()));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_new_success() { unimplemented!() }
}

#[cfg(test)]
mod create_user_tests {
    use super::*;

    #[test]
    fn test_user_already_exists() { unimplemented!() }

    #[test]
    fn test_success() { unimplemented!() }
}
```

## S10

- Nearest preceding heading: ### Newtype mutability
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
struct NonEmptyVec<T>(Vec<T>);

impl<T> NonEmptyVec<T> {
    fn pop(&mut self) -> Option<T> {
        if self.0.len() == 1 { :coderef[11]
            None
        } else {
            self.0.pop()
        }
    }

    fn last(&self) -> &T {
        self.0.last().unwrap() :coderef[12]
    }
}
```

## S11

- Nearest preceding heading: ### `derive` standard traits
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmailAddress(String);
```

## S12

- Nearest preceding heading: ### `derive` standard traits
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
impl Display for EmailAddress {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

## S13

- Nearest preceding heading: ### Manually implement special cases
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
#[derive(Error, Debug, Copy, Clone, PartialEq)]
#[error("subsecond value must be in the range 0.0..1.0, but was {0}")]
pub struct SubsecondError(f64);

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Default)]
pub struct Subsecond(f64);

impl Subsecond {
    pub fn new(raw: f64) -> Result<Self, SubsecondError> {
        if !(0.0..1.0).contains(&raw) {
            Err(SubsecondError(raw))
        } else {
            Ok(Self(raw))
        }
    }
}
```

## S14

- Nearest preceding heading: ### Manually implement special cases
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
#[derive(Debug, Copy, Clone, PartialEq, Default)] :coderef[13]
pub struct Subsecond(f64);

impl Eq for Subsecond {}

impl PartialOrd for Subsecond { :coderef[14]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Subsecond {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.partial_cmp(&other.0) {
            Some(ordering) => ordering,
            None => unreachable!(), :coderef[15]
        }
    }
}
```

## S15

- Nearest preceding heading: ### Infallible conversions
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
struct WrappedI32(i32);

impl From<i32> for WrappedI32 {
    fn from(raw: i32) -> Self {
        Self(raw)
    }
}

fn demonstrate() {
    let _ = WrappedI32::from(84);
    let _: WrappedI32 = 84.into(); :coderef[16]
}
```

## S16

- Nearest preceding heading: ### Fallible conversions
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
impl TryFrom<f64> for Subsecond {
    type Error = SubsecondError;

    fn try_from(raw: f64) -> Result<Self, Self::Error> {
        Subsecond::new(raw) :coderef[17]
    }
}

fn demonstrate() {
    let _ = Subsecond::try_from(0.5);
    let _: Result<Subsecond, _> = 0.5.try_into();
}
```

## S17

- Nearest preceding heading: ### Writing user-friendly getters
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
impl EmailAddress {
    pub fn into_string(self) -> String {
        self.0
    }

	:coderef[18]

}
```

## S18

- Nearest preceding heading: ### `AsRef`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
impl AsRef<str> for EmailAddress {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
```

## S19

- Nearest preceding heading: ### `Deref`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
impl Deref for EmailAddress {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
```

## S20

- Nearest preceding heading: ### `Deref`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
fn demonstrate() {
    let raw = "EMAIL@ADDRESS.COM";
    let email = EmailAddress::new(raw).unwrap();
    takes_a_str(&email); :coderef[19]

    let lower = email.to_lowercase(); :coderef[20]
    assert_eq!(lower, "email@address.com");

	assert_eq!(*raw, *email); :coderef[21]
}

fn takes_a_str(_: &str) {}
```

## S21

- Nearest preceding heading: ### `Deref`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
/// A transparent wrapper around any type `T`.
struct SmartBox<T>(T);

impl<T> Deref for SmartBox<T> {
    type Target = T; :coderef[22]

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> SmartBox<T> {
    fn print_best_rust_resource() { :coderef[23]
        println!("howtocodeit.com");
    }
}

struct ConfusedUnitStruct;

impl ConfusedUnitStruct {
    fn print_best_rust_resource(&self) { :coderef[24]
        println!("Scrabbling around on YouTube or something?");
    }
}

fn demonstrate() {
    let smart_box = SmartBox(ConfusedUnitStruct);
    smart_box.print_best_rust_resource();
    SmartBox::<ConfusedUnitStruct>::print_best_rust_resource(); :coderef[25]
}
```

## S22

- Nearest preceding heading: ### `Deref`
- Fence info string: text
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```text
Scrabbling around on YouTube or something?
howtocodeit.com
```

## S23

- Nearest preceding heading: ### `Borrow`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
impl Borrow<str> for EmailAddress {
    fn borrow(&self) -> &str {
        &self.0
    }
}
```

## S24

- Nearest preceding heading: ### `Borrow`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
impl PartialEq for EmailAddress {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0) :coderef[26]
    }
}

impl Hash for EmailAddress {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_ascii_lowercase().hash(state); :coderef[27]
    }
}

impl Borrow<str> for EmailAddress {
    fn borrow(&self) -> &str {
        &self.0 :coderef[28]
    }
}

fn no_no_no_no_no() {
    let mut login_attempts: HashMap<EmailAddress, i32> = HashMap::new(); :coderef[29]

    let raw_uppercase = "EMAIL@TEST.COM";
    let raw_lowercase = "email@test.com";

    let email = EmailAddress::new(raw_uppercase).unwrap(); :coderef[30]

    login_attempts.insert(email, 2); :coderef[31]

    let got_attempts = login_attempts.get(raw_lowercase); :coderef[32]
    assert_eq!(got_attempts, Some(&2));
}
```

## S25

- Nearest preceding heading: ### `Borrow`
- Fence info string: text
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```text
assertion `left == right` failed
  left: None
  right: Some(2)
```

## S26

- Nearest preceding heading: ### `Borrow`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
impl EmailAddress {
    pub fn new(raw_email: &str) -> Result<Self, EmailAddressError> {
        if email_regex().is_match(raw_email) {
            let lower = raw_email.to_ascii_lowercase();
            Ok(Self(lower))
        } else {
            Err(EmailAddressError(raw_email.to_string()))
        }
    }
}
```

## S27

- Nearest preceding heading: ## Bypassing newtype validations
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
pub unsafe fn from_utf8_unchecked(bytes: Vec<u8>) -> String
```

## S28

- Nearest preceding heading: ## Bypassing newtype validations
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
impl EmailAddress {
    pub fn new(raw_email: &str) -> Result<Self, EmailAddressError> {
        if email_regex().is_match(raw_email) {
            let lower = raw_email.to_ascii_lowercase();
            Ok(Self(lower))
        } else {
            Err(EmailAddressError(raw_email.to_string()))
        }
    }

    pub unsafe fn new_unchecked(raw_email: &str) -> Self {
        Self(raw_email.to_string())
    }

    // ...
}
```

## S29

- Nearest preceding heading: ### Using `derive_more` to... derive more
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, AsRef, Deref)]
struct EmailAddress(String);
```

## S30

- Nearest preceding heading: ### Going all-in with `nutype`
- Fence info string: rust
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```rust
#[nutype(
    sanitize(trim, lowercase),
    validate(regex = EMAIL_REGEX),
	derive(
		Clone, Debug, Display,
		PartialEq, Eq, PartialOrd,
		Ord, AsRef, Deref,
		Borrow, TryFrom
	),
	new_unchecked,
)]
pub struct EmailAddress(String);

pub fn demonstrate() {
    let result = EmailAddress::new("not an email address");
    if let Err(e) = result {
        match e {
            EmailAddressError::RegexViolated => println!("{}", e),
        }
    }
}
```

## S31

- Nearest preceding heading: ### Going all-in with `nutype`
- Fence info string: text
- Source: https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes

```text
EmailAddress violated the regular expression.
```

(end)

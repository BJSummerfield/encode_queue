# Code Snippets: Master hexagonal architecture in Rust

n = 29 fenced code blocks, extracted verbatim in file order.

Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

## S1

Nearest preceding heading: ### What problems does hexagonal architecture solve?
Info string: rust path=src/bin/server/main.rs
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust path=src/bin/server/main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;

    // A minimal tracing middleware for request logging.
    tracing_subscriber::fmt::init();
    let trace_layer = tower_http::trace::TraceLayer::new_for_http().make_span_with(
        |request: &axum::extract::Request<_>| {
            let uri = request.uri().to_string();
            tracing::info_span!("http_request", method = ?request.method(), uri)
        },  :coderef[1]
    );

    let sqlite = SqlitePool::connect_with(    :coderef[2]
        SqliteConnectOptions::from_str(&config.database_url)
            .with_context(|| format!("invalid database path {}", &config.database_url))?
            .pragma("foreign_keys", "ON"),
    )
    .await
    .with_context(|| format!("failed to open database at {}", &config.database_url))?;

    let app_state = AppState {
        sqlite: Arc::new(sqlite),  :coderef[3]
    };
    let router = axum::Router::new()   :coderef[4]
        .route("/authors", post(create_author))
        .layer(trace_layer)
        .with_state(app_state);
    let listener = net::TcpListener::bind(format!("0.0.0.0:{}", &config.server_port))
        .await
        .with_context(|| format!("failed to listen on {}", &config.server_port))?;

    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router)
        .await
        .context("received error from running server")?;

    Ok(())
}
```

## S2

Nearest preceding heading: ### What problems does hexagonal architecture solve?
Info string: rust path=src/lib/routes.rs
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust path=src/lib/routes.rs
pub async fn create_author(
    State(state): State<AppState>,  :coderef[5]
    Json(author): Json<CreateAuthorRequestBody>,
) -> Result<ApiSuccess<CreateAuthorResponseData>, ApiError> {
    if author.name.is_empty() {  :coderef[6]
        return Err(ApiError::UnprocessableEntity(
            "author name cannot be empty".to_string(),
        ));
    }

    let mut tx = state  :coderef[7]
        .sqlite
        .begin()
        .await
        .context("failed to start transaction")?;

    let author_id = save_author(&mut tx, &author.name).await.map_err(|e| {
        if is_unique_constraint_violation(&e) {  :coderef[8]
            ApiError::UnprocessableEntity(format!(
                "author with name {} already exists",
                &author.name
            ))
        } else {
            anyhow!(e).into()
        }
    })?;

    tx.commit().await.context("failed to commit transaction")?;

    Ok(ApiSuccess::new(
        StatusCode::CREATED,
        CreateAuthorResponseData {
            id: author_id.to_string(),
        },
    ))
}
```

## S3

Nearest preceding heading: ### What problems does hexagonal architecture solve?
Info string: rust path=src/lib/routes.rs
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust path=src/lib/routes.rs
async fn save_author(tx: &mut Transaction<'_, Sqlite>, name: &str) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    let id_as_string = id.to_string();
    let query = sqlx::query!(
        "INSERT INTO authors (id, name) VALUES ($1, $2)",
        id_as_string,
        name
    );
    tx.execute(query).await?;
    Ok(id)
}
```

## S4

Nearest preceding heading: ### What problems does hexagonal architecture solve?
Info string: rust path=src/lib/routes.rs
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust path=src/lib/routes.rs
const UNIQUE_CONSTRAINT_VIOLATION_CODE: &str = "2067";

fn is_unique_constraint_violation(err: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db_err) = err {
        if let Some(code) = db_err.code() {
            if code == UNIQUE_CONSTRAINT_VIOLATION_CODE {
                return true;
            }
        }
    }

    false
}
```

## S5

Nearest preceding heading: ### The repository pattern in Rust
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
/// `AuthorRepository` represents a store of author data.
pub trait AuthorRepository {
	/// Persist a new [Author].
	///
	/// # Errors
	///
	/// - MUST return [CreateAuthorError::Duplicate] if an [Author] with the same [AuthorName]
	///   already exists.
	fn create_author(
	    &self,
	    req: &CreateAuthorRequest,  :coderef[9]
	) -> Result<Author, CreateAuthorError>>; :coderef[10]
}
```

## S6

Nearest preceding heading: ### Domain models
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
/// A uniquely identifiable author of blog posts.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Author {  :coderef[11]
    id: Uuid,
    name: AuthorName,
}

impl Author {
    pub fn new(id: Uuid, name: &str) -> Self {
        Self {
            id,
            name: AuthorName::new(name),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn name(&self) -> &AuthorName {
        &self.name
    }
}

/// A validated and formatted name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthorName(String);

#[derive(Clone, Debug, Error)]
#[error("author name cannot be empty")]
pub struct AuthorNameEmptyError;

impl AuthorName {
    pub fn new(raw: &str) -> Result<Self, AuthorNameEmptyError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            Err(AuthorNameEmptyError)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }
}

/// The fields required by the domain to create an [Author].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub struct CreateAuthorRequest {  :coderef[12]
    name: AuthorName,
}

impl CreateAuthorRequest {
    // Constructor and getters ommitted
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CreateAuthorError {  :coderef[13]
    #[error("author with name {name} already exists")]
    Duplicate { name: AuthorName },
    // to be extended as new error scenarios are introduced
}
```

## S7

Nearest preceding heading: ### Domain models
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
pub struct Author {
	id: Option<Uuid>,
	name: AuthorName,
}
```

## S8

Nearest preceding heading: ### Implementing `AuthorRepository`
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
#[derive(Debug, Clone)]
pub struct Sqlite {
    pool: sqlx::SqlitePool,
}

impl Sqlite {
    pub async fn new(path: &str) -> anyhow::Result<Sqlite> {  :coderef[14]
        let pool = sqlx::SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::from_str(path)
                .with_context(|| format!("invalid database path {}", path))?  :coderef[15]
                .pragma("foreign_keys", "ON"),
        )
        .await
        .with_context(|| format!("failed to open database at {}", path))?;

        Ok(Sqlite { pool })
    }
}
```

## S9

Nearest preceding heading: ### Implementing `AuthorRepository`
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
impl AuthorRepository for Sqlite {
    async fn create_author(&self, req: &CreateAuthorRequest) -> Result<Author, CreateAuthorError> {
        let mut tx = self  :coderef[16]
            .pool
            .begin()
            .await
            .context("failed to start SQLite transaction")?;

        let author_id = self.save_author(&mut tx, req.name()) :coderef[17]
	        .await
	        .map_err(|e| {
	            if is_unique_constraint_violation(&e) {  :coderef[18]
	                CreateAuthorError::Duplicate {
	                    name: req.name().clone(),
	                }
	            } else {
	                anyhow!(e)
                        .context(format!("failed to save author with name {:?}", req.name()))
                        .into() :coderef[19]
	            }
	        })?;

        tx.commit()
            .await
            .context("failed to commit SQLite transaction")?;

        Ok(Author::new(author_id, req.name().clone()))
    }
}
```

## S10

Nearest preceding heading: ### Everything but the kitchen async
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
pub trait AuthorRepository {
	async fn create_author(
        &self,
        req: &CreateAuthorRequest,
    ) -> Result<Author, CreateAuthorError>>;
}
```

## S11

Nearest preceding heading: ### Everything but the kitchen async
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
pub trait AuthorRepository {
	fn create_author(
        &self,
        req: &CreateAuthorRequest,
    ) -> impl Future<Output = Result<Author, CreateAuthorError>> + Send; :coderef[20]
}
```

## S12

Nearest preceding heading: ### Everything but the kitchen async
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
pub trait AuthorRepository: Send {
	// ...
}
```

## S13

Nearest preceding heading: ### Everything but the kitchen async
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
pub trait AuthorRepository: Send + Sync {
	// ...
}
```

## S14

Nearest preceding heading: ### Everything but the kitchen async
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
pub trait AuthorRepository: Send + Sync + 'static {
	// ...
}
```

## S15

Nearest preceding heading: ### Everything but the kitchen async
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
pub trait AuthorRepository: Clone + Send + Sync + 'static {
    /// Asynchronously persist a new [Author].
    ///
    /// # Errors
    ///
    /// - MUST return [CreateAuthorError::Duplicate] if an [Author] with the same [AuthorName]
    ///   already exists.
    fn create_author(
        &self,
        req: &CreateAuthorRequest,
    ) -> impl Future<Output = Result<Author, CreateAuthorError>> + Send;
}
```

## S16

Nearest preceding heading: ### From the Very Bad Application to the merely Bad Application
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
#[derive(Debug, Clone)]
/// The application state available to all request handlers.
struct AppState<AR: AuthorRepository> {
    author_repo: Arc<AR>,  :coderef[21]
}
```

## S17

Nearest preceding heading: ### From the Very Bad Application to the merely Bad Application
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
pub async fn create_author<AR: AuthorRepository>(
    State(state): State<AppState<AR>>, :coderef[22]
    Json(body): Json<CreateAuthorHttpRequestBody>,
) -> Result<ApiSuccess<CreateAuthorResponseData>, ApiError> {
	let domain_req = body.try_into_domain()?; :coderef[23]
    state
        .author_repo
        .create_author(&domain_req)
        .await
        .map_err(ApiError::from)  :coderef[24]
        .map(|ref author| ApiSuccess::new(StatusCode::CREATED, author.into())) :coderef[25]
}
```

## S18

Nearest preceding heading: ### From the Very Bad Application to the merely Bad Application
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
/// The body of an [Author] creation request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateAuthorHttpRequestBody {
    name: String,
}

impl CreateAuthorHttpRequestBody {
    /// Converts the HTTP request body into a domain request.
	fn try_into_domain(self) -> Result<CreateAuthorRequest, AuthorNameEmptyError> {
	    let author_name = AuthorName::new(&self.name)?;
	    Ok(CreateAuthorRequest::new(author_name))
	}
}
```

## S19

Nearest preceding heading: ### From the Very Bad Application to the merely Bad Application
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    InternalServerError(String),  :coderef[26]
    UnprocessableEntity(String),  :coderef[27]
}

impl From<CreateAuthorError> for ApiError {
    fn from(e: CreateAuthorError) -> Self {
        match e {
            CreateAuthorError::Duplicate { name } => {
                Self::UnprocessableEntity(format!("author with name {} already exists", name))  :coderef[28]
            }
            CreateAuthorError::Unknown(cause) => {
                tracing::error!("{:?}\n{}", cause, cause.backtrace());
                Self::InternalServerError("Internal server error".to_string())
            }
        }
    }
}

impl From<AuthorNameEmptyError> for ApiError {
    fn from(_: AuthorNameEmptyError) -> Self {
        Self::UnprocessableEntity("author name cannot be empty".to_string())
    }
}
```

## S20

Nearest preceding heading: ### From the Very Bad Application to the merely Bad Application
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
/// The response body data field for successful [Author] creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateAuthorResponseData {
    id: String,
}

impl From<&Author> for CreateAuthorResponseData {
    fn from(author: &Author) -> Self {
        Self {
            id: author.id().to_string(),
        }
    }
}
```

## S21

Nearest preceding heading: ### Testing HTTP handlers with injected repositories
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
#[cfg(test)]
mod tests {
    // Imports omitted.

    #[derive(Clone)]
    struct MockAuthorRepository {
        create_author_result: Arc<Mutex<Result<Author, CreateAuthorError>>>,  :coderef[29]
    }

    impl AuthorRepository for MockAuthorRepository {
        async fn create_author(
            &self,
            _: &CreateAuthorRequest,
        ) -> Result<Author, CreateAuthorError> {
            let mut guard = self.create_author_result.lock().await;
			let mut result = Err(CreateAuthorError::Unknown(anyhow!("substitute error")));
			mem::swap(guard.deref_mut(), &mut result);
			result :coderef[30]
        }
    }
}
```

## S22

Nearest preceding heading: ### Testing HTTP handlers with injected repositories
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_create_author_success() {
    let author_name = AuthorName::new("Angus").unwrap();
    let author_id = Uuid::new_v4();
    let repo = MockAuthorRepository {  :coderef[31]
	    create_author_result: Arc::new(Mutex::new(Ok(Author::new(
	        author_id,
	        author_name.clone(),
	    )))),
	};
    let state = axum::extract::State(AppState {
        author_repo: Arc::new(repo),
    });
    let body = axum::extract::Json(CreateAuthorHttpRequestBody {
        name: author_name.to_string(),
    });
    let expected = ApiSuccess::new(  :coderef[32]
        StatusCode::CREATED,
        CreateAuthorResponseData {
            id: author_id.to_string(),
        },
    );

	let actual = create_author(state, body).await;  :coderef[33]
	assert!(
	    actual.is_ok(),
	    "expected create_author to succeed, but got {:?}",
	    actual
	);

	let actual = actual.unwrap();
	assert_eq!(
	    actual, expected,
	    "expected ApiSuccess {:?}, but got {:?}",
	    expected, actual
	)
}
```

## S23

Nearest preceding heading: ### Introducing the `Service` trait
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
#[derive(Debug, Clone)]
/// The application state available to all request handlers.
struct AppState<AR: AuthorRepository> {
    author_repo: Arc<AR>,
}
```

## S24

Nearest preceding heading: ### Introducing the `Service` trait
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
/// `AuthorMetrics` describes an aggregator of author-related metrics, such as a time-series
/// database.
pub trait AuthorMetrics: Send + Sync + Clone + 'static {  :coderef[34]
    /// Record a successful author creation.
    fn record_creation_success(&self) -> impl Future<Output = ()> + Send;

    /// Record an author creation failure.
    fn record_creation_failure(&self) -> impl Future<Output = ()> + Send;
}

/// `AuthorNotifier` triggers notifications to authors.
pub trait AuthorNotifier: Send + Sync + Clone + 'static {  :coderef[35]
    fn author_created(&self, author: &Author) -> impl Future<Output = ()> + Send;
}
```

## S25

Nearest preceding heading: ### Introducing the `Service` trait
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
#[derive(Debug, Clone)]
/// The application state available to all request handlers.
struct AppState<AS: AuthorService> {
    author_service: Arc<AS>,
}
```

## S26

Nearest preceding heading: ### Introducing the `Service` trait
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
pub trait AuthorService: Clone + Send + Sync + 'static {
    /// Asynchronously create a new [Author].
    ///
	/// # Errors
	///
	/// - [CreateAuthorError::Duplicate] if an [Author] with the same [AuthorName] already exists.
	fn create_author(
        &self,
        req: &CreateAuthorRequest,
    ) -> impl Future<Output = Result<Author, CreateAuthorError>> + Send;
}
```

## S27

Nearest preceding heading: ### Introducing the `Service` trait
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
/// Canonical implementation of the [AuthorService] port, through which the author domain API is
/// consumed.
#[derive(Debug, Clone)]
pub struct Service<R, M, N>  :coderef[36]
where
    R: AuthorRepository,
    M: AuthorMetrics,
    N: AuthorNotifier,
{
    repo: R,
    metrics: M,
    notifier: N,
}

// Constructor implementation omitted

impl<R, M, N> AuthorService for Service<R, M, N>
where
    R: AuthorRepository,
    M: AuthorMetrics,
    N: AuthorNotifier,
{
    /// Create the [Author] specified in `req` and trigger notifications.
	///
	/// # Errors
	///
	/// - Propagates any [CreateAuthorError] returned by the [AuthorRepository].
    async fn create_author(&self, req: &CreateAuthorRequest) -> Result<Author, CreateAuthorError> {
        let result = self.repo.create_author(req).await;  :coderef[37]
        if result.is_err() {
            self.metrics.record_creation_failure().await;
        } else {
            self.metrics.record_creation_success().await;
            self.notifier.author_created(result.as_ref().unwrap()).await;
        }

        result
    }
}
```

## S28

Nearest preceding heading: ### `main` is for bootstrapping
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;

    // A minimal tracing middleware for request logging.
    tracing_subscriber::fmt::init();

    let sqlite = Sqlite::new(&config.database_url).await?;  :coderef[38]
    let metrics = Prometheus::new();
    let email_client = EmailClient::new();
    let author_service = Service::new(sqlite, metrics, email_client);  :coderef[39]

    let server_config = HttpServerConfig {
        port: &config.server_port,
    };
    let http_server = HttpServer::new(author_service, server_config).await?;  :coderef[40]
    http_server.run().await
}
```

## S29

Nearest preceding heading: ### `main` is for bootstrapping
Info string: rust
Source: https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust

```rust
use hexarch::config::Config;
use hexarch::domain::author::service::Service;
use hexarch::inbound::http::{HttpServer, HttpServerConfig};
use hexarch::outbound::email_client::EmailClient;
use hexarch::outbound::prometheus::Prometheus;
use hexarch::outbound::sqlite::Sqlite;
```


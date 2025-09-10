// Example Rust web server for demonstrating ck search capabilities
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Configuration for the web server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
        }
    }
}

/// Main application state
pub struct App {
    config: ServerConfig,
    users: Arc<HashMap<String, User>>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl App {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            users: Arc::new(HashMap::new()),
        }
    }

    /// Start the HTTP server
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        
        println!("Server listening on http://{}", addr);
        
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let app = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = app.handle_connection(stream).await {
                            eprintln!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                    continue;
                }
            }
        }
    }

    /// Handle incoming HTTP connection
    async fn handle_connection(&self, _stream: tokio::net::TcpStream) -> Result<(), Box<dyn Error>> {
        // Simplified connection handling
        // In a real server, you'd parse HTTP requests here
        Ok(())
    }

    /// Authenticate user credentials
    pub async fn authenticate_user(&self, username: &str, password: &str) -> Result<bool, AuthError> {
        // WARNING: This is a demo - never store passwords in plain text!
        // Use proper password hashing like bcrypt, argon2, etc.
        
        if username.is_empty() || password.is_empty() {
            return Err(AuthError::InvalidCredentials);
        }

        // Simulate database lookup
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Demo logic - in real code, hash and compare properly
        match username {
            "admin" => Ok(password == "secret"),
            "user" => Ok(password == "password"),
            _ => Ok(false),
        }
    }

    /// Create a new user account
    pub async fn create_user(&mut self, username: String, email: String) -> Result<User, CreateUserError> {
        // Validate input
        if username.is_empty() {
            return Err(CreateUserError::EmptyUsername);
        }
        
        if !email.contains('@') {
            return Err(CreateUserError::InvalidEmail);
        }

        // Check for existing user
        if self.users.contains_key(&username) {
            return Err(CreateUserError::UserExists);
        }

        let user = User {
            id: rand::random(),
            username: username.clone(),
            email,
            created_at: chrono::Utc::now(),
        };

        // In a real app, you'd save to database here
        Arc::get_mut(&mut self.users)
            .unwrap()
            .insert(username, user.clone());

        Ok(user)
    }

    /// Process user data with error handling
    pub fn process_user_data(&self, user_id: u64) -> Result<String, ProcessingError> {
        // Simulate data processing that might fail
        if user_id == 0 {
            return Err(ProcessingError::InvalidId);
        }

        // Simulate processing
        let result = format!("Processed data for user {}", user_id);
        Ok(result)
    }
}

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials provided")]
    InvalidCredentials,
    #[error("Account locked")]
    AccountLocked,
    #[error("Authentication service unavailable")]
    ServiceUnavailable,
}

/// User creation errors  
#[derive(Debug, thiserror::Error)]
pub enum CreateUserError {
    #[error("Username cannot be empty")]
    EmptyUsername,
    #[error("Invalid email format")]
    InvalidEmail,
    #[error("User already exists")]
    UserExists,
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Data processing errors
#[derive(Debug, thiserror::Error)]
pub enum ProcessingError {
    #[error("Invalid user ID")]
    InvalidId,
    #[error("Processing failed: {0}")]
    ProcessingFailed(String),
}

/// Database connection and operations
pub mod database {
    use super::*;

    pub struct Connection {
        url: String,
    }

    impl Connection {
        pub fn new(url: String) -> Self {
            Self { url }
        }

        pub async fn connect(&self) -> Result<(), DatabaseError> {
            // Simulate connection attempt
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            if self.url.is_empty() {
                return Err(DatabaseError::InvalidUrl);
            }

            println!("Connected to database: {}", self.url);
            Ok(())
        }

        pub async fn execute_query(&self, query: &str) -> Result<Vec<String>, DatabaseError> {
            if query.is_empty() {
                return Err(DatabaseError::EmptyQuery);
            }

            // Simulate query execution
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            
            // Return mock results
            Ok(vec!["result1".to_string(), "result2".to_string()])
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum DatabaseError {
        #[error("Invalid database URL")]
        InvalidUrl,
        #[error("Connection failed")]
        ConnectionFailed,
        #[error("Query cannot be empty")]
        EmptyQuery,
        #[error("Query execution failed: {0}")]
        QueryFailed(String),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = ServerConfig::default();
    let app = App::new(config);
    
    // Test database connection
    let db = database::Connection::new("postgres://localhost/mydb".to_string());
    db.connect().await?;
    
    // Start the server
    app.start().await?;
    
    Ok(())
}
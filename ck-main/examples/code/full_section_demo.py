#!/usr/bin/env python3
"""
Full Section Demo - Python code examples for ck semantic search
This file demonstrates various Python patterns for testing ck's search capabilities
"""

import json
import logging
import asyncio
from typing import Dict, List, Optional, Union
from dataclasses import dataclass
from contextlib import asynccontextmanager


# Setup logging for error handling demonstration
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@dataclass
class UserProfile:
    """User profile data structure"""
    user_id: int
    username: str
    email: str
    is_active: bool = True
    metadata: Dict[str, Union[str, int]] = None

    def __post_init__(self):
        if self.metadata is None:
            self.metadata = {}


class DatabaseConnection:
    """Mock database connection for demonstration"""
    
    def __init__(self, connection_string: str):
        self.connection_string = connection_string
        self.is_connected = False
        self._users: Dict[int, UserProfile] = {}
    
    async def connect(self):
        """Establish database connection with error handling"""
        try:
            # Simulate connection attempt
            await asyncio.sleep(0.1)
            self.is_connected = True
            logger.info("Database connection established")
        except Exception as e:
            logger.error(f"Failed to connect to database: {e}")
            raise ConnectionError("Unable to establish database connection")
    
    async def get_user(self, user_id: int) -> Optional[UserProfile]:
        """Retrieve user profile with error handling"""
        if not self.is_connected:
            raise RuntimeError("Database not connected")
        
        try:
            return self._users.get(user_id)
        except KeyError:
            logger.warning(f"User {user_id} not found")
            return None
        except Exception as e:
            logger.error(f"Error retrieving user {user_id}: {e}")
            raise
    
    async def save_user(self, user: UserProfile):
        """Save user profile with validation and error handling"""
        if not self.is_connected:
            raise RuntimeError("Database not connected")
        
        # Data validation
        if not user.username or len(user.username) < 3:
            raise ValueError("Username must be at least 3 characters")
        
        if '@' not in user.email:
            raise ValueError("Invalid email format")
        
        try:
            self._users[user.user_id] = user
            logger.info(f"User {user.user_id} saved successfully")
        except Exception as e:
            logger.error(f"Failed to save user {user.user_id}: {e}")
            raise


class UserService:
    """User management service with comprehensive error handling"""
    
    def __init__(self, db: DatabaseConnection):
        self.db = db
        self._cache = {}
    
    async def authenticate_user(self, username: str, password: str) -> bool:
        """Authenticate user credentials"""
        if not username or not password:
            raise ValueError("Username and password are required")
        
        try:
            # In real implementation, check against database
            # This is a demo - never hardcode credentials!
            valid_users = {
                'admin': 'secret123',
                'user': 'password',
                'demo': 'demo123'
            }
            
            return valid_users.get(username) == password
        
        except Exception as e:
            logger.error(f"Authentication error for user {username}: {e}")
            return False
    
    async def create_user(self, username: str, email: str) -> UserProfile:
        """Create new user with validation and error handling"""
        try:
            # Validate input
            if not username or len(username) < 3:
                raise ValueError("Username must be at least 3 characters")
            
            if not email or '@' not in email:
                raise ValueError("Valid email address required")
            
            # Create user profile
            user_id = hash(username) % 10000  # Simple ID generation for demo
            user = UserProfile(
                user_id=user_id,
                username=username,
                email=email,
                metadata={'created_by': 'system', 'version': 1}
            )
            
            # Save to database
            await self.db.save_user(user)
            
            logger.info(f"Created user: {username} ({user_id})")
            return user
            
        except ValueError as e:
            logger.warning(f"Validation error creating user {username}: {e}")
            raise
        except Exception as e:
            logger.error(f"Unexpected error creating user {username}: {e}")
            raise RuntimeError("User creation failed")
    
    async def process_user_data(self, user_id: int) -> Dict:
        """Process user data with comprehensive error handling"""
        try:
            user = await self.db.get_user(user_id)
            if not user:
                raise ValueError(f"User {user_id} not found")
            
            # Simulate data processing
            processed_data = {
                'user_info': {
                    'id': user.user_id,
                    'name': user.username,
                    'contact': user.email,
                    'status': 'active' if user.is_active else 'inactive'
                },
                'metadata': user.metadata,
                'processed_at': asyncio.get_event_loop().time()
            }
            
            return processed_data
            
        except ValueError as e:
            logger.warning(f"Data processing validation error: {e}")
            raise
        except Exception as e:
            logger.error(f"Error processing user data for {user_id}: {e}")
            raise RuntimeError("Data processing failed")


@asynccontextmanager
async def database_session(connection_string: str):
    """Context manager for database sessions with proper cleanup"""
    db = DatabaseConnection(connection_string)
    try:
        await db.connect()
        yield db
    except Exception as e:
        logger.error(f"Database session error: {e}")
        raise
    finally:
        # Cleanup would go here in real implementation
        logger.info("Database session closed")


async def main():
    """Main function demonstrating error handling patterns"""
    connection_string = "postgresql://localhost/demo"
    
    try:
        async with database_session(connection_string) as db:
            service = UserService(db)
            
            # Test user creation
            try:
                user = await service.create_user("demo_user", "demo@example.com")
                logger.info(f"Created user: {user}")
                
                # Test authentication
                is_authenticated = await service.authenticate_user("demo_user", "wrong_password")
                logger.info(f"Authentication result: {is_authenticated}")
                
                # Test data processing
                processed = await service.process_user_data(user.user_id)
                logger.info(f"Processed data: {json.dumps(processed, indent=2)}")
                
            except ValueError as e:
                logger.warning(f"Validation error: {e}")
            except RuntimeError as e:
                logger.error(f"Runtime error: {e}")
                
    except ConnectionError as e:
        logger.error(f"Database connection failed: {e}")
        return 1
    except Exception as e:
        logger.error(f"Unexpected error: {e}")
        return 1
    
    return 0


if __name__ == "__main__":
    try:
        exit_code = asyncio.run(main())
        exit(exit_code)
    except KeyboardInterrupt:
        logger.info("Application interrupted by user")
        exit(130)
    except Exception as e:
        logger.error(f"Fatal error: {e}")
        exit(1)
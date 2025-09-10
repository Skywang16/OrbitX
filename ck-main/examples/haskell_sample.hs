-- Haskell sample code for testing semantic search
module Main where

import Data.List (sort)
import Control.Monad (when)

-- Type declarations
data Tree a = Empty | Leaf a | Node a (Tree a) (Tree a)
  deriving (Show, Eq)

type Point = (Double, Double)

-- Function declarations
factorial :: Integer -> Integer
factorial 0 = 1
factorial n = n * factorial (n - 1)

fibonacci :: Integer -> Integer
fibonacci 0 = 0
fibonacci 1 = 1
fibonacci n = fibonacci (n - 1) + fibonacci (n - 2)

-- Higher-order functions and error handling
safeDivide :: Double -> Double -> Maybe Double
safeDivide _ 0 = Nothing
safeDivide x y = Just (x / y)

processNumbers :: [Double] -> [Double]
processNumbers xs = map (*2) . filter (>0) $ xs

-- Data validation and input handling
validateInput :: String -> Either String Integer
validateInput str
  | null str = Left "Empty input"
  | all (`elem` "0123456789") str = Right (read str)
  | otherwise = Left "Invalid characters"

-- Authentication and security patterns
data User = User { username :: String, hashedPassword :: String }
  deriving (Show)

authenticateUser :: String -> String -> [User] -> Bool
authenticateUser name pass users = 
  case filter (\u -> username u == name) users of
    [] -> False
    (u:_) -> hashedPassword u == hashPassword pass

hashPassword :: String -> String
hashPassword = reverse . map (succ)  -- Simple hash for demo

-- Database operations and connection management
data Connection = Connection { host :: String, port :: Int }

connectToDatabase :: String -> Int -> IO (Maybe Connection)
connectToDatabase h p = do
  putStrLn $ "Connecting to " ++ h ++ ":" ++ show p
  -- Simulate connection logic
  return $ Just (Connection h p)

queryData :: Connection -> String -> IO [String]
queryData conn query = do
  putStrLn $ "Executing query: " ++ query
  -- Simulate database query
  return ["result1", "result2", "result3"]

-- Retry mechanisms and error recovery
retryOperation :: Int -> IO a -> IO (Maybe a)
retryOperation 0 action = do
  putStrLn "Max retries reached"
  return Nothing
retryOperation n action = do
  result <- tryOperation action
  case result of
    Just val -> return (Just val)
    Nothing -> do
      putStrLn $ "Retrying... " ++ show (n-1) ++ " attempts left"
      retryOperation (n-1) action

tryOperation :: IO a -> IO (Maybe a)
tryOperation action = do
  -- Simulate operation that might fail
  success <- return True  -- In real code, this would be actual error handling
  if success
    then fmap Just action
    else return Nothing

-- Caching and performance optimization
newtype Cache k v = Cache [(k, v)]

emptyCache :: Cache k v
emptyCache = Cache []

lookupCache :: Eq k => k -> Cache k v -> Maybe v
lookupCache key (Cache pairs) = lookup key pairs

insertCache :: k -> v -> Cache k v -> Cache k v
insertCache key val (Cache pairs) = Cache ((key, val) : pairs)

-- Memory management and resource cleanup
withResource :: IO a -> (a -> IO b) -> IO b
withResource acquire action = do
  resource <- acquire
  result <- action resource
  -- Cleanup would happen here
  putStrLn "Resource cleaned up"
  return result

-- Main function with program entry point
main :: IO ()
main = do
  putStrLn "Starting Haskell application..."
  
  -- Test factorial
  let factResult = factorial 5
  putStrLn $ "Factorial of 5: " ++ show factResult
  
  -- Test validation
  case validateInput "123" of
    Right num -> putStrLn $ "Valid number: " ++ show num
    Left err -> putStrLn $ "Error: " ++ err
    
  -- Test database connection
  maybeConn <- connectToDatabase "localhost" 5432
  case maybeConn of
    Just conn -> do
      results <- queryData conn "SELECT * FROM users"
      putStrLn $ "Query results: " ++ show results
    Nothing -> putStrLn "Failed to connect to database"
    
  putStrLn "Application finished."
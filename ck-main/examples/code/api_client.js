/**
 * API Client for demonstrating ck semantic search capabilities
 * This file contains various patterns for HTTP requests, error handling, and authentication
 */

const https = require('https')
const crypto = require('crypto')

class ApiClient {
  constructor(baseUrl, options = {}) {
    this.baseUrl = baseUrl
    this.timeout = options.timeout || 5000
    this.retryCount = options.retryCount || 3
    this.apiKey = options.apiKey
    this.authToken = null
  }

  /**
   * Authenticate with the API server
   * @param {string} username - User credentials
   * @param {string} password - User password
   * @returns {Promise<Object>} Authentication response
   */
  async authenticate(username, password) {
    try {
      const credentials = {
        username: username,
        password: password,
      }

      const response = await this.post('/auth/login', credentials)

      if (response.token) {
        this.authToken = response.token
        console.log('Authentication successful')
        return response
      } else {
        throw new Error('No token received from authentication')
      }
    } catch (error) {
      console.error('Authentication failed:', error.message)
      throw new AuthenticationError('Failed to authenticate user', error)
    }
  }

  /**
   * Make HTTP GET request with error handling
   * @param {string} endpoint - API endpoint
   * @param {Object} params - Query parameters
   * @returns {Promise<Object>} Response data
   */
  async get(endpoint, params = {}) {
    const url = this.buildUrl(endpoint, params)

    return this.makeRequest('GET', url)
  }

  /**
   * Make HTTP POST request with retry logic
   * @param {string} endpoint - API endpoint
   * @param {Object} data - Request body
   * @returns {Promise<Object>} Response data
   */
  async post(endpoint, data) {
    const url = this.buildUrl(endpoint)

    let lastError
    for (let attempt = 1; attempt <= this.retryCount; attempt++) {
      try {
        return await this.makeRequest('POST', url, data)
      } catch (error) {
        lastError = error

        if (attempt < this.retryCount && this.shouldRetry(error)) {
          console.warn(`Request failed (attempt ${attempt}), retrying...`)
          await this.delay(1000 * attempt) // Exponential backoff
          continue
        }
        break
      }
    }

    throw lastError
  }

  /**
   * Upload file with progress tracking
   * @param {string} endpoint - Upload endpoint
   * @param {Buffer} fileData - File content
   * @param {string} filename - Original filename
   * @returns {Promise<Object>} Upload response
   */
  async uploadFile(endpoint, fileData, filename) {
    try {
      const formData = new FormData()
      formData.append('file', fileData, filename)

      const response = await this.makeRequest('POST', this.buildUrl(endpoint), formData, {
        'Content-Type': 'multipart/form-data',
      })

      console.log(`File ${filename} uploaded successfully`)
      return response
    } catch (error) {
      console.error('File upload failed:', error)
      throw new FileUploadError(`Failed to upload ${filename}`, error)
    }
  }

  /**
   * Make the actual HTTP request
   * @private
   */
  async makeRequest(method, url, data = null, customHeaders = {}) {
    return new Promise((resolve, reject) => {
      const headers = {
        'Content-Type': 'application/json',
        'User-Agent': 'ck-demo-client/1.0',
        ...customHeaders,
      }

      // Add authentication if available
      if (this.authToken) {
        headers.Authorization = `Bearer ${this.authToken}`
      } else if (this.apiKey) {
        headers['X-API-Key'] = this.apiKey
      }

      const requestOptions = {
        method: method,
        headers: headers,
        timeout: this.timeout,
      }

      const request = https.request(url, requestOptions, response => {
        let responseData = ''

        response.on('data', chunk => {
          responseData += chunk
        })

        response.on('end', () => {
          try {
            if (response.statusCode >= 200 && response.statusCode < 300) {
              const parsedData = JSON.parse(responseData)
              resolve(parsedData)
            } else {
              reject(new HttpError(`HTTP ${response.statusCode}`, response.statusCode, responseData))
            }
          } catch (parseError) {
            reject(new ParseError('Failed to parse response JSON', parseError))
          }
        })
      })

      request.on('timeout', () => {
        request.destroy()
        reject(new TimeoutError('Request timeout'))
      })

      request.on('error', error => {
        reject(new NetworkError('Network request failed', error))
      })

      if (data && method !== 'GET') {
        const jsonData = typeof data === 'string' ? data : JSON.stringify(data)
        request.write(jsonData)
      }

      request.end()
    })
  }

  /**
   * Build complete URL with query parameters
   * @private
   */
  buildUrl(endpoint, params = {}) {
    const url = new URL(endpoint, this.baseUrl)

    Object.keys(params).forEach(key => {
      if (params[key] !== null && params[key] !== undefined) {
        url.searchParams.append(key, params[key])
      }
    })

    return url.toString()
  }

  /**
   * Determine if request should be retried
   * @private
   */
  shouldRetry(error) {
    // Retry on network errors and 5xx server errors
    return (
      error instanceof NetworkError ||
      error instanceof TimeoutError ||
      (error instanceof HttpError && error.statusCode >= 500)
    )
  }

  /**
   * Delay helper for retry logic
   * @private
   */
  delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms))
  }

  /**
   * Generate secure hash for data integrity
   * @param {string} data - Data to hash
   * @returns {string} SHA-256 hash
   */
  generateHash(data) {
    try {
      return crypto.createHash('sha256').update(data).digest('hex')
    } catch (error) {
      throw new CryptographicError('Failed to generate hash', error)
    }
  }

  /**
   * Close client and cleanup resources
   */
  close() {
    this.authToken = null
    console.log('API client closed')
  }
}

// Custom error classes for better error handling

class ApiClientError extends Error {
  constructor(message, cause = null) {
    super(message)
    this.name = this.constructor.name
    this.cause = cause
  }
}

class AuthenticationError extends ApiClientError {}
class NetworkError extends ApiClientError {}
class TimeoutError extends ApiClientError {}
class ParseError extends ApiClientError {}
class FileUploadError extends ApiClientError {}
class CryptographicError extends ApiClientError {}

class HttpError extends ApiClientError {
  constructor(message, statusCode, responseBody) {
    super(message)
    this.statusCode = statusCode
    this.responseBody = responseBody
  }
}

// Usage example
async function main() {
  const client = new ApiClient('https://api.example.com', {
    timeout: 10000,
    retryCount: 3,
    apiKey: 'your-api-key-here',
  })

  try {
    // Authenticate user
    await client.authenticate('user@example.com', 'secure-password')

    // Fetch user data
    const userData = await client.get('/users/profile')
    console.log('User data:', userData)

    // Update user settings
    const updateData = {
      name: 'John Doe',
      email: 'john@example.com',
      preferences: {
        theme: 'dark',
        notifications: true,
      },
    }

    await client.post('/users/settings', updateData)
    console.log('Settings updated successfully')
  } catch (error) {
    if (error instanceof AuthenticationError) {
      console.error('Authentication failed. Please check your credentials.')
    } else if (error instanceof NetworkError) {
      console.error('Network error. Please check your connection.')
    } else if (error instanceof HttpError) {
      console.error(`Server error: ${error.statusCode} - ${error.message}`)
    } else {
      console.error('Unexpected error:', error.message)
    }
  } finally {
    client.close()
  }
}

// Export for use in other modules
module.exports = { ApiClient, AuthenticationError, NetworkError, HttpError }

// Run if called directly
if (require.main === module) {
  main().catch(console.error)
}

// 示例JavaScript文件用于测试semantic_search

class UserService {
  constructor() {
    this.users = []
  }

  createUser(userData) {
    const newUser = {
      id: this.users.length + 1,
      ...userData,
    }
    this.users.push(newUser)
    return newUser
  }

  findUserById(id) {
    return this.users.find(user => user.id === id)
  }

  getAllUsers() {
    return this.users
  }
}

function processData(data) {
  const result = {
    processed: true,
    timestamp: new Date(),
    data: data,
  }
  return result
}

const CONFIG = {
  apiUrl: 'https://api.example.com',
  timeout: 5000,
  retries: 3,
}

// 使用示例
const userService = new UserService()
const user = userService.createUser({ name: 'Alice', email: 'alice@example.com' })
const processed = processData(user)

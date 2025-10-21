# Contributing Guide

Thank you for your interest in the OrbitX project! We welcome all forms of contributions.

## 🤝 How to Contribute

### Reporting Issues

If you find a bug or have a feature suggestion:

1. Search [Issues](https://github.com/Skywang16/OrbitX/issues) to see if the issue already exists
2. If not, create a new Issue
3. Use a clear title and detailed description
4. For bugs, provide reproduction steps

### Submitting Code

1. **Fork the repository**

   ```bash
   git clone https://github.com/Skywang16/OrbitX.git
   cd OrbitX
   ```

2. **Create a branch**

   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/your-bug-fix
   ```

3. **Set up development environment**

   ```bash
   npm install
   npm run dev
   ```

4. **Make changes**
   - Follow existing code style
   - Add necessary tests
   - Update relevant documentation

5. **Commit changes**

   ```bash
   git add .
   git commit -m "feat: add your feature description"
   ```

6. **Push branch**

   ```bash
   git push origin feature/your-feature-name
   ```

7. **Create Pull Request**
   - Provide clear PR title and description
   - Link related Issues
   - Wait for code review

## 📝 Code Standards

### Commit Message Format

Use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Types include:

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation update
- `style`: Code formatting
- `refactor`: Code refactoring
- `test`: Add tests
- `chore`: Build process or tooling changes

### Code Style

- Use ESLint and Prettier for code formatting
- Run `npm run lint` to check code style
- Run `npm run format` to auto-format code

### TypeScript

- Add appropriate type definitions for new features
- Avoid using `any` type
- Use interfaces for complex object structures

### Vue.js

- Use Composition API
- Component names use PascalCase
- Props and events use camelCase

## 🧪 Testing

- Write tests for new features
- Ensure all tests pass
- Run `npm run test` to execute tests

## 📚 Documentation

- Update relevant README and documentation
- Add usage examples for new features
- Keep documentation in sync with code

## 🔍 Code Review

All Pull Requests require code review:

- At least one maintainer approval required
- Address all review comments
- Ensure CI checks pass

## 🎯 Development Guide

### Project Structure

```
src/
├── components/     # Reusable components
├── views/         # Page components
├── stores/        # State management
├── utils/         # Utility functions
├── types/         # Type definitions
└── ui/           # UI component library
```

### Adding New Features

1. Create components in `src/components/` or `src/views/`
2. Add store in `src/stores/` if state management is needed
3. Update routing configuration (if needed)
4. Add corresponding type definitions

### Debugging

- Use browser developer tools for frontend debugging
- Use `console.log` or `debugger` for debugging
- Tauri backend can use Rust debugging tools

## 🚀 Release Process

Maintainers handle version releases:

1. Update version number
2. Update CHANGELOG
3. Create Git tag
4. Publish GitHub Release

## 📞 Getting Help

If you encounter issues while contributing:

- Check existing Issues and Discussions
- Ask questions in Issues
- Contact maintainers

## 🙏 Acknowledgments

Thanks to all developers who contribute to the OrbitX project!

---

Thank you again for your contribution! 🎉

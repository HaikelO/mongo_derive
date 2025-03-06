# Contributing to mongo-derive

Thank you for considering contributing to mongo-derive! This document provides guidelines and instructions for contributing to this project.

## Code of Conduct

Please be respectful and considerate of others when contributing to this project. We aim to foster an inclusive and welcoming community.

## How to Contribute

### Reporting Bugs

If you find a bug, please create an issue on GitHub with the following information:

1. A clear, descriptive title
2. A detailed description of the issue
3. Steps to reproduce the bug
4. Expected behavior
5. Actual behavior
6. Your environment (Rust version, MongoDB version, OS, etc.)
7. Any additional context or screenshots

### Suggesting Enhancements

We welcome suggestions for enhancements! Please create an issue on GitHub with:

1. A clear, descriptive title
2. A detailed description of the proposed enhancement
3. The motivation behind the enhancement
4. Any examples of how the enhancement would work

### Pull Requests

1. Fork the repository
2. Create a new branch for your changes
3. Make your changes
4. Write or update tests for your changes
5. Ensure all tests pass with `cargo test`
6. Make sure your code adheres to the project's style using `cargo fmt` and `cargo clippy`
7. Submit a pull request with a clear description of the changes

## Development Setup

1. Clone the repository
2. Install Rust (if you haven't already) from [rustup.rs](https://rustup.rs/)
3. Install MongoDB for running integration tests

## Running Tests

```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run tests with MongoDB integration (requires MongoDB running)
cargo test --features integration_tests
```

## Coding Standards

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format your code
- Use `cargo clippy` to catch common mistakes and improve your code
- Write documentation for public API items
- Include tests for new functionality

## Git Commit Messages

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests liberally after the first line

## License

By contributing to this project, you agree that your contributions will be licensed under the project's [MIT License](LICENSE).

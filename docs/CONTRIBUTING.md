# Contributing to Rulmee

Contributions to **Rulmee** (RUst Login ManagEEr) are welcome! This document outlines our development workflow, Rust coding standards, issue reporting process, and pull request procedures.

---

## Table of Contents

- [Code Guidelines](#code-guidelines)
  - [Prerequisites](#prerequisites)
  - [Development Workflow](#development-workflow)
  - [Code Quality & Formatting](#code-quality--formatting)
  - [Commit Messages](#commit-messages)
- [Issue Tracker](#issue-tracker)
- [Submitting Pull Requests](#submitting-pull-requests)

---

## Code Guidelines

### Prerequisites

Rulmee is written in modern Rust (2024 edition). To build and test locally:

- **Rust Toolchain**: `rustc` and `cargo` (1.85+ recommended).
- **PAM Development Headers**: `libpam0g-dev` (Debian/Ubuntu) or `pam-devel` (Fedora/RHEL/Arch).

### Development Workflow

1. [Fork](https://github.com/BeniaminK/rulmee/fork) the repository and clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/rulmee.git
   cd rulmee
   ```

2. Create a feature or bugfix branch off `main` (or `master`):
   ```bash
   git checkout -b feat/my-new-feature
   ```

3. Build and test your changes:
   ```bash
   cargo build
   cargo test
   ```

### Code Quality & Formatting

Before opening a pull request, ensure all linters, formatters, and unit tests pass cleanly:

- **Format Code**:
  ```bash
  cargo fmt --all
  ```
- **Lint Code**:
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```
- **Run Unit Tests**:
  ```bash
  cargo test
  ```

### Commit Messages

Follow standard conventional commit conventions:

- **Header line**: Concise imperative statement summarizing the change (max 72 chars).
  - Format: `<type>(<scope>): <subject>`
  - Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`.
  - Examples:
    - `feat(ui): add keybinding for session switcher`
    - `fix(pam): handle auth failure gracefully`
    - `docs(standards): update logging architecture details`

---

## Issue Tracker

We use **GitHub Issues** to track bug reports, feature requests, and technical debt.

- Search existing issues before submitting:
  ```bash
  gh issue list --repo BeniaminK/rulmee
  ```
- To view details on a specific issue:
  ```bash
  gh issue view <issue-id>
  ```
- To submit a new issue via `gh` CLI:
  ```bash
  gh issue create --repo BeniaminK/rulmee
  ```

---

## Submitting Pull Requests

1. Push your branch to your fork:
   ```bash
   git push origin feat/my-new-feature
   ```
2. Open a Pull Request against `BeniaminK/rulmee`.
3. Provide a detailed summary of your changes, referencing any associated GitHub Issue IDs (e.g. `Fixes #18`).

# Contributing

Contributions are welcome!

> *Note: This project currently uses a simplified contributing guide. We may move away from this generic `CONTRIBUTING.md` format in the future as project guidelines mature.*

---

## Prerequisites

To build and test the project locally, you will need:

- **Rust Toolchain**: `rustc` and `cargo` (1.85+ / edition 2024 recommended).
- **PAM Development Headers**: `libpam0g-dev` (Debian/Ubuntu) or `pam-devel` (Fedora/Arch/RHEL).

---

## Workflow Rules

### 1. Issues First
- **Every pull request must be backed by an open GitHub issue.**
- Discussion regarding the need, design, or existence of a bug or feature must happen strictly on the issue tracker.
- Issue templates are provided with `Must Have` and `Might Have` sections to scope requirements clearly.

### 2. Pull Requests & Code Reviews
- Pull request comments must be strictly code-related (implementation details, performance, safety, and readability).
- Ensure your changes pass:
  ```bash
  cargo fmt --all
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test
  ```

---

## Commit Messages

Follow standard conventional commit formatting:
- `<type>(<scope>): <subject>` (e.g., `feat(ui): add log viewer overlay`, `fix(pam): handle session teardown safely`).

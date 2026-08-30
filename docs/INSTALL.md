# Installation Guide

- [Packages](#packages)
- [Installing from Source](#installing-from-source)

---

# Packages

There are currently no distribution packages available for Rulmee. Package maintainer outreach is tracked in [Issue #19](https://github.com/BeniaminK/rulmee/issues/19).

---

# Installing from Source

### Prerequisites

- **Rust Toolchain**: `cargo` and `rustc` (1.85+ / edition 2024).
- **PAM Headers**: `libpam0g-dev` (Debian/Ubuntu) or `pam-devel` (Fedora/Arch/RHEL).

### Build Procedure

```sh
# Clone repository
git clone https://github.com/BeniaminK/rulmee.git
cd rulmee

# Build optimized release binary
cargo build --release

# Install binary to system
cargo install --path .
```

The compiled binary is placed at `target/release/rulmee`.

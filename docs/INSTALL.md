# Installation Guide for Rulmee

This guide covers installing **Rulmee** via package managers or building directly from source.

# Table of Contents

- [Service Activation](#service-activation)
- [Packages](#packages)
  - [Void Linux](#void-linux)
  - [Fedora](#fedora)
  - [AUR (Arch Linux)](#aur-arch-linux)
  - [Nix Flake & Module](#nix-flake--module)
- [Installing from Source](#installing-from-source)

---

# Service Activation

Once Rulmee is installed, enable it with your system's init manager.

### Systemd

Disable your existing display manager (e.g. `sddm`, `gdm`, `lightdm`, `ly`):

```sh
sudo systemctl disable gdm
```

Enable Rulmee:

```sh
sudo systemctl enable rulmee
```

---

# Packages

Packages are maintained by community packagers. Report packaging-specific issues to the respective package maintainers.

## Void Linux

Install via `xbps`:

```sh
xbps-install rulmee
```

Enable the service:

```sh
ln -s /etc/sv/rulmee /var/service/
```

## Fedora

Install via COPR repository:

```sh
dnf copr enable celestelove/rulmee
dnf install rulmee
```

## AUR (Arch Linux)

Install using your preferred AUR helper:

```sh
yay -S rulmee
```

## Nix Flake & Module

Try Rulmee via Nix Flake:

```sh
nix run github:BeniaminK/rulmee
```

Or install to profile:

```sh
nix profile install github:BeniaminK/rulmee
```

For NixOS module configuration, import `assets/pkg/nix/module.nix` in `configuration.nix`:

```nix
services.displayManager.enable = true;
systemd.services.rulmee.enable = true;
```

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

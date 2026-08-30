# Packagers Guide

This guide details options and conventions for distribution maintainers packaging the application.

---

## 1. Package Components

When packaging for a Linux distribution, maintainers should include:

1. **Application Binary**: Compiled via `cargo build --release` (`target/release/rulmee`).
2. **Default Configurations**: Default TOML configuration files (`/etc/rulmee/config.toml` & `theme.toml`).
3. **Man Pages**: Located in `assets/man/` (`rulmee.1`, `rulmee-config.5`).
4. **Service Descriptors**: Service definitions in `assets/services/` (`systemd`, `dinit`, `runit`, `openrc`, `s6`).
5. **Stock Themes**: Bundled themes in `themes/`.

---

## 2. Environment Variables & System Paths

Supports environment variables for system path configuration:

| Environment Variable | Default Value | Description |
| :--- | :--- | :--- |
| `RULMEE_CONF` | `/etc/rulmee/config.toml` | Target configuration file path. |
| `RULMEE_AUTH_PAM_SERVICE` | `login` (or `rulmee`) | Default PAM service name for authentication. |

> *Note: The system theme search directory `/usr/share/rulmee/themes` is currently hardcoded. Making `RULMEE_THEME_DIR` configurable is tracked in [Issue #21](https://github.com/BeniaminK/rulmee/issues/21).*

---

## 3. Building & Packaging Commands

Build the release binary using Cargo:

```bash
# Build optimized release binary
cargo build --release --locked

# Install binary to package DESTDIR
install -Dm755 target/release/rulmee "$DESTDIR/usr/bin/rulmee"

# Install default config and man pages
install -Dm644 assets/man/rulmee.1 "$DESTDIR/usr/share/man/man1/rulmee.1"
install -Dm644 assets/man/rulmee-config.5 "$DESTDIR/usr/share/man/man5/rulmee-config.5"
```

---

## 4. Init Service Integration

Rulmee provides ready-to-use service files in `assets/services/`:
- **systemd**: `assets/services/systemd.service` $\rightarrow$ `/usr/lib/systemd/system/rulmee.service`
- **openrc**: `assets/services/openrc` $\rightarrow$ `/etc/init.d/rulmee`
- **runit**: `assets/services/runit/` $\rightarrow$ `/etc/sv/rulmee/`
- **dinit**: `assets/services/dinit` $\rightarrow$ `/etc/dinit.d/rulmee`
- **s6**: `assets/services/s6/` $\rightarrow$ `/etc/s6/sv/rulmee/`

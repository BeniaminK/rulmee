# Rulmee Packagers Guide

This guide details options and conventions for distribution maintainers packaging **Rulmee** (RUst Login ManagEEr).

---

## 1. Package Components

When packaging Rulmee for a Linux distribution, maintainers should include:

1. **Rulmee Binary**: Compiled via `cargo build --release` (`target/release/rulmee`).
2. **Default Configurations**: Default TOML configuration files (`/etc/rulmee/config.toml` & `theme.toml`).
3. **Man Pages**: Located in `assets/man/` (`rulmee.1`, `rulmee-config.5`).
4. **Service Descriptors**: Service definitions in `assets/services/` (`systemd`, `dinit`, `runit`, `openrc`, `s6`).
5. **Stock Themes**: Bundled TOML and legacy themes in `themes/`.

---

## 2. Environment Variables & Fallbacks

Rulmee supports environment variables for system path configuration:

| Environment Variable | Default Value | Description |
| :--- | :--- | :--- |
| `RULMEE_CONF` (or `LIDM_CONF`) | `/etc/rulmee/config.toml` | Target configuration file path. |
| `RULMEE_PAM_SERVICE` (or `LIDM_PAM_SERVICE`) | `login` (or `rulmee`) | Default PAM service name for authentication. |
| `RULMEE_THEME_DIR` | `/usr/share/rulmee/themes` | Directory containing system themes. |

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

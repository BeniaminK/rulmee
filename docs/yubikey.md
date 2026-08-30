# YubiKey & FIDO Authentication Guide

This document explains how to set up YubiKey / FIDO hardware authentication with **Rulmee**.

## Enabling YubiKey Authentication

YubiKey authentication is supported via `pam_u2f`.

1. Ensure `pam_u2f` is installed and key associations are generated using `pamu2fcfg`.
2. Configure a keybinding for FIDO authentication in `/etc/rulmee/config.toml`:

```toml
[functions]
fido = "F2"
```

3. Pressing the designated keybinding triggers the PAM FIDO verification process.

## Sample PAM Module Configuration

Add the `pam_u2f.so` module to your PAM configuration stack (e.g. `/etc/pam.d/rulmee` or `/etc/pam.d/login`):

```pam
#%PAM-1.0

auth       sufficient   pam_u2f.so cue
auth       requisite    pam_nologin.so
auth       include      system-local-login
account    include      system-local-login
session    include      system-local-login
password   include      system-local-login
```

For detailed setup options, consult the [Arch Linux YubiKey Wiki](https://wiki.archlinux.org/title/YubiKey).

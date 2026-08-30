# 0001. Standards and Security Boundaries

* Status: accepted
* Date: 2026-08-30

## Context and Problem Statement

Rulmee (RUst Login ManagEEr) operates with root privileges to manage virtual terminals, PAM authentication, and user desktop sessions. Architectural security and compliance boundaries must be strictly codified to prevent privilege escalation vulnerabilities, broken PAM session states, or incompatible session discovery across Linux distributions.

## Decision Drivers

* **Security**: Root context must never execute or evaluate user-controlled profile scripts (`~/.profile`, `~/.bashrc`).
* **Compliance**: Full alignment with Freedesktop.org specifications (Desktop Entry Spec, XDG Base Directory Spec).
* **Authentication**: Strict ordering of the Linux-PAM lifecycle and environment export (`pam_getenvlist`).
* **Session Tracking**: Integration with `systemd-logind` via `pam_systemd.so` (`XDG_SEAT`, `XDG_VTNR`, `XDG_SESSION_TYPE`).

## Considered Options

1. Ad-hoc session spawning with manual script evaluation inside the root process.
2. POSIX subshell execution (`$SHELL -l -c "exec <cmd>"`) with strict child privilege dropping (`setgid` $\rightarrow$ `initgroups` $\rightarrow$ `setuid` $\rightarrow$ `chdir`) and full PAM lifecycle management.

## Decision Outcome

Chosen option: Option 2.

### Positive Consequences

* **Isolation**: Prevents Local Privilege Escalation (LPE) by delegating profile sourcing to unprivileged subshells after dropping privileges.
* **Consistency**: Sourcing `/etc/profile` and `~/.profile` via login shells guarantees POSIX compliance without duplicating shell parsing in Rust.
* **Integrity**: Standard PAM lifecycle ensures `pam_systemd` creates `/run/user/<UID>` and manages GPU/input permissions cleanly.

### Negative Consequences

* Slight execution overhead for spawning an intermediate login shell during session launch.

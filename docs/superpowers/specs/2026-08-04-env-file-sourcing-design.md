# Design Specification: Environment File Sourcing (`source` and `user_source`)

**Date**: 2026-08-04  
**Status**: Approved  
**Target Module**: `src/env.rs` (re-exported in `src/main.rs` and `src/auth.rs`)

---

## 1. Overview & Objective

In the C implementation of **LiDM**, `source_paths()` reads system scripts (e.g. `/etc/profile`, `/etc/environment`) and user home scripts (e.g. `~/.xprofile`, `~/.pam_environment`) line-by-line for `KEY=VALUE` environment variable definitions before launching a user desktop/shell session.

In the Rust implementation, `config.behavior.source` and `config.behavior.user_source` are defined in the `Behavior` struct in `src/config.rs` but are not yet processed or merged into the session environment.

This specification details a dedicated environment script loader module in `src/env.rs` that safely parses environment files, ignores comments, handles quotes, strips optional `export` prefixes, and uses scope/block tracking to avoid setting variables located inside unexecuted function bodies or control blocks.

---

## 2. Architecture & File Placement

A new module `src/env.rs` will be created with the public interface:

```rust
pub fn source_environment_files(
    env: &mut HashMap<String, String>,
    system_sources: &[String],
    user_sources: &[String],
    home_dir: Option<&Path>,
);
```

### Module Responsibilities
1. **`src/env.rs`**: Environment loader & line parser with scope depth tracking.
2. **`src/main.rs`**: Registers `mod env;` and calls `env::source_environment_files(...)` after PAM authentication and initial `USER`/`HOME` variable setup, prior to `exec::launch_session(...)`.

---

## 3. Parsing & Scope Tracking Rules

For each path in `system_sources` (absolute system paths) and `user_sources` (resolved relative to `home_dir` if present):

1. **Path Resolution**:
   * If a system path is empty or non-existent, log a debug message and skip.
   * If a user path is specified, construct `home_dir.join(path)`. If `home_dir` is `None`, log a warning and skip.

2. **Line-by-Line Lexing & Scope Depth Tracking**:
   * Maintain `block_depth: usize = 0`.
   * For each line:
     1. Trim leading and trailing whitespace.
     2. Skip empty lines and lines where the first non-whitespace character is `#` (comments).
     3. Check for block opening indicators (`{` or function header like `fn_name() {`): increment `block_depth`.
     4. Check for block closing indicators (`}`): decrement `block_depth`.
     5. Check for control block start keywords (`if `, `case `, `for `, `while `): skip parsing assignments on control flow lines.
     6. **Assignment Execution Rule**: Only attempt `KEY=VALUE` extraction when `block_depth == 0` and the line is not inside a control block.

3. **Key-Value Parsing**:
   * Strip optional leading `export ` (e.g. `export PATH="/bin"` → `PATH="/bin"`).
   * Locate the first `=` character. If no `=` is found, skip.
   * Key extraction: `key = line[..eq_pos].trim()`.
   * **POSIX Key Validation**: Key must match `^[a-zA-Z_][a-zA-Z0-9_]*$`. Discard invalid keys (e.g. `func()`, `[`, arithmetic statements).
   * Value extraction: `value = line[eq_pos + 1..].trim()`.
   * **Unquoting**: If `value` starts and ends with matching single quotes (`'`) or double quotes (`"`), strip outer quotes.
   * **Mutation**: `env.insert(key.to_string(), value.to_string());`

---

## 4. Verification & Testing Strategy

Add exhaustive unit tests in `src/env.rs`:

1. **Basic Key-Value**: Test `FOO=bar` and `export BAZ="qux"`.
2. **Quoted Values**: Test `VAR="hello world"` and `SINGLE='value'`.
3. **Comments & Empty Lines**: Verify `# comment` lines and blank lines are ignored.
4. **Function Body Exclusion**: Verify that lines inside `tempfunc() { VAR=123 }` are ignored.
5. **Control Block Exclusion**: Verify that lines inside `if [ ... ]; then VAR=1; fi` are ignored.
6. **Path Integration**: Test `source_environment_files` with mock system files and user home directories via `tempfile`.

---

## 5. Security & Safety Considerations

- **Memory/Resource Protection**: Avoid spawning subshells (`sh -c`) to parse files, preventing command injection and hangs on interactive scripts.
- **Error Handling**: Missing files or permission denied errors log warnings gracefully without aborting session login.

# Freedesktop `Exec` String Parsing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace naive `exec.split_whitespace()` in `src/session.rs` with a spec-compliant Freedesktop `Exec` string lexer handling quotes, escaped spaces, and `%` field codes.

**Architecture:** Implement `pub fn parse_exec_string(exec: &str) -> Vec<String>` using a state-machine character parser in `src/session.rs`, with full unescaping of quotes, backslashes, and `%` specifier stripping, then integrate it into `get_available_sessions()`.

**Tech Stack:** Rust 2024 edition, zero third-party dependencies required.

## Global Constraints

- Must follow the Freedesktop Desktop Entry Specification rules for `Exec` key parsing.
- Must strip `%` field codes (e.g. `%f`, `%u`, `%F`, `%U`, `%i`, `%c`, `%k`) while converting `%%` to literal `%`.
- Must handle double quotes `"..."`, single quotes `'...'`, and backslash escapes `\`.

---

### Task 1: Implement `parse_exec_string` and unit tests in `src/session.rs`

**Files:**
- Modify: `src/session.rs`
- Test: `src/session.rs` (unit tests in `mod tests`)

**Interfaces:**
- Consumes: Raw `&str` from Desktop Entry `Exec` attributes.
- Produces: `pub fn parse_exec_string(exec: &str) -> Vec<String>`

- [ ] **Step 1: Write failing unit tests for `parse_exec_string`**

Add unit tests to `src/session.rs` under `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exec_string_plain() {
        assert_eq!(
            parse_exec_string("sway --unsupported-gpu"),
            vec!["sway", "--unsupported-gpu"]
        );
    }

    #[test]
    fn test_parse_exec_string_quoted_spaces() {
        assert_eq!(
            parse_exec_string("gnome-session --session=\"gnome fallback\""),
            vec!["gnome-session", "--session=gnome fallback"]
        );
        assert_eq!(
            parse_exec_string("'my program' 'arg with spaces'"),
            vec!["my program", "arg with spaces"]
        );
    }

    #[test]
    fn test_parse_exec_string_escaped_spaces() {
        assert_eq!(
            parse_exec_string("my\\ app --arg"),
            vec!["my app", "--arg"]
        );
    }

    #[test]
    fn test_parse_exec_string_field_codes_stripped() {
        assert_eq!(
            parse_exec_string("gnome-terminal %u --dir %f"),
            vec!["gnome-terminal", "--dir"]
        );
        assert_eq!(
            parse_exec_string("app --title=%c %F"),
            vec!["app", "--title="]
        );
    }

    #[test]
    fn test_parse_exec_string_literal_percent() {
        assert_eq!(
            parse_exec_string("echo %%USER%%"),
            vec!["echo", "%USER%"]
        );
    }

    #[test]
    fn test_parse_exec_string_unclosed_quotes() {
        assert_eq!(
            parse_exec_string("sway --config \"/etc/sway/config"),
            vec!["sway", "--config", "/etc/sway/config"]
        );
    }
}
```

- [ ] **Step 2: Run cargo test to verify tests fail to compile**

Run: `cargo test --lib session::tests`
Expected: Failure with `cannot find function parse_exec_string in this scope`

- [ ] **Step 3: Implement `parse_exec_string` in `src/session.rs`**

Add the function to `src/session.rs`:

```rust
pub fn parse_exec_string(exec: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_double_quote = false;
    let mut in_single_quote = false;
    let mut escaped = false;
    let mut saw_quotes_for_arg = false;

    let chars: Vec<char> = exec.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if escaped {
            current_arg.push(c);
            escaped = false;
            i += 1;
            continue;
        }

        if c == '\\' && !in_single_quote {
            escaped = true;
            i += 1;
            continue;
        }

        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            saw_quotes_for_arg = true;
            i += 1;
            continue;
        }

        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            saw_quotes_for_arg = true;
            i += 1;
            continue;
        }

        if (c == ' ' || c == '\t' || c == '\n') && !in_double_quote && !in_single_quote {
            if !current_arg.is_empty() || saw_quotes_for_arg {
                args.push(std::mem::take(&mut current_arg));
                saw_quotes_for_arg = false;
            }
            i += 1;
            continue;
        }

        if c == '%' {
            if i + 1 < chars.len() {
                let next = chars[i + 1];
                if next == '%' {
                    current_arg.push('%');
                    i += 2;
                    continue;
                } else {
                    // Drop any %X field code
                    i += 2;
                    continue;
                }
            } else {
                // Trailing %
                i += 1;
                continue;
            }
        }

        current_arg.push(c);
        i += 1;
    }

    if !current_arg.is_empty() || saw_quotes_for_arg {
        args.push(current_arg);
    }

    args
}
```

- [ ] **Step 4: Run cargo test to verify tests pass**

Run: `cargo test --lib session::tests`
Expected: PASS

- [ ] **Step 5: Commit task 1**

```bash
git add src/session.rs
git commit -m "feat(session): add Freedesktop Exec string parser and unit tests"
```

---

### Task 2: Integrate `parse_exec_string` into `get_available_sessions()`

**Files:**
- Modify: `src/session.rs:50-54`

**Interfaces:**
- Consumes: `parse_exec_string(exec: &str)`
- Produces: Updated `get_available_sessions()` returning `Session` items with parsed arguments.

- [ ] **Step 1: Replace `exec.split_whitespace()` in `get_available_sessions()`**

In `src/session.rs`, replace:

```rust
// Basic parsing of exec string into args
let args: Vec<String> = exec.split_whitespace()
    .map(|s| s.to_string())
    .collect();
```

with:

```rust
let args = parse_exec_string(exec);
```

- [ ] **Step 2: Run all tests & check build**

Run: `cargo check && cargo test`
Expected: PASS

- [ ] **Step 3: Commit task 2**

```bash
git add src/session.rs
git commit -m "refactor(session): replace split_whitespace with parse_exec_string"
```

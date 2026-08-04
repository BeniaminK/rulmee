# Design Specification: Freedesktop `Exec` String Parsing in Rust

**Date:** 2026-08-05  
**Topic:** Freedesktop `Exec` Key Parsing for Session Desktop Files  
**Target File:** `src/session.rs`

## Context & Motivation
In standard Freedesktop `.desktop` files (found in `/usr/share/xsessions` and `/usr/share/wayland-sessions`), the `Exec` key contains command-line arguments that may contain quoted parameters with spaces (`"gnome fallback"`), escaped spaces (`my\ app`), or Freedesktop field codes (`%f`, `%u`, `%F`, `%U`, `%%`).

Previously, `session.rs` used naive `exec.split_whitespace()`, which broke on quoted arguments with spaces and left unhandled `%` field codes in the argument array. This change replaces naive whitespace splitting with an idiomatic, safe Rust parser adhering to the Freedesktop Desktop Entry Specification.

## Design Details

### 1. Function Signature & Location
The parser function `parse_exec_string` will be defined in `src/session.rs`:

```rust
pub fn parse_exec_string(exec: &str) -> Vec<String>
```

### 2. State Machine Tokenizer
The function iterates character-by-character over `exec` with the following state:
- `in_double_quote: bool`: True when inside `"..."`.
- `in_single_quote: bool`: True when inside `'...'`.
- `escaped: bool`: True if the previous character was an unquoted backslash `\`.
- `current_arg: String`: Buffer for accumulating the current argument.
- `args: Vec<String>`: Vector accumulating finished arguments.

#### Tokenization & Escaping Rules:
- **Whitespace (`' '`, `'\t'`, `'\n'`) outside quotes**: Finalizes `current_arg` into `args` if non-empty (or if explicitly quoted), then resets `current_arg`.
- **Quotes (`"` and `'`)**: Toggle corresponding quote mode if unescaped. Quote delimiters are stripped from tokens.
- **Escapes (`\`)**:
  - Inside double quotes: Escapes `"`, `\`, `$`, `` ` ``.
  - Outside quotes: Escapes the subsequent character (e.g. `\ ` becomes literal space).
- **Field Codes (`%`)**:
  - `%%`: Appends literal `%` to `current_arg`.
  - `%X` (e.g., `%f`, `%F`, `%u`, `%U`, `%i`, `%c`, `%k`, `%v`, etc.): Stripped/skipped without adding to `current_arg`.
  - Unmatched standalone field codes that result in an empty string (e.g. `gnome-terminal %u`) produce `["gnome-terminal"]` without empty argument elements.

### 3. Integration in `src/session.rs`
Replace `exec.split_whitespace()` when constructing `Session` structs:

```rust
if let (Some(name), Some(exec)) = (name, exec) {
    let args = parse_exec_string(exec);
    if !args.is_empty() {
        sessions.push(Session {
            name: name.to_string(),
            exec: ExecType::Desktop(args),
            session_type: *session_type,
        });
    }
}
```

### 4. Unit Test Cases (`src/session.rs`)
Exhaustive unit tests will be added to verify:
- Plain split: `"sway --unsupported-gpu"` $\rightarrow$ `["sway", "--unsupported-gpu"]`
- Quoted argument: `"gnome-session --session=\"gnome fallback\""` $\rightarrow$ `["gnome-session", "--session=gnome fallback"]`
- Escaped spaces: `"my\\ app --arg"` $\rightarrow$ `["my app", "--arg"]`
- Stripping field codes: `"gnome-terminal %u --dir %f"` $\rightarrow$ `["gnome-terminal", "--dir"]`
- Literal percent: `"echo %%USER%%"` $\rightarrow$ `["echo", "%USER%"]`
- Trailing backslash / unclosed quotes: Handles gracefully without panics.

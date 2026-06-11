# aqc-fs-utils

Read a single file from disk with fixed rules. Shared by Specular, Fixture3, Guardrail3.

Not a repo walk (`aqc-filetree`). Not Git. Not substring checks.

---

## API

```rust
pub fn read_text(path: impl AsRef<Path>, options: ReadTextOptions) -> Result<String, ReadError>;

pub fn read_bytes(path: impl AsRef<Path>, options: ReadBytesOptions) -> Result<Vec<u8>, ReadError>;
```

**`read_text`:** valid UTF-8 `String` only. Empty file → `Ok("")`.

**`read_bytes`:** raw bytes, no UTF-8 check, no NUL check.

---

## `ReadTextOptions`

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `symlink` | `SymlinkReadPolicy` | `DontFollow` | `DontFollow` \| `Follow` |
| `max_bytes` | `u64` | `1_073_741_824` (1 GiB) | Reject larger files with `ReadError::TooLarge` |
| `normalize_crlf` | `bool` | `false` | If `true`, replace `\r\n` with `\n` in the returned `String` |

No other fields. No lossy UTF-8 mode.

---

## `ReadBytesOptions`

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `symlink` | `SymlinkReadPolicy` | `DontFollow` | |
| `max_bytes` | `u64` | `1_073_741_824` | Same cap |

---

## `SymlinkReadPolicy`

| Variant | Behavior |
|---------|----------|
| `DontFollow` | Open the symlink node |
| `Follow` | Open the target |

---

## `ReadError`

| Variant | When |
|---------|------|
| `NotFound` | Path missing |
| `NotAFile` | Not a regular file (directory, etc.) |
| `TooLarge` | File size > `max_bytes` |
| `ContainsNulByte` | Raw bytes contain `0x00` before UTF-8 decode |
| `InvalidUtf8` | Bytes are not valid UTF-8 |
| `Io { path, source }` | OS error |

All failures are `Err`. No `Ok` branch for “binary” or “skip”.

---

## Read algorithm (`read_text`)

1. Stat/open path per `symlink`.
2. Reject if not a file → `NotAFile`.
3. Reject if size > `max_bytes` → `TooLarge`.
4. Read all bytes (up to cap).
5. If any byte is `0x00` → `ContainsNulByte`.
6. Decode UTF-8 strictly → on failure `InvalidUtf8`.
7. If `normalize_crlf`, normalize the `String`.
8. Return `Ok(string)`.

### NUL byte

Scan raw bytes for `0x00` before decode. Common practice for “text vs binary” (git, ripgrep-style detection): source and config text in AQC repos should not contain NUL; if they do, treat as error, not as searchable text.

Valid UTF-8 can include U+0000; we still reject on raw NUL so behavior stays simple and matches binary-file expectations.

---

## Non-goals

- Repo walk
- Git porcelain
- Lossy UTF-8
- Parsing, reconcile, write
- Substring / regex search

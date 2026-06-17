# aqc-fs-utils

Small filesystem helpers for Agent Quality Controls.

This crate centralizes fixed file-read behavior:

- strict UTF-8 handling.
- NUL byte rejection.
- size caps.
- structured read errors.

It exists so checkers do not each invent slightly different filesystem rules.

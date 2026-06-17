# aqc-filetree

Filesystem tree collection for Agent Quality Controls.

This crate walks a directory root and produces a `FileTree` snapshot. It uses
gitignore-aware traversal first, then applies recovery rules for files that
must remain visible to guardrail checks.

Use it when a checker needs a stable file-tree input instead of direct
filesystem traversal spread across multiple packages.

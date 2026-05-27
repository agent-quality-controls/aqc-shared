# Fix: gitignore __pycache__

Prior commit `f6ca044` accidentally checked in
`scripts/__pycache__/_verify_lib.cpython-312.pyc`. Removed it from the
index and added `__pycache__/` plus `*.pyc` to `.gitignore`.

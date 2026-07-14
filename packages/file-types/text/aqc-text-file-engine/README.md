# aqc-text-file-engine

Reusable AQC file engine for exact text files and required contained byte sequences.

`TextFileRequirements` combines `exact_contents` scalar assertions with
`contents` item requirements. The engine merges both through
`aqc-file-engine-core` before validating or initializing supplied bytes.

# Fixture attribution

`ascii/synthetic_v1999.*` and `ascii/synthetic_v1991.*` are hand-authored for this
project, not vendored from any external source. Values were chosen so the expected
parsed/scaled output can be computed by hand (see the assertions in
`tests/comtrade_parser_tests.rs`), which gives an independently-verified correctness
check that a vendored real-world file (where we'd only have a third-party tool's
output to compare against) doesn't.

Follow-up (not blocking M1): vendor a small set of real-world fixtures from
`dparrini/python-comtrade` (GitHub) and the `comtrade` Rust crate's own test files —
both MIT-licensed — for broader revision/vendor-quirk coverage. Confirm each source's
license file at vendor time and record it here per-file before committing.

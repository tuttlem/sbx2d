# Error reporting policy

Every pack diagnostic has a stable `SBXnnn` code, a source path, and—when the
source was read—a one-based line and column. Schema failures also include the
field path, expected versus received shape/value, and an actionable `help`
message. Validation reports the first error for now; multi-error aggregation is
a later usability improvement and must preserve this information.

Exit status is zero for a valid pack, one for a validation/loading failure, and
two for CLI misuse.

# Changelog

## Unreleased

### Added

- **mux-registry**: `TooManyContracts` error and 128-contract cap enforced in `register()`.
- **mux-registry**: `check_version(name, expected)` read-only query that returns `bool` without mutating storage.
- **mux-registry**: `register_with_metadata(name, version, description, author)` and `get_metadata(name)` for contract metadata.
- **mux-registry**: Audit events tagged `mux_reg` (`init`, `reg`, `regmeta`).
- **mux-registry**: Rust tests for non-admin rejection, 128-cap, `check_version` no-mutation, and `register_with_metadata`.
- **mux-recovery**: New contract with timelocked recovery lifecycle, guardian management, and quorum threshold.
- **mux-recovery**: Rust tests for `cancel_recovery`, guardian add/remove, and `set_quorum_threshold` rejection above guardian count.
- **mux-batcher**: Admin-configurable `FEE_PER_OP` via `set_fee_per_op(fee: u32)` and public `fee_per_op()` query.
- **mux-batcher**: `estimate_fees(op_count)` entrypoint reading the stored fee with fallback to the compile-time default (100 stroops).
- **mux-batcher**: Rust tests for default fee fallback, admin update, and non-admin rejection.
- **bindings**: Type-level test asserting `MuxContractIds` and `DEFAULT_ADDRESSES` share the same key set.

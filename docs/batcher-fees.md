# mux-batcher Fees

## Overview

`mux-batcher` charges a per-operation fee (in stroops) for each operation in a batch. The total fee is computed as:

```
total_fee = op_count × fee_per_op
```

## Querying the Current Fee

Call the `fee_per_op()` entrypoint at runtime to read the current per-operation fee:

```typescript
const fee = await muxBatcher.fee_per_op();
```

## Adjusting the Fee

The admin can update the fee at any time via `set_fee_per_op(fee: u32)`. This is an admin-only operation; non-admin callers are rejected with `Unauthorized`.

## Default

When `FeePerOp` is not set in storage (e.g., a freshly initialized instance), `fee_per_op()` returns the compile-time default of **100 stroops**. This ensures already-deployed instances behave identically after an upgrade.

## Limitations

1. The fee is per-operation, not per-byte; large batches pay linearly with operation count.
2. There is no batch-level discount or tiered pricing.
3. Clients that cache the fee should call `fee_per_op()` at runtime rather than hardcoding the default, since the admin may adjust it via `set_fee_per_op`.

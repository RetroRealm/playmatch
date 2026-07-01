/// Hard cap on the number of items a single bulk request may carry. Uniform
/// across every bulk endpoint so callers learn one limit. A bulk request still
/// counts as one ordinary request against the per-IP rate limiter regardless of
/// how many items it carries.
pub const MAX_BULK_ITEMS: usize = 100;

/// Upper bound on per-item lookups run in parallel within a single bulk batch.
/// Keeps fan-out from a single batch bounded against the shared database and
/// cache pools.
pub const BULK_CONCURRENCY: usize = 8;

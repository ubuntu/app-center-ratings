# Coding standards

Judgement calls a reviewer checks that `cargo fmt`, `clippy` and CI cannot. Each rule names the bug it prevents.

## Testing

- **Discriminator columns get a neighbour.** When a query is scoped by a type column (e.g. `votes.package_type`), the test seeds a row of another type under the same key and asserts it is excluded. Without the neighbour, a missing `WHERE package_type = ...` still passes.
- **Enum rejection uses raw ints.** proto3 enums are open, so clients can send any `i32`. Test rejection by sending an out-of-range value directly, not only the named variants.
- **New request fields keep the legacy path tested.** When a request gains a field with a default, keep a test that omits it and asserts the old behaviour succeeds end to end. Existing clients send the omitted form.
- **Schema migrations round-trip.** A migration with a down file gets a test in `src/db/migration_tests.rs` style: seed realistic rows at the previous schema, migrate up, exercise the new code path, migrate down, assert the schema and the surviving data match.
- **DB tests are isolated.** Each test gets its own pool from `init_pool_from_uri_and_migrate`, never the global `POOL`: every `#[tokio::test]` runs its own runtime, and pooled connections die with the runtime that opened them. Tests that change the schema use a scratch database, never `ratings`.

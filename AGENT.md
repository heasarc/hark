# Agent Guidelines

## Project overview

`hark` is a Rust CLI for searching a bundled, offline HEASARC SQLite database and downloading public HEASARC data from AWS. The user-facing commands and examples are documented in [README.md](README.md); keep it aligned with changes to command behavior.

## Architecture

- `src/main.rs` embeds `hea.db.gz` at compile time, decompresses it into a temporary file at startup, opens the SQLite connection, and runs the rustyline prompt. `SUPPORTED_TABLES` is the canonical allowlist for catalog commands.
- `src/cli.rs` parses prompt input and dispatches to `src/commands.rs`. Keep aliases, argument parsing, and command help consistent with the handlers and README.
- `src/resolver.rs` resolves non-coordinate `query-table` positions through the Sesame service, mirroring astropy's `name_resolve` (plain-text `SNV` query, `%J ra dec` parsing, CDS then VizieR mirror fallback). Coordinate searches remain offline; name searches require network access.
- `src/commands.rs` implements help/about, table and column listing, cone searches, product-link construction, S3 downloads, and self-update.
- The database's `metainfo` table supplies table descriptions, column metadata/default ordering, and default search radii. Cone searches use the precomputed `__x_ra_dec`, `__y_ra_dec`, and `__z_ra_dec` coordinates.
- Product links and downloads target the public `nasa-heasarc` S3 bucket. Keep the table-specific path rules in `get_product_link` consistent with query results and the downloader.

## Build and validation

- Use `cargo check --locked` for a fast compile check and `cargo build --release` for a release build. For a specific release target, pass `--target <triple>` (for example, `aarch64-apple-darwin`).
- Run `cargo test` for unit and local HTTP resolver tests; add coverage for parser, query, or product-path changes as appropriate.
- The database is a large required build input embedded by `include_bytes!`; do not replace, truncate, or regenerate `hea.db.gz` casually. Treat changes to this file as intentional data changes and preserve its compressed database contents.
- If native compilation fails before Rust source is checked, inspect the active `CC`, `CXX`, `AR`, linker, and `PATH`. Toolchains injected by environments such as micromamba can conflict with the host SDK; verify using the system compiler/toolchain before attributing the failure to project code.

## Change conventions

- Update `SUPPORTED_TABLES` when adding or removing supported catalogs, and wire changes through the query/listing behavior, product-link mapping where applicable, command help, and README examples.
- Preserve the current split between prompt parsing in `cli.rs` and command/database operations in `commands.rs`.
- Keep numeric RA/Dec inputs on the existing local search path. Report resolver failures instead of silently substituting coordinates or continuing with a search.
- Keep SQL identifiers interpolated only from trusted internal values or validated table/column metadata. Bind user-provided values as SQLite parameters.
- Surface operational failures with the existing `anyhow::Result`/`?` flow or clear user-facing errors; avoid reporting failed downloads or queries as successful.
- Keep Cargo dependency changes reflected in `Cargo.lock`.

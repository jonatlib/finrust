# Junie Internal Workspace Handling (for this repository)

This repository is a Cargo workspace. Follow these rules to handle changes safely and consistently:

- Architecture:
    - Root package = the main application glueing everything together
    - Workspace libraries under workspace/* = separate implementation modules, which can exists and functions on their
      own.
- Kani usage:
    - Only in libraries (workspace/*). Gate code with cfg(kani).
    - Preferred locations: a dedicated src/verification.rs behind cfg(kani) or colocated small proofs next to verified
      code, also behind cfg(kani).
- Editing rules:
    - Keep changes minimal and localized to the correct crate.
    - Use modern Rust modules (no mod.rs unless required by structure).
- Checklists:
    - Adding a new library:
        1) `cargo new --lib crates/<name> --vcs none`
        3) Add docs, host tests, and optional Kani proofs behind cfg(kani)
- Writing tests

---

# FinRust project

## Rust

We are using modern Rust where mod.rs is not needed and you can directly write same filename and same directory name as
sub-module.

When there is missing implementation use `todo!(...)` macro and write a `// TODO ..` comment what the missing
functionality is.
(Use this for example in missing error handling, like `map_err(|_| todo!(...))`)

When there is only partial missing functionality but the code can function.
Don't use the `todo!` macro but still write down TODO comment.

When the functionality is not finished or not ideal for production grade,
write down `// FIXME ` and explain what is missing.

## Project standards

We are writing documented and tested code:
Documentation:

- All methods, structs, modules have a documentation string
- If possible we are writing in-line tests (using `#[cfg(tests)]`).
- Comments explaining directly what reading the line of code does are useless and must be omitted.
- Comments that describes complex logic inside functions must be added.
  Tests:
- We also write tests that actually call our code. Tests that does not touch anything outside of test module should be
  omitted. Also tests like `assert_eq!(20, 20);` are absolute useless.
- If you are not able to test the actual implementation, ignore those tests and maybe only comment that it is hard to
  test it.

We are keeping modules small and separated by application/business logic into submodules:

- One module should do a one thing
- We can combine multiple modules and struct for complex functionality (don't forget about modern Rust modules).

We are not re-implementing wheel,
so if there is existing crate we rather use that instead of writing our own implementations.

Also use correctly log level:

- Trace for every bit what is the app doing so we can read it as a story.
- Debug for more verbose logging which is still not suitable for production, but the app should not be affected by this.
- Info logs for what is the app doing which can be enabled in production, so it should not log for every little
  function. And not too often.
- Warnings for recoverable conditions and issues.
- Errors for not handled or not recoverable conditions.

## Project description

This repository contains the backend service for a powerful, self-hosted home finance tracking application. Built with
Rust, Axum, SeaORM, and Polars, this tool is designed for users who want granular control over their financial data,
robust forecasting capabilities, and a system based on sound accounting principles.

The core mission of this project is to provide a comprehensive and accurate view of your financial situation, both past
and future. It allows you to model your entire financial ecosystem—from various bank accounts and currencies to complex
recurring transactions—and then use that model to gain insights and forecast with precision.

## Workspace layout and build guide

This repository is a Cargo workspace composed of multiple crates. Here’s what lives where and how to build/run each part.

- Root crate: finrust (backend API and CLI)
  - Purpose: Axum 0.7 web server, router, handlers, OpenAPI (utoipa + Swagger UI), tracing, CORS/gzip/timeout (tower-http), configuration, and CLI (serve, init-db).
  - Database: SeaORM with SQLite and Postgres drivers enabled. Default local dev uses SQLite (e.g., sqlite://finrust.db).
  - OpenAPI/Swagger UI: exposed at /swagger-ui (served by utoipa-swagger-ui). See src/router.rs for integration.
  - Run (server):
    - Env vars (or CLI flags): DATABASE_URL (default sqlite://finrust.db), BIND_ADDRESS (default 0.0.0.0:3000).
    - Example: cargo run -- serve --database-url "sqlite://finrust.db" --bind-address "0.0.0.0:3000".
  - Init DB: cargo run -- init-db --database-url "sqlite://finrust.db".

- workspace/frontend: Yew SPA (WebAssembly) built with Trunk
  - Framework: Yew 0.21 (csr feature) + yew-router for SPA routing.
  - Styling: Tailwind CSS + DaisyUI via CDN in index.html.
  - Prerequisites:
    - rustup target add wasm32-unknown-unknown
    - cargo install trunk
    - cargo install wasm-bindgen-cli
  - Validate frontend code with Trunk (not cargo check):
    - Dev server: cd workspace/frontend && trunk serve (http://localhost:8080)
    - CI/local build: cd workspace/frontend && trunk build --release (artifacts in dist/)
    - Rationale: trunk compiles to wasm32-unknown-unknown and catches WASM-target issues that cargo check for host may miss.
  - API access from WASM:
    - Use gloo-net (http feature) or wasm-compatible reqwest.
    - Backend typically on 3000; ensure CORS is permissive in backend when running on different origins.
  - Logging in browser:
    - Initialize wasm-logger in the frontend entrypoint to see log::info!/warn!/error! in devtools console.
  - Routing:
    - BrowserRouter with SPA routes: e.g., /, /about, /admin.
    - When serving statically, ensure SPA fallback to index.html for non-root paths (e.g., web server rewrite).
  - Production tips:
    - Prefer environment-driven API base URL via Trunk asset pipelines or compile-time cfg if needed.
  - See workspace/frontend/README.md for additional details.

- workspace/compute: computation and analytics
  - Purpose: Converts model domain to Polars DataFrames; calculates balances, recurring items, merges; provides account statistics.
  - Key crates: polars (lazy, cum_agg), chrono, rust_decimal, rusty-money, SeaORM (where needed), async-trait.
  - Integration: Produces data that is converted to transport-friendly structs in common.
  - See workspace/compute/README.md.

- workspace/common: transport-layer types (no Polars)
  - Purpose: Lightweight, serde-serializable DTOs (requests/responses) for API and frontend. No Polars here.
  - Target compatibility (native vs WASM):
    - Keep dependencies WASM-friendly. Avoid server-only crates (e.g., tokio-specific FS, thread-heavy libs).
    - Allowed/common: serde, serde_json, chrono (serialize/deserialize only), rust_decimal, rusty-money.
    - Do NOT depend on polars or axum here. SeaORM entities live in workspace/model, not here.
  - Feature/cfg gating (when needed):
    - If you must include server-only traits (e.g., utoipa::ToSchema), gate them via optional feature flags or cfg(target_arch) blocks.
    - Example pattern:
      - Add an optional feature "schema" in common/Cargo.toml for utoipa; enable it in backend, keep it disabled in frontend.
      - Or, use cfg(target_arch = "wasm32") / cfg(not(target_arch = "wasm32")) to isolate impls.
  - Usage:
    - Define DTOs in common and reuse them in backend handlers and frontend API calls.
    - Prefer generic ApiResponse<T> shape here to mirror backend responses for easy deserialization in the frontend.
  - See workspace/common/README.md for details.

- workspace/model: core domain models and entities
  - Purpose: Transaction and related entities; SeaORM entities; traits like TransactionGenerator.
  - Used by compute and backend.
  - See workspace/model/README.md.

- workspace/migration: database migrations (SeaORM Migrator CLI)
  - For app users: cargo run init-db --database-url "sqlite://finrust.db" (from project root).
  - For developers: use cargo run in this crate for generate/up/down/fresh/status (see workspace/migration/README.md for exact commands).

Notes and conventions:
- Web framework: Axum 0.7 with tower/tower-http; OpenAPI via utoipa + utoipa-swagger-ui.
- Data/analytics: Polars used only in compute; API should expose common transport types, not Polars.
- Kani proofs: only in libraries under workspace/*; gate with cfg(kani). Prefer src/verification.rs or colocated proofs.
- Modern modules: avoid mod.rs unless structure requires it.

# LLM guidelines

If the project can't be compiled use `cargo check --message-format=json` to verify what are the compilation errors.
Be sure to use the JSON format as it will be much more readable for LLM.

# Documentations and code examples

Use context7 MCP.
When the user requests code examples, setup or configuration steps, or library/API documentation, use context7 MCP.
When investigating currently used crates, consult context7 for correct documentation!

You are a large language model equipped with a functional extension: Model Context Protocol (MCP) servers. You have been
configured with access to the following tool: Context7 - a software documentation finder, combined with the
SequentialThought chain-of-thought reasoning framework.

## Formal verification with Kani

We use the Kani model checker to verify properties (protocol, parsers, math). Guidance:

- Where to put proofs:
    - Prefer a dedicated module src/verification.rs gated by cfg(kani) for cross-cutting invariants.
    - It’s also OK to colocate small proofs next to the verified code, but always gate them with cfg(kani) so normal
      builds aren’t impacted.
- What to verify first:
    - Array access: indexing bounded by lengths (e.g., payload slices, parsers) to prevent OOB access.
    - Serialization/Deserialization round-trips and structural invariants.
    - Arithmetic: under realistic bounds, no overflows; use kani::assume to bound nondet inputs.
- Patterns:
    - Use #[kani::proof] harnesses and kani::{any, assume} to generate inputs and constrain them.
    - Prefer verifying small pure functions. If needed, add tiny helper/public functions to expose logic for
      verification. // TODO If refactoring is needed, keep it minimal and documented.
- Running Kani:
    - Install: see Kani docs. Typically: cargo install kani-verifier
    - Run all proofs: cargo kani -p <crate>
    - Run a specific proof: cargo kani -p <crate> --function verify_*


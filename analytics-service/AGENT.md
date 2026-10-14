# analytics-service conventions

Extends root `AGENT.md`.

- Idiom: follow *Effective Kotlin* (Moskała); prefer immutable data classes.
- Apply `/functional-programming-patterns` — the `analytics/fp/` package (`Either`, `RoutePipeline`) already does this; extend it rather than adding ad hoc error handling to new routes.
- Package layout target: flatten to one level under `com.web3analytics.*` — drop the `.analytics` middle segment (`com.web3analytics.analytics.route` → `com.web3analytics.route`, same for `config`, `health`, `stream`, `fp`, `model`, `store`). Current code doesn't match yet; apply on the planned rebuild, not as a drive-by rename.
- Mocking: MockK (already a dependency) — no change.

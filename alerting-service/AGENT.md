# alerting-service conventions

Extends root `AGENT.md`.

- Idiom: follow *Effective Kotlin* (Moskała); prefer immutable data classes.
- Package layout is flat under `com.web3analytics.alerting.*`: `config`, `consumer`, `model`, `store`,
  `websocket`. Keep new packages one level deep.
- Alerts are a sealed interface (`Alert`) with one implementation per Avro alert type; dispatch with
  `when` over the sealed hierarchy rather than string matching where the type is known. Wire-format
  dispatch on `alertType` happens only in `RedisAlertStore.deserializeAlert`.
- Coroutines: blocking Redis calls run under `Dispatchers.IO` inside `RedisAlertStore`. Keep the Ktor
  route handlers suspension-only; do not add blocking calls to them.
- Testing: pure-behavior tests against real fakes; no mocking framework. Inject dependencies
  (`AlertStore`, `AlertBroadcaster` with an overridden `broadcast`) rather than stubbing frameworks.
  There is currently one test, `RedisAlertStoreTest` — new behavior needs a test beside it.
- WebSocket filters live in `AlertFilters` and are applied server-side in `AlertBroadcaster`; a new
  alert type must be added to `AlertFilters.matches` or `chain` filtering silently breaks for it.

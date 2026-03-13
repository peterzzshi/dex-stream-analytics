# Functional Programming Patterns Skill

Recognize opportunities to eliminate duplication through higher-order functions and generics, encode behavior in types rather than runtime checks, separate pure logic from side effects, make invalid states unrepresentable, prefer immutability, and inject behavior as function parameters.

Apply these patterns incrementally, one at a time, with concrete before/after code.

---

## Core Patterns

### 1. Encode Behavior in Types
**Detect:** Logic branches on `type`, `kind`, `severity` fields via if/switch chains. Helper functions like `isX()` that only classify and then branch.

**Refactor:** Replace runtime field checks with distinct types. Let the type system enforce exhaustive handling. Once behavior is encoded in types, classification helpers become redundant, so match directly at call sites.

```
// Before: runtime branching on a string field
handle(error) {
    if error.type == "retryable" -> retry()
    if error.type == "fatal"     -> abort()
}

// After: each variant is a distinct type, compiler enforces all cases handled
type AppError = RetryableError | FatalError | WarningError

handle(error: AppError) -> match on type
```

**Guideline:** Group typed errors by **domain cause** (config, data, connection), not by fatality.

**Go caveat:** Prefer `errors.As` to handle wrapped errors. Avoid `switch err.(type)` for wrapped chains.

---

### 2. Extract Isomorphic Structure
**Detect:** Multiple functions with identical control flow but different parameters or types.
**Keywords:** `parseX`, `buildX`, `handleX`, `processX` with the same shape.

**Refactor:** Extract the common structure and parameterize the differences.

```
// Before: two functions with identical structure
processA(data) -> unmarshal into RawA -> build TypeA
processB(data) -> unmarshal into RawB -> build TypeB

// After: one generic function, callers supply the varying parts
process[T, R](data, parse: bytes -> T, build: T -> R) -> R
```

**When only the output type differs**, generics alone are enough, no higher-order function needed:
```
decode[T](data, target: *T) -> unmarshal into target
```

---

### 3. Separate Pure from Impure
**Detect:** I/O mixed with business logic. Calculations interleaved with persistence or network calls. Mutable config structs where stateless accessors would suffice.

**Refactor:** Extract pure functions. Orchestrators call pure steps, then delegate I/O. Prefer immutable values and stateless accessors over mutable state.

```
// Before: I/O and logic interleaved
calculateTotal(id) {
    data = db.get(id)               // I/O
    total = data.amount * data.rate // pure
    db.save(id, total)              // I/O
}

// After: pure function + thin orchestrator
calculateTotal(amount, rate) -> amount * rate   // pure, testable

updateTotal(id) {
    data = db.get(id)
    total = calculateTotal(data.amount, data.rate)
    db.save(id, total)
}
```

**For complex orchestrators**, decompose into named pure steps, each independently testable:

```
orchestrate(input, doIO) {
    prepared = prepare(input)   // pure
    encoded  = encode(prepared) // pure
    doIO(encoded)               // impure, injected
}
```

**Prefer stateless accessors** over mutable config structs:
```
getPort() -> readEnvInt("PORT") // stateless accessor; validation lives in readEnvInt
```

---

### 4. Function Types as Dependencies
**Detect:** Single-method interfaces used primarily for mocking. Struct fields that wrap a dependency and call one method.

**Refactor:** Replace with **named function types**. Inject behavior as parameters. Use partial application to close over heavy dependencies.

```
// Before: interface with one method, struct holds it
interface Caller { call(ctx, msg) -> bytes }
struct Service { client: Caller }
Service.fetch(ctx) -> client.call(ctx, msg)

// After: function type, passed directly
type ContractCaller = (ctx, msg) -> bytes

fetch(ctx, caller: ContractCaller) -> caller(ctx, msg)

// Partial application: close over a dependency
newFetcher(caller) -> (ctx) -> fetch(ctx, caller)
```

**Testing becomes trivial, no mock framework needed:**
```
mockCaller = (ctx, msg) -> expectedBytes
result = fetch(ctx, mockCaller)

// Capture side effects via closure
var captured
spyCaller = (ctx, msg) -> { captured = msg; return nil }
```

---

## Advanced Patterns

### 5. Explicit Result Types
**Detect:** Mixed error handling, exceptions for control flow, sentinel values (`-1`, `null`), or unclear propagation.

**Refactor:** Use explicit Result/Either to make success/failure paths visible and composable.

```
divide(a, b) -> Result<number, string>
    if b == 0 -> Failure("division by zero")
    else      -> Success(a / b)

// Chain: short-circuits on first failure
divide(10, 2).flatMap(x -> divide(x, 0)).map(x -> x * 2)
```

**Language note:** Go's `(T, error)` is already a Result type. Use it idiomatically.

---

### 6. Tri-State Optional
**Detect:** Need to distinguish "not set" vs "explicitly null" vs "has value".
**Keywords:** API partial updates, async resolution states, serialization control.

```
type TriState<T> = Defined(value: T) | Undefined | Null

fold(triState, onDefined, onUndefined, onNull) -> R
```

---

## Constraints

**When NOT to refactor:**
- Less than 3 instances (rule of three)
- Rapidly changing code (wait for stability)
- Performance-critical paths (profile first)
- Domain boundaries (duplication can be acceptable)

**Always:**
- Keep behavior identical (no feature additions)
- Run tests after each change
- Refactor one pattern at a time

---

## Language Idioms

**Go:** Generics `[T any]`, named function types as dependencies, partial application, `errors.As` for type-based handling, `(T, error)` as Result type, closures for test injection

**TypeScript:** Discriminated unions, `readonly`, type guards, `as const`, higher-order functions for DI

**Scala:** Sealed traits, case classes, exhaustive matching, `Either[E, A]`, `Option[T]`

**Java:** Records, sealed classes (17+), pattern matching in switch (21+), custom Result types

**Kotlin:** Sealed classes, data classes, `when` exhaustiveness, `Result<T>`

**Rust:** Enums with data, `match`, `Result<T, E>`, `Option<T>`, trait objects

**Swift:** Enums with associated values, protocols, `let` immutability, `Result<Success, Failure>`

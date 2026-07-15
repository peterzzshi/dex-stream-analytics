package com.web3analytics.analytics.fp

/**
 * Typed error handling without exceptions.
 *
 * Either<E, T> is a sum type: computation is either Left(error) or Right(value).
 * Errors propagate through map/flatMap without branching at every step —
 * the pipeline runs to completion and handles the error once at the boundary.
 */
sealed class Either<out E, out T> {
    data class Left<out E>(val value: E) : Either<E, Nothing>()
    data class Right<out T>(val value: T) : Either<Nothing, T>()

    inline fun <U> map(transform: (T) -> U): Either<E, U> = when (this) {
        is Left -> this
        is Right -> Right(transform(value))
    }

    inline fun <U> flatMap(transform: (T) -> Either<@UnsafeVariance E, U>): Either<E, U> = when (this) {
        is Left -> this
        is Right -> transform(value)
    }

    inline fun <R> fold(onLeft: (E) -> R, onRight: (T) -> R): R = when (this) {
        is Left -> onLeft(value)
        is Right -> onRight(value)
    }

    fun getOrNull(): T? = when (this) {
        is Left -> null
        is Right -> value
    }
}

fun <T> T.right(): Either<Nothing, T> = Either.Right(this)
fun <E> E.left(): Either<E, Nothing> = Either.Left(this)

inline fun <T> catching(block: () -> T): Either<Throwable, T> =
    try { Either.Right(block()) } catch (e: Exception) { Either.Left(e) }

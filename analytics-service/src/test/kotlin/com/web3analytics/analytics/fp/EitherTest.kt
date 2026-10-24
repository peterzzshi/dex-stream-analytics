package com.web3analytics.analytics.fp

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EitherTest {
    @Test
    fun `map transforms a Right value and leaves Left untouched`() {
        val right: Either<String, Int> = 1.right()
        val left: Either<String, Int> = "boom".left()

        assertEquals(2.right(), right.map { it + 1 })
        assertEquals(left, left.map { it + 1 })
    }

    @Test
    fun `flatMap chains Right values and short-circuits on the first Left`() {
        val start: Either<String, Int> = 1.right()
        val afterFirstLeft: Either<String, String> =
            start
                .flatMap { (it + 1).right() }
                .flatMap { "stop: $it".left() }

        val result: Either<String, String> =
            afterFirstLeft.flatMap { throw AssertionError("should not run after a Left") }

        assertEquals(Either.Left("stop: 2"), result)
    }

    @Test
    fun `fold calls the matching branch exactly once`() {
        var rightCalls = 0
        var leftCalls = 0
        val right: Either<String, Int> = 5.right()
        val left: Either<String, Int> = "err".left()

        right.fold(onLeft = { leftCalls++ }, onRight = { rightCalls++ })
        left.fold(onLeft = { leftCalls++ }, onRight = { rightCalls++ })

        assertEquals(1, rightCalls)
        assertEquals(1, leftCalls)
    }

    @Test
    fun `getOrNull unwraps Right and discards Left`() {
        val right: Either<String, Int> = 42.right()
        val left: Either<String, Int> = "err".left()

        assertEquals(42, right.getOrNull())
        assertEquals(null, left.getOrNull())
    }

    @Test
    fun `catching captures a thrown exception as Left without propagating it`() {
        val result = catching { throw IllegalStateException("kaboom") }

        assertTrue(result is Either.Left)
        assertEquals("kaboom", (result as Either.Left).value.message)
    }

    @Test
    fun `catching returns Right when the block succeeds`() {
        val result = catching { 10 / 2 }

        assertEquals(Either.Right(5), result)
    }
}

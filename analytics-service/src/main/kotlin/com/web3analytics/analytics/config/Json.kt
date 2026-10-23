package com.web3analytics.analytics.config

import com.fasterxml.jackson.databind.DeserializationFeature
import com.fasterxml.jackson.databind.ObjectMapper
import com.fasterxml.jackson.databind.SerializationFeature
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule
import com.fasterxml.jackson.module.kotlin.registerKotlinModule

/**
 * Single source of truth for JSON (de)serialization configuration.
 *
 * [jacksonConfig] is applied both to the standalone [ObjectMapper] used by the
 * subscription handlers and to Ktor's ContentNegotiation mapper, so both stay
 * in lockstep.
 */
val jacksonConfig: ObjectMapper.() -> Unit = {
    registerKotlinModule()
    registerModule(JavaTimeModule())
    disable(SerializationFeature.WRITE_DATES_AS_TIMESTAMPS)
    disable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES)
}

fun analyticsObjectMapper(): ObjectMapper = ObjectMapper().apply(jacksonConfig)

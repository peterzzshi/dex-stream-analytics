package com.web3analytics.analytics.route

import io.ktor.server.application.call
import io.ktor.server.html.respondHtml
import io.ktor.server.routing.Route
import io.ktor.server.routing.get
import kotlinx.html.*

private const val METRIC_ROW_CLASS = "metric-row"

/**
 * Serves the real-time analytics dashboard using Ktor's HTML DSL.
 *
 * Demonstrates Kotlin type-safe builders — the HTML is constructed using
 * Kotlin lambda-with-receiver syntax, providing compile-time safety
 * (no unclosed tags, no typos in attribute names). CSS and JS live as static
 * assets under resources/static rather than embedded strings.
 */
fun Route.dashboardRoutes() {
    get("/dashboard") {
        call.respondHtml {
            head {
                title("DEX Stream Analytics")
                meta { charset = "utf-8" }
                meta {
                    name = "viewport"
                    content = "width=device-width, initial-scale=1"
                }
                link(rel = "stylesheet", href = "/static/dashboard.css")
            }
            body {
                div("container") {
                    header {
                        h1 { +"DEX Stream Analytics" }
                        p("subtitle") { +"Real-time pool health monitoring" }
                        div("connection-status") {
                            span("status-dot") { id = "status-dot" }
                            span {
                                id = "status-text"
                                +"Connecting..."
                            }
                        }
                    }

                    div("grid") {
                        // Pool Health Card
                        section("card") {
                            h2 { +"Pool Health" }
                            div(METRIC_ROW_CLASS) {
                                metricBox("overall-score", "Overall", "—")
                                metricBox("trading-score", "Trading", "—")
                                metricBox("liquidity-score", "Liquidity", "—")
                                metricBox("safety-score", "Safety", "—")
                            }
                            div("trend-badge") {
                                id = "trend-badge"
                                +"UNKNOWN"
                            }
                        }

                        // Trading Card
                        section("card") {
                            h2 { +"Trading Activity" }
                            div(METRIC_ROW_CLASS) {
                                metricBox("twap", "TWAP", "—")
                                metricBox("volume", "Volume USD", "—")
                                metricBox("swap-count", "Swaps", "—")
                                metricBox("traders", "Traders", "—")
                            }
                            div("price-bar") {
                                div("ohlc") {
                                    id = "ohlc"
                                    +"O: — H: — L: — C: —"
                                }
                            }
                        }

                        // MEV Alerts Card
                        section("card alert-card") {
                            h2 { +"MEV Alerts" }
                            div("alert-count") {
                                span("big-number") {
                                    id = "alert-count"
                                    +"0"
                                }
                                span { +" alerts (24h)" }
                            }
                            ul("alert-feed") { id = "alert-feed" }
                        }

                        // Market Trends Card
                        section("card") {
                            h2 { +"Market Trends" }
                            div(METRIC_ROW_CLASS) {
                                metricBox("price-change", "Price Δ", "—")
                                metricBox("volatility", "Volatility", "—")
                            }
                            ul("trend-feed") { id = "trend-feed" }
                        }
                    }

                    // Event Log
                    section("card log-card") {
                        h2 {
                            +"Live Event Stream"
                            span("event-counter") {
                                id = "event-counter"
                                +"0 events"
                            }
                        }
                        div("event-log") { id = "event-log" }
                    }
                }

                script(src = "/static/dashboard.js") {}
            }
        }
    }
}

private fun FlowContent.metricBox(
    id: String,
    label: String,
    initial: String,
) {
    div("metric-box") {
        div("metric-value") {
            this.id = id
            +initial
        }
        div("metric-label") { +label }
    }
}

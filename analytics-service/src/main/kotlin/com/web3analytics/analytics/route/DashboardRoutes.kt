package com.web3analytics.analytics.route

import io.ktor.server.application.call
import io.ktor.server.html.respondHtml
import io.ktor.server.routing.Route
import io.ktor.server.routing.get
import kotlinx.html.*

/**
 * Serves the real-time analytics dashboard using Ktor's HTML DSL.
 *
 * Demonstrates Kotlin type-safe builders — the HTML is constructed using
 * Kotlin lambda-with-receiver syntax, providing compile-time safety
 * (no unclosed tags, no typos in attribute names).
 */
fun Route.dashboardRoutes() {

    get("/dashboard") {
        call.respondHtml {
            head {
                title("DEX Stream Analytics")
                meta { charset = "utf-8" }
                meta { name = "viewport"; content = "width=device-width, initial-scale=1" }
                style { unsafe { raw(DASHBOARD_CSS) } }
            }
            body {
                div("container") {
                    header {
                        h1 { +"DEX Stream Analytics" }
                        p("subtitle") { +"Real-time pool health monitoring" }
                        div("connection-status") {
                            span("status-dot") { id = "status-dot" }
                            span { id = "status-text"; +"Connecting..." }
                        }
                    }

                    div("grid") {
                        // Pool Health Card
                        section("card") {
                            h2 { +"Pool Health" }
                            div("metric-row") {
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
                            div("metric-row") {
                                metricBox("twap", "TWAP", "—")
                                metricBox("volume", "Volume USD", "—")
                                metricBox("swap-count", "Swaps", "—")
                                metricBox("traders", "Traders", "—")
                            }
                            div("price-bar") {
                                div("ohlc") { id = "ohlc"; +"O: — H: — L: — C: —" }
                            }
                        }

                        // MEV Alerts Card
                        section("card alert-card") {
                            h2 { +"MEV Alerts" }
                            div("alert-count") {
                                span("big-number") { id = "alert-count"; +"0" }
                                span { +" alerts (24h)" }
                            }
                            ul("alert-feed") { id = "alert-feed" }
                        }

                        // Market Trends Card
                        section("card") {
                            h2 { +"Market Trends" }
                            div("metric-row") {
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
                            span("event-counter") { id = "event-counter"; +"0 events" }
                        }
                        div("event-log") { id = "event-log" }
                    }
                }

                script { unsafe { raw(DASHBOARD_JS) } }
            }
        }
    }
}

private fun FlowContent.metricBox(id: String, label: String, initial: String) {
    div("metric-box") {
        div("metric-value") { this.id = id; +initial }
        div("metric-label") { +label }
    }
}

// language=CSS
private val DASHBOARD_CSS = """
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
       background: #0a0e17; color: #e0e6f0; }
.container { max-width: 1200px; margin: 0 auto; padding: 24px; }
header { display: flex; align-items: baseline; gap: 16px; margin-bottom: 24px; flex-wrap: wrap; }
h1 { font-size: 24px; color: #fff; }
.subtitle { color: #6b7a99; font-size: 14px; }
.connection-status { margin-left: auto; display: flex; align-items: center; gap: 6px; font-size: 13px; }
.status-dot { width: 8px; height: 8px; border-radius: 50%; background: #f59e0b; display: inline-block; }
.status-dot.connected { background: #10b981; }
.status-dot.disconnected { background: #ef4444; }
.grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 16px; margin-bottom: 16px; }
.card { background: #111827; border: 1px solid #1e293b; border-radius: 12px; padding: 20px; }
.card h2 { font-size: 15px; color: #94a3b8; margin-bottom: 16px; display: flex; justify-content: space-between; }
.metric-row { display: flex; gap: 12px; flex-wrap: wrap; }
.metric-box { flex: 1; min-width: 80px; text-align: center; padding: 12px 8px;
              background: #1a2235; border-radius: 8px; }
.metric-value { font-size: 20px; font-weight: 700; color: #fff; font-variant-numeric: tabular-nums; }
.metric-label { font-size: 11px; color: #6b7a99; margin-top: 4px; text-transform: uppercase; letter-spacing: 0.5px; }
.trend-badge { display: inline-block; margin-top: 12px; padding: 4px 12px; border-radius: 20px;
               font-size: 12px; font-weight: 600; background: #1e293b; color: #94a3b8; }
.trend-badge.BULLISH { background: #064e3b; color: #6ee7b7; }
.trend-badge.BEARISH { background: #7f1d1d; color: #fca5a5; }
.trend-badge.NEUTRAL { background: #1e293b; color: #94a3b8; }
.ohlc { font-size: 13px; color: #6b7a99; margin-top: 12px; font-family: 'SF Mono', monospace; }
.alert-card { border-color: #7f1d1d; }
.alert-count { margin-bottom: 12px; }
.big-number { font-size: 28px; font-weight: 700; color: #f87171; }
.alert-feed, .trend-feed { list-style: none; max-height: 120px; overflow-y: auto; }
.alert-feed li, .trend-feed li { font-size: 12px; padding: 6px 0; border-bottom: 1px solid #1e293b;
                                  font-family: 'SF Mono', monospace; color: #94a3b8; }
.alert-feed li .severity { font-weight: 600; margin-right: 6px; }
.severity.HIGH { color: #ef4444; }
.severity.MEDIUM { color: #f59e0b; }
.severity.LOW { color: #6b7a99; }
.log-card { grid-column: span 2; }
.event-log { max-height: 200px; overflow-y: auto; font-size: 12px; font-family: 'SF Mono', monospace;
             background: #0d1117; border-radius: 6px; padding: 12px; line-height: 1.6; }
.event-log .entry { color: #6b7a99; }
.event-log .channel-trading { color: #60a5fa; }
.event-log .channel-liquidity { color: #a78bfa; }
.event-log .channel-mev { color: #f87171; }
.event-log .channel-trend { color: #34d399; }
.event-counter { font-size: 12px; color: #475569; font-weight: 400; }
@media (max-width: 768px) { .grid { grid-template-columns: 1fr; } .log-card { grid-column: span 1; } }
""".trimIndent()

// language=JavaScript
private val DASHBOARD_JS = """
(function() {
    const ${'$'} = id => document.getElementById(id);
    let eventCount = 0;
    let alertCount = 0;

    function connect() {
        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        const ws = new WebSocket(proto + '//' + location.host + '/ws/analytics');

        ws.onopen = () => {
            ${'$'}('status-dot').className = 'status-dot connected';
            ${'$'}('status-text').textContent = 'Connected';
            appendLog('system', 'WebSocket connected');
        };

        ws.onclose = () => {
            ${'$'}('status-dot').className = 'status-dot disconnected';
            ${'$'}('status-text').textContent = 'Reconnecting...';
            appendLog('system', 'Disconnected — reconnecting in 3s');
            setTimeout(connect, 3000);
        };

        ws.onerror = () => ws.close();

        ws.onmessage = (e) => {
            eventCount++;
            ${'$'}('event-counter').textContent = eventCount + ' events';
            try {
                const msg = JSON.parse(e.data);
                handleEvent(msg);
                appendLog(msg.channel, JSON.stringify(msg.data).substring(0, 120));
            } catch (err) {
                appendLog('system', 'Parse error: ' + err.message);
            }
        };
    }

    function handleEvent(msg) {
        const d = msg.data;
        switch (msg.channel) {
            case 'trading':
                setText('twap', fmt(d.twap, 4));
                setText('volume', '$' + fmt(d.volumeUSD, 0));
                setText('swap-count', d.swapCount ?? '—');
                setText('traders', d.uniqueTraders ?? '—');
                ${'$'}('ohlc').textContent = 'O: ' + fmt(d.openPrice,4) + '  H: ' + fmt(d.highPrice,4) +
                    '  L: ' + fmt(d.lowPrice,4) + '  C: ' + fmt(d.closePrice,4);
                break;
            case 'liquidity':
                break;
            case 'mev':
                alertCount++;
                ${'$'}('alert-count').textContent = alertCount;
                prependToFeed('alert-feed',
                    '<span class="severity ' + (d.severity||'') + '">' + (d.severity||'?') + '</span>' +
                    (d.alertType||'unknown') + ' — ' + (d.pairAddress||'').substring(0,10) + '...');
                break;
            case 'trend':
                setText('price-change', fmt(d.priceChangePercent, 2) + '%');
                setText('volatility', fmt(d.volatility, 4));
                const badge = ${'$'}('trend-badge');
                badge.textContent = d.trend || 'UNKNOWN';
                badge.className = 'trend-badge ' + (d.trend || '');
                prependToFeed('trend-feed', (d.trend||'?') + ' Δ' + fmt(d.priceChangePercent,2) +
                    '% vol=' + fmt(d.volatility,4));
                break;
        }
    }

    function fmt(v, decimals) { return v != null ? Number(v).toFixed(decimals) : '—'; }
    function setText(id, val_) { const el = ${'$'}(id); if (el) el.textContent = val_; }

    function prependToFeed(id, html) {
        const ul = ${'$'}(id);
        if (!ul) return;
        const li = document.createElement('li');
        li.innerHTML = html;
        ul.prepend(li);
        while (ul.children.length > 20) ul.lastChild.remove();
    }

    function appendLog(channel, text) {
        const log = ${'$'}('event-log');
        if (!log) return;
        const div = document.createElement('div');
        div.className = 'entry channel-' + channel;
        const ts = new Date().toLocaleTimeString();
        div.textContent = ts + ' [' + channel + '] ' + text;
        log.appendChild(div);
        log.scrollTop = log.scrollHeight;
        while (log.children.length > 100) log.firstChild.remove();
    }

    // Initial data load from REST API
    fetch('/analytics/summary').then(r => r.json()).then(s => {
        appendLog('system', 'Loaded: ' + (s.tradingPairCount||0) + ' trading pairs, ' +
            (s.totalTradingWindows||0) + ' windows');
    }).catch(() => {});

    connect();
})();
""".trimIndent()

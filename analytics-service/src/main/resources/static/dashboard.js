(function() {
    const $ = id => document.getElementById(id);
    let eventCount = 0;
    let alertCount = 0;

    function connect() {
        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        const ws = new WebSocket(proto + '//' + location.host + '/ws/analytics');

        ws.onopen = () => {
            $('status-dot').className = 'status-dot connected';
            $('status-text').textContent = 'Connected';
            appendLog('system', 'WebSocket connected');
        };

        ws.onclose = () => {
            $('status-dot').className = 'status-dot disconnected';
            $('status-text').textContent = 'Reconnecting...';
            appendLog('system', 'Disconnected — reconnecting in 3s');
            setTimeout(connect, 3000);
        };

        ws.onerror = () => ws.close();

        ws.onmessage = (e) => {
            eventCount++;
            $('event-counter').textContent = eventCount + ' events';
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
                $('ohlc').textContent = 'O: ' + fmt(d.openPrice,4) + '  H: ' + fmt(d.highPrice,4) +
                    '  L: ' + fmt(d.lowPrice,4) + '  C: ' + fmt(d.closePrice,4);
                break;
            case 'liquidity':
                break;
            case 'mev':
                alertCount++;
                $('alert-count').textContent = alertCount;
                prependToFeed('alert-feed',
                    '<span class="severity ' + (d.severity||'') + '">' + (d.severity||'?') + '</span>' +
                    (d.alertType||'unknown') + ' — ' + (d.pairAddress||'').substring(0,10) + '...');
                break;
            case 'trend': {
                setText('price-change', fmt(d.priceChangePercent, 2) + '%');
                setText('volatility', fmt(d.volatility, 4));
                const badge = $('trend-badge');
                badge.textContent = d.trend || 'UNKNOWN';
                badge.className = 'trend-badge ' + (d.trend || '');
                prependToFeed('trend-feed', (d.trend||'?') + ' Δ' + fmt(d.priceChangePercent,2) +
                    '% vol=' + fmt(d.volatility,4));
                break;
            }
            default:
                // Mirrors AnalyticsChannel.wireName on the server by hand (static JS
                // can't reference the Kotlin enum) — an unmatched channel means the
                // two have drifted, so surface it loudly instead of dropping silently.
                console.warn('Unknown analytics channel:', msg.channel);
                appendLog('system', 'Unknown channel: ' + msg.channel);
        }
    }

    function fmt(v, decimals) { return v != null ? Number(v).toFixed(decimals) : '—'; }
    function setText(id, val_) { const el = $(id); if (el) el.textContent = val_; }

    function prependToFeed(id, html) {
        const ul = $(id);
        if (!ul) return;
        const li = document.createElement('li');
        li.innerHTML = html;
        ul.prepend(li);
        while (ul.children.length > 20) ul.lastChild.remove();
    }

    function appendLog(channel, text) {
        const log = $('event-log');
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
        appendLog('system', 'Loaded: ' + (s.tradingPairsTracked||0) + ' trading pairs, ' +
            (s.totalTradingWindows||0) + ' windows');
    }).catch(() => {});

    connect();
})();

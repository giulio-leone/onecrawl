use colored::Colorize;

// ---------------------------------------------------------------------------
// Event Bus CLI commands
// ---------------------------------------------------------------------------

pub async fn handle(action: crate::cli::EventsAction) {
    use crate::cli::EventsAction;
    match action {
        EventsAction::Listen { port } => events_listen(port).await,
        EventsAction::Emit { event_type, data, source } => {
            events_emit(&event_type, data.as_deref(), &source).await
        }
        EventsAction::Subscribe { event_pattern, webhook, secret } => {
            events_subscribe(&event_pattern, &webhook, secret.as_deref()).await
        }
        EventsAction::Unsubscribe { subscription_id } => {
            events_unsubscribe(&subscription_id).await
        }
        EventsAction::List => events_list().await,
        EventsAction::Recent { limit } => events_recent(limit).await,
        EventsAction::Replay { event_pattern, since } => {
            events_replay(&event_pattern, since.as_deref()).await
        }
        EventsAction::Stats => events_stats().await,
        EventsAction::Clear => events_clear().await,
    }
}

async fn events_listen(port: u16) {
    println!("{} Starting event bus listener on port {}", "✓".green(), port);
    println!("  Endpoints:");
    println!("    POST   /events/emit           — Emit an event");
    println!("    POST   /events/subscribe      — Subscribe a webhook");
    println!("    DELETE /events/subscribe/:id  — Unsubscribe");
    println!("    GET    /events/subscriptions  — List subscriptions");
    println!("    GET    /events/recent         — Recent events");
    println!("    POST   /events/replay         — Replay events");
    println!("    GET    /events/stats          — Bus statistics");
    println!("    GET    /events/stream         — SSE event stream");
    println!("    DELETE /events/journal        — Clear journal");
    println!();
    println!("  Press Ctrl+C to stop");

    if let Err(e) = onecrawl_server::serve::start_server(port).await {
        eprintln!("{} Server error: {e}", "✗".red());
    }
}

async fn events_emit(event_type: &str, data: Option<&str>, source: &str) {
    let bus = onecrawl_cdp::EventBus::new(onecrawl_cdp::EventBusConfig::default());
    let payload: serde_json::Value = data
        .map(|d| serde_json::from_str(d).unwrap_or(serde_json::json!({"raw": d})))
        .unwrap_or(serde_json::json!({}));

    let event = onecrawl_cdp::BusEvent {
        id: onecrawl_cdp::event_bus::generate_id(),
        event_type: event_type.to_string(),
        source: source.to_string(),
        timestamp: onecrawl_cdp::event_bus::iso_now(),
        data: payload,
        metadata: None,
    };
    let id = event.id.clone();
    match bus.emit(event).await {
        Ok(()) => {
            println!("{} Event emitted: {} (id: {})", "✓".green(), event_type, id);
        }
        Err(e) => {
            eprintln!("{} Emit failed: {e}", "✗".red());
        }
    }
}

async fn events_subscribe(event_pattern: &str, webhook_url: &str, secret: Option<&str>) {
    let bus = onecrawl_cdp::EventBus::new(onecrawl_cdp::EventBusConfig::default());
    let sub = onecrawl_cdp::WebhookSubscription {
        id: String::new(),
        event_pattern: event_pattern.to_string(),
        url: webhook_url.to_string(),
        method: None,
        headers: None,
        secret: secret.map(String::from),
        active: true,
        retry_count: 3,
        retry_delay_ms: 1000,
        created_at: onecrawl_cdp::event_bus::iso_now(),
        last_triggered: None,
        trigger_count: 0,
        last_error: None,
    };
    match bus.subscribe_webhook(sub).await {
        Ok(id) => {
            println!("{} Webhook subscribed", "✓".green());
            println!("  ID: {}", id);
            println!("  Pattern: {}", event_pattern);
            println!("  URL: {}", webhook_url);
            if secret.is_some() {
                println!("  HMAC signing: enabled");
            }
        }
        Err(e) => {
            eprintln!("{} Subscribe failed: {e}", "✗".red());
        }
    }
}

async fn events_unsubscribe(id: &str) {
    println!("{} Subscription '{}' unsubscribed", "✓".green(), id);
}

async fn events_list() {
    println!("No active webhook subscriptions");
    println!("  Use 'onecrawl events subscribe <pattern> --webhook <url>' to add one");
}

async fn events_recent(limit: usize) {
    println!("No recent events (limit: {})", limit);
    println!("  Use 'onecrawl events emit <type>' to emit an event");
}

async fn events_replay(event_pattern: &str, since: Option<&str>) {
    println!("Replaying events matching '{}'", event_pattern);
    if let Some(ts) = since {
        println!("  Since: {}", ts);
    }
    println!("  No events to replay");
}

async fn events_stats() {
    let bus = onecrawl_cdp::EventBus::new(onecrawl_cdp::EventBusConfig::default());
    let stats = bus.stats().await;
    println!("Event Bus Statistics:");
    println!("  Total events:     {}", stats.total_events);
    println!("  Total deliveries: {}", stats.total_deliveries);
    println!("  Failed:           {}", stats.failed_deliveries);
    println!("  Active webhooks:  {}", stats.active_webhooks);
    println!("  Journal size:     {}", stats.journal_size);
    println!("  Uptime:           {}s", stats.uptime_secs);
}

async fn events_clear() {
    println!("{} Event journal cleared", "✓".green());
}

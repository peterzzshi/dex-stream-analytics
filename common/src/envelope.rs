use crate::EventMeta;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use cloudevents::{Event as CloudEvent, EventBuilder, EventBuilderV10};

pub const AVRO_CONTENT_TYPE: &str = "application/avro-binary";
pub const DLQ_EVENT_TYPE: &str = "com.bridge.events.dlq";

/// Avro payload carried as `data_base64`, `time` taken from the event timestamp (seconds).
pub fn event_envelope(source: &str, meta: &EventMeta, avro_payload: Vec<u8>) -> Result<CloudEvent> {
    let time = DateTime::from_timestamp(meta.event_timestamp, 0).ok_or_else(|| {
        anyhow!(
            "event_timestamp {} is not a valid unix timestamp",
            meta.event_timestamp
        )
    })?;

    Ok(EventBuilderV10::new()
        .id(meta.event_id.clone())
        .source(source)
        .ty(meta.cloudevent_type)
        .subject(meta.pair_address.clone())
        .time(time)
        .data(AVRO_CONTENT_TYPE, avro_payload)
        .build()?)
}

/// JSON payload carried as `data`; the id is `<event_id>-dlq`.
pub fn dlq_envelope(
    source: &str,
    meta: &EventMeta,
    failed_at: DateTime<Utc>,
    payload: serde_json::Value,
) -> Result<CloudEvent> {
    Ok(EventBuilderV10::new()
        .id(format!("{}-dlq", meta.event_id))
        .source(source)
        .ty(DLQ_EVENT_TYPE)
        .subject(meta.pair_address.clone())
        .time(failed_at)
        .data("application/json", payload)
        .build()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn meta() -> EventMeta {
        EventMeta {
            event_id: "evt-1".to_string(),
            block_number: 100,
            pair_address: "0xbridge".to_string(),
            cloudevent_type: "com.bridge.events.lock",
            event_timestamp: 1_700_000_000,
        }
    }

    fn to_json(event: &CloudEvent) -> Value {
        serde_json::to_value(event).expect("envelope must serialize")
    }

    #[test]
    fn event_envelope_carries_avro_as_base64_with_routing_attributes() {
        let envelope = event_envelope("evm-ingester/x", &meta(), vec![1, 2, 3]).unwrap();
        let json = to_json(&envelope);

        assert_eq!(json["specversion"], "1.0");
        assert_eq!(json["id"], "evt-1");
        assert_eq!(json["source"], "evm-ingester/x");
        assert_eq!(json["type"], "com.bridge.events.lock");
        assert_eq!(json["subject"], "0xbridge");
        assert_eq!(json["datacontenttype"], AVRO_CONTENT_TYPE);
        assert_eq!(json["data_base64"], "AQID");
        assert!(json.get("data").is_none());
    }

    #[test]
    fn event_envelope_rejects_out_of_range_timestamp() {
        let mut m = meta();
        m.event_timestamp = i64::MAX;
        let error = event_envelope("s", &m, vec![0]).unwrap_err();
        assert!(
            error.to_string().contains("not a valid unix timestamp"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn dlq_envelope_carries_json_data_and_dlq_type() {
        let failed_at = DateTime::from_timestamp(1_700_000_500, 0).unwrap();
        let envelope =
            dlq_envelope("s/dlq", &meta(), failed_at, json!({ "error": "boom" })).unwrap();
        let json = to_json(&envelope);

        assert_eq!(json["id"], "evt-1-dlq");
        assert_eq!(json["type"], DLQ_EVENT_TYPE);
        assert_eq!(json["subject"], "0xbridge");
        assert_eq!(json["datacontenttype"], "application/json");
        assert_eq!(json["data"]["error"], "boom");
        assert!(json.get("data_base64").is_none());
    }
}

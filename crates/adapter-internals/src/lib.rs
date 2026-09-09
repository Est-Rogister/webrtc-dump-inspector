use chrono::DateTime;
use rtc_domain::{
    DumpFormat, ImportIssue, RtcEvent, RtcSession, SourceInfo, SourceRef, StatsSample,
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::Read;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InternalsError {
    #[error("failed to parse webrtc-internals JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("webrtc-internals dump root must be a JSON object")]
    InvalidRoot,
    #[error("JSON does not contain a webrtc-internals structure")]
    InvalidStructure,
}

pub fn parse<R: Read>(
    reader: R,
    file_name: String,
    file_size: u64,
    compressed: bool,
) -> Result<RtcSession, InternalsError> {
    let root: Value = serde_json::from_reader(reader)?;
    let Some(data) = root.as_object() else {
        return Err(InternalsError::InvalidRoot);
    };
    if !looks_like_internals(data) {
        return Err(InternalsError::InvalidStructure);
    }

    let source = SourceInfo {
        format: DumpFormat::WebRtcInternals,
        file_name,
        file_size,
        compressed,
        format_version: chrome_major(data.get("UserAgent").and_then(Value::as_str)),
    };
    let mut session = RtcSession::new(source);
    session.metadata = data
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "PeerConnections" | "getUserMedia"))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    session.client.user_agent = data
        .get("UserAgent")
        .and_then(Value::as_str)
        .map(str::to_string);
    session.client.user_agent_data = data.get("UserAgentData").cloned();

    if !data.contains_key("UserAgentData") {
        session.issues.push(ImportIssue::warning(
            "internals.user-agent-data-missing",
            "The dump predates UserAgentData; browser version detection may be incomplete.",
        ));
    }

    let start_timestamp = data.get("timestamp").and_then(Value::as_f64).unwrap_or(0.0);
    add_client_create_event(&mut session, data, start_timestamp);
    add_get_user_media_events(&mut session, data);
    session
        .client_events
        .sort_by(|left, right| left.timestamp.total_cmp(&right.timestamp));

    if let Some(connections) = data.get("PeerConnections").and_then(Value::as_object) {
        for (id, raw_connection) in connections {
            let Some(connection) = raw_connection.as_object() else {
                session.issues.push(ImportIssue::warning(
                    "internals.connection-invalid",
                    format!("PeerConnection {id} is not an object and was skipped."),
                ));
                continue;
            };
            parse_connection(&mut session, id, connection, start_timestamp);
        }
    }

    session.capabilities.has_api_trace = session
        .peer_connections
        .iter()
        .any(|connection| !connection.events.is_empty());
    session.capabilities.has_stats = session
        .peer_connections
        .iter()
        .any(|connection| !connection.stats.is_empty());
    session.capabilities.has_raw_sdp = session.peer_connections.iter().any(|connection| {
        connection
            .events
            .iter()
            .any(|event| event.value.get("sdp").and_then(Value::as_str).is_some())
    });
    session.capabilities.has_candidate_address =
        session.peer_connections.iter().any(|connection| {
            connection.stats.iter().any(|sample| {
                sample.reports.values().any(|report| {
                    report
                        .get("address")
                        .or_else(|| report.get("ip"))
                        .and_then(Value::as_str)
                        .is_some_and(|address| !address.is_empty())
                })
            })
        });

    Ok(session)
}

fn looks_like_internals(data: &Map<String, Value>) -> bool {
    data.contains_key("PeerConnections")
        || data.contains_key("getUserMedia")
        || data.contains_key("UserAgent")
        || data.contains_key("UserAgentData")
}

fn chrome_major(user_agent: Option<&str>) -> Option<String> {
    let user_agent = user_agent?;
    let marker = "Chrome/";
    let start = user_agent.find(marker)? + marker.len();
    let version = user_agent[start..].split_whitespace().next()?;
    Some(version.split('.').next().unwrap_or(version).to_string())
}

fn source_ref(path: impl Into<String>) -> SourceRef {
    SourceRef {
        line: None,
        json_path: Some(path.into()),
    }
}

fn add_client_create_event(session: &mut RtcSession, data: &Map<String, Value>, timestamp: f64) {
    let mut value = Map::new();
    for key in [
        "cpuPerformance",
        "deviceMemory",
        "hardwareConcurrency",
        "UserAgent",
        "UserAgentData",
    ] {
        if let Some(item) = data.get(key) {
            let normalized = match key {
                "UserAgent" => "userAgent",
                "UserAgentData" => "userAgentData",
                other => other,
            };
            value.insert(normalized.into(), item.clone());
        }
    }
    session.client_events.push(RtcEvent {
        timestamp,
        event_type: "create".into(),
        value: Value::Object(value),
        extra: Vec::new(),
        source_ref: source_ref("$.timestamp"),
    });
}

fn add_get_user_media_events(session: &mut RtcSession, data: &Map<String, Value>) {
    let Some(entries) = data.get("getUserMedia").and_then(Value::as_array) else {
        return;
    };
    for (index, entry) in entries.iter().enumerate() {
        let Some(entry) = entry.as_object() else {
            continue;
        };
        let request_type = entry
            .get("request_type")
            .and_then(Value::as_str)
            .unwrap_or("getUserMedia");
        let base_type = format!("navigator.mediaDevices.{request_type}");
        let timestamp = entry
            .get("timestamp")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);

        let (event_type, value) = if let Some(error) = entry.get("error") {
            (format!("{base_type}OnFailure"), error.clone())
        } else if entry.contains_key("audio_track_info") || entry.contains_key("video_track_info") {
            let mut tracks = Vec::new();
            add_track_info(&mut tracks, entry, "audio");
            add_track_info(&mut tracks, entry, "video");
            (format!("{base_type}OnSuccess"), Value::Array(tracks))
        } else {
            let mut constraints = Map::new();
            for kind in ["audio", "video"] {
                if let Some(value) = entry.get(kind) {
                    constraints.insert(
                        kind.into(),
                        if value.as_str() == Some("") {
                            Value::Bool(true)
                        } else {
                            parse_embedded_json(value)
                        },
                    );
                }
            }
            (base_type, Value::Object(constraints))
        };

        session.client_events.push(RtcEvent {
            timestamp,
            event_type,
            value,
            extra: Vec::new(),
            source_ref: source_ref(format!("$.getUserMedia[{index}]")),
        });
    }
}

fn add_track_info(tracks: &mut Vec<Value>, entry: &Map<String, Value>, kind: &'static str) {
    let key = format!("{kind}_track_info");
    let Some(info) = entry.get(&key).map(parse_embedded_json) else {
        return;
    };
    let Some(info) = info.as_object() else {
        return;
    };
    let stream_id = entry.get("stream_id").cloned().unwrap_or(Value::Null);
    tracks.push(Value::Array(vec![
        Value::String(kind.into()),
        info.get("id").cloned().unwrap_or(Value::Null),
        info.get("label").cloned().unwrap_or(Value::Null),
        stream_id,
    ]));
}

fn parse_connection(
    session: &mut RtcSession,
    id: &str,
    data: &Map<String, Value>,
    start_timestamp: f64,
) {
    let configuration = data
        .get("rtcConfiguration")
        .map(parse_embedded_json)
        .unwrap_or(Value::Null);
    let url = data.get("url").and_then(Value::as_str).map(str::to_string);
    let created_at = data
        .get("updateLog")
        .and_then(Value::as_array)
        .and_then(|events| events.first())
        .and_then(event_timestamp)
        .unwrap_or(start_timestamp);

    {
        let connection = session.connection_mut(id);
        connection.url = url;
        connection.configuration = configuration.clone();
        connection.events.push(RtcEvent {
            timestamp: created_at,
            event_type: "create".into(),
            value: configuration,
            extra: Vec::new(),
            source_ref: source_ref(format!("$.PeerConnections.{id}.rtcConfiguration")),
        });
    }

    if let Some(events) = data.get("updateLog").and_then(Value::as_array) {
        for (index, event) in events.iter().enumerate() {
            let Some(event) = event.as_object() else {
                continue;
            };
            let event_type = event
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let timestamp = event_timestamp(&Value::Object(event.clone())).unwrap_or(0.0);
            let value = event
                .get("value")
                .map(parse_embedded_json)
                .unwrap_or(Value::Null);
            session.connection_mut(id).events.push(RtcEvent {
                timestamp,
                event_type,
                value,
                extra: Vec::new(),
                source_ref: source_ref(format!("$.PeerConnections.{id}.updateLog[{index}]")),
            });
        }
    }

    match parse_stats(id, data) {
        Ok(samples) => session.connection_mut(id).stats = samples,
        Err(message) => session.issues.push(ImportIssue::warning(
            "internals.stats-incomplete",
            format!("PeerConnection {id}: {message}"),
        )),
    }
    let connection = session.connection_mut(id);
    connection
        .events
        .sort_by(|left, right| left.timestamp.total_cmp(&right.timestamp));
    connection
        .stats
        .sort_by(|left, right| left.timestamp.total_cmp(&right.timestamp));
}

fn event_timestamp(event: &Value) -> Option<f64> {
    let event = event.as_object()?;
    event.get("timestamp").and_then(Value::as_f64).or_else(|| {
        event
            .get("time")
            .and_then(Value::as_str)
            .and_then(parse_legacy_time)
    })
}

fn parse_legacy_time(value: &str) -> Option<f64> {
    if let Ok(timestamp) = value.parse::<f64>() {
        return Some(timestamp);
    }
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return Some(timestamp.timestamp_millis() as f64);
    }
    if let Ok(timestamp) = DateTime::parse_from_rfc2822(value) {
        return Some(timestamp.timestamp_millis() as f64);
    }

    // Older Chrome dumps used Date.prototype.toString(), including a zone name.
    let without_zone_name = value.split(" (").next().unwrap_or(value);
    DateTime::parse_from_str(without_zone_name, "%a %b %e %Y %H:%M:%S GMT%z")
        .ok()
        .map(|timestamp| timestamp.timestamp_millis() as f64)
}

fn parse_embedded_json(value: &Value) -> Value {
    let Some(text) = value.as_str() else {
        return value.clone();
    };
    if text.is_empty() {
        return Value::String(String::new());
    }
    serde_json::from_str(text).unwrap_or_else(|_| value.clone())
}

fn parse_stats(connection_id: &str, data: &Map<String, Value>) -> Result<Vec<StatsSample>, String> {
    let Some(stats) = data.get("stats").and_then(Value::as_object) else {
        return Ok(Vec::new());
    };
    let mut samples: Vec<StatsSample> = Vec::new();
    let mut sample_index: HashMap<u64, usize> = HashMap::new();

    for (report_name, raw_series) in stats {
        let Some(series) = raw_series.as_object() else {
            continue;
        };
        let Some((stats_id, property)) = split_report_name(report_name) else {
            continue;
        };
        if property == "type" || property == "timestamp" {
            continue;
        }
        let timestamp_key = format!("{stats_id}-timestamp");
        let Some(timestamp_series) = stats.get(&timestamp_key).and_then(Value::as_object) else {
            return Err(format!(
                "missing timestamp series for stats report {stats_id}"
            ));
        };
        let timestamps = parse_values(timestamp_series.get("values"))?;
        let values = parse_values(series.get("values"))?;
        if values.len() > timestamps.len() {
            return Err(format!(
                "stats report {stats_id} has more values than timestamps"
            ));
        }
        let offset = timestamps.len() - values.len();
        let stats_type = series
            .get("statsType")
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        for (index, value) in values.into_iter().enumerate() {
            let Some(timestamp) = timestamps[index + offset].as_f64() else {
                continue;
            };
            let entry_index = *sample_index.entry(timestamp.to_bits()).or_insert_with(|| {
                samples.push(StatsSample {
                    timestamp,
                    reports: Map::new(),
                    source_ref: source_ref(format!(
                        "$.PeerConnections.{connection_id}.stats.{report_name}"
                    )),
                });
                samples.len() - 1
            });
            let report = samples[entry_index]
                .reports
                .entry(stats_id.clone())
                .or_insert_with(|| {
                    serde_json::json!({
                        "id": stats_id,
                        "type": stats_type,
                        "timestamp": timestamp
                    })
                });
            if let Some(report) = report.as_object_mut() {
                report.insert(property.clone(), value);
            }
        }
    }

    Ok(samples)
}

fn split_report_name(name: &str) -> Option<(String, String)> {
    if let Some(bracket) = name.rfind('[') {
        let id = name[..bracket]
            .strip_suffix('-')
            .unwrap_or(&name[..bracket]);
        return Some((id.to_string(), name[bracket..].to_string()));
    }
    let (id, property) = name.rsplit_once('-')?;
    Some((id.to_string(), property.to_string()))
}

fn parse_values(value: Option<&Value>) -> Result<Vec<Value>, String> {
    let Some(value) = value else {
        return Err("stats series is missing values".into());
    };
    match value {
        Value::Array(values) => Ok(values.clone()),
        Value::String(text) => serde_json::from_str::<Vec<Value>>(text)
            .map_err(|error| format!("invalid stats values: {error}")),
        _ => Err("stats values must be a JSON array or encoded array".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, parse_legacy_time};
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn converts_updates_and_timeseries() {
        let input = json!({
            "timestamp": 1000,
            "UserAgent": "Chrome/149.0.0.0",
            "PeerConnections": {
                "23-3": {
                    "rtcConfiguration": "{\"iceServers\":[]}",
                    "updateLog": [
                        {"type": "setLocalDescription", "value": "{\"type\":\"offer\",\"sdp\":\"v=0\\r\\n\"}", "timestamp": 1100}
                    ],
                    "stats": {
                        "OT01-bytesSent": {"statsType": "outbound-rtp", "values": "[10,25]"},
                        "OT01-timestamp": {"statsType": "outbound-rtp", "values": "[1200,1300]"}
                    }
                }
            }
        });
        let bytes = serde_json::to_vec(&input).unwrap();
        let session = parse(
            Cursor::new(&bytes),
            "internals.json".into(),
            bytes.len() as u64,
            false,
        )
        .unwrap();
        assert_eq!(session.source.format_version.as_deref(), Some("149"));
        assert_eq!(session.peer_connections[0].events.len(), 2);
        assert_eq!(session.peer_connections[0].stats.len(), 2);
        assert_eq!(
            session.peer_connections[0].stats[1].reports["OT01"]["bytesSent"],
            25
        );
        assert!(session.capabilities.has_raw_sdp);
    }

    #[test]
    fn rejects_arbitrary_json() {
        let error = parse(Cursor::new(b"{}"), "unknown.json".into(), 2, false).unwrap_err();
        assert!(error.to_string().contains("does not contain"));
    }

    #[test]
    fn aligns_properties_that_appear_late() {
        let input = json!({
            "timestamp": 1000,
            "PeerConnections": {
                "pc": {
                    "stats": {
                        "out-targetBitrate": {"statsType": "outbound-rtp", "values": "[3000]"},
                        "out-timestamp": {"statsType": "outbound-rtp", "values": "[1100,1200,1300]"}
                    }
                }
            }
        });
        let bytes = serde_json::to_vec(&input).unwrap();
        let session = parse(
            Cursor::new(&bytes),
            "internals.json".into(),
            bytes.len() as u64,
            false,
        )
        .unwrap();
        assert_eq!(session.peer_connections[0].stats.len(), 1);
        assert_eq!(session.peer_connections[0].stats[0].timestamp, 1300.0);
        assert_eq!(
            session.peer_connections[0].stats[0].reports["out"]["targetBitrate"],
            3000
        );
    }

    #[test]
    fn excludes_large_trace_fields_from_metadata() {
        let input = json!({
            "timestamp": 1000,
            "getUserMedia": [],
            "PeerConnections": {},
            "UserAgent": "Chrome/149.0.0.0"
        });
        let bytes = serde_json::to_vec(&input).unwrap();
        let session = parse(
            Cursor::new(&bytes),
            "internals.json".into(),
            bytes.len() as u64,
            false,
        )
        .unwrap();
        assert!(!session.metadata.contains_key("PeerConnections"));
        assert!(!session.metadata.contains_key("getUserMedia"));
        assert_eq!(session.metadata["UserAgent"], "Chrome/149.0.0.0");
    }

    #[test]
    fn parses_legacy_chrome_time_strings() {
        let timestamp =
            parse_legacy_time("Mon Jan 01 2024 12:00:00 GMT+0800 (China Standard Time)").unwrap();
        assert_eq!(timestamp, 1_704_081_600_000.0);
    }
}

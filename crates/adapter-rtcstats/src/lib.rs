mod compression;

use compression::{decompress_description, decompress_method, decompress_stats};
use rtc_domain::{DumpFormat, RtcEvent, RtcSession, SourceInfo, SourceRef, StatsSample};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::BufRead;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtcStatsError {
    #[error("failed to read line {line}: {source}")]
    Read {
        line: usize,
        #[source]
        source: std::io::Error,
    },
    #[error("not an RTCStats dump")]
    InvalidHeader,
    #[error("invalid metadata on line 2: {0}")]
    InvalidMetadata(serde_json::Error),
    #[error("metadata on line 2 must be a JSON object")]
    InvalidMetadataType,
    #[error("invalid JSON on line {line}: {source}")]
    InvalidJson {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("event on line {line} must contain method, connection, value and timestamp delta")]
    InvalidEvent { line: usize },
    #[error("event timestamp delta on line {line} must be numeric")]
    InvalidTimestamp { line: usize },
    #[error("getStats value on line {line} must be an object")]
    InvalidStats { line: usize },
}

pub fn parse<R: BufRead>(
    mut reader: R,
    file_name: String,
    file_size: u64,
    compressed: bool,
) -> Result<RtcSession, RtcStatsError> {
    let mut line = String::new();
    read_line(&mut reader, &mut line, 1)?;
    if trim_line_ending(&line) != "RTCStatsDump" {
        return Err(RtcStatsError::InvalidHeader);
    }

    line.clear();
    read_line(&mut reader, &mut line, 2)?;
    let metadata_value: Value =
        serde_json::from_str(trim_line_ending(&line)).map_err(RtcStatsError::InvalidMetadata)?;
    let Some(metadata) = metadata_value.as_object() else {
        return Err(RtcStatsError::InvalidMetadataType);
    };

    let format_version = metadata.get("fileFormat").map(value_as_string);
    let source = SourceInfo {
        format: DumpFormat::RtcStats,
        file_name,
        file_size,
        compressed,
        format_version,
    };
    let mut session = RtcSession::new(source);
    session.metadata = metadata.clone();
    session.client.user_agent = metadata
        .get("userAgent")
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut base_stats: HashMap<String, Map<String, Value>> = HashMap::new();
    let mut last_time = 0.0;
    let mut line_number = 2;

    loop {
        line.clear();
        line_number += 1;
        let bytes = reader
            .read_line(&mut line)
            .map_err(|source| RtcStatsError::Read {
                line: line_number,
                source,
            })?;
        if bytes == 0 {
            break;
        }
        let text = trim_line_ending(&line);
        if text.is_empty() {
            continue;
        }
        let value: Value =
            serde_json::from_str(text).map_err(|source| RtcStatsError::InvalidJson {
                line: line_number,
                source,
            })?;
        let Some(mut parts) = value.as_array().cloned() else {
            continue;
        };
        if parts.len() < 4 {
            return Err(RtcStatsError::InvalidEvent { line: line_number });
        }

        let delta = parts
            .pop()
            .and_then(|value| value.as_f64())
            .ok_or(RtcStatsError::InvalidTimestamp { line: line_number })?;
        last_time += delta;
        let method = decompress_method(&parts[0]);
        let connection_id = connection_id(&parts[1]);
        let mut event_value = parts[2].clone();
        let extra = parts.drain(3..).collect::<Vec<_>>();

        normalize_state_value(&method, &mut event_value);

        if method == "getStats" {
            let Some(delta_stats) = event_value.as_object() else {
                return Err(RtcStatsError::InvalidStats { line: line_number });
            };
            let base = base_stats.entry(connection_id.clone()).or_default();
            let reports = decompress_stats(base, delta_stats);
            *base = reports.clone();
            let report_count = reports.len();
            session
                .connection_mut(&connection_id)
                .stats
                .push(StatsSample {
                    timestamp: last_time,
                    reports,
                    source_ref: SourceRef {
                        line: Some(line_number),
                        json_path: None,
                    },
                });
            event_value = serde_json::json!({"reportCount": report_count});
            session.capabilities.has_stats = true;
        } else if is_description_event(&method) && event_value.is_object() {
            if let Some(base) =
                find_description_base(&session, &connection_id, &method, &event_value)
            {
                event_value = decompress_description(base, &event_value);
            }
            if event_value.get("sdp").and_then(Value::as_str).is_some() {
                session.capabilities.has_raw_sdp = true;
            }
        }

        let event = RtcEvent {
            timestamp: last_time,
            event_type: method,
            value: event_value,
            extra,
            source_ref: SourceRef {
                line: Some(line_number),
                json_path: None,
            },
        };

        if parts[1].is_null() {
            session.client_events.push(event);
        } else {
            let connection = session.connection_mut(&connection_id);
            if event.event_type == "create" {
                connection.configuration = event.value.clone();
                connection.url = event
                    .extra
                    .first()
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
            connection.events.push(event);
        }
        session.capabilities.has_api_trace = true;
    }

    session.capabilities.has_candidate_address =
        session.peer_connections.iter().any(|connection| {
            connection.stats.iter().any(|sample| {
                sample.reports.values().any(|report| {
                    report
                        .get("address")
                        .and_then(Value::as_str)
                        .is_some_and(|address| !address.is_empty())
                })
            })
        });

    Ok(session)
}

fn read_line<R: BufRead>(
    reader: &mut R,
    target: &mut String,
    number: usize,
) -> Result<(), RtcStatsError> {
    reader
        .read_line(target)
        .map_err(|source| RtcStatsError::Read {
            line: number,
            source,
        })?;
    Ok(())
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn value_as_string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn connection_id(value: &Value) -> String {
    match value {
        Value::Null => "__client__".into(),
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

fn normalize_state_value(method: &str, value: &mut Value) {
    let Value::String(current) = value else {
        return;
    };
    let supported = match method {
        "onsignalingstatechange" => [
            "stable",
            "have-local-offer",
            "have-remote-offer",
            "have-local-pranswer",
            "have-remote-pranswer",
            "closed",
        ]
        .as_slice(),
        "oniceconnectionstatechange" => [
            "new",
            "checking",
            "connected",
            "completed",
            "failed",
            "disconnected",
            "closed",
        ]
        .as_slice(),
        "onconnectionstatechange" => [
            "new",
            "connecting",
            "connected",
            "disconnected",
            "failed",
            "closed",
        ]
        .as_slice(),
        "onicegatheringstatechange" => ["new", "gathering", "complete"].as_slice(),
        _ => return,
    };
    if let Ok(unwrapped) = serde_json::from_str::<String>(current) {
        if supported.contains(&unwrapped.as_str()) {
            *current = unwrapped;
        }
    }
}

fn is_description_event(method: &str) -> bool {
    matches!(
        method,
        "setLocalDescription"
            | "setRemoteDescription"
            | "createOfferOnSuccess"
            | "createAnswerOnSuccess"
    )
}

fn find_description_base<'a>(
    session: &'a RtcSession,
    connection_id: &str,
    method: &str,
    value: &Value,
) -> Option<&'a Value> {
    let description_type = value.get("type")?.as_str()?;
    let target = match method {
        "setLocalDescription" if description_type == "offer" => "createOfferOnSuccess",
        "setLocalDescription" if description_type == "answer" => "createAnswerOnSuccess",
        "createOfferOnSuccess" => "setLocalDescription",
        "createAnswerOnSuccess" => "setLocalDescription",
        _ => return None,
    };
    session
        .peer_connections
        .iter()
        .find(|connection| connection.id == connection_id)?
        .events
        .iter()
        .rev()
        .find(|event| {
            event.event_type == target
                && event.value.get("type").and_then(Value::as_str) == Some(description_type)
        })
        .map(|event| &event.value)
}

#[cfg(test)]
mod tests {
    use super::{parse, RtcStatsError};
    use std::io::Cursor;

    #[test]
    fn parses_minimal_dump() {
        let input = b"RTCStatsDump\n{\"fileFormat\":3,\"userAgent\":\"Chrome\"}\n[39,\"pc-1\",{},1]\n[24,\"pc-1\",\"checking\",1]\n";
        let session = parse(
            Cursor::new(input),
            "sample.jsonl".into(),
            input.len() as u64,
            false,
        )
        .unwrap();
        assert_eq!(session.peer_connections.len(), 1);
        assert_eq!(
            session.peer_connections[0].events[1].event_type,
            "oniceconnectionstatechange"
        );
        assert_eq!(session.summary().event_count, 2);
    }

    #[test]
    fn parses_and_restores_stats_delta() {
        let input = b"RTCStatsDump\n{\"fileFormat\":3}\n[39,\"pc-1\",{},1]\n[\"g\",\"pc-1\",{\"t\":100,\"0\":{\"type\":2,\"80\":10}},1]\n[\"g\",\"pc-1\",{\"t\":200,\"0\":{\"80\":25}},1]\n";
        let session = parse(
            Cursor::new(input),
            "stats.jsonl".into(),
            input.len() as u64,
            false,
        )
        .unwrap();
        let samples = &session.peer_connections[0].stats;
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].reports["0"]["type"], "outbound-rtp");
        assert_eq!(samples[1].reports["0"]["bytesSent"], 25);
        assert_eq!(samples[1].reports["0"]["timestamp"], 200);
    }

    #[test]
    fn reports_the_jsonl_line_number() {
        let input = b"RTCStatsDump\n{\"fileFormat\":3}\n[39,null,{},1]\nnot-json\n";
        let error = parse(
            Cursor::new(input),
            "broken.jsonl".into(),
            input.len() as u64,
            false,
        )
        .unwrap_err();
        assert!(matches!(error, RtcStatsError::InvalidJson { line: 4, .. }));
    }

    #[test]
    fn removes_reports_marked_null_by_a_delta() {
        let input = b"RTCStatsDump\n{\"fileFormat\":3}\n[\"g\",\"pc-1\",{\"t\":100,\"0\":{\"type\":2,\"80\":10}},1]\n[\"g\",\"pc-1\",{\"t\":200,\"0\":null},1]\n";
        let session = parse(
            Cursor::new(input),
            "stats.jsonl".into(),
            input.len() as u64,
            false,
        )
        .unwrap();
        let samples = &session.peer_connections[0].stats;
        assert!(samples[1].reports.is_empty());
    }

    #[test]
    fn rejects_non_object_metadata() {
        let input = b"RTCStatsDump\n[]\n";
        let error = parse(
            Cursor::new(input),
            "broken.jsonl".into(),
            input.len() as u64,
            false,
        )
        .unwrap_err();
        assert!(matches!(error, RtcStatsError::InvalidMetadataType));
    }
}

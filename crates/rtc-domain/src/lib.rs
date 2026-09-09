use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DumpFormat {
    RtcStats,
    WebRtcInternals,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub format: DumpFormat,
    pub file_name: String,
    pub file_size: u64,
    pub compressed: bool,
    pub format_version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub user_agent: Option<String>,
    pub user_agent_data: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub has_api_trace: bool,
    pub has_raw_sdp: bool,
    pub has_stats: bool,
    pub has_candidate_address: bool,
    pub is_possibly_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Fatal,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportIssue {
    pub code: String,
    pub severity: IssueSeverity,
    pub message: String,
    pub line: Option<usize>,
    pub json_path: Option<String>,
}

impl ImportIssue {
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: IssueSeverity::Warning,
            message: message.into(),
            line: None,
            json_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub line: Option<usize>,
    pub json_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RtcEvent {
    pub timestamp: f64,
    pub event_type: String,
    pub value: Value,
    pub extra: Vec<Value>,
    pub source_ref: SourceRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatsSample {
    pub timestamp: f64,
    pub reports: Map<String, Value>,
    pub source_ref: SourceRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeerConnection {
    pub id: String,
    pub url: Option<String>,
    pub configuration: Value,
    pub events: Vec<RtcEvent>,
    pub stats: Vec<StatsSample>,
}

impl PeerConnection {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            url: None,
            configuration: Value::Null,
            events: Vec::new(),
            stats: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RtcSession {
    pub schema_version: u32,
    pub source: SourceInfo,
    pub client: ClientInfo,
    pub capabilities: Capabilities,
    pub metadata: Map<String, Value>,
    pub client_events: Vec<RtcEvent>,
    pub peer_connections: Vec<PeerConnection>,
    pub issues: Vec<ImportIssue>,
}

impl RtcSession {
    pub fn new(source: SourceInfo) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            source,
            client: ClientInfo::default(),
            capabilities: Capabilities::default(),
            metadata: Map::new(),
            client_events: Vec::new(),
            peer_connections: Vec::new(),
            issues: Vec::new(),
        }
    }

    pub fn connection_mut(&mut self, id: &str) -> &mut PeerConnection {
        if let Some(index) = self.peer_connections.iter().position(|pc| pc.id == id) {
            return &mut self.peer_connections[index];
        }
        self.peer_connections.push(PeerConnection::new(id));
        self.peer_connections.last_mut().expect("just inserted")
    }

    pub fn summary(&self) -> SessionSummary {
        let event_count = self.client_events.len()
            + self
                .peer_connections
                .iter()
                .map(|pc| pc.events.len())
                .sum::<usize>();
        let stats_sample_count = self
            .peer_connections
            .iter()
            .map(|pc| pc.stats.len())
            .sum::<usize>();

        let mut timestamps = self
            .client_events
            .iter()
            .map(|event| event.timestamp)
            .chain(self.peer_connections.iter().flat_map(|pc| {
                pc.events
                    .iter()
                    .map(|event| event.timestamp)
                    .chain(pc.stats.iter().map(|sample| sample.timestamp))
            }))
            .filter(|timestamp| timestamp.is_finite());
        let first = timestamps.next();
        let (start_time, end_time) = timestamps.fold((first, first), |(min, max), timestamp| {
            (
                Some(min.map_or(timestamp, |value| value.min(timestamp))),
                Some(max.map_or(timestamp, |value| value.max(timestamp))),
            )
        });

        SessionSummary {
            format: self.source.format.clone(),
            file_name: self.source.file_name.clone(),
            file_size: self.source.file_size,
            compressed: self.source.compressed,
            user_agent: self.client.user_agent.clone(),
            peer_connection_count: self.peer_connections.len(),
            event_count,
            stats_sample_count,
            start_time,
            end_time,
            warning_count: self
                .issues
                .iter()
                .filter(|issue| issue.severity == IssueSeverity::Warning)
                .count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub format: DumpFormat,
    pub file_name: String,
    pub file_size: u64,
    pub compressed: bool,
    pub user_agent: Option<String>,
    pub peer_connection_count: usize,
    pub event_count: usize,
    pub stats_sample_count: usize,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub warning_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeerConnectionSummary {
    pub id: String,
    pub url: Option<String>,
    pub event_count: usize,
    pub stats_sample_count: usize,
}

impl From<&PeerConnection> for PeerConnectionSummary {
    fn from(connection: &PeerConnection) -> Self {
        Self {
            id: connection.id.clone(),
            url: connection.url.clone(),
            event_count: connection.events.len(),
            stats_sample_count: connection.stats.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionAnalysis {
    pub connections: Vec<ConnectionAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionAnalysis {
    pub id: String,
    pub url: Option<String>,
    pub configuration: Value,
    pub states: ConnectionStates,
    pub events: Vec<EventSummary>,
    pub descriptions: Vec<SessionDescriptionSummary>,
    pub ice_candidates: Vec<IceCandidateSummary>,
    pub candidate_pairs: Vec<CandidatePairSummary>,
    pub stats: Vec<StatsPoint>,
    pub media: MediaSummary,
    pub findings: Vec<AnalysisFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisFinding {
    pub severity: IssueSeverity,
    pub title: String,
    pub message: String,
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStates {
    pub signaling: Option<String>,
    pub connection: Option<String>,
    pub ice_connection: Option<String>,
    pub ice_gathering: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventSummary {
    pub timestamp: f64,
    pub event_type: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionDescriptionSummary {
    pub timestamp: f64,
    pub event_type: String,
    pub description_type: String,
    pub sdp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IceCandidateSummary {
    pub id: String,
    pub side: String,
    pub address: Option<String>,
    pub port: Option<u64>,
    pub protocol: Option<String>,
    pub candidate_type: Option<String>,
    pub network_type: Option<String>,
    pub relay_protocol: Option<String>,
    pub priority: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CandidatePairSummary {
    pub id: String,
    pub state: Option<String>,
    pub nominated: bool,
    pub selected: bool,
    pub local_candidate_id: Option<String>,
    pub remote_candidate_id: Option<String>,
    pub current_round_trip_time_ms: Option<f64>,
    pub available_outgoing_bitrate_kbps: Option<f64>,
    pub bytes_sent: Option<u64>,
    pub bytes_received: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatsPoint {
    pub timestamp: f64,
    pub outgoing_bitrate_kbps: Option<f64>,
    pub incoming_bitrate_kbps: Option<f64>,
    pub packets_lost: Option<i64>,
    pub current_round_trip_time_ms: Option<f64>,
    pub available_outgoing_bitrate_kbps: Option<f64>,
    pub frames_per_second: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MediaSummary {
    pub inbound_audio: usize,
    pub inbound_video: usize,
    pub outbound_audio: usize,
    pub outbound_video: usize,
    pub codecs: Vec<String>,
}

impl RtcSession {
    pub fn analysis(&self) -> SessionAnalysis {
        SessionAnalysis {
            connections: self
                .peer_connections
                .iter()
                .map(|connection| analyze_connection(connection, &self.source.format))
                .collect(),
        }
    }
}

fn analyze_connection(connection: &PeerConnection, format: &DumpFormat) -> ConnectionAnalysis {
    let reports = latest_reports(connection, format);
    ConnectionAnalysis {
        id: connection.id.clone(),
        url: connection.url.clone(),
        configuration: connection.configuration.clone(),
        states: connection_states(connection),
        events: connection
            .events
            .iter()
            .filter(|event| event.event_type != "getStats")
            .map(|event| EventSummary {
                timestamp: event.timestamp,
                event_type: event.event_type.clone(),
                value: event.value.clone(),
            })
            .collect(),
        descriptions: session_descriptions(connection),
        ice_candidates: ice_candidates(&reports),
        candidate_pairs: candidate_pairs(&reports),
        stats: stats_points(connection),
        media: media_summary(&reports),
        findings: connection_findings(connection, &reports),
    }
}

fn connection_findings(
    connection: &PeerConnection,
    reports: &Map<String, Value>,
) -> Vec<AnalysisFinding> {
    let mut findings = Vec::new();
    for event in &connection.events {
        if event.event_type == "onicecandidateerror" {
            let detail = event
                .value
                .as_object()
                .and_then(|value| {
                    string_field(value, "error_text").or_else(|| string_field(value, "errorText"))
                })
                .unwrap_or_else(|| "ICE candidate gathering reported an error.".into());
            findings.push(AnalysisFinding {
                severity: IssueSeverity::Warning,
                title: "ICE candidate error".into(),
                message: detail,
                timestamp: Some(event.timestamp),
            });
        }
        if matches!(
            event.event_type.as_str(),
            "oniceconnectionstatechange" | "onconnectionstatechange"
        ) && event.value.as_str() == Some("failed")
        {
            findings.push(AnalysisFinding {
                severity: IssueSeverity::Error,
                title: "Connection entered failed state".into(),
                message: format!("{} changed to failed.", event.event_type),
                timestamp: Some(event.timestamp),
            });
        }
    }

    let candidates = ice_candidates(reports);
    let pairs = candidate_pairs(reports);
    if !candidates.is_empty() && !pairs.iter().any(|pair| pair.selected || pair.nominated) {
        findings.push(AnalysisFinding {
            severity: IssueSeverity::Warning,
            title: "No selected ICE candidate pair".into(),
            message: "Candidates were gathered, but no selected or nominated pair was found."
                .into(),
            timestamp: None,
        });
    }
    findings
}

fn connection_states(connection: &PeerConnection) -> ConnectionStates {
    let mut states = ConnectionStates::default();
    for event in &connection.events {
        let state = event.value.as_str().map(str::to_string);
        match event.event_type.as_str() {
            "onsignalingstatechange" => states.signaling = state,
            "onconnectionstatechange" => states.connection = state,
            "oniceconnectionstatechange" => states.ice_connection = state,
            "onicegatheringstatechange" => states.ice_gathering = state,
            _ => {}
        }
    }
    states
}

fn session_descriptions(connection: &PeerConnection) -> Vec<SessionDescriptionSummary> {
    connection
        .events
        .iter()
        .filter_map(|event| {
            let value = event.value.as_object()?;
            let sdp = value.get("sdp")?.as_str()?;
            Some(SessionDescriptionSummary {
                timestamp: event.timestamp,
                event_type: event.event_type.clone(),
                description_type: value
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                sdp: sdp.to_string(),
            })
        })
        .collect()
}

fn latest_reports(connection: &PeerConnection, format: &DumpFormat) -> Map<String, Value> {
    if *format == DumpFormat::RtcStats {
        return connection
            .stats
            .last()
            .map(|sample| sample.reports.clone())
            .unwrap_or_default();
    }

    let mut reports: Map<String, Value> = Map::new();
    for sample in &connection.stats {
        for (id, value) in &sample.reports {
            let Some(update) = value.as_object() else {
                continue;
            };
            let report = reports
                .entry(id.clone())
                .or_insert_with(|| Value::Object(Map::new()));
            if let Some(report) = report.as_object_mut() {
                report.extend(update.clone());
            }
        }
    }
    reports
}

fn ice_candidates(reports: &Map<String, Value>) -> Vec<IceCandidateSummary> {
    reports
        .iter()
        .filter_map(|(id, value)| {
            let report = value.as_object()?;
            let side = match report.get("type").and_then(Value::as_str)? {
                "local-candidate" => "local",
                "remote-candidate" => "remote",
                _ => return None,
            };
            Some(IceCandidateSummary {
                id: id.clone(),
                side: side.into(),
                address: string_field(report, "address").or_else(|| string_field(report, "ip")),
                port: report.get("port").and_then(Value::as_u64),
                protocol: string_field(report, "protocol"),
                candidate_type: string_field(report, "candidateType"),
                network_type: string_field(report, "networkType"),
                relay_protocol: string_field(report, "relayProtocol"),
                priority: report.get("priority").and_then(Value::as_u64),
            })
        })
        .collect()
}

fn candidate_pairs(reports: &Map<String, Value>) -> Vec<CandidatePairSummary> {
    let selected_pair_ids = reports
        .values()
        .filter_map(Value::as_object)
        .filter_map(|report| {
            report
                .get("selectedCandidatePairId")
                .and_then(Value::as_str)
        })
        .collect::<Vec<_>>();
    reports
        .iter()
        .filter_map(|(id, value)| {
            let report = value.as_object()?;
            if report.get("type").and_then(Value::as_str) != Some("candidate-pair") {
                return None;
            }
            Some(CandidatePairSummary {
                id: id.clone(),
                state: string_field(report, "state"),
                nominated: report
                    .get("nominated")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                selected: selected_pair_ids.contains(&id.as_str()),
                local_candidate_id: string_field(report, "localCandidateId"),
                remote_candidate_id: string_field(report, "remoteCandidateId"),
                current_round_trip_time_ms: number_field(report, "currentRoundTripTime")
                    .map(|value| value * 1000.0),
                available_outgoing_bitrate_kbps: number_field(report, "availableOutgoingBitrate")
                    .map(|value| value / 1000.0),
                bytes_sent: report.get("bytesSent").and_then(Value::as_u64),
                bytes_received: report.get("bytesReceived").and_then(Value::as_u64),
            })
        })
        .collect()
}

fn media_summary(reports: &Map<String, Value>) -> MediaSummary {
    let mut summary = MediaSummary::default();
    for value in reports.values() {
        let Some(report) = value.as_object() else {
            continue;
        };
        match (
            report.get("type").and_then(Value::as_str),
            report.get("kind").and_then(Value::as_str),
        ) {
            (Some("inbound-rtp"), Some("audio")) => summary.inbound_audio += 1,
            (Some("inbound-rtp"), Some("video")) => summary.inbound_video += 1,
            (Some("outbound-rtp"), Some("audio")) => summary.outbound_audio += 1,
            (Some("outbound-rtp"), Some("video")) => summary.outbound_video += 1,
            (Some("codec"), _) => {
                if let Some(codec) = report.get("mimeType").and_then(Value::as_str) {
                    if !summary.codecs.iter().any(|current| current == codec) {
                        summary.codecs.push(codec.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    summary.codecs.sort();
    summary
}

fn stats_points(connection: &PeerConnection) -> Vec<StatsPoint> {
    let mut previous_sent: Option<(f64, f64)> = None;
    let mut previous_received: Option<(f64, f64)> = None;
    let mut points = Vec::with_capacity(connection.stats.len());
    for sample in &connection.stats {
        let mut sent = None;
        let mut received = None;
        let mut packets_lost = None;
        let mut rtt = None;
        let mut bandwidth = None;
        let mut fps = None;
        for value in sample.reports.values() {
            let Some(report) = value.as_object() else {
                continue;
            };
            match report.get("type").and_then(Value::as_str) {
                Some("outbound-rtp") => {
                    sent = sum_number(sent, number_field(report, "bytesSent"));
                    fps = number_field(report, "framesPerSecond").or(fps);
                }
                Some("inbound-rtp") => {
                    received = sum_number(received, number_field(report, "bytesReceived"));
                    packets_lost = sum_integer(packets_lost, integer_field(report, "packetsLost"));
                    fps = number_field(report, "framesPerSecond").or(fps);
                }
                Some("candidate-pair") => {
                    let active = report
                        .get("nominated")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        || report.get("state").and_then(Value::as_str) == Some("succeeded");
                    if active {
                        rtt = number_field(report, "currentRoundTripTime")
                            .map(|value| value * 1000.0)
                            .or(rtt);
                        bandwidth = number_field(report, "availableOutgoingBitrate")
                            .map(|value| value / 1000.0)
                            .or(bandwidth);
                    }
                }
                _ => {}
            }
        }

        let outgoing_bitrate_kbps = rate_kbps(sample.timestamp, sent, &mut previous_sent);
        let incoming_bitrate_kbps = rate_kbps(sample.timestamp, received, &mut previous_received);
        points.push(StatsPoint {
            timestamp: sample.timestamp,
            outgoing_bitrate_kbps,
            incoming_bitrate_kbps,
            packets_lost,
            current_round_trip_time_ms: rtt,
            available_outgoing_bitrate_kbps: bandwidth,
            frames_per_second: fps,
        });
    }

    const MAX_POINTS: usize = 1200;
    if points.len() <= MAX_POINTS {
        return points;
    }
    let stride = points.len().div_ceil(MAX_POINTS);
    points.into_iter().step_by(stride).collect()
}

fn rate_kbps(timestamp: f64, bytes: Option<f64>, previous: &mut Option<(f64, f64)>) -> Option<f64> {
    let bytes = bytes?;
    let rate = previous.and_then(|(old_timestamp, old_bytes)| {
        let seconds = (timestamp - old_timestamp) / 1000.0;
        (seconds > 0.0 && bytes >= old_bytes)
            .then_some((bytes - old_bytes) * 8.0 / seconds / 1000.0)
    });
    *previous = Some((timestamp, bytes));
    rate
}

fn string_field(report: &Map<String, Value>, key: &str) -> Option<String> {
    report.get(key).and_then(Value::as_str).map(str::to_string)
}

fn number_field(report: &Map<String, Value>, key: &str) -> Option<f64> {
    report.get(key).and_then(Value::as_f64)
}

fn integer_field(report: &Map<String, Value>, key: &str) -> Option<i64> {
    report.get(key).and_then(Value::as_i64)
}

fn sum_number(current: Option<f64>, next: Option<f64>) -> Option<f64> {
    next.map(|value| current.unwrap_or(0.0) + value).or(current)
}

fn sum_integer(current: Option<i64>, next: Option<i64>) -> Option<i64> {
    next.map(|value| current.unwrap_or(0) + value).or(current)
}

#[cfg(test)]
mod tests {
    use super::{DumpFormat, RtcEvent, RtcSession, SourceInfo, SourceRef, StatsSample};
    use serde_json::{json, Map};

    fn reports(value: serde_json::Value) -> Map<String, serde_json::Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn derives_connection_states_ice_and_bitrate() {
        let mut session = RtcSession::new(SourceInfo {
            format: DumpFormat::RtcStats,
            file_name: "call.jsonl".into(),
            file_size: 100,
            compressed: false,
            format_version: Some("3".into()),
        });
        let connection = session.connection_mut("pc-1");
        connection.events.push(RtcEvent {
            timestamp: 1_000.0,
            event_type: "oniceconnectionstatechange".into(),
            value: json!("connected"),
            extra: Vec::new(),
            source_ref: SourceRef {
                line: Some(3),
                json_path: None,
            },
        });
        connection.events.push(RtcEvent {
            timestamp: 1_100.0,
            event_type: "onicecandidateerror".into(),
            value: json!({"error_text":"STUN binding request timed out."}),
            extra: Vec::new(),
            source_ref: SourceRef {
                line: Some(4),
                json_path: None,
            },
        });
        connection.stats.push(StatsSample {
            timestamp: 1_000.0,
            reports: reports(json!({
                "out": {"type":"outbound-rtp", "kind":"video", "bytesSent":1000},
                "transport": {"type":"transport", "selectedCandidatePairId":"pair"},
                "pair": {"type":"candidate-pair", "state":"succeeded", "nominated":true, "localCandidateId":"local", "remoteCandidateId":"remote", "currentRoundTripTime":0.02},
                "local": {"type":"local-candidate", "address":"10.0.0.1", "port":5000, "protocol":"udp", "candidateType":"host"},
                "remote": {"type":"remote-candidate", "address":"203.0.113.1", "port":3478, "protocol":"udp", "candidateType":"srflx"}
            })),
            source_ref: SourceRef { line: Some(4), json_path: None },
        });
        connection.stats.push(StatsSample {
            timestamp: 2_000.0,
            reports: reports(json!({
                "out": {"type":"outbound-rtp", "kind":"video", "bytesSent":3000},
                "transport": {"type":"transport", "selectedCandidatePairId":"pair"},
                "pair": {"type":"candidate-pair", "state":"succeeded", "nominated":true, "localCandidateId":"local", "remoteCandidateId":"remote", "currentRoundTripTime":0.03},
                "local": {"type":"local-candidate", "address":"10.0.0.1", "port":5000, "protocol":"udp", "candidateType":"host"},
                "remote": {"type":"remote-candidate", "address":"203.0.113.1", "port":3478, "protocol":"udp", "candidateType":"srflx"}
            })),
            source_ref: SourceRef { line: Some(5), json_path: None },
        });

        let analysis = session.analysis();
        let result = &analysis.connections[0];
        assert_eq!(result.states.ice_connection.as_deref(), Some("connected"));
        assert_eq!(result.ice_candidates.len(), 2);
        assert!(result.candidate_pairs[0].selected);
        assert_eq!(
            result.candidate_pairs[0].current_round_trip_time_ms,
            Some(30.0)
        );
        assert_eq!(result.stats[1].outgoing_bitrate_kbps, Some(16.0));
        assert_eq!(result.media.outbound_video, 1);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(
            result.findings[0].message,
            "STUN binding request timed out."
        );
    }
}

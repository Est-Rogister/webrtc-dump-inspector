use rtc_domain::{
    AnalysisFinding, CandidatePairSummary, Capabilities, ConnectionAnalysis, ConnectionStates,
    DumpFormat, EventSummary, IceCandidateSummary, ImportIssue, IssueSeverity, MediaSummary,
    RtcSession, SessionAnalysis, SessionDescriptionSummary, StatsPoint,
};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use uuid::Uuid;

const SESSION_COLORS: [&str; 6] = [
    "#147a4b", "#2676b8", "#c07818", "#a34f78", "#5f6db3", "#a6533d",
];

#[derive(Debug, Clone)]
pub struct StoredSource {
    pub id: String,
    pub path: String,
    pub session: RtcSession,
}

impl StoredSource {
    pub fn new(path: String, session: RtcSession) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            path,
            session,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceError {
    pub path: String,
    pub name: String,
    pub error: StoreErrorDto,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StoreErrorDto {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone)]
struct StoredSession {
    id: String,
    name: String,
    color: String,
    sources: Vec<StoredSource>,
    source_errors: Vec<SourceError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSourceDto {
    pub id: String,
    pub path: String,
    pub name: String,
    pub format: DumpFormat,
    pub file_size: u64,
    pub compressed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSessionSummary {
    pub source_count: usize,
    pub file_names: Vec<String>,
    pub formats: Vec<DumpFormat>,
    pub file_size: u64,
    pub user_agent: Option<String>,
    pub peer_connection_count: usize,
    pub event_count: usize,
    pub stats_sample_count: usize,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub warning_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSessionDto {
    pub id: String,
    pub name: String,
    pub color: String,
    pub sources: Vec<SessionSourceDto>,
    pub source_errors: Vec<SourceError>,
    pub summary: WorkspaceSessionSummary,
    pub capabilities: Capabilities,
    pub issues: Vec<ImportIssue>,
    pub analysis: SessionAnalysis,
}

#[derive(Debug, Default)]
pub struct SessionStore {
    sessions: Vec<StoredSession>,
}

impl SessionStore {
    pub fn add_sources(
        &mut self,
        target_session_id: Option<&str>,
        sources: Vec<StoredSource>,
        errors: Vec<SourceError>,
    ) -> Result<Option<String>, StoreErrorDto> {
        if sources.is_empty() {
            return Ok(None);
        }

        if let Some(session_id) = target_session_id {
            let session = self
                .sessions
                .iter_mut()
                .find(|session| session.id == session_id)
                .ok_or_else(|| StoreErrorDto {
                    code: "session.not-found".into(),
                    message: format!("session {session_id} was not found"),
                })?;
            session.sources.extend(sources);
            session.source_errors.extend(errors);
            return Ok(Some(session.id.clone()));
        }

        let index = self.sessions.len();
        let name = if sources.len() == 1 {
            file_name(&sources[0].path)
        } else {
            format!("Session {}", index + 1)
        };
        let id = Uuid::new_v4().to_string();
        self.sessions.push(StoredSession {
            id: id.clone(),
            name,
            color: SESSION_COLORS[index % SESSION_COLORS.len()].into(),
            sources,
            source_errors: errors,
        });
        Ok(Some(id))
    }

    pub fn get(&self, session_id: &str, max_stats_points: usize) -> Option<WorkspaceSessionDto> {
        self.sessions
            .iter()
            .find(|session| session.id == session_id)
            .map(|session| session.to_dto(max_stats_points))
    }

    pub fn list(&self, max_stats_points: usize) -> Vec<WorkspaceSessionDto> {
        self.sessions
            .iter()
            .map(|session| session.to_dto(max_stats_points))
            .collect()
    }

    pub fn remove(&mut self, session_id: &str) -> bool {
        let previous = self.sessions.len();
        self.sessions.retain(|session| session.id != session_id);
        previous != self.sessions.len()
    }
}

impl StoredSession {
    fn to_dto(&self, max_stats_points: usize) -> WorkspaceSessionDto {
        let mut connections: Vec<ConnectionAnalysis> = Vec::new();
        for source in &self.sources {
            for incoming in source.session.analysis().connections {
                if let Some(current) = connections
                    .iter_mut()
                    .find(|current| current.id == incoming.id)
                {
                    merge_connection(current, incoming);
                } else {
                    connections.push(incoming);
                }
            }
        }
        let stats_sample_count = connections
            .iter()
            .map(|connection| connection.stats.len())
            .sum();

        for connection in &mut connections {
            infer_ice_gathering(connection);
            if max_stats_points > 0 {
                connection.stats = downsample(&connection.stats, max_stats_points);
            } else {
                connection.stats.clear();
            }
        }

        let issues = unique_issues(
            self.sources
                .iter()
                .flat_map(|source| source.session.issues.iter().cloned())
                .collect(),
        );
        let formats = unique_formats(
            self.sources
                .iter()
                .map(|source| source.session.source.format.clone()),
        );
        let start_time = finite_min(
            self.sources
                .iter()
                .filter_map(|source| source.session.summary().start_time),
        );
        let end_time = finite_max(
            self.sources
                .iter()
                .filter_map(|source| source.session.summary().end_time),
        );
        let event_count = connections
            .iter()
            .map(|connection| connection.events.len())
            .sum();
        let capabilities = Capabilities {
            has_api_trace: self
                .sources
                .iter()
                .any(|source| source.session.capabilities.has_api_trace),
            has_raw_sdp: self
                .sources
                .iter()
                .any(|source| source.session.capabilities.has_raw_sdp),
            has_stats: self
                .sources
                .iter()
                .any(|source| source.session.capabilities.has_stats),
            has_candidate_address: self
                .sources
                .iter()
                .any(|source| source.session.capabilities.has_candidate_address),
            is_possibly_truncated: self
                .sources
                .iter()
                .any(|source| source.session.capabilities.is_possibly_truncated),
        };

        WorkspaceSessionDto {
            id: self.id.clone(),
            name: self.name.clone(),
            color: self.color.clone(),
            sources: self
                .sources
                .iter()
                .map(|source| SessionSourceDto {
                    id: source.id.clone(),
                    path: source.path.clone(),
                    name: file_name(&source.path),
                    format: source.session.source.format.clone(),
                    file_size: source.session.source.file_size,
                    compressed: source.session.source.compressed,
                })
                .collect(),
            source_errors: self.source_errors.clone(),
            summary: WorkspaceSessionSummary {
                source_count: self.sources.len(),
                file_names: self
                    .sources
                    .iter()
                    .map(|source| file_name(&source.path))
                    .collect(),
                formats,
                file_size: self
                    .sources
                    .iter()
                    .map(|source| source.session.source.file_size)
                    .sum(),
                user_agent: self
                    .sources
                    .iter()
                    .find_map(|source| source.session.client.user_agent.clone()),
                peer_connection_count: connections.len(),
                event_count,
                stats_sample_count,
                start_time,
                end_time,
                warning_count: issues
                    .iter()
                    .filter(|issue| issue.severity == IssueSeverity::Warning)
                    .count()
                    + self.source_errors.len(),
            },
            capabilities,
            issues,
            analysis: SessionAnalysis { connections },
        }
    }
}

fn merge_connection(current: &mut ConnectionAnalysis, incoming: ConnectionAnalysis) {
    current.url = incoming.url.or_else(|| current.url.clone());
    if !incoming.configuration.is_null() {
        current.configuration = incoming.configuration;
    }
    merge_states(&mut current.states, incoming.states);
    append_unique_events(&mut current.events, incoming.events);
    append_unique_descriptions(&mut current.descriptions, incoming.descriptions);
    append_unique_candidates(&mut current.ice_candidates, incoming.ice_candidates);
    merge_pairs(&mut current.candidate_pairs, incoming.candidate_pairs);
    append_unique_stats(&mut current.stats, incoming.stats);
    merge_media(&mut current.media, incoming.media);
    append_unique_findings(&mut current.findings, incoming.findings);
}

fn merge_states(current: &mut ConnectionStates, incoming: ConnectionStates) {
    current.signaling = incoming.signaling.or_else(|| current.signaling.clone());
    current.connection = incoming.connection.or_else(|| current.connection.clone());
    current.ice_connection = incoming
        .ice_connection
        .or_else(|| current.ice_connection.clone());
    if incoming.ice_gathering.is_some() {
        current.ice_gathering = incoming.ice_gathering;
        current.ice_gathering_inferred = false;
    } else {
        current.ice_gathering_inferred |= incoming.ice_gathering_inferred;
    }
}

fn infer_ice_gathering(connection: &mut ConnectionAnalysis) {
    if connection.states.ice_gathering.is_none()
        && !connection.ice_candidates.is_empty()
        && connection
            .candidate_pairs
            .iter()
            .any(|pair| pair.selected || pair.nominated)
    {
        connection.states.ice_gathering = Some("complete".into());
        connection.states.ice_gathering_inferred = true;
    }
}

fn append_unique_events(current: &mut Vec<EventSummary>, incoming: Vec<EventSummary>) {
    let mut seen: HashSet<String> = current.iter().map(event_key).collect();
    current.extend(
        incoming
            .into_iter()
            .filter(|value| seen.insert(event_key(value))),
    );
    current.sort_by(|left, right| left.timestamp.total_cmp(&right.timestamp));
}

fn event_key(value: &EventSummary) -> String {
    format!(
        "{}:{}:{}",
        value.event_type,
        serde_json::to_string(&value.value).unwrap_or_default(),
        value.timestamp.round()
    )
}

fn append_unique_descriptions(
    current: &mut Vec<SessionDescriptionSummary>,
    incoming: Vec<SessionDescriptionSummary>,
) {
    let mut seen: HashSet<String> = current.iter().map(description_key).collect();
    current.extend(
        incoming
            .into_iter()
            .filter(|value| seen.insert(description_key(value))),
    );
    current.sort_by(|left, right| left.timestamp.total_cmp(&right.timestamp));
}

fn description_key(value: &SessionDescriptionSummary) -> String {
    format!("{}:{}", value.description_type, value.sdp)
}

fn append_unique_candidates(
    current: &mut Vec<IceCandidateSummary>,
    incoming: Vec<IceCandidateSummary>,
) {
    let mut seen: HashSet<String> = current.iter().map(candidate_key).collect();
    current.extend(
        incoming
            .into_iter()
            .filter(|value| seen.insert(candidate_key(value))),
    );
}

fn candidate_key(value: &IceCandidateSummary) -> String {
    format!(
        "{}:{:?}:{:?}:{:?}:{:?}",
        value.side, value.address, value.port, value.protocol, value.candidate_type
    )
}

fn merge_pairs(current: &mut Vec<CandidatePairSummary>, incoming: Vec<CandidatePairSummary>) {
    for pair in incoming {
        if let Some(index) = current.iter().position(|value| value.id == pair.id) {
            if pair.selected || pair.nominated {
                current[index] = pair;
            }
        } else {
            current.push(pair);
        }
    }
}

fn append_unique_stats(current: &mut Vec<StatsPoint>, incoming: Vec<StatsPoint>) {
    let mut seen: HashSet<String> = current.iter().map(stats_key).collect();
    current.extend(
        incoming
            .into_iter()
            .filter(|value| seen.insert(stats_key(value))),
    );
    current.sort_by(|left, right| left.timestamp.total_cmp(&right.timestamp));
}

fn stats_key(value: &StatsPoint) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| value.timestamp.to_string())
}

fn merge_media(current: &mut MediaSummary, incoming: MediaSummary) {
    current.inbound_audio = current.inbound_audio.max(incoming.inbound_audio);
    current.inbound_video = current.inbound_video.max(incoming.inbound_video);
    current.outbound_audio = current.outbound_audio.max(incoming.outbound_audio);
    current.outbound_video = current.outbound_video.max(incoming.outbound_video);
    current.codecs.extend(incoming.codecs);
    current.codecs.sort();
    current.codecs.dedup();
}

fn append_unique_findings(current: &mut Vec<AnalysisFinding>, incoming: Vec<AnalysisFinding>) {
    let mut seen: HashSet<String> = current.iter().map(finding_key).collect();
    current.extend(
        incoming
            .into_iter()
            .filter(|value| seen.insert(finding_key(value))),
    );
    current.sort_by(|left, right| {
        left.timestamp
            .unwrap_or(0.0)
            .total_cmp(&right.timestamp.unwrap_or(0.0))
    });
}

fn finding_key(value: &AnalysisFinding) -> String {
    format!(
        "{:?}:{}:{}:{:?}",
        value.severity, value.title, value.message, value.timestamp
    )
}

fn unique_issues(values: Vec<ImportIssue>) -> Vec<ImportIssue> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|issue| {
            seen.insert(format!(
                "{}:{}:{:?}:{:?}",
                issue.code, issue.message, issue.line, issue.json_path
            ))
        })
        .collect()
}

fn unique_formats(values: impl Iterator<Item = DumpFormat>) -> Vec<DumpFormat> {
    let mut seen = HashSet::new();
    values
        .filter(|format| seen.insert(format!("{format:?}")))
        .collect()
}

fn finite_min(values: impl Iterator<Item = f64>) -> Option<f64> {
    values
        .filter(|value| value.is_finite())
        .min_by(f64::total_cmp)
}

fn finite_max(values: impl Iterator<Item = f64>) -> Option<f64> {
    values
        .filter(|value| value.is_finite())
        .max_by(f64::total_cmp)
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn downsample(points: &[StatsPoint], max_points: usize) -> Vec<StatsPoint> {
    if max_points == 0 || points.len() <= max_points {
        return points.to_vec();
    }
    if max_points == 1 {
        return vec![points[0].clone()];
    }
    (0..max_points)
        .map(|index| {
            let source_index = index * (points.len() - 1) / (max_points - 1);
            points[source_index].clone()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{SessionStore, StoredSource};
    use rtc_domain::{
        CandidatePairSummary, DumpFormat, IceCandidateSummary, RtcSession, SourceInfo, StatsPoint,
    };

    fn source(path: &str, format: DumpFormat, stats_count: usize) -> StoredSource {
        let mut session = RtcSession::new(SourceInfo {
            format,
            file_name: path.into(),
            file_size: 100,
            compressed: true,
            format_version: None,
        });
        let connection = session.connection_mut("19-1");
        connection.stats = (0..stats_count)
            .map(|index| rtc_domain::StatsSample {
                timestamp: index as f64,
                reports: Default::default(),
                source_ref: rtc_domain::SourceRef {
                    line: None,
                    json_path: None,
                },
            })
            .collect();
        StoredSource::new(path.into(), session)
    }

    #[test]
    fn stores_multiple_sources_in_one_session() {
        let mut store = SessionStore::default();
        let id = store
            .add_sources(
                None,
                vec![
                    source("rtc.gz", DumpFormat::RtcStats, 1),
                    source("internals.gz", DumpFormat::WebRtcInternals, 1),
                ],
                vec![],
            )
            .unwrap()
            .unwrap();
        let session = store.get(&id, 2_000).unwrap();
        assert_eq!(session.summary.source_count, 2);
        assert_eq!(session.analysis.connections.len(), 1);
    }

    #[test]
    fn can_remove_a_session() {
        let mut store = SessionStore::default();
        let id = store
            .add_sources(
                None,
                vec![source("rtc.gz", DumpFormat::RtcStats, 0)],
                vec![],
            )
            .unwrap()
            .unwrap();
        assert!(store.remove(&id));
        assert!(store.get(&id, 100).is_none());
    }

    #[test]
    fn limits_returned_stats_points() {
        let points: Vec<StatsPoint> = (0..10)
            .map(|index| StatsPoint {
                timestamp: index as f64,
                outgoing_bitrate_kbps: Some(index as f64),
                incoming_bitrate_kbps: None,
                packets_lost: None,
                current_round_trip_time_ms: None,
                available_outgoing_bitrate_kbps: None,
                frames_per_second: None,
            })
            .collect();
        let sampled = super::downsample(&points, 4);
        assert_eq!(sampled.len(), 4);
        assert_eq!(sampled.first().unwrap().timestamp, 0.0);
        assert_eq!(sampled.last().unwrap().timestamp, 9.0);
    }

    #[test]
    fn infers_ice_gathering_from_candidates_and_pair() {
        let mut store = SessionStore::default();
        let mut stored = source("rtc.gz", DumpFormat::RtcStats, 0);
        let connection = stored.session.connection_mut("19-1");
        connection.events.clear();
        let id = store
            .add_sources(None, vec![stored], vec![])
            .unwrap()
            .unwrap();
        let mut dto = store.get(&id, 100).unwrap();
        let connection = &mut dto.analysis.connections[0];
        connection.ice_candidates.push(IceCandidateSummary {
            id: "local".into(),
            side: "local".into(),
            address: None,
            port: None,
            protocol: None,
            candidate_type: None,
            network_type: None,
            relay_protocol: None,
            priority: None,
        });
        connection.candidate_pairs.push(CandidatePairSummary {
            id: "pair".into(),
            state: None,
            nominated: true,
            selected: true,
            local_candidate_id: None,
            remote_candidate_id: None,
            current_round_trip_time_ms: None,
            available_outgoing_bitrate_kbps: None,
            bytes_sent: None,
            bytes_received: None,
        });
        super::infer_ice_gathering(connection);
        assert_eq!(connection.states.ice_gathering.as_deref(), Some("complete"));
    }
}

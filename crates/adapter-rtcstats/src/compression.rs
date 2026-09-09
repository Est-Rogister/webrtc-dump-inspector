use serde_json::{Map, Value};

pub fn decompress_method(value: &Value) -> String {
    match value {
        Value::String(value) if value == "g" => "getStats".into(),
        Value::String(value) => value.clone(),
        Value::Number(value) => match value.as_i64() {
            Some(1) => "getStats".into(),
            Some(2) => "createOffer".into(),
            Some(3) => "createOfferOnSuccess".into(),
            Some(4) => "createAnswer".into(),
            Some(5) => "createAnswerOnSuccess".into(),
            Some(6) => "setLocalDescription".into(),
            Some(7) => "setLocalDescriptionOnSuccess".into(),
            Some(8) => "setRemoteDescription".into(),
            Some(9) => "setRemoteDescriptionOnSuccess".into(),
            Some(10) => "addIceCandidate".into(),
            Some(11) => "addIceCandidateOnSuccess".into(),
            Some(12) => "createOfferOnFailure".into(),
            Some(13) => "createAnswerOnFailure".into(),
            Some(14) => "setLocalDescriptionOnFailure".into(),
            Some(15) => "setRemoteDescriptionOnFailure".into(),
            Some(16) => "addIceCandidateOnFailure".into(),
            Some(20) => "onicecandidate".into(),
            Some(21) => "onicecandidateerror".into(),
            Some(22) => "ontrack".into(),
            Some(23) => "onsignalingstatechange".into(),
            Some(24) => "oniceconnectionstatechange".into(),
            Some(25) => "onconnectionstatechange".into(),
            Some(26) => "onicegatheringstatechange".into(),
            Some(27) => "onnegotiationneeded".into(),
            Some(28) => "ondatachannel".into(),
            Some(30) => "addTrack".into(),
            Some(31) => "addTrackOnSuccess".into(),
            Some(32) => "addTransceiver".into(),
            Some(33) => "addTransceiverOnSuccess".into(),
            Some(34) => "removeTrack".into(),
            Some(35) => "createDataChannel".into(),
            Some(36) => "close".into(),
            Some(37) => "restartIce".into(),
            Some(38) => "setConfiguration".into(),
            Some(39) => "create".into(),
            Some(40) => "constraints".into(),
            Some(41) => "setCodecPreferences".into(),
            Some(42) => "setHeaderExtensionsToNegotiate".into(),
            Some(50) => "setParameters".into(),
            Some(51) => "replaceTrack".into(),
            Some(60) => "navigator.mediaDevices.getUserMedia".into(),
            Some(61) => "navigator.mediaDevices.getUserMediaOnSuccess".into(),
            Some(63) => "navigator.mediaDevices.getDisplayMedia".into(),
            Some(64) => "navigator.mediaDevices.getDisplayMediaOnSuccess".into(),
            Some(65) => "navigator.mediaDevices.getUserMediaOnFailure".into(),
            Some(66) => "navigator.mediaDevices.enumerateDevices".into(),
            Some(67) => "navigator.mediaDevices.ondevicechange".into(),
            Some(68) => "navigator.mediaDevices.getDisplayMediaOnFailure".into(),
            Some(70) => "MediaStreamTrack.stop".into(),
            Some(71) => "MediaStreamTrack.applyConstraints".into(),
            Some(72) => "MediaStreamTrack.onended".into(),
            Some(73) => "MediaStreamTrack.onmute".into(),
            Some(74) => "MediaStreamTrack.onunmute".into(),
            Some(80) => "HTMLMediaElement.resize".into(),
            _ => value.to_string(),
        },
        other => other.to_string(),
    }
}

fn decompress_stats_type(value: &mut Value) {
    let Some(number) = value.as_i64() else {
        return;
    };
    let result = match number {
        1 => "inbound-rtp",
        2 => "outbound-rtp",
        3 => "remote-inbound-rtp",
        4 => "remote-outbound-rtp",
        5 => "transport",
        6 => "candidate-pair",
        7 => "local-candidate",
        8 => "remote-candidate",
        9 => "data-channel",
        10 => "peer-connection",
        11 => "certificate",
        12 => "media-source",
        13 => "media-playout",
        14 => "codec",
        _ => return,
    };
    *value = Value::String(result.into());
}

fn decompress_property(key: &str) -> &str {
    match key {
        "t" => "timestamp",
        "4" => "payloadType",
        "5" => "transportId",
        "6" => "mimeType",
        "7" => "clockRate",
        "8" => "channels",
        "9" => "sdpFmtpLine",
        "10" => "ssrc",
        "11" => "kind",
        "12" => "codecId",
        "14" => "packetsReceived",
        "15" => "packetsLost",
        "16" => "jitter",
        "17" => "trackIdentifier",
        "18" => "mid",
        "19" => "remoteId",
        "20" => "framesDecoded",
        "21" => "keyFramesDecoded",
        "22" => "framesDropped",
        "23" => "frameWidth",
        "24" => "frameHeight",
        "25" => "framesPerSecond",
        "26" => "qpSum",
        "27" => "totalDecodeTime",
        "28" => "totalInterFrameDelay",
        "29" => "totalSquaredInterFrameDelay",
        "30" => "pauseCount",
        "31" => "totalPausesDuration",
        "32" => "freezeCount",
        "33" => "totalFreezesDuration",
        "34" => "lastPacketReceivedTimestamp",
        "35" => "headerBytesReceived",
        "36" => "packetsDiscarded",
        "37" => "fecPacketsReceived",
        "38" => "fecPacketsDiscarded",
        "39" => "fecBytesReceived",
        "40" => "fecSsrc",
        "41" => "bytesReceived",
        "42" => "nackCount",
        "43" => "firCount",
        "44" => "pliCount",
        "45" => "totalProcessingDelay",
        "46" => "estimatedPlayoutTimestamp",
        "47" => "jitterBufferDelay",
        "48" => "jitterBufferTargetDelay",
        "49" => "jitterBufferEmittedCount",
        "50" => "jitterBufferMinimumDelay",
        "51" => "totalSamplesReceived",
        "52" => "concealedSamples",
        "53" => "silentConcealedSamples",
        "54" => "concealmentEvents",
        "55" => "insertedSamplesForDeceleration",
        "56" => "removedSamplesForAcceleration",
        "57" => "audioLevel",
        "58" => "totalAudioEnergy",
        "59" => "totalSamplesDuration",
        "60" => "framesReceived",
        "61" => "decoderImplementation",
        "62" => "playoutId",
        "63" => "powerEfficientDecoder",
        "64" => "framesAssembledFromMultiplePackets",
        "65" => "totalAssemblyTime",
        "66" => "contentType",
        "67" => "googTimingFrameInfo",
        "68" => "retransmittedPacketsReceived",
        "69" => "retransmittedBytesReceived",
        "70" => "rtxSsrc",
        "71" => "totalCorruptionProbability",
        "72" => "totalSquaredCorruptionProbability",
        "73" => "corruptionMeasurements",
        "74" => "localId",
        "75" => "roundTripTime",
        "76" => "totalRoundTripTime",
        "77" => "fractionLost",
        "78" => "roundTripTimeMeasurements",
        "79" => "packetsSent",
        "80" => "bytesSent",
        "81" => "mediaSourceId",
        "82" => "rid",
        "83" => "encodingIndex",
        "84" => "headerBytesSent",
        "85" => "retransmittedPacketsSent",
        "86" => "retransmittedBytesSent",
        "87" => "targetBitrate",
        "88" => "totalEncodedBytesTarget",
        "89" => "framesSent",
        "90" => "hugeFramesSent",
        "91" => "framesEncoded",
        "92" => "keyFramesEncoded",
        "93" => "totalEncodeTime",
        "94" => "totalPacketSendDelay",
        "95" => "qualityLimitationReason",
        "96" => "qualityLimitationDurations",
        "97" => "qualityLimitationResolutionChanges",
        "98" => "encoderImplementation",
        "99" => "powerEfficientEncoder",
        "100" => "active",
        "101" => "scalabilityMode",
        "102" => "remoteTimestamp",
        "103" => "reportsSent",
        "104" => "echoReturnLoss",
        "105" => "echoReturnLossEnhancement",
        "106" => "width",
        "107" => "height",
        "108" => "frames",
        "109" => "synthesizedSamplesDuration",
        "110" => "synthesizedSamplesEvents",
        "111" => "totalPlayoutDelay",
        "112" => "totalSamplesCount",
        "113" => "dataChannelsOpened",
        "114" => "dataChannelsClosed",
        "115" => "label",
        "116" => "protocol",
        "117" => "dataChannelIdentifier",
        "118" => "state",
        "119" => "messagesSent",
        "120" => "messagesReceived",
        "121" => "iceRole",
        "122" => "iceLocalUsernameFragment",
        "123" => "dtlsState",
        "124" => "iceState",
        "125" => "selectedCandidatePairId",
        "126" => "localCertificateId",
        "127" => "remoteCertificateId",
        "128" => "tlsVersion",
        "129" => "dtlsCipher",
        "130" => "dtlsRole",
        "131" => "srtpCipher",
        "132" => "selectedCandidatePairChanges",
        "133" => "rtcpTransportStatsId",
        "134" => "address",
        "135" => "port",
        "136" => "candidateType",
        "137" => "priority",
        "138" => "url",
        "139" => "relayProtocol",
        "140" => "foundation",
        "141" => "relatedAddress",
        "142" => "relatedPort",
        "143" => "usernameFragment",
        "144" => "tcpType",
        "145" => "networkType",
        "148" => "localCandidateId",
        "149" => "remoteCandidateId",
        "150" => "nominated",
        "151" => "lastPacketSentTimestamp",
        "152" => "currentRoundTripTime",
        "153" => "availableOutgoingBitrate",
        "154" => "availableIncomingBitrate",
        "155" => "requestsReceived",
        "156" => "requestsSent",
        "157" => "responsesReceived",
        "158" => "responsesSent",
        "159" => "consentRequestsSent",
        "160" => "packetsDiscardedOnSend",
        "161" => "bytesDiscardedOnSend",
        "163" => "fingerprint",
        "164" => "fingerprintAlgorithm",
        "165" => "base64Certificate",
        "166" => "issuerCertificateId",
        "167" => "psnrMeasurements",
        "168" => "psnrSum",
        "169" => "packetsReceivedWithEct1",
        "170" => "packetsReceivedWithCe",
        "171" => "packetsReportedAsLost",
        "172" => "packetsReportedAsLostButRecovered",
        "173" => "packetsWithBleachedEct1Marking",
        "174" => "packetsSentWithEct1",
        "175" => "ccfbMessagesSent",
        "176" => "ccfbMessagesReceived",
        _ => key,
    }
}

pub fn decompress_stats(
    base: &Map<String, Value>,
    delta: &Map<String, Value>,
) -> Map<String, Value> {
    let mut current = Map::new();
    let mut collapsed_timestamp = None;

    for (id, value) in delta {
        if id == "t" {
            collapsed_timestamp = Some(value.clone());
            continue;
        }
        if value.is_null() {
            current.insert(id.clone(), Value::Null);
            continue;
        }
        let Some(report) = value.as_object() else {
            continue;
        };
        let mut restored = Map::new();
        for (key, value) in report {
            let restored_key = decompress_property(key).to_string();
            let mut restored_value = value.clone();
            if restored_key == "type" {
                decompress_stats_type(&mut restored_value);
            }
            restored.insert(restored_key, restored_value);
        }
        current.insert(id.clone(), Value::Object(restored));
    }

    for (id, base_value) in base {
        if current.get(id).is_some_and(Value::is_null) {
            current.remove(id);
            continue;
        }

        let explicit_timestamp = current
            .get(id)
            .and_then(Value::as_object)
            .is_some_and(|report| report.contains_key("timestamp"));

        match current.get_mut(id) {
            None => {
                current.insert(id.clone(), base_value.clone());
            }
            Some(Value::Object(next_report)) => {
                if let Some(base_report) = base_value.as_object() {
                    for (name, old_value) in base_report {
                        match next_report.get_mut(name) {
                            None => {
                                next_report.insert(name.clone(), old_value.clone());
                            }
                            Some(Value::Object(next_object)) if old_value.is_object() => {
                                if let Some(old_object) = old_value.as_object() {
                                    for (nested_key, nested_value) in old_object {
                                        next_object
                                            .entry(nested_key.clone())
                                            .or_insert_with(|| nested_value.clone());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }

        if !explicit_timestamp {
            if let (Some(timestamp), Some(report)) = (
                collapsed_timestamp.as_ref(),
                current.get_mut(id).and_then(Value::as_object_mut),
            ) {
                report.insert("timestamp".into(), timestamp.clone());
            }
        }
    }

    if let Some(timestamp) = collapsed_timestamp {
        for value in current.values_mut() {
            if let Some(report) = value.as_object_mut() {
                report
                    .entry("timestamp")
                    .or_insert_with(|| timestamp.clone());
            }
        }
    }

    current
}

pub fn decompress_description(base: &Value, next: &Value) -> Value {
    let (Some(base_object), Some(next_object)) = (base.as_object(), next.as_object()) else {
        return next.clone();
    };
    let (Some(base_type), Some(next_type), Some(base_sdp), Some(next_sdp)) = (
        base_object.get("type").and_then(Value::as_str),
        next_object.get("type").and_then(Value::as_str),
        base_object.get("sdp").and_then(Value::as_str),
        next_object.get("sdp").and_then(Value::as_str),
    ) else {
        return next.clone();
    };
    if base_type != next_type || base_sdp.is_empty() || next_sdp.is_empty() {
        return next.clone();
    }

    let base_sections = split_sections(base_sdp);
    let next_sections = split_sections(next_sdp);
    let restored = next_sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            if *section == if index == 0 { "v=" } else { "m=" } {
                base_sections.get(index).copied().unwrap_or(section)
            } else {
                section
            }
        })
        .collect::<Vec<_>>()
        .join("\r\n")
        .trim()
        .to_string()
        + "\r\n";

    let mut result = next_object.clone();
    result.insert("sdp".into(), Value::String(restored));
    Value::Object(result)
}

fn split_sections(sdp: &str) -> Vec<&str> {
    let mut sections = Vec::new();
    let mut start = 0;
    for (index, _) in sdp.match_indices("\nm=") {
        sections.push(sdp[start..index].trim());
        start = index + 1;
    }
    sections.push(sdp[start..].trim());
    sections
}

#[cfg(test)]
mod tests {
    use super::{decompress_description, decompress_stats};
    use serde_json::{json, Map, Value};

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn restores_delta_stats_and_falsy_values() {
        let base = object(json!({
            "id": {"type": "outbound-rtp", "timestamp": 1, "framesPerSecond": 30, "active": true}
        }));
        let delta = object(json!({
            "t": 2,
            "id": {"25": 0, "100": false}
        }));
        assert_eq!(
            decompress_stats(&base, &delta),
            object(json!({
                "id": {"type": "outbound-rtp", "timestamp": 2, "framesPerSecond": 0, "active": false}
            }))
        );
    }

    #[test]
    fn keeps_explicit_report_timestamp() {
        let base = object(json!({
            "a": {"timestamp": 100, "packetsReceived": 1},
            "b": {"timestamp": 50, "roundTripTime": 0.1}
        }));
        let delta = object(json!({
            "t": 200,
            "a": {"14": 2},
            "b": {"t": 150, "75": 0.2}
        }));
        let result = decompress_stats(&base, &delta);
        assert_eq!(result["a"]["timestamp"], 200);
        assert_eq!(result["b"]["timestamp"], 150);
    }

    #[test]
    fn restores_compressed_sdp_sections() {
        let base = json!({"type": "offer", "sdp": "v=0\r\na=one\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\na=two\r\n"});
        let next = json!({"type": "offer", "sdp": "v=\r\nm=\r\n"});
        assert_eq!(decompress_description(&base, &next), base);
    }
}

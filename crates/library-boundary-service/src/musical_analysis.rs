use std::path::{Path, PathBuf};

use analyzer_musical_stratum::{
    ADAPTER_KEY, ADAPTER_VERSION, DekzerKeyMode, MusicalAnalysisFailure, MusicalAnalysisInput,
    MusicalAnalysisResult, MusicalAnalysisStatus, MusicalAnalysisWarningKind, UPSTREAM_CRATE_NAME,
    UPSTREAM_CRATE_VERSION, analyze_musical_input,
};
use library_boundary_protocol as protocol;
use library_store_sqlite::{
    SqliteDurableStore, StorePlayableMediaAnalysisTarget, StorePlayableMediaAnalysisTargetError,
};

use crate::service::{map_store_error, require_positive_i64};

const DECODER_POLICY: &str = "hound_16_bit_integer_pcm_wav_to_mono_f32_v1";
const BLOCKED_INPUT_POLICY: &str = "blocked_before_adapter_invocation_v1";
const BLOCKED_CHANNEL_MIXDOWN_POLICY: &str = "not_applicable_adapter_not_invoked_v1";
const BLOCKED_NORMALIZATION_POLICY: &str = "not_applicable_adapter_not_invoked_v1";
const BLOCKED_ANALYSIS_CONFIG_POLICY: &str = "not_applicable_adapter_not_invoked_v1";
const AUTHORITY_POLICY: &str = "non_authoritative_advisory_v1";
const BEAT_PREVIEW_LIMIT: usize = 8;

pub(crate) fn handle_musical_analysis_command(
    store: &SqliteDurableStore,
    refresh_source: impl FnOnce(i64) -> protocol::ProtocolResult<()>,
    command: protocol::MusicalAnalysisCommand,
) -> protocol::ProtocolResult<protocol::MusicalAnalysisReply> {
    match command {
        protocol::MusicalAnalysisCommand::AnalyzePlayableMedia(request) => {
            analyze_playable_media(store, refresh_source, request)
                .map(protocol::MusicalAnalysisReply::AnalyzePlayableMedia)
        }
    }
}

fn analyze_playable_media(
    store: &SqliteDurableStore,
    refresh_source: impl FnOnce(i64) -> protocol::ProtocolResult<()>,
    request: protocol::AnalyzePlayableMediaRequest,
) -> protocol::ProtocolResult<protocol::AnalyzePlayableMediaReply> {
    let playable_media_id = require_positive_i64(request.playable_media_id, "playableMediaId")?;
    let source_id = require_positive_i64(request.source_id, "sourceId")?;
    let source_file_id = require_positive_i64(request.source_file_id, "sourceFileId")?;
    let attachment_id = require_positive_i64(request.attachment_id, "attachmentId")?;

    let request_target = protocol::TrackMusicalAnalysisTarget {
        playable_media_id,
        source_id,
        source_file_id,
        attachment_id,
        relative_path: None,
        media_kind: None,
    };
    let source_file_basis =
        match store.read_playable_media_analysis_source_file_basis(playable_media_id) {
            Ok(source_file_basis) => source_file_basis,
            Err(StorePlayableMediaAnalysisTargetError::Store(error)) => {
                return Err(map_store_error(error));
            }
            Err(error) => {
                return Ok(protocol::AnalyzePlayableMediaReply {
                    result: blocked_target_resolution_result(request_target, error),
                });
            }
        };

    refresh_source(source_file_basis.source_id)?;

    let target = match store.resolve_playable_media_analysis_target(playable_media_id) {
        Ok(target) => target,
        Err(StorePlayableMediaAnalysisTargetError::Store(error)) => {
            return Err(map_store_error(error));
        }
        Err(error) => {
            return Ok(protocol::AnalyzePlayableMediaReply {
                result: blocked_target_resolution_result(request_target, error),
            });
        }
    };
    let target_view = target_view(&target);

    if let Some(result) = validate_selected_target_consistency(
        &target,
        target_view.clone(),
        source_id,
        source_file_id,
        attachment_id,
    ) {
        return Ok(protocol::AnalyzePlayableMediaReply { result });
    }

    if target.media_kind != "audio" {
        return Ok(protocol::AnalyzePlayableMediaReply {
            result: unsupported_result(
                target_view,
                "unsupported_media_kind",
                format!(
                    "Musical analysis V1 only attempts audio playable media; selected media_kind is {:?}.",
                    target.media_kind
                ),
            ),
        });
    }

    let decoded = match decode_hound_wav_16_bit_mono_f32(&target.path) {
        Ok(decoded) => decoded,
        Err(error) => {
            return Ok(protocol::AnalyzePlayableMediaReply {
                result: decode_failure_result(target_view, error),
            });
        }
    };

    match analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_dekzer_wav(
        &decoded.samples,
        decoded.sample_rate_hz,
    )) {
        Ok(result) => Ok(protocol::AnalyzePlayableMediaReply {
            result: adapter_result(target_view, result),
        }),
        Err(error) => Ok(protocol::AnalyzePlayableMediaReply {
            result: adapter_failure_result(target_view, decoded.sample_rate_hz, error),
        }),
    }
}

fn validate_selected_target_consistency(
    target: &StorePlayableMediaAnalysisTarget,
    target_view: protocol::TrackMusicalAnalysisTarget,
    source_id: i64,
    source_file_id: i64,
    attachment_id: i64,
) -> Option<protocol::TrackMusicalAnalysisResult> {
    let current_source_file_id = target.source_file_basis.source_file_id;
    if current_source_file_id != source_file_id {
        return Some(blocked_result(
            target_view,
            "selected_source_file_mismatch",
            format!(
                "The selected row is stale: requested sourceFileId {source_file_id}, but playableMediaId {} currently resolves to sourceFileId {current_source_file_id}.",
                target.playable_media_id
            ),
        ));
    }

    let current_source_id = target.source_file_basis.source_id;
    if current_source_id != source_id {
        return Some(blocked_result(
            target_view,
            "selected_source_mismatch",
            format!(
                "The selected row is stale: requested sourceId {source_id}, but sourceFileId {source_file_id} currently belongs to sourceId {current_source_id}."
            ),
        ));
    }

    if target.attachment_id != attachment_id {
        return Some(blocked_result(
            target_view,
            "selected_attachment_mismatch",
            format!(
                "The selected row is stale: requested attachmentId {attachment_id}, but playableMediaId {} currently has attachmentId {}.",
                target.playable_media_id, target.attachment_id
            ),
        ));
    }

    None
}

fn decode_hound_wav_16_bit_mono_f32(path: &Path) -> Result<DecodedWavInput, DecodeFailure> {
    let mut reader =
        hound::WavReader::open(path).map_err(|error| map_wav_open_error(path, error))?;
    let spec = reader.spec();

    if spec.sample_format != hound::SampleFormat::Int {
        return Err(DecodeFailure::UnsupportedSampleFormat(
            "only integer PCM WAV is supported in this V1 slice".to_string(),
        ));
    }
    if spec.bits_per_sample != 16 {
        return Err(DecodeFailure::UnsupportedBitDepth(format!(
            "only 16-bit PCM WAV is supported in this V1 slice; file reports {} bits per sample",
            spec.bits_per_sample
        )));
    }
    if spec.channels == 0 {
        return Err(DecodeFailure::MalformedWav(
            "WAV reports zero channels".to_string(),
        ));
    }

    let channel_count = usize::from(spec.channels);
    let frame_capacity = usize::try_from(reader.duration()).unwrap_or(0);
    let mut mono = Vec::with_capacity(frame_capacity);
    let mut frame_sum = 0.0_f32;
    let mut channel_index = 0_usize;

    for sample in reader.samples::<i16>() {
        let sample = sample.map_err(|error| DecodeFailure::DecodeError(error.to_string()))?;
        frame_sum += f32::from(sample) / 32768.0;
        channel_index += 1;

        if channel_index == channel_count {
            mono.push(frame_sum / channel_count as f32);
            frame_sum = 0.0;
            channel_index = 0;
        }
    }

    if channel_index != 0 {
        return Err(DecodeFailure::DecodeError(
            "decoded sample count ended inside an unfinished frame".to_string(),
        ));
    }

    Ok(DecodedWavInput {
        samples: mono,
        sample_rate_hz: spec.sample_rate,
    })
}

fn adapter_result(
    target: protocol::TrackMusicalAnalysisTarget,
    mut result: MusicalAnalysisResult,
) -> protocol::TrackMusicalAnalysisResult {
    let mut warnings = result
        .warnings
        .drain(..)
        .map(adapter_warning)
        .collect::<Vec<_>>();

    let status = match result.status {
        MusicalAnalysisStatus::Accepted => {
            warnings.push(warning(
                protocol::TrackMusicalAnalysisWarningSeverity::Warning,
                "adapter_accepted_downgraded",
                "Adapter returned Accepted, but Dekzer V1 exposes stratum-dsp output as advisory only.",
            ));
            protocol::TrackMusicalAnalysisStatus::Advisory
        }
        MusicalAnalysisStatus::Advisory => protocol::TrackMusicalAnalysisStatus::Advisory,
        MusicalAnalysisStatus::Inconclusive => protocol::TrackMusicalAnalysisStatus::Inconclusive,
        MusicalAnalysisStatus::Rejected | MusicalAnalysisStatus::Blocked => {
            protocol::TrackMusicalAnalysisStatus::Blocked
        }
    };

    protocol::TrackMusicalAnalysisResult {
        target,
        status,
        status_detail: status_detail(status),
        bpm: result.bpm.map(|bpm| protocol::TrackBpmEvidence {
            bpm: f64::from(bpm),
            confidence: result.bpm_confidence.map(f64::from),
        }),
        key: result.key.map(|key| protocol::TrackKeyEvidence {
            notation: key.notation,
            mode: match key.mode {
                DekzerKeyMode::Major => protocol::TrackKeyMode::Major,
                DekzerKeyMode::Minor => protocol::TrackKeyMode::Minor,
            },
            tonic_index: key.tonic_index,
            confidence: result.key_confidence.map(f64::from),
        }),
        beatgrid: Some(protocol::TrackBeatgridEvidence {
            beat_count: result.beat_positions.len(),
            preview_seconds: result
                .beat_positions
                .iter()
                .take(BEAT_PREVIEW_LIMIT)
                .map(|beat| f64::from(*beat))
                .collect(),
            grid_stability: result.grid_stability.map(f64::from),
        }),
        warnings,
        basis: protocol::TrackMusicalAnalysisBasis {
            adapter_key: result.basis.adapter_key.to_string(),
            adapter_version: result.basis.adapter_version.to_string(),
            upstream_crate_name: result.basis.upstream_crate_name.to_string(),
            upstream_crate_version: result.basis.upstream_crate_version.to_string(),
            upstream_feature_flags: result
                .basis
                .upstream_feature_flags
                .into_iter()
                .map(str::to_string)
                .collect(),
            decoder_policy: DECODER_POLICY.to_string(),
            input_policy: result.basis.input_policy.to_string(),
            channel_mixdown_policy: result.basis.channel_mixdown_policy.to_string(),
            normalization_policy: result.basis.normalization_policy.to_string(),
            upstream_analysis_config_policy: result
                .basis
                .upstream_analysis_config_policy
                .to_string(),
            sample_rate_hz: Some(result.basis.sample_rate_hz),
            ml_enabled: result.basis.ml_enabled,
            persistence_authorized: false,
            authority: AUTHORITY_POLICY.to_string(),
        },
    }
}

fn adapter_failure_result(
    target: protocol::TrackMusicalAnalysisTarget,
    sample_rate_hz: u32,
    error: MusicalAnalysisFailure,
) -> protocol::TrackMusicalAnalysisResult {
    let mut result = blocked_result(
        target,
        "adapter_failure",
        format!(
            "Dekzer decoded the WAV input, but analyzer-musical-stratum could not produce an advisory result: {:?}: {}",
            error.kind, error.message
        ),
    );
    result.basis.sample_rate_hz = Some(sample_rate_hz);
    result
}

fn blocked_target_resolution_result(
    target: protocol::TrackMusicalAnalysisTarget,
    error: StorePlayableMediaAnalysisTargetError,
) -> protocol::TrackMusicalAnalysisResult {
    let (code, message) = match error {
        StorePlayableMediaAnalysisTargetError::Store(_) => {
            unreachable!("store errors are mapped before result construction")
        }
        StorePlayableMediaAnalysisTargetError::PlayableMediaNotFound { playable_media_id } => (
            "playable_media_not_found",
            format!("playableMediaId {playable_media_id} is not available for analysis."),
        ),
        StorePlayableMediaAnalysisTargetError::InvalidEvidenceSourceFileId {
            playable_media_id,
            source_file_id,
        } => (
            "invalid_evidence_source_file",
            format!(
                "playableMediaId {playable_media_id} has invalid evidenceSourceFileId {source_file_id}."
            ),
        ),
        StorePlayableMediaAnalysisTargetError::SourceFileNotFound { source_file_id } => (
            "source_file_not_found",
            format!("sourceFileId {source_file_id} is not available for analysis."),
        ),
        StorePlayableMediaAnalysisTargetError::SourceFileUnavailable {
            source_file_id,
            presence_state,
        } => (
            "source_file_unavailable",
            format!(
                "sourceFileId {source_file_id} is not present; presence_state is {presence_state}."
            ),
        ),
        StorePlayableMediaAnalysisTargetError::SourceRootUnavailable {
            source_file_id,
            mount_status,
            access_state,
            access_issue_kind,
            ..
        } => (
            "source_root_unavailable",
            format!(
                "source root for sourceFileId {source_file_id} is unavailable: mountStatus={mount_status:?}, accessState={access_state:?}, accessIssueKind={access_issue_kind:?}."
            ),
        ),
        StorePlayableMediaAnalysisTargetError::SourceRootMissing {
            source_file_id,
            detail,
            ..
        } => (
            "source_root_missing",
            format!("source root for sourceFileId {source_file_id} is missing: {detail:?}."),
        ),
        StorePlayableMediaAnalysisTargetError::SourceRootBlocked {
            source_file_id,
            access_issue_kind,
            detail,
            ..
        } => (
            "source_root_blocked",
            format!(
                "source root for sourceFileId {source_file_id} is blocked: accessIssueKind={access_issue_kind:?}, detail={detail:?}."
            ),
        ),
        StorePlayableMediaAnalysisTargetError::InvalidRelativePath {
            source_file_id,
            reason,
            ..
        } => (
            "invalid_relative_path",
            format!("sourceFileId {source_file_id} has an invalid relative path: {reason}."),
        ),
        StorePlayableMediaAnalysisTargetError::SourceFilePathEscapesRoot { source_file_id } => (
            "source_file_path_escapes_root",
            format!("sourceFileId {source_file_id} resolves outside its admitted source root."),
        ),
        StorePlayableMediaAnalysisTargetError::PhysicalFileMissing {
            source_file_id,
            path,
        } => (
            "physical_file_missing",
            format!(
                "sourceFileId {source_file_id} is missing on disk at {}.",
                path.display()
            ),
        ),
        StorePlayableMediaAnalysisTargetError::PhysicalFileBlocked {
            source_file_id,
            detail,
            ..
        } => (
            "physical_file_blocked",
            format!("sourceFileId {source_file_id} could not be accessed on disk: {detail}."),
        ),
    };

    blocked_result(target, code, message)
}

fn decode_failure_result(
    target: protocol::TrackMusicalAnalysisTarget,
    error: DecodeFailure,
) -> protocol::TrackMusicalAnalysisResult {
    let (status, code, message) = match error {
        DecodeFailure::UnsupportedContainer(message)
        | DecodeFailure::UnsupportedSampleFormat(message)
        | DecodeFailure::UnsupportedBitDepth(message) => (
            protocol::TrackMusicalAnalysisStatus::Unsupported,
            "unsupported_input",
            message,
        ),
        DecodeFailure::MalformedWav(message) | DecodeFailure::DecodeError(message) => (
            protocol::TrackMusicalAnalysisStatus::Blocked,
            "decode_failed",
            message,
        ),
        DecodeFailure::InputUnreadable { path, detail } => (
            protocol::TrackMusicalAnalysisStatus::Blocked,
            "input_unreadable",
            format!("could not read {}: {detail}", path.display()),
        ),
    };

    result_with_single_warning(
        target,
        status,
        code,
        format!("{message} Decoder policy: {DECODER_POLICY}."),
    )
}

fn unsupported_result(
    target: protocol::TrackMusicalAnalysisTarget,
    code: &'static str,
    message: String,
) -> protocol::TrackMusicalAnalysisResult {
    result_with_single_warning(
        target,
        protocol::TrackMusicalAnalysisStatus::Unsupported,
        code,
        message,
    )
}

fn blocked_result(
    target: protocol::TrackMusicalAnalysisTarget,
    code: &'static str,
    message: String,
) -> protocol::TrackMusicalAnalysisResult {
    result_with_single_warning(
        target,
        protocol::TrackMusicalAnalysisStatus::Blocked,
        code,
        message,
    )
}

fn result_with_single_warning(
    target: protocol::TrackMusicalAnalysisTarget,
    status: protocol::TrackMusicalAnalysisStatus,
    code: &'static str,
    message: String,
) -> protocol::TrackMusicalAnalysisResult {
    protocol::TrackMusicalAnalysisResult {
        target,
        status,
        status_detail: status_detail(status),
        bpm: None,
        key: None,
        beatgrid: None,
        warnings: vec![
            warning(
                protocol::TrackMusicalAnalysisWarningSeverity::Info,
                "non_authoritative",
                "Musical analysis V1 is advisory only; results are not persisted or canonical product facts.",
            ),
            warning(
                protocol::TrackMusicalAnalysisWarningSeverity::Warning,
                code,
                message,
            ),
        ],
        basis: blocked_basis(),
    }
}

fn status_detail(status: protocol::TrackMusicalAnalysisStatus) -> String {
    match status {
        protocol::TrackMusicalAnalysisStatus::Advisory => {
            "Advisory stratum-dsp evidence only; Dekzer has not validated it as product truth."
        }
        protocol::TrackMusicalAnalysisStatus::Inconclusive => {
            "Analysis ran, but evidence is inconclusive and non-authoritative."
        }
        protocol::TrackMusicalAnalysisStatus::Blocked => {
            "Analysis did not run to completion; the result is a visible blocked state."
        }
        protocol::TrackMusicalAnalysisStatus::Unsupported => {
            "This V1 path supports only selected audio inputs that decode as 16-bit PCM WAV."
        }
    }
    .to_string()
}

fn blocked_basis() -> protocol::TrackMusicalAnalysisBasis {
    protocol::TrackMusicalAnalysisBasis {
        adapter_key: ADAPTER_KEY.to_string(),
        adapter_version: ADAPTER_VERSION.to_string(),
        upstream_crate_name: UPSTREAM_CRATE_NAME.to_string(),
        upstream_crate_version: UPSTREAM_CRATE_VERSION.to_string(),
        upstream_feature_flags: vec![
            "default=[]".to_string(),
            "default-features=false".to_string(),
            "ml=disabled".to_string(),
            "ort=disabled".to_string(),
        ],
        decoder_policy: DECODER_POLICY.to_string(),
        input_policy: BLOCKED_INPUT_POLICY.to_string(),
        channel_mixdown_policy: BLOCKED_CHANNEL_MIXDOWN_POLICY.to_string(),
        normalization_policy: BLOCKED_NORMALIZATION_POLICY.to_string(),
        upstream_analysis_config_policy: BLOCKED_ANALYSIS_CONFIG_POLICY.to_string(),
        sample_rate_hz: None,
        ml_enabled: false,
        persistence_authorized: false,
        authority: AUTHORITY_POLICY.to_string(),
    }
}

fn target_view(target: &StorePlayableMediaAnalysisTarget) -> protocol::TrackMusicalAnalysisTarget {
    protocol::TrackMusicalAnalysisTarget {
        playable_media_id: target.playable_media_id,
        source_id: target.source_file_basis.source_id,
        source_file_id: target.source_file_basis.source_file_id,
        attachment_id: target.attachment_id,
        relative_path: Some(target.source_file_basis.relative_path.clone()),
        media_kind: Some(target.media_kind.clone()),
    }
}

fn adapter_warning(
    adapter_warning: analyzer_musical_stratum::MusicalAnalysisWarning,
) -> protocol::TrackMusicalAnalysisWarning {
    let severity = match adapter_warning.kind {
        MusicalAnalysisWarningKind::NonAuthoritativeSpike => {
            protocol::TrackMusicalAnalysisWarningSeverity::Info
        }
        MusicalAnalysisWarningKind::EmptyInput
        | MusicalAnalysisWarningKind::SilentInput
        | MusicalAnalysisWarningKind::TooShortForConfiguredFrame
        | MusicalAnalysisWarningKind::BpmUnavailable
        | MusicalAnalysisWarningKind::LowBpmConfidence
        | MusicalAnalysisWarningKind::LowKeyConfidence
        | MusicalAnalysisWarningKind::WeakTonality
        | MusicalAnalysisWarningKind::EmptyBeatGrid
        | MusicalAnalysisWarningKind::LowGridStability
        | MusicalAnalysisWarningKind::UpstreamWarning
        | MusicalAnalysisWarningKind::UpstreamFlag => {
            protocol::TrackMusicalAnalysisWarningSeverity::Warning
        }
        MusicalAnalysisWarningKind::UpstreamFailureObserved => {
            protocol::TrackMusicalAnalysisWarningSeverity::Error
        }
    };

    warning(
        severity,
        warning_code(adapter_warning.kind),
        adapter_warning.message,
    )
}

fn warning(
    severity: protocol::TrackMusicalAnalysisWarningSeverity,
    code: impl Into<String>,
    message: impl Into<String>,
) -> protocol::TrackMusicalAnalysisWarning {
    protocol::TrackMusicalAnalysisWarning {
        severity,
        code: code.into(),
        message: message.into(),
    }
}

fn warning_code(kind: MusicalAnalysisWarningKind) -> &'static str {
    match kind {
        MusicalAnalysisWarningKind::NonAuthoritativeSpike => "non_authoritative",
        MusicalAnalysisWarningKind::EmptyInput => "empty_input",
        MusicalAnalysisWarningKind::SilentInput => "silent_input",
        MusicalAnalysisWarningKind::TooShortForConfiguredFrame => "too_short_for_configured_frame",
        MusicalAnalysisWarningKind::BpmUnavailable => "bpm_unavailable",
        MusicalAnalysisWarningKind::LowBpmConfidence => "low_bpm_confidence",
        MusicalAnalysisWarningKind::LowKeyConfidence => "low_key_confidence",
        MusicalAnalysisWarningKind::WeakTonality => "weak_tonality",
        MusicalAnalysisWarningKind::EmptyBeatGrid => "empty_beatgrid",
        MusicalAnalysisWarningKind::LowGridStability => "low_grid_stability",
        MusicalAnalysisWarningKind::UpstreamWarning => "upstream_warning",
        MusicalAnalysisWarningKind::UpstreamFlag => "upstream_flag",
        MusicalAnalysisWarningKind::UpstreamFailureObserved => "upstream_failure_observed",
    }
}

fn map_wav_open_error(path: &Path, error: hound::Error) -> DecodeFailure {
    match error {
        hound::Error::IoError(error) => DecodeFailure::InputUnreadable {
            path: path.to_path_buf(),
            detail: error.to_string(),
        },
        hound::Error::FormatError(message) => {
            if message.contains("RIFF") || message.contains("WAVE") {
                DecodeFailure::UnsupportedContainer(
                    "input is not a supported WAV container for this V1 path".to_string(),
                )
            } else {
                DecodeFailure::MalformedWav(message.to_string())
            }
        }
        hound::Error::Unsupported => DecodeFailure::UnsupportedContainer(
            "WAV codec or variant is not supported by the V1 hound decoder path".to_string(),
        ),
        hound::Error::InvalidSampleFormat => DecodeFailure::UnsupportedSampleFormat(
            "only 16-bit integer PCM WAV is supported in this V1 slice".to_string(),
        ),
        hound::Error::TooWide => DecodeFailure::UnsupportedBitDepth(
            "only 16-bit integer PCM WAV is supported in this V1 slice".to_string(),
        ),
        hound::Error::UnfinishedSample => {
            DecodeFailure::DecodeError("WAV data ended with an unfinished sample".to_string())
        }
    }
}

#[derive(Debug, Clone)]
struct DecodedWavInput {
    samples: Vec<f32>,
    sample_rate_hz: u32,
}

#[derive(Debug, Clone)]
enum DecodeFailure {
    UnsupportedContainer(String),
    UnsupportedSampleFormat(String),
    UnsupportedBitDepth(String),
    MalformedWav(String),
    DecodeError(String),
    InputUnreadable { path: PathBuf, detail: String },
}

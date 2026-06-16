#![deny(unsafe_code)]

use std::panic::{AssertUnwindSafe, catch_unwind};

pub const ADAPTER_KEY: &str = "dekzer_analyzer_musical_stratum_spike";
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const UPSTREAM_CRATE_NAME: &str = "stratum-dsp";
pub const UPSTREAM_CRATE_VERSION: &str = "1.0.0";
pub const INPUT_POLICY_MONO_NORMALIZED_F32_FIXTURE_V1: &str = "mono_normalized_f32_fixture_v1";
pub const INPUT_POLICY_MONO_NORMALIZED_F32_DEKZER_WAV_V1: &str =
    "mono_normalized_f32_dekzer_wav_v1";
pub const CHANNEL_MIXDOWN_POLICY: &str = "already_mono_no_mixdown_fixture_v1";
pub const CHANNEL_MIXDOWN_POLICY_DEKZER_WAV_V1: &str = "dekzer_hound_wav_mean_mixdown_to_mono_v1";
pub const NORMALIZATION_POLICY: &str = "caller_supplied_normalized_f32_checked_v1";
pub const NORMALIZATION_POLICY_DEKZER_WAV_V1: &str =
    "dekzer_hound_16_bit_integer_pcm_full_scale_v1";
pub const UPSTREAM_ANALYSIS_CONFIG_POLICY: &str =
    "stratum_default_with_fixture_normalization_and_silence_trimming_disabled_v1";

const STRATUM_FRAME_SIZE: usize = 2_048;
const SILENCE_PEAK_THRESHOLD: f32 = 1e-7;
const LOW_BPM_CONFIDENCE_THRESHOLD: f32 = 0.50;
const LOW_KEY_CONFIDENCE_THRESHOLD: f32 = 0.30;
const LOW_KEY_CLARITY_THRESHOLD: f32 = 0.20;
const LOW_GRID_STABILITY_THRESHOLD: f32 = 0.50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicalAnalysisInputPolicy {
    MonoNormalizedF32FixtureV1,
    MonoNormalizedF32DekzerWavV1,
}

impl MusicalAnalysisInputPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MonoNormalizedF32FixtureV1 => INPUT_POLICY_MONO_NORMALIZED_F32_FIXTURE_V1,
            Self::MonoNormalizedF32DekzerWavV1 => INPUT_POLICY_MONO_NORMALIZED_F32_DEKZER_WAV_V1,
        }
    }

    pub fn channel_mixdown_policy(self) -> &'static str {
        match self {
            Self::MonoNormalizedF32FixtureV1 => CHANNEL_MIXDOWN_POLICY,
            Self::MonoNormalizedF32DekzerWavV1 => CHANNEL_MIXDOWN_POLICY_DEKZER_WAV_V1,
        }
    }

    pub fn normalization_policy(self) -> &'static str {
        match self {
            Self::MonoNormalizedF32FixtureV1 => NORMALIZATION_POLICY,
            Self::MonoNormalizedF32DekzerWavV1 => NORMALIZATION_POLICY_DEKZER_WAV_V1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicalAnalysisInput<'a> {
    pub samples: &'a [f32],
    pub sample_rate_hz: u32,
    pub input_policy: MusicalAnalysisInputPolicy,
}

impl<'a> MusicalAnalysisInput<'a> {
    pub fn mono_normalized_f32_fixture(samples: &'a [f32], sample_rate_hz: u32) -> Self {
        Self {
            samples,
            sample_rate_hz,
            input_policy: MusicalAnalysisInputPolicy::MonoNormalizedF32FixtureV1,
        }
    }

    pub fn mono_normalized_f32_dekzer_wav(samples: &'a [f32], sample_rate_hz: u32) -> Self {
        Self {
            samples,
            sample_rate_hz,
            input_policy: MusicalAnalysisInputPolicy::MonoNormalizedF32DekzerWavV1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicalAnalysisBasis {
    pub adapter_key: &'static str,
    pub adapter_version: &'static str,
    pub upstream_crate_name: &'static str,
    pub upstream_crate_version: &'static str,
    pub upstream_feature_flags: Vec<&'static str>,
    pub upstream_analysis_config_policy: &'static str,
    pub sample_rate_hz: u32,
    pub input_policy: &'static str,
    pub channel_mixdown_policy: &'static str,
    pub normalization_policy: &'static str,
    pub ml_enabled: bool,
    pub persistence_authorized: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MusicalAnalysisResult {
    pub basis: MusicalAnalysisBasis,
    pub bpm: Option<f32>,
    pub bpm_confidence: Option<f32>,
    pub key: Option<DekzerMusicalKey>,
    pub key_confidence: Option<f32>,
    pub beat_positions: Vec<f32>,
    pub grid_stability: Option<f32>,
    pub warnings: Vec<MusicalAnalysisWarning>,
    pub status: MusicalAnalysisStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DekzerMusicalKey {
    pub tonic: DekzerPitchClass,
    pub tonic_index: u8,
    pub mode: DekzerKeyMode,
    pub notation: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DekzerPitchClass {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DekzerKeyMode {
    Major,
    Minor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicalAnalysisStatus {
    Accepted,
    Advisory,
    Rejected,
    Blocked,
    Inconclusive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicalAnalysisWarning {
    pub kind: MusicalAnalysisWarningKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicalAnalysisWarningKind {
    NonAuthoritativeSpike,
    EmptyInput,
    SilentInput,
    TooShortForConfiguredFrame,
    BpmUnavailable,
    LowBpmConfidence,
    LowKeyConfidence,
    WeakTonality,
    EmptyBeatGrid,
    LowGridStability,
    UpstreamWarning,
    UpstreamFlag,
    UpstreamFailureObserved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicalAnalysisFailure {
    pub kind: MusicalAnalysisFailureKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicalAnalysisFailureKind {
    InvalidSampleRate,
    NonFiniteSample,
    OutOfRangeSample,
    UpstreamInvalidInput,
    UpstreamDecodingError,
    UpstreamProcessingError,
    UpstreamNotImplemented,
    UpstreamNumericalError,
    UpstreamPanic,
}

pub fn analyze_musical_input(
    input: MusicalAnalysisInput<'_>,
) -> Result<MusicalAnalysisResult, MusicalAnalysisFailure> {
    validate_input(input)?;

    let profile = SampleProfile::from_samples(input.samples);
    match invoke_stratum(input.samples, input.sample_rate_hz) {
        Ok(upstream) => Ok(map_upstream_result(input, profile, upstream)),
        Err(error) if profile.can_be_non_authoritative_result() => {
            Ok(non_authoritative_failure_result(input, profile, error))
        }
        Err(error) => Err(error),
    }
}

fn validate_input(input: MusicalAnalysisInput<'_>) -> Result<(), MusicalAnalysisFailure> {
    if input.sample_rate_hz == 0 {
        return Err(failure(
            MusicalAnalysisFailureKind::InvalidSampleRate,
            "sample_rate_hz must be greater than zero before invoking stratum-dsp",
        ));
    }

    for (index, sample) in input.samples.iter().enumerate() {
        if !sample.is_finite() {
            return Err(failure(
                MusicalAnalysisFailureKind::NonFiniteSample,
                format!(
                    "sample at index {index} is not finite; rejected before invoking stratum-dsp"
                ),
            ));
        }

        if !(-1.0..=1.0).contains(sample) {
            return Err(failure(
                MusicalAnalysisFailureKind::OutOfRangeSample,
                format!(
                    "sample at index {index} is outside [-1.0, 1.0]; rejected before invoking stratum-dsp"
                ),
            ));
        }
    }

    Ok(())
}

fn invoke_stratum(
    samples: &[f32],
    sample_rate_hz: u32,
) -> Result<stratum_dsp::AnalysisResult, MusicalAnalysisFailure> {
    match catch_unwind(AssertUnwindSafe(|| {
        stratum_dsp::analyze_audio(samples, sample_rate_hz, stratum_config())
    })) {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(error)) => Err(map_upstream_error(error)),
        Err(_) => Err(failure(
            MusicalAnalysisFailureKind::UpstreamPanic,
            "stratum-dsp panicked while analyzing controlled fixture input",
        )),
    }
}

fn stratum_config() -> stratum_dsp::AnalysisConfig {
    let mut config = stratum_dsp::AnalysisConfig::default();
    config.enable_normalization = false;
    config.enable_silence_trimming = false;
    config.emit_tempogram_candidates = true;
    config
}

fn map_upstream_result(
    input: MusicalAnalysisInput<'_>,
    profile: SampleProfile,
    upstream: stratum_dsp::AnalysisResult,
) -> MusicalAnalysisResult {
    let mut warnings = base_warnings(profile);

    let bpm = finite_positive(upstream.bpm);
    if bpm.is_none() {
        warnings.push(warning(
            MusicalAnalysisWarningKind::BpmUnavailable,
            "stratum-dsp did not produce a positive finite BPM estimate",
        ));
    }

    let bpm_confidence = finite_unit_interval(upstream.bpm_confidence);
    if bpm_confidence.unwrap_or(0.0) < LOW_BPM_CONFIDENCE_THRESHOLD {
        warnings.push(warning(
            MusicalAnalysisWarningKind::LowBpmConfidence,
            format!(
                "stratum-dsp BPM confidence is below spike threshold: {:.3}",
                upstream.bpm_confidence
            ),
        ));
    }

    let key_confidence = finite_unit_interval(upstream.key_confidence);
    if key_confidence.unwrap_or(0.0) < LOW_KEY_CONFIDENCE_THRESHOLD {
        warnings.push(warning(
            MusicalAnalysisWarningKind::LowKeyConfidence,
            format!(
                "stratum-dsp key confidence is below spike threshold: {:.3}",
                upstream.key_confidence
            ),
        ));
    }

    let key_clarity = finite_unit_interval(upstream.key_clarity);
    if key_clarity.unwrap_or(0.0) < LOW_KEY_CLARITY_THRESHOLD {
        warnings.push(warning(
            MusicalAnalysisWarningKind::WeakTonality,
            format!(
                "stratum-dsp key clarity is below spike threshold: {:.3}",
                upstream.key_clarity
            ),
        ));
    }

    let key = if key_confidence.unwrap_or(0.0) >= LOW_KEY_CONFIDENCE_THRESHOLD
        && key_clarity.unwrap_or(0.0) >= LOW_KEY_CLARITY_THRESHOLD
    {
        Some(map_key(upstream.key))
    } else {
        None
    };

    let beat_positions: Vec<f32> = upstream
        .beat_grid
        .beats
        .into_iter()
        .filter(|beat| beat.is_finite() && *beat >= 0.0)
        .collect();

    if beat_positions.is_empty() {
        warnings.push(warning(
            MusicalAnalysisWarningKind::EmptyBeatGrid,
            "stratum-dsp returned no finite non-negative beat positions",
        ));
    }

    let grid_stability = finite_unit_interval(upstream.grid_stability);
    if grid_stability.unwrap_or(0.0) < LOW_GRID_STABILITY_THRESHOLD {
        warnings.push(warning(
            MusicalAnalysisWarningKind::LowGridStability,
            format!(
                "stratum-dsp grid stability is below spike threshold: {:.3}",
                upstream.grid_stability
            ),
        ));
    }

    for upstream_warning in upstream.metadata.confidence_warnings {
        warnings.push(warning(
            MusicalAnalysisWarningKind::UpstreamWarning,
            upstream_warning,
        ));
    }

    for upstream_flag in upstream.metadata.flags {
        warnings.push(warning(
            MusicalAnalysisWarningKind::UpstreamFlag,
            format!("stratum-dsp analysis flag: {upstream_flag:?}"),
        ));
    }

    let status = classify_status(profile, bpm, &key, &beat_positions);

    MusicalAnalysisResult {
        basis: basis(input),
        bpm,
        bpm_confidence,
        key,
        key_confidence,
        beat_positions,
        grid_stability,
        warnings,
        status,
    }
}

fn non_authoritative_failure_result(
    input: MusicalAnalysisInput<'_>,
    profile: SampleProfile,
    error: MusicalAnalysisFailure,
) -> MusicalAnalysisResult {
    let mut warnings = base_warnings(profile);
    warnings.push(warning(
        MusicalAnalysisWarningKind::UpstreamFailureObserved,
        format!(
            "stratum-dsp returned a safe upstream failure for adversarial fixture input: {:?}: {}",
            error.kind, error.message
        ),
    ));

    MusicalAnalysisResult {
        basis: basis(input),
        bpm: None,
        bpm_confidence: None,
        key: None,
        key_confidence: None,
        beat_positions: Vec::new(),
        grid_stability: None,
        warnings,
        status: MusicalAnalysisStatus::Inconclusive,
    }
}

fn classify_status(
    profile: SampleProfile,
    bpm: Option<f32>,
    key: &Option<DekzerMusicalKey>,
    beat_positions: &[f32],
) -> MusicalAnalysisStatus {
    if profile.can_be_non_authoritative_result()
        || (bpm.is_none() && key.is_none() && beat_positions.is_empty())
    {
        MusicalAnalysisStatus::Inconclusive
    } else {
        MusicalAnalysisStatus::Advisory
    }
}

fn base_warnings(profile: SampleProfile) -> Vec<MusicalAnalysisWarning> {
    let mut warnings = vec![warning(
        MusicalAnalysisWarningKind::NonAuthoritativeSpike,
        "stratum-dsp output is adapter-spike evidence only; persistence and production authority are not authorized",
    )];

    if profile.empty {
        warnings.push(warning(
            MusicalAnalysisWarningKind::EmptyInput,
            "input fixture contains no samples",
        ));
    }

    if profile.too_short {
        warnings.push(warning(
            MusicalAnalysisWarningKind::TooShortForConfiguredFrame,
            format!(
                "input fixture has {} samples, less than the stratum-dsp frame size used by this spike ({STRATUM_FRAME_SIZE})",
                profile.sample_count
            ),
        ));
    }

    if profile.silent {
        warnings.push(warning(
            MusicalAnalysisWarningKind::SilentInput,
            "input fixture peak is at or below the spike silence threshold",
        ));
    }

    warnings
}

fn map_key(key: stratum_dsp::Key) -> DekzerMusicalKey {
    match key {
        stratum_dsp::Key::Major(index) => key_from_parts(index, DekzerKeyMode::Major),
        stratum_dsp::Key::Minor(index) => key_from_parts(index, DekzerKeyMode::Minor),
    }
}

fn key_from_parts(index: u32, mode: DekzerKeyMode) -> DekzerMusicalKey {
    let tonic_index = (index % 12) as u8;
    let tonic = pitch_class_from_index(tonic_index);
    let notation = match mode {
        DekzerKeyMode::Major => pitch_class_notation(tonic).to_string(),
        DekzerKeyMode::Minor => format!("{}m", pitch_class_notation(tonic)),
    };

    DekzerMusicalKey {
        tonic,
        tonic_index,
        mode,
        notation,
    }
}

fn pitch_class_from_index(index: u8) -> DekzerPitchClass {
    match index {
        0 => DekzerPitchClass::C,
        1 => DekzerPitchClass::CSharp,
        2 => DekzerPitchClass::D,
        3 => DekzerPitchClass::DSharp,
        4 => DekzerPitchClass::E,
        5 => DekzerPitchClass::F,
        6 => DekzerPitchClass::FSharp,
        7 => DekzerPitchClass::G,
        8 => DekzerPitchClass::GSharp,
        9 => DekzerPitchClass::A,
        10 => DekzerPitchClass::ASharp,
        11 => DekzerPitchClass::B,
        _ => unreachable!("pitch class index is reduced modulo 12"),
    }
}

fn pitch_class_notation(pitch_class: DekzerPitchClass) -> &'static str {
    match pitch_class {
        DekzerPitchClass::C => "C",
        DekzerPitchClass::CSharp => "C#",
        DekzerPitchClass::D => "D",
        DekzerPitchClass::DSharp => "D#",
        DekzerPitchClass::E => "E",
        DekzerPitchClass::F => "F",
        DekzerPitchClass::FSharp => "F#",
        DekzerPitchClass::G => "G",
        DekzerPitchClass::GSharp => "G#",
        DekzerPitchClass::A => "A",
        DekzerPitchClass::ASharp => "A#",
        DekzerPitchClass::B => "B",
    }
}

fn basis(input: MusicalAnalysisInput<'_>) -> MusicalAnalysisBasis {
    MusicalAnalysisBasis {
        adapter_key: ADAPTER_KEY,
        adapter_version: ADAPTER_VERSION,
        upstream_crate_name: UPSTREAM_CRATE_NAME,
        upstream_crate_version: UPSTREAM_CRATE_VERSION,
        upstream_feature_flags: vec![
            "default=[]",
            "default-features=false",
            "ml=disabled",
            "ort=disabled",
        ],
        upstream_analysis_config_policy: UPSTREAM_ANALYSIS_CONFIG_POLICY,
        sample_rate_hz: input.sample_rate_hz,
        input_policy: input.input_policy.as_str(),
        channel_mixdown_policy: input.input_policy.channel_mixdown_policy(),
        normalization_policy: input.input_policy.normalization_policy(),
        ml_enabled: false,
        persistence_authorized: false,
    }
}

fn finite_positive(value: f32) -> Option<f32> {
    (value.is_finite() && value > 0.0).then_some(value)
}

fn finite_unit_interval(value: f32) -> Option<f32> {
    (value.is_finite() && (0.0..=1.0).contains(&value)).then_some(value)
}

fn map_upstream_error(error: stratum_dsp::AnalysisError) -> MusicalAnalysisFailure {
    match error {
        stratum_dsp::AnalysisError::InvalidInput(message) => failure(
            MusicalAnalysisFailureKind::UpstreamInvalidInput,
            format!("stratum-dsp rejected input: {message}"),
        ),
        stratum_dsp::AnalysisError::DecodingError(message) => failure(
            MusicalAnalysisFailureKind::UpstreamDecodingError,
            format!("stratum-dsp reported a decoding error: {message}"),
        ),
        stratum_dsp::AnalysisError::ProcessingError(message) => failure(
            MusicalAnalysisFailureKind::UpstreamProcessingError,
            format!("stratum-dsp processing failed: {message}"),
        ),
        stratum_dsp::AnalysisError::NotImplemented(message) => failure(
            MusicalAnalysisFailureKind::UpstreamNotImplemented,
            format!("stratum-dsp reported an unimplemented path: {message}"),
        ),
        stratum_dsp::AnalysisError::NumericalError(message) => failure(
            MusicalAnalysisFailureKind::UpstreamNumericalError,
            format!("stratum-dsp reported a numerical error: {message}"),
        ),
    }
}

fn failure(kind: MusicalAnalysisFailureKind, message: impl Into<String>) -> MusicalAnalysisFailure {
    MusicalAnalysisFailure {
        kind,
        message: message.into(),
    }
}

fn warning(kind: MusicalAnalysisWarningKind, message: impl Into<String>) -> MusicalAnalysisWarning {
    MusicalAnalysisWarning {
        kind,
        message: message.into(),
    }
}

#[derive(Clone, Copy, Debug)]
struct SampleProfile {
    sample_count: usize,
    empty: bool,
    too_short: bool,
    silent: bool,
}

impl SampleProfile {
    fn from_samples(samples: &[f32]) -> Self {
        let peak = samples
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);

        Self {
            sample_count: samples.len(),
            empty: samples.is_empty(),
            too_short: samples.len() < STRATUM_FRAME_SIZE,
            silent: peak <= SILENCE_PEAK_THRESHOLD,
        }
    }

    fn can_be_non_authoritative_result(self) -> bool {
        self.empty || self.too_short || self.silent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::type_name;
    use std::fs;
    use std::path::Path;

    const SAMPLE_RATE_HZ: u32 = 44_100;

    #[test]
    fn controlled_pulse_train_returns_dekzer_owned_result() {
        let samples = pulse_train(120.0, 16.0, SAMPLE_RATE_HZ);
        let result = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
            &samples,
            SAMPLE_RATE_HZ,
        ))
        .expect("pulse train should produce a safe adapter result");

        assert_eq!(
            type_name::<MusicalAnalysisResult>(),
            "analyzer_musical_stratum::MusicalAnalysisResult"
        );
        assert_eq!(result.basis.upstream_crate_name, UPSTREAM_CRATE_NAME);
        assert_eq!(result.basis.upstream_crate_version, UPSTREAM_CRATE_VERSION);
        assert_ne!(result.status, MusicalAnalysisStatus::Accepted);
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.kind == MusicalAnalysisWarningKind::NonAuthoritativeSpike)
        );

        let bpm = result
            .bpm
            .expect("current stratum-dsp 1.0.0 spike should return a BPM candidate");
        assert!(
            (110.0..=125.0).contains(&bpm),
            "120 BPM fixture is evidence only, but should remain in the observed broad neighborhood: {result:?}"
        );
        let bpm_confidence = result
            .bpm_confidence
            .expect("current stratum-dsp 1.0.0 spike should return BPM confidence");
        assert!(
            bpm_confidence < LOW_BPM_CONFIDENCE_THRESHOLD,
            "current 120 BPM pulse evidence should keep exposing low confidence instead of hiding it: {result:?}"
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.kind == MusicalAnalysisWarningKind::LowBpmConfidence),
            "{result:?}"
        );

        let plausible = has_plausible_tempo_evidence(&result, 120.0);
        if !plausible {
            assert!(
                matches!(
                    result.status,
                    MusicalAnalysisStatus::Advisory | MusicalAnalysisStatus::Inconclusive
                ),
                "surprising pulse output must not be accepted: {result:?}"
            );
            assert!(
                !result.warnings.is_empty(),
                "surprising pulse output must carry warnings: {result:?}"
            );
        }
    }

    #[test]
    fn current_safe_fixtures_never_return_accepted_status() {
        let fixtures = [
            ("empty", Vec::new()),
            ("silence", vec![0.0_f32; SAMPLE_RATE_HZ as usize * 4]),
            ("too_short_constant", vec![0.25_f32; 64]),
            ("pulse_120_bpm", pulse_train(120.0, 16.0, SAMPLE_RATE_HZ)),
            ("pulse_90_bpm", pulse_train(90.0, 16.0, SAMPLE_RATE_HZ)),
        ];

        for (name, samples) in fixtures {
            let result = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
                &samples,
                SAMPLE_RATE_HZ,
            ))
            .unwrap_or_else(|error| panic!("{name} should produce a safe spike result: {error:?}"));

            assert_ne!(
                result.status,
                MusicalAnalysisStatus::Accepted,
                "{name} must not become authoritative spike output: {result:?}"
            );
            assert!(
                matches!(
                    result.status,
                    MusicalAnalysisStatus::Advisory | MusicalAnalysisStatus::Inconclusive
                ),
                "{name} should remain non-authoritative: {result:?}"
            );
            assert!(
                result.warnings.iter().any(
                    |warning| warning.kind == MusicalAnalysisWarningKind::NonAuthoritativeSpike
                ),
                "{name} must carry the non-authoritative spike warning: {result:?}"
            );
        }
    }

    #[test]
    fn pulse_train_120_bpm_is_repeatable_within_one_process() {
        let samples = pulse_train(120.0, 16.0, SAMPLE_RATE_HZ);
        let baseline = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
            &samples,
            SAMPLE_RATE_HZ,
        ))
        .expect("baseline pulse train should produce a safe adapter result");

        for iteration in 1..8 {
            let next = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
                &samples,
                SAMPLE_RATE_HZ,
            ))
            .unwrap_or_else(|error| panic!("repeat iteration {iteration} failed: {error:?}"));

            assert_eq!(
                next.basis, baseline.basis,
                "basis drifted in repeat iteration {iteration}"
            );
            assert_eq!(
                next.status, baseline.status,
                "status drifted in repeat iteration {iteration}: baseline={baseline:?}, next={next:?}"
            );
            assert_optional_f32_close(
                "bpm",
                baseline.bpm,
                next.bpm,
                0.01,
                iteration,
                &baseline,
                &next,
            );
            assert_optional_f32_close(
                "bpm_confidence",
                baseline.bpm_confidence,
                next.bpm_confidence,
                0.0001,
                iteration,
                &baseline,
                &next,
            );
            assert_optional_f32_close(
                "grid_stability",
                baseline.grid_stability,
                next.grid_stability,
                0.0001,
                iteration,
                &baseline,
                &next,
            );
            assert_eq!(
                warning_kinds(&next),
                warning_kinds(&baseline),
                "warning kinds drifted in repeat iteration {iteration}: baseline={baseline:?}, next={next:?}"
            );
            assert_eq!(
                next.beat_positions.len(),
                baseline.beat_positions.len(),
                "beat count drifted in repeat iteration {iteration}: baseline={baseline:?}, next={next:?}"
            );

            for (beat_index, (baseline_beat, next_beat)) in baseline
                .beat_positions
                .iter()
                .zip(next.beat_positions.iter())
                .enumerate()
            {
                assert_f32_close(
                    "beat_position",
                    *baseline_beat,
                    *next_beat,
                    0.001,
                    iteration,
                    beat_index,
                    &baseline,
                    &next,
                );
            }
        }

        assert_ne!(baseline.status, MusicalAnalysisStatus::Accepted);
    }

    #[test]
    fn second_pulse_fixture_low_confidence_or_surprise_remains_non_authoritative() {
        let samples = pulse_train(90.0, 16.0, SAMPLE_RATE_HZ);
        let result = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
            &samples,
            SAMPLE_RATE_HZ,
        ))
        .expect("90 BPM pulse train should produce a safe adapter result");

        assert_ne!(result.status, MusicalAnalysisStatus::Accepted);

        let surprising = !has_plausible_tempo_evidence(&result, 90.0);
        let low_bpm_confidence = result
            .bpm_confidence
            .map_or(true, |confidence| confidence < LOW_BPM_CONFIDENCE_THRESHOLD);
        assert!(
            surprising || low_bpm_confidence,
            "90 BPM fixture should continue exercising surprising or low-confidence spike evidence: {result:?}"
        );
        assert!(
            matches!(
                result.status,
                MusicalAnalysisStatus::Advisory | MusicalAnalysisStatus::Inconclusive
            ),
            "surprising or low-confidence pulse output must remain non-authoritative: {result:?}"
        );
        assert!(
            !result.warnings.is_empty(),
            "surprising or low-confidence pulse output must carry warnings: {result:?}"
        );
        assert!(
            result.warnings.iter().any(|warning| matches!(
                warning.kind,
                MusicalAnalysisWarningKind::BpmUnavailable
                    | MusicalAnalysisWarningKind::LowBpmConfidence
                    | MusicalAnalysisWarningKind::LowGridStability
                    | MusicalAnalysisWarningKind::EmptyBeatGrid
                    | MusicalAnalysisWarningKind::UpstreamWarning
                    | MusicalAnalysisWarningKind::UpstreamFlag
                    | MusicalAnalysisWarningKind::NonAuthoritativeSpike
            )),
            "{result:?}"
        );
    }

    #[test]
    fn basis_records_upstream_and_feature_policy() {
        let samples = pulse_train(120.0, 8.0, SAMPLE_RATE_HZ);
        let result = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
            &samples,
            SAMPLE_RATE_HZ,
        ))
        .expect("analysis");

        assert_eq!(result.basis.adapter_key, ADAPTER_KEY);
        assert_eq!(result.basis.adapter_version, ADAPTER_VERSION);
        assert_eq!(result.basis.upstream_crate_name, "stratum-dsp");
        assert_eq!(result.basis.upstream_crate_version, "1.0.0");
        assert_eq!(result.basis.sample_rate_hz, SAMPLE_RATE_HZ);
        assert_eq!(
            result.basis.input_policy,
            INPUT_POLICY_MONO_NORMALIZED_F32_FIXTURE_V1
        );
        assert_eq!(result.basis.channel_mixdown_policy, CHANNEL_MIXDOWN_POLICY);
        assert_eq!(result.basis.normalization_policy, NORMALIZATION_POLICY);
        assert!(!result.basis.ml_enabled);
        assert!(!result.basis.persistence_authorized);
        for required_flag in [
            "default=[]",
            "default-features=false",
            "ml=disabled",
            "ort=disabled",
        ] {
            assert!(
                result.basis.upstream_feature_flags.contains(&required_flag),
                "basis must record upstream feature flag {required_flag}: {result:?}"
            );
        }
    }

    #[test]
    fn dekzer_wav_input_policy_records_real_file_basis_policy() {
        let samples = pulse_train(120.0, 8.0, SAMPLE_RATE_HZ);
        let result = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_dekzer_wav(
            &samples,
            SAMPLE_RATE_HZ,
        ))
        .expect("analysis");

        assert_eq!(
            result.basis.input_policy,
            INPUT_POLICY_MONO_NORMALIZED_F32_DEKZER_WAV_V1
        );
        assert_eq!(
            result.basis.channel_mixdown_policy,
            CHANNEL_MIXDOWN_POLICY_DEKZER_WAV_V1
        );
        assert_eq!(
            result.basis.normalization_policy,
            NORMALIZATION_POLICY_DEKZER_WAV_V1
        );
        assert!(!result.basis.persistence_authorized);
        assert!(!result.basis.ml_enabled);
        assert_ne!(result.status, MusicalAnalysisStatus::Accepted);
    }

    #[test]
    fn silence_is_safe_and_non_authoritative() {
        let samples = vec![0.0_f32; SAMPLE_RATE_HZ as usize * 4];
        let result = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
            &samples,
            SAMPLE_RATE_HZ,
        ))
        .expect("silence should return a safe non-authoritative result");

        assert_ne!(result.status, MusicalAnalysisStatus::Accepted);
        assert!(
            matches!(
                result.status,
                MusicalAnalysisStatus::Advisory | MusicalAnalysisStatus::Inconclusive
            ),
            "silence must remain non-authoritative even if upstream behavior changes: {result:?}"
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.kind == MusicalAnalysisWarningKind::SilentInput),
            "{result:?}"
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.kind == MusicalAnalysisWarningKind::NonAuthoritativeSpike),
            "{result:?}"
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.kind == MusicalAnalysisWarningKind::LowBpmConfidence),
            "{result:?}"
        );

        if result.bpm.is_some() {
            assert!(
                result
                    .bpm_confidence
                    .map_or(true, |confidence| confidence < LOW_BPM_CONFIDENCE_THRESHOLD),
                "silence BPM candidates must remain low-confidence spike evidence: {result:?}"
            );
            assert_ne!(
                result.status,
                MusicalAnalysisStatus::Accepted,
                "silence BPM candidates must not make silence authoritative: {result:?}"
            );
        }
    }

    #[test]
    fn too_short_buffer_is_safe_and_non_authoritative() {
        let samples = vec![0.25_f32; 64];
        let result = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
            &samples,
            SAMPLE_RATE_HZ,
        ))
        .expect("too-short input should return a safe non-authoritative result");

        assert_eq!(result.status, MusicalAnalysisStatus::Inconclusive);
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.kind
                    == MusicalAnalysisWarningKind::TooShortForConfiguredFrame),
            "{result:?}"
        );
    }

    #[test]
    fn nan_sample_is_rejected_before_upstream_call() {
        let samples = [0.0_f32, f32::NAN, 0.0];
        let error = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
            &samples,
            SAMPLE_RATE_HZ,
        ))
        .expect_err("NaN must be rejected");

        assert_eq!(error.kind, MusicalAnalysisFailureKind::NonFiniteSample);
        assert!(error.message.contains("before invoking stratum-dsp"));
    }

    #[test]
    fn infinity_sample_is_rejected_before_upstream_call() {
        let samples = [0.0_f32, f32::INFINITY, 0.0];
        let error = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
            &samples,
            SAMPLE_RATE_HZ,
        ))
        .expect_err("infinity must be rejected");

        assert_eq!(error.kind, MusicalAnalysisFailureKind::NonFiniteSample);
        assert!(error.message.contains("before invoking stratum-dsp"));
    }

    #[test]
    fn out_of_range_sample_is_rejected_before_upstream_call() {
        let samples = [0.0_f32, 1.01, 0.0];
        let error = analyze_musical_input(MusicalAnalysisInput::mono_normalized_f32_fixture(
            &samples,
            SAMPLE_RATE_HZ,
        ))
        .expect_err("out-of-range sample must be rejected");

        assert_eq!(error.kind, MusicalAnalysisFailureKind::OutOfRangeSample);
        assert!(error.message.contains("before invoking stratum-dsp"));
    }

    #[test]
    fn crate_manifest_keeps_stratum_isolated_from_forbidden_direct_dependencies() {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let manifest = fs::read_to_string(manifest_path).expect("adapter manifest");

        assert!(manifest.contains("stratum-dsp"));
        assert!(manifest.contains("=1.0.0"));
        assert!(manifest.contains("default-features = false"));

        for forbidden in [
            "symphonia",
            "rusqlite",
            "library-store-sqlite",
            "library-boundary",
            "renderer",
            "electron",
            "ts-rs",
        ] {
            assert!(
                !manifest.contains(forbidden),
                "adapter manifest must not directly depend on {forbidden}"
            );
        }
    }

    #[test]
    fn analyzer_core_does_not_depend_on_stratum() {
        let analyzer_core_manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates dir")
            .join("analyzer-core")
            .join("Cargo.toml");
        let manifest = fs::read_to_string(analyzer_core_manifest).expect("analyzer-core manifest");

        assert!(!manifest.contains("stratum-dsp"));
        assert!(!manifest.contains("stratum_dsp"));
    }

    #[test]
    fn lockfile_avoids_optional_ort_and_obvious_onnx_runtime_dependencies() {
        let lockfile = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("Cargo.lock");
        let lock = fs::read_to_string(lockfile).expect("Cargo.lock");

        for forbidden in [
            "ort",
            "ort-sys",
            "onnxruntime",
            "onnxruntime-sys",
            "tract-core",
            "tract-onnx",
        ] {
            assert!(
                !lock_contains_package(&lock, forbidden),
                "Cargo.lock must not include optional ML/native runtime package {forbidden}"
            );
        }
    }

    fn pulse_train(bpm: f32, duration_seconds: f32, sample_rate_hz: u32) -> Vec<f32> {
        let sample_count = (duration_seconds * sample_rate_hz as f32).round() as usize;
        let beat_interval = (60.0 / bpm * sample_rate_hz as f32).round() as usize;
        let click_len = (0.010 * sample_rate_hz as f32).round() as usize;
        let mut samples = vec![0.0_f32; sample_count];

        let mut beat = 0_usize;
        let mut beat_number = 0_usize;
        while beat < sample_count {
            let accent = if beat_number % 4 == 0 { 0.90 } else { 0.65 };
            for offset in 0..click_len {
                let index = beat + offset;
                if index >= sample_count {
                    break;
                }

                let phase = offset as f32 / click_len as f32;
                let envelope = (1.0 - phase).max(0.0);
                let polarity = if offset % 2 == 0 { 1.0 } else { -1.0 };
                let value = polarity * accent * envelope;
                samples[index] = (samples[index] + value).clamp(-1.0, 1.0);
            }

            beat += beat_interval;
            beat_number += 1;
        }

        samples
    }

    fn has_plausible_tempo_evidence(result: &MusicalAnalysisResult, expected_bpm: f32) -> bool {
        result
            .bpm
            .is_some_and(|bpm| (bpm - expected_bpm).abs() <= 5.0)
            || beat_positions_imply_bpm(&result.beat_positions)
                .is_some_and(|bpm| (bpm - expected_bpm).abs() <= 5.0)
    }

    fn beat_positions_imply_bpm(beat_positions: &[f32]) -> Option<f32> {
        if beat_positions.len() < 4 {
            return None;
        }

        let mut intervals: Vec<f32> = beat_positions
            .windows(2)
            .filter_map(|window| {
                let interval = window[1] - window[0];
                (interval.is_finite() && interval > 0.0).then_some(interval)
            })
            .collect();

        if intervals.len() < 3 {
            return None;
        }

        intervals
            .sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

        let median_interval = intervals[intervals.len() / 2];
        Some(60.0 / median_interval)
    }

    fn warning_kinds(result: &MusicalAnalysisResult) -> Vec<MusicalAnalysisWarningKind> {
        result.warnings.iter().map(|warning| warning.kind).collect()
    }

    fn assert_optional_f32_close(
        label: &str,
        expected: Option<f32>,
        actual: Option<f32>,
        tolerance: f32,
        iteration: usize,
        baseline: &MusicalAnalysisResult,
        next: &MusicalAnalysisResult,
    ) {
        match (expected, actual) {
            (Some(expected), Some(actual)) => assert!(
                (expected - actual).abs() <= tolerance,
                "{label} drifted in repeat iteration {iteration}: expected {expected}, got {actual}; baseline={baseline:?}, next={next:?}"
            ),
            (None, None) => {}
            _ => panic!(
                "{label} presence drifted in repeat iteration {iteration}: baseline={baseline:?}, next={next:?}"
            ),
        }
    }

    fn assert_f32_close(
        label: &str,
        expected: f32,
        actual: f32,
        tolerance: f32,
        iteration: usize,
        index: usize,
        baseline: &MusicalAnalysisResult,
        next: &MusicalAnalysisResult,
    ) {
        assert!(
            (expected - actual).abs() <= tolerance,
            "{label}[{index}] drifted in repeat iteration {iteration}: expected {expected}, got {actual}; baseline={baseline:?}, next={next:?}"
        );
    }

    fn lock_contains_package(lock: &str, package_name: &str) -> bool {
        let needle = format!("name = \"{package_name}\"");
        lock.lines().any(|line| line.trim() == needle)
    }
}

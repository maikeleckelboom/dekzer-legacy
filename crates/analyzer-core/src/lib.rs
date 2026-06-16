#![deny(unsafe_code)]

use std::path::Path;

pub const DECODER_CRATE: &str = "hound";
pub const DECODER_VERSION: &str = "3.5.1";
pub const SAMPLE_FORMAT_POLICY: &str = "integer_pcm_wav_hound_slice1";
pub const NORMALIZATION_POLICY: &str = "integer_pcm_full_scale_v0";
pub const STEREO_NEAR_ZERO_CHANNEL_RMS: f64 = 1e-12;

#[derive(Clone, Debug, PartialEq)]
pub struct TechnicalSampleAnalysis {
    pub basis: AnalysisBasis,
    pub facts: TechnicalSampleFacts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisBasis {
    pub decoder_crate: &'static str,
    pub decoder_version: &'static str,
    pub sample_format_policy: &'static str,
    pub normalization_policy: &'static str,
    pub sample_rate_hz: u32,
    pub channel_count: u16,
    pub frame_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TechnicalSampleFacts {
    pub frame_count: u64,
    pub channel_count: u16,
    pub sample_peak: f64,
    pub peak_dbfs: Option<f64>,
    pub rms: f64,
    pub rms_dbfs: Option<f64>,
    pub crest_factor: Option<f64>,
    pub dc_offset: f64,
    pub clipping_count: u64,
    pub clipping_ratio: f64,
    pub stereo_balance: Option<f64>,
    pub stereo_phase_correlation: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisFailure {
    pub kind: AnalysisFailureKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisFailureKind {
    InputNotFound,
    InputNotFile,
    InputUnreadable,
    UnsupportedContainer,
    UnsupportedCodec,
    UnsupportedSampleFormat,
    UnsupportedBitDepth,
    MalformedWav,
    DecodeError,
    BasisChanged,
    NumericDomainError,
    InternalError,
}

pub fn analyze_wav_file(
    path: impl AsRef<Path>,
) -> Result<TechnicalSampleAnalysis, AnalysisFailure> {
    let path = path.as_ref();
    let metadata = path.metadata().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            failure(
                AnalysisFailureKind::InputNotFound,
                "input file was not found",
            )
        } else {
            failure(
                AnalysisFailureKind::InputUnreadable,
                "input metadata could not be read",
            )
        }
    })?;

    if !metadata.is_file() {
        return Err(failure(
            AnalysisFailureKind::InputNotFile,
            "input path is not a file",
        ));
    }

    let reader = hound::WavReader::open(path).map_err(map_open_error)?;
    analyze_hound_reader(reader)
}

fn analyze_hound_reader<R: std::io::Read>(
    mut reader: hound::WavReader<R>,
) -> Result<TechnicalSampleAnalysis, AnalysisFailure> {
    let spec = reader.spec();

    if spec.sample_format != hound::SampleFormat::Int {
        return Err(failure(
            AnalysisFailureKind::UnsupportedSampleFormat,
            "integer PCM WAV is required for analyzer V0 slice 1",
        ));
    }

    if !matches!(spec.bits_per_sample, 8 | 16 | 24 | 32) {
        return Err(failure(
            AnalysisFailureKind::UnsupportedBitDepth,
            "only 8, 16, 24, and 32 bit integer PCM WAV input is supported",
        ));
    }

    let channel_count = spec.channels;
    let frame_count = u64::from(reader.duration());
    let total_sample_count = u64::from(reader.len());
    let (full_scale_min, full_scale_max, full_scale_denominator) =
        integer_full_scale(spec.bits_per_sample)?;

    let mut sample_index = 0_u64;
    let mut sample_peak = 0.0_f64;
    let mut sum = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    let mut clipping_count = 0_u64;

    let mut left_sum_squares = 0.0_f64;
    let mut right_sum_squares = 0.0_f64;
    let mut stereo_cross_sum = 0.0_f64;
    let mut pending_left = 0.0_f64;

    for sample in reader.samples::<i32>() {
        let sample = sample.map_err(map_decode_error)?;
        let sample_i64 = i64::from(sample);
        let normalized = sample as f64 / full_scale_denominator;
        let square = normalized * normalized;

        sample_peak = sample_peak.max(normalized.abs());
        sum += normalized;
        sum_squares += square;

        if sample_i64 == full_scale_min || sample_i64 == full_scale_max {
            clipping_count += 1;
        }

        if channel_count == 2 {
            if sample_index % 2 == 0 {
                pending_left = normalized;
                left_sum_squares += square;
            } else {
                right_sum_squares += square;
                stereo_cross_sum += pending_left * normalized;
            }
        }

        sample_index += 1;
    }

    if sample_index != total_sample_count {
        return Err(failure(
            AnalysisFailureKind::DecodeError,
            "decoded sample count did not match WAV data length",
        ));
    }

    let total_sample_count_f64 = total_sample_count as f64;
    let rms = if total_sample_count == 0 {
        0.0
    } else {
        (sum_squares / total_sample_count_f64).sqrt()
    };
    let dc_offset = if total_sample_count == 0 {
        0.0
    } else {
        sum / total_sample_count_f64
    };
    let clipping_ratio = if total_sample_count == 0 {
        0.0
    } else {
        clipping_count as f64 / total_sample_count_f64
    };

    let (stereo_balance, stereo_phase_correlation) = stereo_facts(
        channel_count,
        frame_count,
        left_sum_squares,
        right_sum_squares,
        stereo_cross_sum,
    );

    let facts = TechnicalSampleFacts {
        frame_count,
        channel_count,
        sample_peak,
        peak_dbfs: dbfs(sample_peak),
        rms,
        rms_dbfs: dbfs(rms),
        crest_factor: finite_option((rms != 0.0).then(|| sample_peak / rms)),
        dc_offset,
        clipping_count,
        clipping_ratio,
        stereo_balance,
        stereo_phase_correlation,
    };

    validate_public_numbers(&facts)?;

    Ok(TechnicalSampleAnalysis {
        basis: AnalysisBasis {
            decoder_crate: DECODER_CRATE,
            decoder_version: DECODER_VERSION,
            sample_format_policy: SAMPLE_FORMAT_POLICY,
            normalization_policy: NORMALIZATION_POLICY,
            sample_rate_hz: spec.sample_rate,
            channel_count,
            frame_count,
        },
        facts,
    })
}

fn integer_full_scale(bits_per_sample: u16) -> Result<(i64, i64, f64), AnalysisFailure> {
    if bits_per_sample == 0 || bits_per_sample > 32 {
        return Err(failure(
            AnalysisFailureKind::UnsupportedBitDepth,
            "integer PCM bit depth is outside the supported range",
        ));
    }

    let half_range = 1_i64 << (bits_per_sample - 1);
    Ok((-half_range, half_range - 1, half_range as f64))
}

fn dbfs(value: f64) -> Option<f64> {
    finite_option((value != 0.0).then(|| 20.0 * value.log10()))
}

fn stereo_facts(
    channel_count: u16,
    frame_count: u64,
    left_sum_squares: f64,
    right_sum_squares: f64,
    stereo_cross_sum: f64,
) -> (Option<f64>, Option<f64>) {
    if channel_count != 2 || frame_count == 0 {
        return (None, None);
    }

    let frame_count_f64 = frame_count as f64;
    let left_rms = (left_sum_squares / frame_count_f64).sqrt();
    let right_rms = (right_sum_squares / frame_count_f64).sqrt();

    if left_rms < STEREO_NEAR_ZERO_CHANNEL_RMS || right_rms < STEREO_NEAR_ZERO_CHANNEL_RMS {
        return (None, None);
    }

    let balance = finite_option(Some((right_rms - left_rms) / (right_rms + left_rms)));
    let correlation = finite_option(Some(
        stereo_cross_sum / (left_sum_squares * right_sum_squares).sqrt(),
    ));

    (balance, correlation)
}

fn finite_option(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite())
}

fn validate_public_numbers(facts: &TechnicalSampleFacts) -> Result<(), AnalysisFailure> {
    let required = [
        facts.sample_peak,
        facts.rms,
        facts.dc_offset,
        facts.clipping_ratio,
    ];

    let options = [
        facts.peak_dbfs,
        facts.rms_dbfs,
        facts.crest_factor,
        facts.stereo_balance,
        facts.stereo_phase_correlation,
    ];

    if required.iter().all(|value| value.is_finite())
        && options
            .iter()
            .all(|value| value.is_none_or(|value| value.is_finite()))
    {
        Ok(())
    } else {
        Err(failure(
            AnalysisFailureKind::NumericDomainError,
            "analysis produced a non-finite public numeric result",
        ))
    }
}

fn map_open_error(error: hound::Error) -> AnalysisFailure {
    match error {
        hound::Error::IoError(_) => failure(
            AnalysisFailureKind::InputUnreadable,
            "input file could not be read",
        ),
        hound::Error::FormatError(message) => {
            if message.contains("RIFF") || message.contains("WAVE") {
                failure(
                    AnalysisFailureKind::UnsupportedContainer,
                    "input is not a supported WAV container",
                )
            } else {
                failure(AnalysisFailureKind::MalformedWav, message)
            }
        }
        hound::Error::Unsupported => failure(
            AnalysisFailureKind::UnsupportedCodec,
            "WAV codec or variant is not supported",
        ),
        hound::Error::InvalidSampleFormat => failure(
            AnalysisFailureKind::UnsupportedSampleFormat,
            "integer PCM WAV is required for analyzer V0 slice 1",
        ),
        hound::Error::TooWide => failure(
            AnalysisFailureKind::UnsupportedBitDepth,
            "integer PCM bit depth is too wide",
        ),
        hound::Error::UnfinishedSample => failure(
            AnalysisFailureKind::MalformedWav,
            "WAV data ended with an unfinished sample",
        ),
    }
}

fn map_decode_error(error: hound::Error) -> AnalysisFailure {
    match error {
        hound::Error::InvalidSampleFormat => failure(
            AnalysisFailureKind::UnsupportedSampleFormat,
            "integer PCM WAV is required for analyzer V0 slice 1",
        ),
        hound::Error::TooWide | hound::Error::Unsupported => failure(
            AnalysisFailureKind::UnsupportedBitDepth,
            "integer PCM bit depth cannot be decoded deterministically",
        ),
        hound::Error::FormatError(message) => failure(AnalysisFailureKind::MalformedWav, message),
        hound::Error::IoError(_) => failure(
            AnalysisFailureKind::DecodeError,
            "sample decoding failed while reading WAV data",
        ),
        hound::Error::UnfinishedSample => failure(
            AnalysisFailureKind::DecodeError,
            "sample decoding ended with an unfinished sample",
        ),
    }
}

fn failure(kind: AnalysisFailureKind, message: impl Into<String>) -> AnalysisFailure {
    AnalysisFailure {
        kind,
        message: message.into(),
    }
}

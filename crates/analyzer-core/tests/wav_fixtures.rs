use std::f64::consts::SQRT_2;
use std::fs;
use std::path::{Path, PathBuf};

use analyzer_core::{
    AnalysisFailureKind, DECODER_CRATE, DECODER_VERSION, NORMALIZATION_POLICY,
    SAMPLE_FORMAT_POLICY, TechnicalSampleFacts, analyze_wav_file,
};
use hound::{SampleFormat, WavSpec, WavWriter};
use tempfile::TempDir;

const SAMPLE_RATE_HZ: u32 = 44_100;
const BIT_DEPTH: u16 = 16;
const SCALE: f64 = 32_768.0;
const TOLERANCE: f64 = 1e-12;
const DB_TOLERANCE: f64 = 1e-9;

#[test]
fn generated_integer_fixtures_produce_contract_facts() {
    let temp = TempDir::new().expect("temp dir");
    let clipped_positive = i16::MAX as f64 / SCALE;
    let clipped_rms = ((1.0 + clipped_positive.powi(2) + clipped_positive.powi(2)) / 4.0).sqrt();
    let clipped_left_rms = ((1.0 + clipped_positive.powi(2)) / 2.0).sqrt();
    let clipped_right_rms = (clipped_positive.powi(2) / 2.0).sqrt();

    let cases = [
        FixtureCase {
            name: "silence",
            metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=1;bits=16;format=pcm_integer;frames=8;recipe=all zero samples",
            channels: 1,
            samples: vec![0, 0, 0, 0, 0, 0, 0, 0],
            expected: ExpectedFacts {
                frame_count: 8,
                channel_count: 1,
                sample_peak: 0.0,
                peak_dbfs: None,
                rms: 0.0,
                rms_dbfs: None,
                crest_factor: None,
                dc_offset: 0.0,
                clipping_count: 0,
                clipping_ratio: 0.0,
                stereo_balance: None,
                stereo_phase_correlation: None,
            },
        },
        FixtureCase {
            name: "constant_dc",
            metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=1;bits=16;format=pcm_integer;frames=4;recipe=constant sample value 8192",
            channels: 1,
            samples: vec![8_192, 8_192, 8_192, 8_192],
            expected: ExpectedFacts {
                frame_count: 4,
                channel_count: 1,
                sample_peak: 0.25,
                peak_dbfs: Some(dbfs(0.25)),
                rms: 0.25,
                rms_dbfs: Some(dbfs(0.25)),
                crest_factor: Some(1.0),
                dc_offset: 0.25,
                clipping_count: 0,
                clipping_ratio: 0.0,
                stereo_balance: None,
                stereo_phase_correlation: None,
            },
        },
        FixtureCase {
            name: "sine",
            metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=1;bits=16;format=pcm_integer;frames=4;recipe=one four-point sine period at amplitude 0.5",
            channels: 1,
            samples: vec![0, 16_384, 0, -16_384],
            expected: ExpectedFacts {
                frame_count: 4,
                channel_count: 1,
                sample_peak: 0.5,
                peak_dbfs: Some(dbfs(0.5)),
                rms: 0.5 / SQRT_2,
                rms_dbfs: Some(dbfs(0.5 / SQRT_2)),
                crest_factor: Some(SQRT_2),
                dc_offset: 0.0,
                clipping_count: 0,
                clipping_ratio: 0.0,
                stereo_balance: None,
                stereo_phase_correlation: None,
            },
        },
        FixtureCase {
            name: "impulse",
            metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=1;bits=16;format=pcm_integer;frames=4;recipe=single positive non-clipped impulse at amplitude 0.5",
            channels: 1,
            samples: vec![16_384, 0, 0, 0],
            expected: ExpectedFacts {
                frame_count: 4,
                channel_count: 1,
                sample_peak: 0.5,
                peak_dbfs: Some(dbfs(0.5)),
                rms: 0.25,
                rms_dbfs: Some(dbfs(0.25)),
                crest_factor: Some(2.0),
                dc_offset: 0.125,
                clipping_count: 0,
                clipping_ratio: 0.0,
                stereo_balance: None,
                stereo_phase_correlation: None,
            },
        },
        FixtureCase {
            name: "hard_clipped_samples",
            metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=2;bits=16;format=pcm_integer;frames=2;recipe=three full-scale samples across four total samples",
            channels: 2,
            samples: vec![i16::MIN, i16::MAX, i16::MAX, 0],
            expected: ExpectedFacts {
                frame_count: 2,
                channel_count: 2,
                sample_peak: 1.0,
                peak_dbfs: Some(0.0),
                rms: clipped_rms,
                rms_dbfs: Some(dbfs(clipped_rms)),
                crest_factor: Some(1.0 / clipped_rms),
                dc_offset: (-1.0 + clipped_positive + clipped_positive) / 4.0,
                clipping_count: 3,
                clipping_ratio: 0.75,
                stereo_balance: Some(
                    (clipped_right_rms - clipped_left_rms) / (clipped_right_rms + clipped_left_rms),
                ),
                stereo_phase_correlation: Some(
                    -clipped_positive
                        / ((1.0 + clipped_positive.powi(2)) * clipped_positive.powi(2)).sqrt(),
                ),
            },
        },
        FixtureCase {
            name: "left_only_stereo",
            metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=2;bits=16;format=pcm_integer;frames=2;recipe=left channel amplitude 0.5, right channel silent",
            channels: 2,
            samples: vec![16_384, 0, 16_384, 0],
            expected: ExpectedFacts {
                frame_count: 2,
                channel_count: 2,
                sample_peak: 0.5,
                peak_dbfs: Some(dbfs(0.5)),
                rms: 0.5 / SQRT_2,
                rms_dbfs: Some(dbfs(0.5 / SQRT_2)),
                crest_factor: Some(SQRT_2),
                dc_offset: 0.25,
                clipping_count: 0,
                clipping_ratio: 0.0,
                stereo_balance: None,
                stereo_phase_correlation: None,
            },
        },
        FixtureCase {
            name: "right_only_stereo",
            metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=2;bits=16;format=pcm_integer;frames=2;recipe=left channel silent, right channel amplitude 0.5",
            channels: 2,
            samples: vec![0, 16_384, 0, 16_384],
            expected: ExpectedFacts {
                frame_count: 2,
                channel_count: 2,
                sample_peak: 0.5,
                peak_dbfs: Some(dbfs(0.5)),
                rms: 0.5 / SQRT_2,
                rms_dbfs: Some(dbfs(0.5 / SQRT_2)),
                crest_factor: Some(SQRT_2),
                dc_offset: 0.25,
                clipping_count: 0,
                clipping_ratio: 0.0,
                stereo_balance: None,
                stereo_phase_correlation: None,
            },
        },
        FixtureCase {
            name: "in_phase_stereo",
            metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=2;bits=16;format=pcm_integer;frames=2;recipe=matching left and right polarity at amplitude 0.5",
            channels: 2,
            samples: vec![16_384, 16_384, -16_384, -16_384],
            expected: ExpectedFacts {
                frame_count: 2,
                channel_count: 2,
                sample_peak: 0.5,
                peak_dbfs: Some(dbfs(0.5)),
                rms: 0.5,
                rms_dbfs: Some(dbfs(0.5)),
                crest_factor: Some(1.0),
                dc_offset: 0.0,
                clipping_count: 0,
                clipping_ratio: 0.0,
                stereo_balance: Some(0.0),
                stereo_phase_correlation: Some(1.0),
            },
        },
        FixtureCase {
            name: "polarity_inverted_stereo",
            metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=2;bits=16;format=pcm_integer;frames=2;recipe=right channel is the inverse of left at amplitude 0.5",
            channels: 2,
            samples: vec![16_384, -16_384, -16_384, 16_384],
            expected: ExpectedFacts {
                frame_count: 2,
                channel_count: 2,
                sample_peak: 0.5,
                peak_dbfs: Some(dbfs(0.5)),
                rms: 0.5,
                rms_dbfs: Some(dbfs(0.5)),
                crest_factor: Some(1.0),
                dc_offset: 0.0,
                clipping_count: 0,
                clipping_ratio: 0.0,
                stereo_balance: Some(0.0),
                stereo_phase_correlation: Some(-1.0),
            },
        },
    ];

    for case in cases {
        let path = write_int_fixture(temp.path(), &case);
        let analysis = analyze_wav_file(&path).unwrap_or_else(|error| {
            panic!("{} failed: {:?}: {}", case.name, error.kind, error.message)
        });

        assert_eq!(analysis.basis.decoder_crate, DECODER_CRATE, "{}", case.name);
        assert_eq!(
            analysis.basis.decoder_version, DECODER_VERSION,
            "{}",
            case.name
        );
        assert_eq!(
            analysis.basis.sample_format_policy, SAMPLE_FORMAT_POLICY,
            "{}",
            case.name
        );
        assert_eq!(
            analysis.basis.normalization_policy, NORMALIZATION_POLICY,
            "{}",
            case.name
        );
        assert_eq!(
            analysis.basis.sample_rate_hz, SAMPLE_RATE_HZ,
            "{}",
            case.name
        );
        assert_eq!(analysis.basis.channel_count, case.channels, "{}", case.name);
        assert_eq!(
            analysis.basis.frame_count, case.expected.frame_count,
            "{}",
            case.name
        );

        assert_eq!(
            case.samples.len() as u64,
            case.expected.frame_count * u64::from(case.channels)
        );
        assert!(!case.metadata.is_empty());
        assert_facts(case.name, &analysis.facts, &case.expected);
    }
}

#[test]
fn silence_uses_null_equivalent_domain_results() {
    let temp = TempDir::new().expect("temp dir");
    let case = FixtureCase {
        name: "stereo_silence",
        metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=2;bits=16;format=pcm_integer;frames=2;recipe=all zero samples",
        channels: 2,
        samples: vec![0, 0, 0, 0],
        expected: ExpectedFacts::unused(),
    };
    let path = write_int_fixture(temp.path(), &case);

    let facts = analyze_wav_file(&path).expect("analysis").facts;

    assert_eq!(facts.sample_peak, 0.0);
    assert_eq!(facts.peak_dbfs, None);
    assert_eq!(facts.rms, 0.0);
    assert_eq!(facts.rms_dbfs, None);
    assert_eq!(facts.crest_factor, None);
    assert_eq!(facts.stereo_balance, None);
    assert_eq!(facts.stereo_phase_correlation, None);
}

#[test]
fn stereo_correlation_polarity_sign_is_reported() {
    let temp = TempDir::new().expect("temp dir");

    let in_phase = write_int_fixture(
        temp.path(),
        &FixtureCase {
            name: "in_phase",
            metadata: "generator=analyzer_core_tests_v1;recipe=in-phase stereo",
            channels: 2,
            samples: vec![16_384, 16_384, -16_384, -16_384],
            expected: ExpectedFacts::unused(),
        },
    );
    let inverted = write_int_fixture(
        temp.path(),
        &FixtureCase {
            name: "inverted",
            metadata: "generator=analyzer_core_tests_v1;recipe=polarity-inverted stereo",
            channels: 2,
            samples: vec![16_384, -16_384, -16_384, 16_384],
            expected: ExpectedFacts::unused(),
        },
    );

    let positive = analyze_wav_file(in_phase)
        .expect("in-phase analysis")
        .facts
        .stereo_phase_correlation
        .expect("in-phase correlation");
    let negative = analyze_wav_file(inverted)
        .expect("inverted analysis")
        .facts
        .stereo_phase_correlation
        .expect("inverted correlation");

    assert!(positive > 0.0);
    assert!(negative < 0.0);
}

#[test]
fn clipping_counts_samples_and_ratio_uses_all_channels() {
    let temp = TempDir::new().expect("temp dir");
    let case = FixtureCase {
        name: "clipping_ratio",
        metadata: "generator=analyzer_core_tests_v1;sample_rate=44100;channels=2;bits=16;format=pcm_integer;frames=2;recipe=three clipped samples in two frames",
        channels: 2,
        samples: vec![i16::MIN, i16::MAX, i16::MAX, 0],
        expected: ExpectedFacts::unused(),
    };
    let path = write_int_fixture(temp.path(), &case);

    let facts = analyze_wav_file(path).expect("analysis").facts;

    assert_eq!(facts.frame_count, 2);
    assert_eq!(facts.clipping_count, 3);
    assert_close(facts.clipping_ratio, 3.0 / 4.0, TOLERANCE, "clipping_ratio");
}

#[test]
fn float_wav_is_rejected_with_typed_failure() {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("float.wav");
    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE_HZ,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(&path, spec).expect("create float wav");
    writer.write_sample(0.0_f32).expect("write float sample");
    writer.write_sample(0.5_f32).expect("write float sample");
    writer.finalize().expect("finalize float wav");

    let error = analyze_wav_file(path).expect_err("float WAV should be rejected");

    assert_eq!(error.kind, AnalysisFailureKind::UnsupportedSampleFormat);
}

#[test]
fn analyzer_core_manifest_has_no_forbidden_boundary_or_renderer_dependencies() {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest = fs::read_to_string(manifest_path).expect("manifest");

    for forbidden in [
        "library-store-sqlite",
        "library-boundary-service",
        "library-boundary-protocol",
        "generated",
        "renderer",
        "electron",
        "rusqlite",
        "symphonia",
        "serde",
        "ts-rs",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "analyzer-core manifest must not depend on {forbidden}"
        );
    }
}

struct FixtureCase {
    name: &'static str,
    metadata: &'static str,
    channels: u16,
    samples: Vec<i16>,
    expected: ExpectedFacts,
}

struct ExpectedFacts {
    frame_count: u64,
    channel_count: u16,
    sample_peak: f64,
    peak_dbfs: Option<f64>,
    rms: f64,
    rms_dbfs: Option<f64>,
    crest_factor: Option<f64>,
    dc_offset: f64,
    clipping_count: u64,
    clipping_ratio: f64,
    stereo_balance: Option<f64>,
    stereo_phase_correlation: Option<f64>,
}

impl ExpectedFacts {
    fn unused() -> Self {
        Self {
            frame_count: 0,
            channel_count: 0,
            sample_peak: 0.0,
            peak_dbfs: None,
            rms: 0.0,
            rms_dbfs: None,
            crest_factor: None,
            dc_offset: 0.0,
            clipping_count: 0,
            clipping_ratio: 0.0,
            stereo_balance: None,
            stereo_phase_correlation: None,
        }
    }
}

fn write_int_fixture(root: &Path, case: &FixtureCase) -> PathBuf {
    assert_eq!(
        case.samples.len() % usize::from(case.channels),
        0,
        "{} fixture must contain complete frames",
        case.name
    );

    let path = root.join(format!("{}.wav", case.name));
    let spec = WavSpec {
        channels: case.channels,
        sample_rate: SAMPLE_RATE_HZ,
        bits_per_sample: BIT_DEPTH,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&path, spec).expect("create wav");
    for sample in &case.samples {
        writer.write_sample(*sample).expect("write sample");
    }
    writer.finalize().expect("finalize wav");

    path
}

fn assert_facts(name: &str, actual: &TechnicalSampleFacts, expected: &ExpectedFacts) {
    assert_eq!(actual.frame_count, expected.frame_count, "{name}");
    assert_eq!(actual.channel_count, expected.channel_count, "{name}");
    assert_eq!(actual.clipping_count, expected.clipping_count, "{name}");

    assert_close(actual.sample_peak, expected.sample_peak, TOLERANCE, name);
    assert_close(actual.rms, expected.rms, TOLERANCE, name);
    assert_close(actual.dc_offset, expected.dc_offset, TOLERANCE, name);
    assert_close(
        actual.clipping_ratio,
        expected.clipping_ratio,
        TOLERANCE,
        name,
    );

    assert_option_close(actual.peak_dbfs, expected.peak_dbfs, DB_TOLERANCE, name);
    assert_option_close(actual.rms_dbfs, expected.rms_dbfs, DB_TOLERANCE, name);
    assert_option_close(actual.crest_factor, expected.crest_factor, TOLERANCE, name);
    assert_option_close(
        actual.stereo_balance,
        expected.stereo_balance,
        TOLERANCE,
        name,
    );
    assert_option_close(
        actual.stereo_phase_correlation,
        expected.stereo_phase_correlation,
        TOLERANCE,
        name,
    );
}

fn assert_close(actual: f64, expected: f64, tolerance: f64, context: &str) {
    assert!(
        actual.is_finite(),
        "{context}: actual value should be finite, got {actual}"
    );
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: expected {expected}, got {actual}"
    );
}

fn assert_option_close(actual: Option<f64>, expected: Option<f64>, tolerance: f64, context: &str) {
    match (actual, expected) {
        (Some(actual), Some(expected)) => assert_close(actual, expected, tolerance, context),
        (None, None) => {}
        _ => panic!("{context}: expected {expected:?}, got {actual:?}"),
    }
}

fn dbfs(value: f64) -> f64 {
    20.0 * value.log10()
}

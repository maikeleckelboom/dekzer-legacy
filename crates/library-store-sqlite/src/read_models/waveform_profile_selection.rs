/// Shared relevant-profile law for browse-grade waveform reads and browser-row
/// waveform summaries:
/// prefer the highest-ranked resolved waveform target profile, then fall back
/// to the waveform capability spec's default profile when no target exists.
pub(crate) const RELEVANT_WAVEFORM_TARGET_ORDER_SQL: &str = "\
rpt.target_quality DESC,
                                 CASE rpt.target_stability_class WHEN 'stable' THEN 0 ELSE 1 END,
                                 CASE rpt.priority_class
                                     WHEN 'urgent' THEN 0
                                     WHEN 'interactive' THEN 1
                                     ELSE 2
                                 END,
                                 CASE
                                     WHEN rpt.target_profile_key = capability_defaults.waveform_default_profile_key THEN 0
                                     ELSE 1
                                 END,
                                 rpt.target_profile_key ASC";

// Stored candidate evidence currentness. Callers provide the positional
// placeholder that owns the BLAKE3 algorithm parameter in their SQL statement.
pub(crate) fn current_track_identity_candidate_evidence_predicate(
    hash_algorithm_parameter_placeholder: &str,
) -> String {
    format!(
        "file.source_id = observations.basis_source_id
    AND file.presence_state = 'present'
    AND file.file_class = 'audio'
    AND file.file_kind = 'audio'
    AND observations.source_file_id IS NOT NULL
    AND file.relative_path = observations.basis_relative_path
    AND file.size_bytes IS observations.basis_size_bytes
    AND file.mtime_ns IS observations.basis_mtime_ns
    AND file.presence_state = observations.basis_presence_state
    AND evidence.content_hash_algorithm = {hash_algorithm_parameter_placeholder}
    AND observations.content_hash_algorithm = evidence.content_hash_algorithm
    AND observations.content_hash_value = evidence.content_hash_value
    AND link.source_file_attachment_link_id = evidence.source_file_attachment_link_id
    AND link.source_file_id = evidence.source_file_id
    AND link.source_id = evidence.source_id
    AND link.attachment_id = evidence.attachment_id
    AND attachment.content_hash_algorithm = evidence.content_hash_algorithm
    AND attachment.content_hash_value = evidence.content_hash_value
    AND observations.media_kind = 'audio'
    AND (
        observations.mime_type IS NOT NULL
        OR observations.duration_ms IS NOT NULL
        OR observations.sample_rate_hz IS NOT NULL
        OR observations.channels IS NOT NULL
        OR observations.bit_depth IS NOT NULL
        OR observations.codec IS NOT NULL
    )
    AND observations.accepted_artifact_id = evidence.probe_accepted_artifact_id"
    )
}

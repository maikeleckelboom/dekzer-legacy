-- Canonical baseline schema.
-- Epoch: 20260502000000
-- Generation: 20260502000000_substrate_baseline

PRAGMA auto_vacuum = INCREMENTAL;

CREATE TABLE LibraryMetadata
(
    library_id         INTEGER PRIMARY KEY CHECK (library_id = 1),
    schema_generation  TEXT    NOT NULL CHECK (length(trim(schema_generation)) > 0),
    created_at         INTEGER NOT NULL
) STRICT;

CREATE TABLE sources
(
    source_id       INTEGER PRIMARY KEY,
    source_class    TEXT    NOT NULL
        CHECK (source_class IN ('internal', 'external_mounted', 'removable_mounted')),
    authority       TEXT    NOT NULL
        CHECK (authority IN ('system', 'device')),
    identity_key    TEXT    NOT NULL UNIQUE CHECK (length(trim(identity_key)) > 0),
    display_name    TEXT    NOT NULL CHECK (length(trim(display_name)) > 0),
    medium_label    TEXT,
    is_user_visible INTEGER NOT NULL DEFAULT 1 CHECK (is_user_visible IN (0, 1)),
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    CHECK (
        (source_class = 'internal' AND authority = 'system')
            OR (source_class = 'external_mounted' AND authority = 'device')
            OR (source_class = 'removable_mounted' AND authority = 'device')
    )
) STRICT;

CREATE TABLE source_locators
(
    source_id              INTEGER PRIMARY KEY REFERENCES sources (source_id) ON DELETE CASCADE,
    locator_kind           TEXT NOT NULL
        CHECK (locator_kind IN ('absolute_path', 'removable_volume')),
    absolute_path          TEXT,
    device_identity_kind   TEXT CHECK (device_identity_kind IS NULL OR length(trim(device_identity_kind)) > 0),
    device_identity_value  TEXT CHECK (device_identity_value IS NULL OR length(trim(device_identity_value)) > 0),
    relative_suffix        TEXT NOT NULL DEFAULT '',
    CHECK (
        (
            locator_kind = 'absolute_path'
                AND absolute_path IS NOT NULL
                AND device_identity_kind IS NULL
                AND device_identity_value IS NULL
                AND relative_suffix = ''
        )
            OR (
            locator_kind = 'removable_volume'
                AND absolute_path IS NULL
                AND device_identity_kind IS NOT NULL
                AND device_identity_value IS NOT NULL
        )
    )
) STRICT;

CREATE UNIQUE INDEX source_locators_absolute_path
    ON source_locators (absolute_path)
    WHERE absolute_path IS NOT NULL;

CREATE UNIQUE INDEX source_locators_device_identity
    ON source_locators (device_identity_kind, device_identity_value, relative_suffix)
    WHERE device_identity_kind IS NOT NULL;

CREATE TABLE source_state
(
    source_id                 INTEGER PRIMARY KEY REFERENCES sources (source_id) ON DELETE CASCADE,
    mount_status              TEXT    NOT NULL
        CHECK (mount_status IN ('unknown', 'mounted', 'unmounted', 'eject_requested', 'eject_pending')),
    mount_epoch               INTEGER NOT NULL CHECK (mount_epoch >= 0),
    access_state              TEXT    NOT NULL
        CHECK (access_state IN ('accessible', 'missing', 'blocked', 'unknown')),
    access_issue_kind         TEXT
        CHECK (
            access_issue_kind IS NULL
                OR access_issue_kind IN (
                    'missing',
                    'not_directory',
                    'permission_denied',
                    'privacy_permission_required',
                    'unavailable_mount',
                    'resource_busy',
                    'stale_network_handle',
                    'symlink_loop',
                    'symlink_escape_blocked',
                    'unsupported_path',
                    'invalid_path',
                    'io_interrupted',
                    'timed_out',
                    'unknown_io'
                )
        ),
    access_error_detail       TEXT,
    access_checked_at         INTEGER,
    mount_root                TEXT,
    effective_path            TEXT,
    observed_volume_label     TEXT,
    filesystem_type           TEXT,
    last_seen_at              INTEGER,
    updated_at                INTEGER NOT NULL
) STRICT;

CREATE TABLE source_scan_state
(
    source_id                  INTEGER PRIMARY KEY REFERENCES sources (source_id) ON DELETE CASCADE,
    scan_phase                 TEXT    NOT NULL
        CHECK (scan_phase IN ('idle', 'scanning', 'complete', 'partial', 'blocked', 'failed')),
    last_scan_started_at       INTEGER,
    last_scan_finished_at      INTEGER,
    last_successful_scan_at    INTEGER,
    scan_issue_kind            TEXT
        CHECK (
            scan_issue_kind IS NULL
                OR scan_issue_kind IN (
                    'missing',
                    'not_directory',
                    'permission_denied',
                    'privacy_permission_required',
                    'unavailable_mount',
                    'resource_busy',
                    'stale_network_handle',
                    'symlink_loop',
                    'symlink_escape_blocked',
                    'unsupported_path',
                    'invalid_path',
                    'io_interrupted',
                    'timed_out',
                    'unknown_io'
                )
        ),
    error_detail               TEXT,
    updated_at                 INTEGER NOT NULL,
    CHECK (
        last_scan_finished_at IS NULL
            OR (
                last_scan_started_at IS NOT NULL
                    AND last_scan_finished_at >= last_scan_started_at
            )
    ),
    CHECK (
        last_successful_scan_at IS NULL
            OR last_scan_finished_at IS NULL
            OR last_successful_scan_at <= last_scan_finished_at
    ),
    CHECK (scan_phase NOT IN ('blocked', 'partial') OR scan_issue_kind IS NOT NULL)
) STRICT;

CREATE TABLE source_root_navigation_state
(
    source_id                         INTEGER PRIMARY KEY REFERENCES sources (source_id) ON DELETE CASCADE,
    root_window_state                 TEXT    NOT NULL
        CHECK (root_window_state IN ('unknown', 'established', 'empty', 'missing', 'blocked', 'failed')),
    immediate_child_directory_count   INTEGER NOT NULL DEFAULT 0 CHECK (immediate_child_directory_count >= 0),
    issue_kind                        TEXT
        CHECK (
            issue_kind IS NULL
                OR issue_kind IN (
                    'missing',
                    'not_directory',
                    'permission_denied',
                    'privacy_permission_required',
                    'unavailable_mount',
                    'resource_busy',
                    'stale_network_handle',
                    'symlink_loop',
                    'symlink_escape_blocked',
                    'unsupported_path',
                    'invalid_path',
                    'io_interrupted',
                    'timed_out',
                    'unknown_io'
                )
        ),
    detail                            TEXT,
    checked_at                        INTEGER,
    updated_at                        INTEGER NOT NULL,
    CHECK (root_window_state NOT IN ('missing', 'blocked', 'failed') OR issue_kind IS NOT NULL),
    CHECK (root_window_state != 'established' OR immediate_child_directory_count > 0),
    CHECK (root_window_state != 'empty' OR immediate_child_directory_count = 0)
) STRICT;

CREATE TABLE source_registration_proposals
(
    source_registration_proposal_id INTEGER PRIMARY KEY,
    proposal_status                 TEXT    NOT NULL
        CHECK (proposal_status IN ('proposed', 'discarded', 'expired')),
    root_class                      TEXT    NOT NULL
        CHECK (root_class IN (
            'normal_music_root',
            'broad_drive_root',
            'system_volume_root',
            'user_profile_root',
            'cloud_backed_root',
            'network_root',
            'protected_root',
            'indirection_root',
            'unknown_root'
        )),
    requested_path                  TEXT    NOT NULL CHECK (length(trim(requested_path)) > 0),
    canonical_path                  TEXT CHECK (canonical_path IS NULL OR length(trim(canonical_path)) > 0),
    confirmation_required_reason    TEXT    NOT NULL CHECK (length(trim(confirmation_required_reason)) > 0),
    suggested_roots_json            TEXT    NOT NULL CHECK (json_valid(suggested_roots_json)),
    created_at                      INTEGER NOT NULL,
    updated_at                      INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    CHECK (proposal_status != 'proposed' OR root_class != 'normal_music_root'),
    CHECK (proposal_status != 'proposed' OR root_class != 'protected_root')
) STRICT;

CREATE UNIQUE INDEX source_registration_proposals_active_canonical_path
    ON source_registration_proposals (COALESCE(canonical_path, requested_path))
    WHERE proposal_status = 'proposed';

CREATE TABLE source_locations
(
    source_location_id INTEGER PRIMARY KEY,
    source_id          INTEGER NOT NULL REFERENCES sources (source_id) ON DELETE CASCADE,
    authority          TEXT    NOT NULL CHECK (authority IN ('device', 'user')),
    location_kind      TEXT    NOT NULL CHECK (location_kind IN ('observed_path', 'registered_subpath')),
    relative_path      TEXT    NOT NULL CHECK (length(trim(relative_path)) > 0),
    display_name       TEXT,
    is_user_visible    INTEGER NOT NULL DEFAULT 1 CHECK (is_user_visible IN (0, 1)),
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    CHECK (relative_path NOT LIKE '/%'),
    CHECK (relative_path NOT GLOB '[A-Za-z]:*'),
    CHECK (relative_path NOT LIKE '%://%'),
    CHECK (relative_path NOT LIKE '%\\%'),
    CHECK (relative_path NOT LIKE '%//%'),
    CHECK (relative_path NOT LIKE '%/'),
    CHECK (
        relative_path NOT IN ('.', '..')
            AND relative_path NOT LIKE './%'
            AND relative_path NOT LIKE '../%'
            AND relative_path NOT LIKE '%/./%'
            AND relative_path NOT LIKE '%/../%'
            AND relative_path NOT LIKE '%/.'
            AND relative_path NOT LIKE '%/..'
    ),
    CHECK (
        (location_kind = 'observed_path' AND authority = 'device')
            OR (location_kind = 'registered_subpath' AND authority = 'user')
    ),
    UNIQUE (source_id, relative_path)
) STRICT;

CREATE INDEX source_locations_source
    ON source_locations (source_id);

CREATE TABLE browser_user_order
(
    order_id     INTEGER PRIMARY KEY,
    node_domain  TEXT    NOT NULL CHECK (node_domain IN ('source', 'source_location')),
    node_id      TEXT    NOT NULL CHECK (length(trim(node_id)) > 0),
    parent_scope TEXT,
    ordinal      INTEGER NOT NULL CHECK (ordinal >= 0),
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    CHECK (
        (node_domain = 'source' AND parent_scope IS NULL)
            OR (
                node_domain = 'source_location'
                    AND parent_scope IS NOT NULL
                    AND length(trim(parent_scope)) > 0
            )
    )
) STRICT;

CREATE UNIQUE INDEX browser_user_order_source_node
    ON browser_user_order (node_domain, node_id)
    WHERE node_domain = 'source'
      AND parent_scope IS NULL;

CREATE UNIQUE INDEX browser_user_order_source_location_node
    ON browser_user_order (node_domain, node_id)
    WHERE node_domain = 'source_location'
      AND parent_scope IS NOT NULL;

CREATE INDEX browser_user_order_domain_parent_ordinal
    ON browser_user_order (node_domain, parent_scope, ordinal);

CREATE TABLE browser_user_prefs
(
    prefs_id                   INTEGER PRIMARY KEY,
    user_scope_key             TEXT    NOT NULL UNIQUE CHECK (length(trim(user_scope_key)) > 0),
    expanded_node_keys_json    TEXT,
    hidden_optional_nodes_json TEXT,
    created_at                 INTEGER NOT NULL,
    updated_at                 INTEGER NOT NULL,
    CHECK (updated_at >= created_at)
) STRICT;

CREATE TABLE source_directories
(
    source_directory_id                INTEGER PRIMARY KEY,
    source_id                          INTEGER NOT NULL REFERENCES sources (source_id) ON DELETE CASCADE,
    parent_source_directory_id         INTEGER REFERENCES source_directories (source_directory_id) ON DELETE CASCADE,
    name                               TEXT    NOT NULL,
    name_browse_sort_key               TEXT    NOT NULL,
    relative_path                      TEXT    NOT NULL,
    presence_state              TEXT    NOT NULL
        CHECK (presence_state IN ('present', 'missing', 'removed')),
    has_child_directories       INTEGER NOT NULL DEFAULT 0
        CHECK (has_child_directories IN (0, 1)),
    has_primary_media_descendant INTEGER NOT NULL DEFAULT 0
        CHECK (has_primary_media_descendant IN (0, 1)),
    has_image_media_descendant  INTEGER NOT NULL DEFAULT 0
        CHECK (has_image_media_descendant IN (0, 1)),
    dir_scan_state              TEXT    NOT NULL DEFAULT 'pending'
        CHECK (dir_scan_state IN ('pending', 'scanning', 'complete', 'failed', 'blocked')),
    dir_scan_issue_kind         TEXT
        CHECK (
            dir_scan_issue_kind IS NULL
                OR dir_scan_issue_kind IN (
                    'missing',
                    'not_directory',
                    'permission_denied',
                    'privacy_permission_required',
                    'unavailable_mount',
                    'resource_busy',
                    'stale_network_handle',
                    'symlink_loop',
                    'symlink_escape_blocked',
                    'unsupported_path',
                    'invalid_path',
                    'io_interrupted',
                    'timed_out',
                    'unknown_io'
                )
        ),
    dir_scan_error_detail       TEXT,
    dir_scan_updated_at         INTEGER NOT NULL,
    scanned_at                  INTEGER,
    mtime_ns                    INTEGER CHECK (mtime_ns IS NULL OR mtime_ns >= 0),
    created_at                  INTEGER NOT NULL,
    updated_at                  INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    CHECK (parent_source_directory_id IS NULL OR relative_path <> ''),
    CHECK (dir_scan_state NOT IN ('blocked', 'failed') OR dir_scan_issue_kind IS NOT NULL),
    UNIQUE (source_id, relative_path)
) STRICT;

CREATE INDEX source_directories_parent
    ON source_directories (parent_source_directory_id);

CREATE INDEX source_directories_source_relative_path_binary
    ON source_directories (source_id, relative_path COLLATE BINARY);

CREATE INDEX source_directories_parent_browse
    ON source_directories (source_id, parent_source_directory_id, presence_state, name_browse_sort_key, name);

CREATE TABLE source_files
(
    source_file_id                       INTEGER PRIMARY KEY,
    source_id                            INTEGER NOT NULL REFERENCES sources (source_id) ON DELETE CASCADE,
    parent_source_directory_id           INTEGER REFERENCES source_directories (source_directory_id) ON DELETE CASCADE,
    name                                 TEXT    NOT NULL CHECK (length(name) > 0),
    name_browse_sort_key                 TEXT    NOT NULL,
    relative_path_browse_sort_key        TEXT    NOT NULL,
    relative_path                        TEXT    NOT NULL CHECK (length(relative_path) > 0),
    size_bytes                  INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
    mtime_ns                    INTEGER CHECK (mtime_ns IS NULL OR mtime_ns >= 0),
    file_kind                   TEXT    NOT NULL DEFAULT 'unknown'
        CHECK (file_kind IN ('audio', 'video', 'image', 'cue_sheet', 'log_doc', 'text_doc', 'archive', 'other', 'unknown')),
    file_class                 TEXT    NOT NULL DEFAULT 'none'
        CHECK (file_class IN ('audio', 'video', 'image', 'unsupported', 'none')),
    presence_state              TEXT    NOT NULL
        CHECK (presence_state IN ('present', 'missing', 'removed')),
    first_discovered_at         INTEGER NOT NULL,
    last_observed_at            INTEGER,
    last_presence_change_at     INTEGER NOT NULL,
    created_at                  INTEGER NOT NULL,
    updated_at                  INTEGER NOT NULL,
    CHECK (last_observed_at IS NULL OR last_observed_at >= first_discovered_at),
    CHECK (last_presence_change_at >= first_discovered_at),
    CHECK (updated_at >= created_at),
    UNIQUE (source_id, relative_path)
) STRICT;

CREATE INDEX source_files_source_presence
    ON source_files (source_id, presence_state);

CREATE INDEX source_files_file_class_parent
    ON source_files (source_id, presence_state, file_class, file_kind, parent_source_directory_id)
    WHERE file_class IN ('audio', 'video', 'image', 'unsupported')
      AND parent_source_directory_id IS NOT NULL;

CREATE INDEX source_files_parent_source_directory
    ON source_files (parent_source_directory_id);

CREATE INDEX source_files_source_relative_path_binary
    ON source_files (source_id, relative_path COLLATE BINARY);

CREATE INDEX source_files_source_browse_order
    ON source_files (source_id, relative_path_browse_sort_key, relative_path, source_file_id);

CREATE INDEX source_files_parent_browse
    ON source_files (source_id, parent_source_directory_id, presence_state, name_browse_sort_key, name, file_class, file_kind);

CREATE TABLE WorkItems
(
    work_item_id        INTEGER PRIMARY KEY,
    subject_kind        TEXT    NOT NULL
        CHECK (subject_kind IN ('source_file', 'projection_domain')),
    subject_id          TEXT    NOT NULL CHECK (length(trim(subject_id)) > 0),
    work_kind           TEXT    NOT NULL
        CHECK (work_kind IN (
            'inspect_source',
            'rebuild_projection'
        )),
    priority_class      TEXT    NOT NULL
        CHECK (priority_class IN ('urgent', 'interactive', 'background')),
    basis_fingerprint   TEXT    NOT NULL CHECK (length(trim(basis_fingerprint)) > 0),
    state               TEXT    NOT NULL
        CHECK (state IN ('queued', 'leased', 'completed', 'blocked', 'failed', 'canceled')),
    leased_until        INTEGER,
    attempt_count       INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    blocked_reason      TEXT,
    failure_kind        TEXT,
    error_detail        TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    CHECK ((state = 'leased' AND leased_until IS NOT NULL) OR state <> 'leased'),
    CHECK (
        subject_kind <> 'projection_domain'
            OR subject_id = 'navigation'
    ),
    CHECK (updated_at >= created_at)
) STRICT;

CREATE INDEX WorkItems_subject_state
    ON WorkItems (subject_kind, subject_id, state, priority_class);

CREATE UNIQUE INDEX WorkItems_active_work
    ON WorkItems (subject_kind, subject_id, work_kind, basis_fingerprint)
    WHERE state IN ('queued', 'leased', 'blocked');

CREATE TABLE WorkRuns
(
    work_run_id       INTEGER PRIMARY KEY,
    work_item_id      INTEGER NOT NULL REFERENCES WorkItems (work_item_id) ON DELETE CASCADE,
    adapter_key       TEXT    NOT NULL CHECK (length(trim(adapter_key)) > 0),
    adapter_version   TEXT    NOT NULL CHECK (length(trim(adapter_version)) > 0),
    started_at        INTEGER NOT NULL,
    finished_at       INTEGER,
    outcome           TEXT    NOT NULL CHECK (length(trim(outcome)) > 0),
    failure_kind      TEXT,
    error_detail      TEXT,
    CHECK (finished_at IS NULL OR finished_at >= started_at)
) STRICT;

CREATE INDEX WorkRuns_work_item_started_at
    ON WorkRuns (work_item_id, started_at DESC);

CREATE TABLE Artifacts
(
    artifact_id         INTEGER PRIMARY KEY,
    work_run_id         INTEGER NOT NULL REFERENCES WorkRuns (work_run_id) ON DELETE CASCADE,
    subject_kind        TEXT    NOT NULL
        CHECK (subject_kind IN ('source_file', 'projection_domain')),
    subject_id          TEXT    NOT NULL CHECK (length(trim(subject_id)) > 0),
    artifact_kind       TEXT    NOT NULL
        CHECK (artifact_kind IN (
            'inspection_result',
            'projection_snapshot'
        )),
    artifact_role       TEXT    NOT NULL
        CHECK (artifact_role IN (
            'primary_result',
            'preview_summary',
            'manifest',
            'diagnostic_payload',
            'intermediate_output'
        )),
    adapter_key         TEXT    NOT NULL CHECK (length(trim(adapter_key)) > 0),
    adapter_version     TEXT    NOT NULL CHECK (length(trim(adapter_version)) > 0),
    basis_fingerprint   TEXT    NOT NULL CHECK (length(trim(basis_fingerprint)) > 0),
    media_type          TEXT    NOT NULL CHECK (length(trim(media_type)) > 0),
    storage_kind        TEXT    NOT NULL
        CHECK (storage_kind IN ('inline_payload', 'file_store')),
    payload_hash        TEXT    NOT NULL CHECK (length(trim(payload_hash)) > 0),
    created_at          INTEGER NOT NULL,
    CHECK (artifact_kind <> 'inspection_result' OR subject_kind = 'source_file'),
    CHECK (
        artifact_kind <> 'projection_snapshot'
            OR subject_kind = 'projection_domain'
    ),
    CHECK (
        subject_kind <> 'projection_domain'
            OR subject_id = 'navigation'
    )
) STRICT;

CREATE INDEX Artifacts_work_run
    ON Artifacts (work_run_id);

CREATE INDEX Artifacts_subject_created_at
    ON Artifacts (subject_kind, subject_id, created_at DESC);

CREATE INDEX Artifacts_kind_role_lookup
    ON Artifacts (subject_kind, subject_id, artifact_kind, artifact_role);

CREATE TABLE ArtifactInlinePayloads
(
    artifact_id  INTEGER PRIMARY KEY REFERENCES Artifacts (artifact_id) ON DELETE CASCADE,
    payload      BLOB    NOT NULL,
    created_at   INTEGER NOT NULL
) STRICT;

CREATE TABLE ArtifactFileStoreEntries
(
    artifact_id    INTEGER PRIMARY KEY REFERENCES Artifacts (artifact_id) ON DELETE CASCADE,
    root_kind      TEXT    NOT NULL CHECK (length(trim(root_kind)) > 0),
    relative_path  TEXT    NOT NULL CHECK (length(trim(relative_path)) > 0),
    payload_bytes  INTEGER NOT NULL CHECK (payload_bytes >= 0),
    created_at     INTEGER NOT NULL
) STRICT;

CREATE TABLE ArtifactClaims
(
    claim_id         INTEGER PRIMARY KEY,
    artifact_id      INTEGER NOT NULL REFERENCES Artifacts (artifact_id) ON DELETE CASCADE,
    claimant_kind    TEXT    NOT NULL CHECK (length(trim(claimant_kind)) > 0),
    claimant_key     TEXT    NOT NULL CHECK (length(trim(claimant_key)) > 0),
    release_policy   TEXT    NOT NULL CHECK (length(trim(release_policy)) > 0),
    claimed_at       INTEGER NOT NULL,
    released_at      INTEGER,
    CHECK (released_at IS NULL OR released_at >= claimed_at)
) STRICT;

CREATE UNIQUE INDEX ArtifactClaims_active_claim
    ON ArtifactClaims (artifact_id, claimant_kind, claimant_key)
    WHERE released_at IS NULL;

CREATE TABLE SourceFacts
(
    source_file_id        INTEGER PRIMARY KEY REFERENCES source_files (source_file_id) ON DELETE CASCADE,
    fact_kind             TEXT    NOT NULL CHECK (fact_kind IN ('source_inspection')),
    basis_fingerprint     TEXT    NOT NULL CHECK (length(trim(basis_fingerprint)) > 0),
    basis_source_id       INTEGER NOT NULL REFERENCES sources (source_id) ON DELETE CASCADE,
    basis_relative_path   TEXT    NOT NULL CHECK (length(trim(basis_relative_path)) > 0),
    basis_size_bytes      INTEGER CHECK (basis_size_bytes IS NULL OR basis_size_bytes >= 0),
    basis_mtime_ns        INTEGER CHECK (basis_mtime_ns IS NULL OR basis_mtime_ns >= 0),
    basis_presence_state  TEXT    NOT NULL CHECK (basis_presence_state IN ('present', 'missing', 'removed')),
    observed_at_ms        INTEGER NOT NULL,
    content_hash_algorithm TEXT CHECK (content_hash_algorithm IS NULL OR length(trim(content_hash_algorithm)) > 0),
    content_hash_value    TEXT CHECK (content_hash_value IS NULL OR length(trim(content_hash_value)) > 0),
    media_kind            TEXT    NOT NULL CHECK (length(trim(media_kind)) > 0),
    mime_type             TEXT,
    duration_ms           INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
    sample_rate_hz        INTEGER CHECK (sample_rate_hz IS NULL OR sample_rate_hz > 0),
    channels              INTEGER CHECK (channels IS NULL OR channels > 0),
    bit_depth             INTEGER CHECK (bit_depth IS NULL OR bit_depth > 0),
    codec                 TEXT,
    updated_at            INTEGER NOT NULL,
    accepted_artifact_id  INTEGER NOT NULL REFERENCES Artifacts (artifact_id),
    CHECK (
        (content_hash_algorithm IS NULL AND content_hash_value IS NULL)
            OR (content_hash_algorithm IS NOT NULL AND content_hash_value IS NOT NULL)
    ),
    CHECK (updated_at >= observed_at_ms)
) STRICT;

CREATE INDEX SourceFacts_source_basis
    ON SourceFacts (basis_source_id, basis_relative_path);

CREATE INDEX SourceFacts_media_kind
    ON SourceFacts (media_kind);

CREATE TABLE content_attachments
(
    attachment_id           INTEGER PRIMARY KEY,
    content_hash_algorithm  TEXT    NOT NULL CHECK (content_hash_algorithm = 'blake3'),
    content_hash_value      TEXT    NOT NULL CHECK (length(trim(content_hash_value)) > 0),
    first_observed_at       INTEGER NOT NULL,
    updated_at              INTEGER NOT NULL,
    CHECK (updated_at >= first_observed_at),
    UNIQUE (content_hash_algorithm, content_hash_value)
) STRICT;

CREATE TABLE source_file_attachment_links
(
    source_file_attachment_link_id  INTEGER PRIMARY KEY,
    attachment_id                   INTEGER NOT NULL REFERENCES content_attachments (attachment_id) ON DELETE CASCADE,
    source_file_id                  INTEGER NOT NULL REFERENCES source_files (source_file_id) ON DELETE CASCADE,
    source_id                       INTEGER NOT NULL REFERENCES sources (source_id) ON DELETE CASCADE,
    file_kind                       TEXT    NOT NULL
        CHECK (file_kind IN ('audio', 'video', 'image', 'cue_sheet', 'log_doc', 'text_doc', 'archive', 'other', 'unknown')),
    created_at                      INTEGER NOT NULL,
    updated_at                      INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    UNIQUE (source_file_id)
) STRICT;

CREATE INDEX source_file_attachment_links_source_file
    ON source_file_attachment_links (source_file_id);

CREATE INDEX source_file_attachment_links_attachment
    ON source_file_attachment_links (attachment_id);

CREATE TABLE search_filter_index_metadata
(
    search_filter_index_id INTEGER PRIMARY KEY CHECK (search_filter_index_id = 1),
    indexer_version        TEXT    NOT NULL CHECK (indexer_version = 'search_filter_v0'),
    generation             INTEGER NOT NULL CHECK (generation >= 0),
    state                  TEXT    NOT NULL CHECK (state IN ('ready', 'rebuilding', 'partial', 'failed')),
    updated_at             INTEGER NOT NULL
) STRICT;

CREATE TABLE search_filter_index_source_coverage
(
    source_id       INTEGER PRIMARY KEY REFERENCES sources (source_id) ON DELETE CASCADE,
    indexer_version TEXT    NOT NULL CHECK (indexer_version = 'search_filter_v0'),
    generation      INTEGER NOT NULL CHECK (generation >= 0),
    state           TEXT    NOT NULL CHECK (state IN ('ready', 'rebuilding', 'partial', 'failed')),
    rebuilt_at      INTEGER,
    updated_at      INTEGER NOT NULL,
    detail          TEXT,
    CHECK (rebuilt_at IS NULL OR updated_at >= rebuilt_at)
) STRICT;

CREATE TABLE search_filter_index_rows
(
    row_id                       INTEGER PRIMARY KEY,
    generation                   INTEGER NOT NULL CHECK (generation >= 0),
    result_kind                  TEXT    NOT NULL
        CHECK (result_kind IN ('source', 'source_location', 'directory', 'source_file')),
    authority_layer              TEXT    NOT NULL
        CHECK (authority_layer IN ('source', 'source_location', 'source_hierarchy', 'source_file_inventory')),
    stable_key                   TEXT    NOT NULL UNIQUE CHECK (length(trim(stable_key)) > 0),
    source_id                    INTEGER REFERENCES sources (source_id) ON DELETE CASCADE,
    source_location_id           INTEGER REFERENCES source_locations (source_location_id) ON DELETE CASCADE,
    source_directory_id          INTEGER REFERENCES source_directories (source_directory_id) ON DELETE CASCADE,
    parent_source_directory_id   INTEGER REFERENCES source_directories (source_directory_id) ON DELETE CASCADE,
    source_file_id               INTEGER REFERENCES source_files (source_file_id) ON DELETE CASCADE,
    display_label                TEXT    NOT NULL CHECK (length(trim(display_label)) > 0),
    display_path                 TEXT,
    relative_path                TEXT,
    sort_key                     TEXT    NOT NULL,
    file_class                   TEXT
        CHECK (file_class IS NULL OR file_class IN ('audio', 'video', 'image', 'unsupported', 'none')),
    file_kind                    TEXT
        CHECK (file_kind IS NULL OR file_kind IN ('audio', 'video', 'image', 'cue_sheet', 'log_doc', 'text_doc', 'archive', 'other', 'unknown')),
    media_relevance              TEXT
        CHECK (media_relevance IS NULL OR media_relevance IN ('audio_workflow', 'playable_media', 'explicit_inventory', 'companion_file', 'not_media_relevant')),
    presence_state               TEXT
        CHECK (presence_state IS NULL OR presence_state IN ('present', 'missing', 'removed')),
    source_access_state          TEXT
        CHECK (source_access_state IS NULL OR source_access_state IN ('accessible', 'missing', 'blocked', 'unknown')),
    source_scan_phase            TEXT
        CHECK (source_scan_phase IS NULL OR source_scan_phase IN ('idle', 'scanning', 'complete', 'partial', 'blocked', 'failed')),
    has_current_blake3           INTEGER NOT NULL DEFAULT 0 CHECK (has_current_blake3 IN (0, 1)),
    has_current_probe            INTEGER NOT NULL DEFAULT 0 CHECK (has_current_probe IN (0, 1)),
    attachment_link_state        TEXT    NOT NULL DEFAULT 'not_applicable'
        CHECK (attachment_link_state IN ('current', 'stale', 'missing', 'not_applicable')),
    attachment_id                INTEGER REFERENCES content_attachments (attachment_id) ON DELETE SET NULL,
    content_hash_algorithm       TEXT CHECK (content_hash_algorithm IS NULL OR content_hash_algorithm = 'blake3'),
    content_hash_value           TEXT,
    evidence_coverage_state      TEXT    NOT NULL CHECK (evidence_coverage_state IN ('indexed', 'not_applicable')),
    updated_at                   INTEGER NOT NULL,
    CHECK (
        (result_kind = 'source' AND source_id IS NOT NULL AND source_location_id IS NULL AND source_directory_id IS NULL AND source_file_id IS NULL)
            OR (result_kind = 'source_location' AND source_id IS NOT NULL AND source_location_id IS NOT NULL AND source_directory_id IS NULL AND source_file_id IS NULL)
            OR (result_kind = 'directory' AND source_id IS NOT NULL AND source_location_id IS NULL AND source_directory_id IS NOT NULL AND source_file_id IS NULL)
            OR (result_kind = 'source_file' AND source_id IS NOT NULL AND source_location_id IS NULL AND source_file_id IS NOT NULL)
    )
) STRICT;

CREATE INDEX search_filter_index_rows_source
    ON search_filter_index_rows (source_id, result_kind, sort_key, stable_key);

CREATE INDEX search_filter_index_rows_directory_scope
    ON search_filter_index_rows (source_id, parent_source_directory_id, result_kind, sort_key, stable_key);

CREATE INDEX search_filter_index_rows_filters
    ON search_filter_index_rows (
        result_kind,
        file_class,
        file_kind,
        presence_state,
        source_access_state,
        has_current_blake3,
        has_current_probe,
        attachment_link_state
    );

CREATE VIRTUAL TABLE search_filter_index_fts USING fts5(
    display_label,
    display_path,
    tokenize = 'unicode61 remove_diacritics 1'
);

CREATE TABLE primary_media_candidates
(
    primary_media_candidate_id  INTEGER PRIMARY KEY,
    attachment_id               INTEGER NOT NULL UNIQUE REFERENCES content_attachments (attachment_id) ON DELETE CASCADE,
    evidence_source_file_id     INTEGER NOT NULL REFERENCES source_files (source_file_id) ON DELETE CASCADE,
    evidence_basis_fingerprint  TEXT    NOT NULL CHECK (length(trim(evidence_basis_fingerprint)) > 0),
    media_kind                  TEXT    NOT NULL CHECK (media_kind = 'audio'),
    mime_type                   TEXT,
    duration_ms                 INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
    sample_rate_hz              INTEGER CHECK (sample_rate_hz IS NULL OR sample_rate_hz > 0),
    channels                    INTEGER CHECK (channels IS NULL OR channels > 0),
    bit_depth                   INTEGER CHECK (bit_depth IS NULL OR bit_depth > 0),
    codec                       TEXT,
    created_at                  INTEGER NOT NULL,
    updated_at                  INTEGER NOT NULL,
    CHECK (updated_at >= created_at)
) STRICT;

CREATE INDEX primary_media_candidates_evidence_source_file
    ON primary_media_candidates (evidence_source_file_id);

CREATE TABLE track_identity_candidates
(
    track_identity_candidate_id  INTEGER PRIMARY KEY,
    candidate_kind               TEXT    NOT NULL CHECK (candidate_kind = 'exact_primary_media_content'),
    evidence_basis               TEXT    NOT NULL CHECK (evidence_basis = 'current_primary_media_exact_blake3'),
    evidence_key_algorithm       TEXT    NOT NULL CHECK (evidence_key_algorithm = 'blake3'),
    evidence_key_value           TEXT    NOT NULL CHECK (length(trim(evidence_key_value)) > 0),
    status                       TEXT    NOT NULL CHECK (status IN ('active', 'stale', 'superseded')),
    created_at                   INTEGER NOT NULL,
    updated_at                   INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    UNIQUE (candidate_kind, evidence_basis, evidence_key_algorithm, evidence_key_value)
) STRICT;

CREATE INDEX track_identity_candidates_status
    ON track_identity_candidates (status);

CREATE TABLE track_identity_candidate_members
(
    track_identity_candidate_member_id  INTEGER PRIMARY KEY,
    track_identity_candidate_id         INTEGER NOT NULL REFERENCES track_identity_candidates (track_identity_candidate_id) ON DELETE CASCADE,
    primary_media_candidate_id          INTEGER NOT NULL REFERENCES primary_media_candidates (primary_media_candidate_id) ON DELETE CASCADE,
    attachment_id                       INTEGER NOT NULL REFERENCES content_attachments (attachment_id) ON DELETE CASCADE,
    evidence_source_file_id             INTEGER NOT NULL REFERENCES source_files (source_file_id) ON DELETE CASCADE,
    evidence_basis_fingerprint          TEXT    NOT NULL CHECK (length(trim(evidence_basis_fingerprint)) > 0),
    content_hash_algorithm              TEXT    NOT NULL CHECK (content_hash_algorithm = 'blake3'),
    content_hash_value                  TEXT    NOT NULL CHECK (length(trim(content_hash_value)) > 0),
    created_at                          INTEGER NOT NULL,
    updated_at                          INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    UNIQUE (primary_media_candidate_id)
) STRICT;

CREATE INDEX track_identity_candidate_members_candidate
    ON track_identity_candidate_members (track_identity_candidate_id);

CREATE INDEX track_identity_candidate_members_attachment
    ON track_identity_candidate_members (attachment_id);

CREATE TABLE track_identity_candidate_evidence
(
    track_identity_candidate_evidence_id  INTEGER PRIMARY KEY,
    track_identity_candidate_id           INTEGER NOT NULL REFERENCES track_identity_candidates (track_identity_candidate_id) ON DELETE CASCADE,
    primary_media_candidate_id            INTEGER NOT NULL REFERENCES primary_media_candidates (primary_media_candidate_id) ON DELETE CASCADE,
    attachment_id                         INTEGER NOT NULL REFERENCES content_attachments (attachment_id) ON DELETE CASCADE,
    source_file_attachment_link_id        INTEGER NOT NULL,
    source_file_id                        INTEGER NOT NULL REFERENCES source_files (source_file_id) ON DELETE CASCADE,
    source_id                             INTEGER NOT NULL REFERENCES sources (source_id) ON DELETE CASCADE,
    evidence_basis_fingerprint            TEXT    NOT NULL CHECK (length(trim(evidence_basis_fingerprint)) > 0),
    content_hash_algorithm                TEXT    NOT NULL CHECK (content_hash_algorithm = 'blake3'),
    content_hash_value                    TEXT    NOT NULL CHECK (length(trim(content_hash_value)) > 0),
    probe_accepted_artifact_id            INTEGER NOT NULL REFERENCES Artifacts (artifact_id),
    created_at                            INTEGER NOT NULL,
    updated_at                            INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    UNIQUE (track_identity_candidate_id, primary_media_candidate_id, source_file_id)
) STRICT;

CREATE INDEX track_identity_candidate_evidence_candidate
    ON track_identity_candidate_evidence (track_identity_candidate_id);

CREATE INDEX track_identity_candidate_evidence_source_file
    ON track_identity_candidate_evidence (source_file_id);

CREATE TABLE track_identity_decisions
(
    track_identity_decision_id   INTEGER PRIMARY KEY,
    track_identity_candidate_id  INTEGER NOT NULL,
    decision_state               TEXT    NOT NULL CHECK (decision_state IN ('accepted', 'rejected', 'deferred', 'superseded')),
    decision_source              TEXT    NOT NULL CHECK (length(trim(decision_source)) > 0),
    decision_basis               TEXT    NOT NULL CHECK (length(trim(decision_basis)) > 0),
    decision_reason              TEXT    NOT NULL CHECK (length(trim(decision_reason)) > 0),
    candidate_kind               TEXT    NOT NULL CHECK (candidate_kind = 'exact_primary_media_content'),
    candidate_evidence_basis     TEXT    NOT NULL CHECK (candidate_evidence_basis = 'current_primary_media_exact_blake3'),
    candidate_status_at_decision TEXT    NOT NULL CHECK (candidate_status_at_decision IN ('active', 'stale', 'superseded')),
    evidence_key_algorithm       TEXT    NOT NULL CHECK (evidence_key_algorithm = 'blake3'),
    evidence_key_value           TEXT    NOT NULL CHECK (length(trim(evidence_key_value)) > 0),
    superseded_by_decision_id    INTEGER REFERENCES track_identity_decisions (track_identity_decision_id) ON DELETE SET NULL,
    created_at                   INTEGER NOT NULL,
    updated_at                   INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    CHECK (
        superseded_by_decision_id IS NULL
        OR superseded_by_decision_id != track_identity_decision_id
    )
) STRICT;

CREATE UNIQUE INDEX track_identity_decisions_current_source
    ON track_identity_decisions (track_identity_candidate_id, decision_source)
    WHERE superseded_by_decision_id IS NULL;

CREATE INDEX track_identity_decisions_candidate
    ON track_identity_decisions (track_identity_candidate_id);

CREATE INDEX track_identity_decisions_state
    ON track_identity_decisions (decision_state);

CREATE TABLE track_identity_decision_source_scope
(
    track_identity_decision_source_scope_id  INTEGER PRIMARY KEY,
    track_identity_decision_id               INTEGER NOT NULL REFERENCES track_identity_decisions (track_identity_decision_id) ON DELETE CASCADE,
    track_identity_candidate_id              INTEGER NOT NULL,
    source_id                                INTEGER NOT NULL,
    scope_basis                              TEXT    NOT NULL CHECK (scope_basis IN ('current_decision_evidence_source_v0', 'candidate_source_provenance_v0')),
    created_at                               INTEGER NOT NULL,
    updated_at                               INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    UNIQUE (track_identity_decision_id, source_id)
) STRICT;

CREATE INDEX track_identity_decision_source_scope_decision
    ON track_identity_decision_source_scope (track_identity_decision_id);

CREATE INDEX track_identity_decision_source_scope_source
    ON track_identity_decision_source_scope (source_id);

CREATE TABLE track_identity_decision_evidence
(
    track_identity_decision_evidence_id  INTEGER PRIMARY KEY,
    track_identity_decision_id           INTEGER NOT NULL REFERENCES track_identity_decisions (track_identity_decision_id) ON DELETE CASCADE,
    track_identity_candidate_id          INTEGER NOT NULL,
    track_identity_candidate_member_id   INTEGER NOT NULL,
    track_identity_candidate_evidence_id INTEGER NOT NULL,
    primary_media_candidate_id           INTEGER NOT NULL,
    attachment_id                        INTEGER NOT NULL,
    source_file_attachment_link_id       INTEGER NOT NULL,
    source_file_id                       INTEGER NOT NULL,
    source_id                            INTEGER NOT NULL,
    evidence_basis_fingerprint           TEXT    NOT NULL CHECK (length(trim(evidence_basis_fingerprint)) > 0),
    content_hash_algorithm               TEXT    NOT NULL CHECK (content_hash_algorithm = 'blake3'),
    content_hash_value                   TEXT    NOT NULL CHECK (length(trim(content_hash_value)) > 0),
    probe_accepted_artifact_id           INTEGER NOT NULL,
    created_at                           INTEGER NOT NULL,
    updated_at                           INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    UNIQUE (track_identity_decision_id, track_identity_candidate_evidence_id)
) STRICT;

CREATE INDEX track_identity_decision_evidence_decision
    ON track_identity_decision_evidence (track_identity_decision_id);

CREATE INDEX track_identity_decision_evidence_candidate
    ON track_identity_decision_evidence (track_identity_candidate_id);

CREATE INDEX track_identity_decision_evidence_source_file
    ON track_identity_decision_evidence (source_file_id);

CREATE TABLE navigation_rows
(
    navigation_row_id         INTEGER PRIMARY KEY,
    stable_key                TEXT    NOT NULL UNIQUE CHECK (length(trim(stable_key)) > 0),
    parent_navigation_row_id  INTEGER REFERENCES navigation_rows (navigation_row_id) ON DELETE CASCADE,
    family                    TEXT CHECK (
        family IS NULL
            OR family IN ('Views', 'Sources')
    ),
    row_kind                  TEXT    NOT NULL CHECK (length(trim(row_kind)) > 0),
    display_name              TEXT    NOT NULL CHECK (length(trim(display_name)) > 0),
    sibling_position          INTEGER NOT NULL CHECK (sibling_position >= 0),
    selectable                INTEGER NOT NULL CHECK (selectable IN (0, 1)),
    selector_kind             TEXT CHECK (selector_kind IS NULL OR length(trim(selector_kind)) > 0),
    selector_payload          TEXT,
    updated_at                INTEGER NOT NULL,
    row_version               INTEGER NOT NULL CHECK (row_version >= 0),
    CHECK (
        (
            selectable = 1
                AND selector_kind IS NOT NULL
                AND selector_payload IS NOT NULL
        )
            OR (
            selectable = 0
                AND selector_kind IS NULL
                AND selector_payload IS NULL
        )
    ),
    CHECK (
        (parent_navigation_row_id IS NULL AND family IS NOT NULL)
            OR (parent_navigation_row_id IS NOT NULL AND family IS NULL)
    )
) STRICT;

CREATE INDEX navigation_rows_parent_sibling
    ON navigation_rows (parent_navigation_row_id, sibling_position);

CREATE TABLE ProjectionChangeLog
(
    change_sequence  INTEGER PRIMARY KEY,
    projection_domain TEXT NOT NULL
        CHECK (projection_domain = 'navigation'),
    row_key          TEXT    NOT NULL CHECK (length(trim(row_key)) > 0),
    change_kind      TEXT    NOT NULL CHECK (length(trim(change_kind)) > 0),
    row_version      INTEGER NOT NULL CHECK (row_version >= 0),
    payload_json     TEXT    NOT NULL,
    created_at       INTEGER NOT NULL
) STRICT;

CREATE INDEX ProjectionChangeLog_domain_sequence
    ON ProjectionChangeLog (projection_domain, change_sequence);

CREATE TABLE ProjectionSubscribers
(
    subscriber_id  INTEGER PRIMARY KEY,
    owner_kind     TEXT    NOT NULL CHECK (length(trim(owner_kind)) > 0),
    last_seen_at   INTEGER NOT NULL,
    expires_at     INTEGER NOT NULL,
    CHECK (expires_at >= last_seen_at)
) STRICT;

CREATE INDEX ProjectionSubscribers_expires_at
    ON ProjectionSubscribers (expires_at);

CREATE TABLE ProjectionCursors
(
    subscriber_id      INTEGER NOT NULL REFERENCES ProjectionSubscribers (subscriber_id) ON DELETE CASCADE,
    projection_domain  TEXT    NOT NULL
        CHECK (projection_domain = 'navigation'),
    position           INTEGER NOT NULL CHECK (position >= 0),
    updated_at         INTEGER NOT NULL,
    PRIMARY KEY (subscriber_id, projection_domain)
) STRICT;

CREATE TABLE ProjectionRetentionWatermarks
(
    projection_domain                  TEXT PRIMARY KEY
        CHECK (projection_domain = 'navigation'),
    live_subscriber_count              INTEGER NOT NULL CHECK (live_subscriber_count >= 0),
    live_min_position                  INTEGER CHECK (
        live_min_position IS NULL
            OR live_min_position >= 0
    ),
    earliest_retained_change_sequence  INTEGER CHECK (
        earliest_retained_change_sequence IS NULL
            OR earliest_retained_change_sequence >= 0
    ),
    updated_at                         INTEGER NOT NULL,
    CHECK (
        (
            live_subscriber_count = 0
                AND live_min_position IS NULL
        )
            OR (
            live_subscriber_count > 0
                AND live_min_position IS NOT NULL
        )
    )
) STRICT;

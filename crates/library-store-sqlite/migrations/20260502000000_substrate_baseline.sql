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
    source_directory_id         INTEGER PRIMARY KEY,
    source_id                   INTEGER NOT NULL REFERENCES sources (source_id) ON DELETE CASCADE,
    parent_source_directory_id  INTEGER REFERENCES source_directories (source_directory_id) ON DELETE CASCADE,
    name                        TEXT    NOT NULL,
    relative_path               TEXT    NOT NULL,
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

CREATE TABLE source_files
(
    source_file_id              INTEGER PRIMARY KEY,
    source_id                   INTEGER NOT NULL REFERENCES sources (source_id) ON DELETE CASCADE,
    parent_source_directory_id  INTEGER REFERENCES source_directories (source_directory_id) ON DELETE CASCADE,
    name                        TEXT    NOT NULL CHECK (length(name) > 0),
    relative_path               TEXT    NOT NULL CHECK (length(relative_path) > 0),
    size_bytes                  INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
    mtime_ns                    INTEGER CHECK (mtime_ns IS NULL OR mtime_ns >= 0),
    file_kind                   TEXT    NOT NULL DEFAULT 'unknown'
        CHECK (file_kind IN ('audio', 'video', 'image', 'cue_sheet', 'log_doc', 'text_doc', 'archive', 'other', 'unknown')),
    media_class                 TEXT    NOT NULL DEFAULT 'none'
        CHECK (media_class IN ('audio', 'video', 'image', 'unsupported', 'none')),
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

CREATE INDEX source_files_media_class_parent
    ON source_files (source_id, presence_state, media_class, file_kind, parent_source_directory_id)
    WHERE media_class IN ('audio', 'video', 'image', 'unsupported')
      AND parent_source_directory_id IS NOT NULL;

CREATE INDEX source_files_parent_source_directory
    ON source_files (parent_source_directory_id);

CREATE INDEX source_files_source_relative_path_binary
    ON source_files (source_id, relative_path COLLATE BINARY);

-- TOMBSTONE: LibraryAssets is dormant for new content identity work. It is
-- replaced by content_attachments plus future track/media layers, and the
-- media-candidate/track-identity slice must delete or rename it once asset-prep
-- and browser paths no longer depend on it. Current code must not use it for
-- new content identity.
CREATE TABLE LibraryAssets
(
    library_asset_id          INTEGER PRIMARY KEY,
    equivalence_fingerprint   TEXT    NOT NULL UNIQUE CHECK (length(trim(equivalence_fingerprint)) > 0),
    retention_policy          TEXT    NOT NULL DEFAULT 'keep_metadata'
        CHECK (retention_policy IN ('keep_metadata', 'purge')),
    created_at                INTEGER NOT NULL,
    updated_at                INTEGER NOT NULL,
    CHECK (updated_at >= created_at)
) STRICT;

CREATE TABLE Playlists
(
    playlist_id   INTEGER PRIMARY KEY,
    display_name  TEXT    NOT NULL CHECK (length(trim(display_name)) > 0),
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    CHECK (updated_at >= created_at)
) STRICT;

CREATE TABLE PlaylistEntries
(
    playlist_entry_id  INTEGER PRIMARY KEY,
    playlist_id        INTEGER NOT NULL REFERENCES Playlists (playlist_id) ON DELETE CASCADE,
    library_asset_id   INTEGER NOT NULL REFERENCES LibraryAssets (library_asset_id) ON DELETE CASCADE,
    position           INTEGER NOT NULL CHECK (position >= 0),
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    UNIQUE (playlist_id, library_asset_id),
    UNIQUE (playlist_id, position)
) STRICT;

CREATE INDEX PlaylistEntries_library_asset
    ON PlaylistEntries (library_asset_id);

CREATE TABLE CapabilitySpecs
(
    capability_kind     TEXT PRIMARY KEY CHECK (length(trim(capability_kind)) > 0),
    display_name        TEXT    NOT NULL CHECK (length(trim(display_name)) > 0),
    quality_aware       INTEGER NOT NULL CHECK (quality_aware IN (0, 1)),
    default_profile_key TEXT    NOT NULL CHECK (length(trim(default_profile_key)) > 0),
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    CHECK (updated_at >= created_at)
) STRICT;

CREATE TABLE CapabilityDependencies
(
    upstream_capability_kind    TEXT    NOT NULL REFERENCES CapabilitySpecs (capability_kind) ON DELETE CASCADE,
    downstream_capability_kind  TEXT    NOT NULL REFERENCES CapabilitySpecs (capability_kind) ON DELETE CASCADE,
    invalidation_mode           TEXT    NOT NULL CHECK (length(trim(invalidation_mode)) > 0),
    created_at                  INTEGER NOT NULL,
    updated_at                  INTEGER NOT NULL,
    PRIMARY KEY (upstream_capability_kind, downstream_capability_kind),
    CHECK (upstream_capability_kind <> downstream_capability_kind),
    CHECK (updated_at >= created_at)
) STRICT;

CREATE TABLE PrepPolicies
(
    prep_policy_id     INTEGER PRIMARY KEY,
    policy_name        TEXT    NOT NULL CHECK (length(trim(policy_name)) > 0),
    is_system_policy   INTEGER NOT NULL CHECK (is_system_policy IN (0, 1)),
    is_user_editable   INTEGER NOT NULL CHECK (is_user_editable IN (0, 1)),
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    UNIQUE (policy_name)
) STRICT;

CREATE TABLE PrepPolicyTargets
(
    prep_policy_id          INTEGER NOT NULL REFERENCES PrepPolicies (prep_policy_id) ON DELETE CASCADE,
    capability_kind         TEXT    NOT NULL REFERENCES CapabilitySpecs (capability_kind),
    target_profile_key      TEXT    NOT NULL CHECK (length(trim(target_profile_key)) > 0),
    target_quality          INTEGER NOT NULL CHECK (target_quality >= 0),
    target_stability_class  TEXT    NOT NULL
        CHECK (target_stability_class IN ('provisional', 'stable')),
    priority_class          TEXT    NOT NULL
        CHECK (priority_class IN ('urgent', 'interactive', 'background')),
    PRIMARY KEY (prep_policy_id, capability_kind, target_profile_key)
) STRICT;

CREATE TABLE PrepAssignments
(
    prep_assignment_id  INTEGER PRIMARY KEY,
    scope_kind          TEXT    NOT NULL
        CHECK (scope_kind IN ('library', 'source', 'library_asset')),
    scope_id            TEXT    NOT NULL CHECK (length(trim(scope_id)) > 0),
    prep_policy_id      INTEGER NOT NULL REFERENCES PrepPolicies (prep_policy_id) ON DELETE CASCADE,
    precedence_rank     INTEGER NOT NULL CHECK (precedence_rank >= 0),
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    CHECK (updated_at >= created_at)
) STRICT;

CREATE INDEX PrepAssignments_scope_precedence
    ON PrepAssignments (scope_kind, scope_id, precedence_rank);

CREATE TABLE ResolvedLibraryAssetPrepTargets
(
    library_asset_id         INTEGER NOT NULL REFERENCES LibraryAssets (library_asset_id) ON DELETE CASCADE,
    capability_kind          TEXT    NOT NULL REFERENCES CapabilitySpecs (capability_kind),
    target_profile_key       TEXT    NOT NULL CHECK (length(trim(target_profile_key)) > 0),
    target_quality           INTEGER NOT NULL CHECK (target_quality >= 0),
    target_stability_class   TEXT    NOT NULL
        CHECK (target_stability_class IN ('provisional', 'stable')),
    priority_class           TEXT    NOT NULL
        CHECK (priority_class IN ('urgent', 'interactive', 'background')),
    resolved_from_policy_id  INTEGER NOT NULL REFERENCES PrepPolicies (prep_policy_id),
    updated_at               INTEGER NOT NULL,
    PRIMARY KEY (library_asset_id, capability_kind, target_profile_key)
) STRICT;

CREATE INDEX ResolvedLibraryAssetPrepTargets_policy
    ON ResolvedLibraryAssetPrepTargets (resolved_from_policy_id);

CREATE TABLE WorkItems
(
    work_item_id        INTEGER PRIMARY KEY,
    subject_kind        TEXT    NOT NULL
        CHECK (subject_kind IN ('source_file', 'library_asset', 'projection_domain')),
    subject_id          TEXT    NOT NULL CHECK (length(trim(subject_id)) > 0),
    work_kind           TEXT    NOT NULL
        CHECK (work_kind IN (
            'inspect_source',
            'accept_segmentation',
            'resolve_library_asset',
            'compute_capability',
            'rebind_source',
            'rebuild_projection'
        )),
    capability_kind     TEXT REFERENCES CapabilitySpecs (capability_kind),
    target_profile_key  TEXT CHECK (target_profile_key IS NULL OR length(trim(target_profile_key)) > 0),
    target_quality      INTEGER CHECK (target_quality IS NULL OR target_quality >= 0),
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
    CHECK (
        (
            work_kind = 'compute_capability'
                AND capability_kind IS NOT NULL
                AND target_profile_key IS NOT NULL
                AND target_quality IS NOT NULL
        )
            OR (
            work_kind <> 'compute_capability'
                AND capability_kind IS NULL
                AND target_profile_key IS NULL
                AND target_quality IS NULL
        )
    ),
    CHECK ((state = 'leased' AND leased_until IS NOT NULL) OR state <> 'leased'),
    CHECK (
        subject_kind <> 'projection_domain'
            OR subject_id IN ('library_browser', 'navigation')
    ),
    CHECK (updated_at >= created_at)
) STRICT;

CREATE INDEX WorkItems_subject_state
    ON WorkItems (subject_kind, subject_id, state, priority_class);

CREATE UNIQUE INDEX WorkItems_active_compute_capability
    ON WorkItems (
        subject_kind,
        subject_id,
        work_kind,
        capability_kind,
        target_profile_key,
        target_quality,
        basis_fingerprint
    )
    WHERE work_kind = 'compute_capability'
      AND state IN ('queued', 'leased', 'blocked');

CREATE UNIQUE INDEX WorkItems_active_non_capability
    ON WorkItems (subject_kind, subject_id, work_kind, basis_fingerprint)
    WHERE work_kind <> 'compute_capability'
      AND state IN ('queued', 'leased', 'blocked');

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
        CHECK (subject_kind IN ('source_file', 'library_asset', 'projection_domain')),
    subject_id          TEXT    NOT NULL CHECK (length(trim(subject_id)) > 0),
    capability_kind     TEXT REFERENCES CapabilitySpecs (capability_kind),
    profile_key         TEXT CHECK (profile_key IS NULL OR length(trim(profile_key)) > 0),
    artifact_kind       TEXT    NOT NULL
        CHECK (artifact_kind IN (
            'inspection_result',
            'segmentation_result',
            'capability_result',
            'projection_snapshot',
            'diagnostic_result'
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
    CHECK (
        artifact_kind <> 'capability_result'
            OR (
                subject_kind = 'library_asset'
                    AND capability_kind IS NOT NULL
                    AND profile_key IS NOT NULL
            )
    ),
    CHECK (
        artifact_kind NOT IN ('inspection_result', 'segmentation_result')
            OR (
                subject_kind = 'source_file'
                    AND capability_kind IS NULL
                    AND profile_key IS NULL
            )
    ),
    CHECK (
        artifact_kind <> 'projection_snapshot'
            OR (
                subject_kind = 'projection_domain'
                    AND capability_kind IS NULL
                    AND profile_key IS NULL
            )
    ),
    CHECK (
        subject_kind <> 'projection_domain'
            OR subject_id IN ('library_browser', 'navigation')
    )
) STRICT;

CREATE INDEX Artifacts_work_run
    ON Artifacts (work_run_id);

CREATE INDEX Artifacts_subject_created_at
    ON Artifacts (subject_kind, subject_id, created_at DESC);

CREATE INDEX Artifacts_capability_lookup
    ON Artifacts (subject_kind, subject_id, capability_kind, profile_key, artifact_kind, artifact_role);

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
    content_hash_value              TEXT    NOT NULL CHECK (length(trim(content_hash_value)) > 0),
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

CREATE TABLE SourceSegmentSets
(
    source_segment_set_id  INTEGER PRIMARY KEY,
    source_file_id         INTEGER NOT NULL REFERENCES source_files (source_file_id) ON DELETE CASCADE,
    segment_set_kind       TEXT    NOT NULL CHECK (length(trim(segment_set_kind)) > 0),
    basis_fingerprint      TEXT    NOT NULL CHECK (length(trim(basis_fingerprint)) > 0),
    accepted_at            INTEGER NOT NULL,
    accepted_artifact_id   INTEGER NOT NULL REFERENCES Artifacts (artifact_id),
    updated_at             INTEGER NOT NULL,
    CHECK (updated_at >= accepted_at),
    UNIQUE (source_file_id, segment_set_kind)
) STRICT;

CREATE TABLE SourceSegments
(
    source_segment_id      INTEGER PRIMARY KEY,
    source_segment_set_id  INTEGER NOT NULL REFERENCES SourceSegmentSets (source_segment_set_id) ON DELETE CASCADE,
    segment_kind           TEXT    NOT NULL CHECK (length(trim(segment_kind)) > 0),
    ordinal                INTEGER NOT NULL CHECK (ordinal >= 0),
    start_offset_ms        INTEGER NOT NULL CHECK (start_offset_ms >= 0),
    end_offset_ms          INTEGER CHECK (end_offset_ms IS NULL OR end_offset_ms > start_offset_ms),
    display_title          TEXT,
    display_artist         TEXT,
    display_album          TEXT,
    created_at             INTEGER NOT NULL,
    updated_at             INTEGER NOT NULL,
    CHECK (updated_at >= created_at),
    UNIQUE (source_segment_set_id, ordinal)
) STRICT;

CREATE INDEX SourceSegments_segment_set_range
    ON SourceSegments (source_segment_set_id, start_offset_ms);

-- TOMBSTONE: LibraryAssetAttachments is dormant for new attachment identity
-- work. It is replaced by source_file_attachment_links, and the future CUE
-- association plus media-candidate/subtrack slice must delete it when segment
-- promotion is replaced. Current code must not use it for attachment identity.
CREATE TABLE LibraryAssetAttachments
(
    library_asset_attachment_id  INTEGER PRIMARY KEY,
    library_asset_id             INTEGER NOT NULL REFERENCES LibraryAssets (library_asset_id) ON DELETE CASCADE,
    source_segment_id            INTEGER NOT NULL UNIQUE REFERENCES SourceSegments (source_segment_id) ON DELETE CASCADE,
    accepted_at                  INTEGER NOT NULL,
    updated_at                   INTEGER NOT NULL,
    CHECK (updated_at >= accepted_at)
) STRICT;

CREATE INDEX LibraryAssetAttachments_library_asset
    ON LibraryAssetAttachments (library_asset_id);

CREATE TABLE LibraryAssetMetadataCorrections
(
    correction_id        INTEGER PRIMARY KEY,
    library_asset_id     INTEGER NOT NULL REFERENCES LibraryAssets (library_asset_id) ON DELETE CASCADE,
    field_name           TEXT    NOT NULL CHECK (length(trim(field_name)) > 0),
    value_text           TEXT,
    value_int            INTEGER,
    is_null_correction   INTEGER NOT NULL DEFAULT 0 CHECK (is_null_correction IN (0, 1)),
    source_kind          TEXT    NOT NULL CHECK (length(trim(source_kind)) > 0),
    applied_at           INTEGER NOT NULL,
    retracted_at         INTEGER,
    CHECK (retracted_at IS NULL OR retracted_at >= applied_at),
    CHECK (
        (
            is_null_correction = 1
                AND value_text IS NULL
                AND value_int IS NULL
        )
            OR (
            is_null_correction = 0
                AND (
                    (value_text IS NOT NULL AND value_int IS NULL)
                        OR (value_text IS NULL AND value_int IS NOT NULL)
                )
        )
    )
) STRICT;

CREATE UNIQUE INDEX LibraryAssetMetadataCorrections_active_field
    ON LibraryAssetMetadataCorrections (library_asset_id, field_name)
    WHERE retracted_at IS NULL;

CREATE INDEX LibraryAssetMetadataCorrections_item_applied
    ON LibraryAssetMetadataCorrections (library_asset_id, applied_at DESC);

CREATE TABLE LibraryAssetCapabilities
(
    library_asset_id      INTEGER NOT NULL REFERENCES LibraryAssets (library_asset_id) ON DELETE CASCADE,
    capability_kind       TEXT    NOT NULL REFERENCES CapabilitySpecs (capability_kind),
    profile_key           TEXT    NOT NULL CHECK (length(trim(profile_key)) > 0),
    state                 TEXT    NOT NULL
        CHECK (state IN ('missing', 'queued', 'leased', 'ready', 'stale', 'blocked', 'failed')),
    stability_class       TEXT CHECK (
        stability_class IS NULL
            OR stability_class IN ('provisional', 'stable')
    ),
    quality_current       INTEGER CHECK (quality_current IS NULL OR quality_current >= 0),
    basis_fingerprint     TEXT CHECK (basis_fingerprint IS NULL OR length(trim(basis_fingerprint)) > 0),
    selected_artifact_id  INTEGER REFERENCES Artifacts (artifact_id),
    updated_at            INTEGER NOT NULL,
    PRIMARY KEY (library_asset_id, capability_kind, profile_key),
    CHECK (
        (
            state IN ('ready', 'stale')
                AND stability_class IS NOT NULL
                AND basis_fingerprint IS NOT NULL
        )
            OR state NOT IN ('ready', 'stale')
    ),
    CHECK (selected_artifact_id IS NULL OR state IN ('ready', 'stale'))
) STRICT;

CREATE INDEX LibraryAssetCapabilities_state
    ON LibraryAssetCapabilities (state, capability_kind);

-- TOMBSTONE: primaryMedia is dormant as a row-profile/projection shape. It is
-- replaced by the future media-candidate/track-identity projection, and that
-- slice must explicitly activate it as a projection or delete the shape once
-- replacement read surfaces exist. Current code must not treat it as default
-- content, track identity, or a new read/write target.
CREATE TABLE LibraryBrowserRows
(
    library_asset_id           INTEGER PRIMARY KEY REFERENCES LibraryAssets (library_asset_id) ON DELETE CASCADE,
    row_version                INTEGER NOT NULL CHECK (row_version >= 0),
    primary_source_file_id     INTEGER REFERENCES source_files (source_file_id),
    availability_state         TEXT    NOT NULL
        CHECK (availability_state IN ('available', 'unavailable', 'degraded')),
    title                      TEXT,
    artist                     TEXT,
    album                      TEXT,
    duration_ms                INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
    musical_key                TEXT,
    tempo_bpm                  REAL CHECK (tempo_bpm IS NULL OR tempo_bpm >= 0),
    -- Browse-facing summaries derived by projection rebuild logic from
    -- capability/artifact state; they are not canonical ownership.
    waveform_quality_current   INTEGER CHECK (
        waveform_quality_current IS NULL
            OR waveform_quality_current >= 0
    ),
    waveform_quality_target    INTEGER CHECK (
        waveform_quality_target IS NULL
            OR waveform_quality_target >= 0
    ),
    stems_state_summary        TEXT,
    prep_readiness_summary     TEXT    NOT NULL
        CHECK (prep_readiness_summary IN (
            'not_required',
            'ready',
            'preparing',
            'underprepared',
            'blocked',
            'failed'
        )),
    updated_at                 INTEGER NOT NULL
) STRICT;

CREATE INDEX LibraryBrowserRows_availability
    ON LibraryBrowserRows (availability_state, updated_at DESC);

CREATE INDEX LibraryBrowserRows_prep_readiness
    ON LibraryBrowserRows (prep_readiness_summary, updated_at DESC);

CREATE VIRTUAL TABLE LibraryBrowserRows_fts USING fts5(
    title,
    artist,
    album,
    tokenize = 'unicode61 remove_diacritics 1'
);

CREATE TABLE navigation_rows
(
    navigation_row_id         INTEGER PRIMARY KEY,
    stable_key                TEXT    NOT NULL UNIQUE CHECK (length(trim(stable_key)) > 0),
    parent_navigation_row_id  INTEGER REFERENCES navigation_rows (navigation_row_id) ON DELETE CASCADE,
    family                    TEXT CHECK (
        family IS NULL
            OR family IN ('Views', 'Collections', 'Preparation', 'Sources')
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
        CHECK (projection_domain IN ('library_browser', 'navigation')),
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
        CHECK (projection_domain IN ('library_browser', 'navigation')),
    position           INTEGER NOT NULL CHECK (position >= 0),
    updated_at         INTEGER NOT NULL,
    PRIMARY KEY (subscriber_id, projection_domain)
) STRICT;

CREATE TABLE ProjectionRetentionWatermarks
(
    projection_domain                  TEXT PRIMARY KEY
        CHECK (projection_domain IN ('library_browser', 'navigation')),
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

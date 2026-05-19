use rusqlite::{OptionalExtension, params};

use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertSourceLocationInput {
    pub source_location_id: Option<i64>,
    pub source_id: i64,
    pub authority: String,
    pub location_kind: String,
    pub relative_path: String,
    pub display_name: Option<String>,
    pub is_user_visible: bool,
    pub browser_order_ordinal: Option<i64>,
    pub first_created_at: Option<i64>,
    pub changed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteSourceLocationInput {
    pub source_location_id: i64,
}

pub struct SourceLocationsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceLocationsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn upsert_source_location(
        &self,
        input: &UpsertSourceLocationInput,
    ) -> LibrarySqliteResult<i64> {
        validate_source_location_shape(&input.authority, &input.location_kind)?;
        let relative_path = canonicalize_source_location_relative_path(&input.relative_path)?;
        let existing_id = self.resolve_existing_id(input, &relative_path)?;

        if let Some(source_location_id) = existing_id {
            self.tx.execute(
                "UPDATE source_locations
                 SET source_id = ?2,
                     authority = ?3,
                     location_kind = ?4,
                     relative_path = ?5,
                     display_name = ?6,
                     is_user_visible = ?7,
                     updated_at = ?8
                 WHERE source_location_id = ?1",
                params![
                    source_location_id,
                    input.source_id,
                    input.authority,
                    input.location_kind,
                    relative_path,
                    input.display_name,
                    if input.is_user_visible { 1 } else { 0 },
                    input.changed_at,
                ],
            )?;
            self.upsert_source_location_order(
                source_location_id,
                input.source_id,
                input.browser_order_ordinal,
                input.changed_at,
            )?;
            return Ok(source_location_id);
        }

        let created_at = input.first_created_at.unwrap_or(input.changed_at);
        let source_location_id = match input.source_location_id {
            Some(source_location_id) => {
                self.tx.execute(
                    "INSERT INTO source_locations (
                         source_location_id,
                         source_id,
                         authority,
                         location_kind,
                         relative_path,
                         display_name,
                         is_user_visible,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        source_location_id,
                        input.source_id,
                        input.authority,
                        input.location_kind,
                        relative_path,
                        input.display_name,
                        if input.is_user_visible { 1 } else { 0 },
                        created_at,
                        input.changed_at,
                    ],
                )?;
                source_location_id
            }
            None => {
                self.tx.execute(
                    "INSERT INTO source_locations (
                         source_id,
                         authority,
                         location_kind,
                         relative_path,
                         display_name,
                         is_user_visible,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        input.source_id,
                        input.authority,
                        input.location_kind,
                        relative_path,
                        input.display_name,
                        if input.is_user_visible { 1 } else { 0 },
                        created_at,
                        input.changed_at,
                    ],
                )?;
                self.tx.last_insert_rowid()
            }
        };

        self.upsert_source_location_order(
            source_location_id,
            input.source_id,
            input.browser_order_ordinal,
            input.changed_at,
        )?;
        Ok(source_location_id)
    }

    pub fn delete_source_location(
        &self,
        input: &DeleteSourceLocationInput,
    ) -> LibrarySqliteResult<bool> {
        let deleted = self.tx.execute(
            "DELETE FROM source_locations
             WHERE source_location_id = ?1",
            [input.source_location_id],
        )? > 0;
        if deleted {
            self.tx.execute(
                "DELETE FROM browser_user_order
                 WHERE node_domain = 'source_location'
                   AND node_id = ?1",
                [input.source_location_id.to_string()],
            )?;
        }
        Ok(deleted)
    }

    fn resolve_existing_id(
        &self,
        input: &UpsertSourceLocationInput,
        relative_path: &str,
    ) -> LibrarySqliteResult<Option<i64>> {
        if let Some(source_location_id) = input.source_location_id
            && self.tx.query_row(
                "SELECT EXISTS(
                     SELECT 1
                     FROM source_locations
                     WHERE source_location_id = ?1
                 )",
                [source_location_id],
                |row| row.get::<_, i64>(0),
            )? != 0
        {
            return Ok(Some(source_location_id));
        }

        self.tx
            .query_row(
                "SELECT source_location_id
                 FROM source_locations
                 WHERE source_id = ?1
                   AND relative_path = ?2",
                params![input.source_id, relative_path],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    fn upsert_source_location_order(
        &self,
        source_location_id: i64,
        source_id: i64,
        ordinal: Option<i64>,
        changed_at: i64,
    ) -> LibrarySqliteResult<()> {
        if let Some(ordinal) = ordinal {
            self.tx.execute(
                "INSERT INTO browser_user_order (
                     node_domain,
                     node_id,
                     parent_scope,
                     ordinal,
                     created_at,
                     updated_at
                 )
                 VALUES ('source_location', ?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT(node_domain, node_id)
                 WHERE node_domain = 'source_location'
                   AND parent_scope IS NOT NULL
                 DO UPDATE
                 SET parent_scope = excluded.parent_scope,
                     ordinal = excluded.ordinal,
                     updated_at = excluded.updated_at",
                params![
                    source_location_id.to_string(),
                    source_id.to_string(),
                    ordinal,
                    changed_at,
                ],
            )?;
        }
        Ok(())
    }
}

pub fn canonicalize_source_location_relative_path(raw: &str) -> LibrarySqliteResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(invalid_source_location_path(raw, "path is empty"));
    }
    if trimmed.contains("://") {
        return Err(invalid_source_location_path(
            raw,
            "path contains a URI scheme",
        ));
    }
    if has_drive_prefix(trimmed) {
        return Err(invalid_source_location_path(
            raw,
            "path contains a drive prefix",
        ));
    }

    let slash_normalized = trimmed.replace('\\', "/");
    if slash_normalized.starts_with('/') {
        return Err(invalid_source_location_path(raw, "path is absolute"));
    }

    let mut segments = Vec::new();
    for segment in slash_normalized.split('/') {
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." {
            return Err(invalid_source_location_path(
                raw,
                "path contains a traversal segment",
            ));
        }
        segments.push(segment);
    }

    if segments.is_empty() {
        return Err(invalid_source_location_path(
            raw,
            "path is empty after normalization",
        ));
    }

    Ok(segments.join("/"))
}

fn validate_source_location_shape(authority: &str, location_kind: &str) -> LibrarySqliteResult<()> {
    match (authority, location_kind) {
        ("device", "observed_path") | ("user", "registered_subpath") => Ok(()),
        _ => Err(LibrarySqliteError::WriteInvariant(format!(
            "invalid source_location authority/kind pair: authority={authority:?}, location_kind={location_kind:?}"
        ))),
    }
}

fn has_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn invalid_source_location_path(raw: &str, reason: &str) -> LibrarySqliteError {
    LibrarySqliteError::WriteInvariant(format!(
        "invalid source_locations.relative_path {raw:?}: {reason}"
    ))
}

#[cfg(test)]
mod tests {
    use super::canonicalize_source_location_relative_path;

    #[test]
    fn canonicalize_source_location_relative_path_normalizes_lawful_paths() {
        assert_eq!(
            canonicalize_source_location_relative_path(r" Music\\DJ Pool// ")
                .expect("canonical path"),
            "Music/DJ Pool"
        );
        assert_eq!(
            canonicalize_source_location_relative_path("Music///Tracks/").expect("canonical path"),
            "Music/Tracks"
        );
    }

    #[test]
    fn canonicalize_source_location_relative_path_rejects_unlawful_paths() {
        for path in [
            "",
            "   ",
            "/Music",
            r"\\server\\Music",
            "C:/Music",
            "C:Music",
            ".",
            "./Music",
            "Music/./Tracks",
            "..",
            "../Music",
            "Music/../Tracks",
            "file://Music",
        ] {
            canonicalize_source_location_relative_path(path).expect_err("path should be rejected");
        }
    }
}

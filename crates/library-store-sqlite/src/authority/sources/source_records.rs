use rusqlite::{OptionalExtension, params};

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;

pub fn format_source_identity_key(identity_kind: &str, identity_value: &str) -> String {
    format!("{identity_kind}:{identity_value}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertSourceInput {
    pub source_id: Option<i64>,
    pub source_class: String,
    pub authority: String,
    pub identity_kind: String,
    pub identity_value: String,
    pub display_name: String,
    pub medium_label: Option<String>,
    pub is_user_visible: bool,
    pub browser_order_ordinal: Option<i64>,
    pub changed_at: i64,
}

pub struct SourcesAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourcesAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn upsert_source(&self, input: &UpsertSourceInput) -> LibrarySqliteResult<i64> {
        let existing_id = self.resolve_existing_id(input)?;

        if let Some(source_id) = existing_id {
            self.tx.execute(
                "UPDATE sources
                 SET source_class = ?2,
                     authority = ?3,
                     identity_key = ?4,
                     display_name = ?5,
                     medium_label = ?6,
                     is_user_visible = ?7,
                     updated_at = ?8
                 WHERE source_id = ?1",
                params![
                    source_id,
                    input.source_class,
                    input.authority,
                    format_source_identity_key(&input.identity_kind, &input.identity_value),
                    input.display_name,
                    input.medium_label,
                    if input.is_user_visible { 1 } else { 0 },
                    input.changed_at,
                ],
            )?;
            self.upsert_source_order(source_id, input.browser_order_ordinal, input.changed_at)?;
            return Ok(source_id);
        }

        let identity_key = format_source_identity_key(&input.identity_kind, &input.identity_value);
        let source_id = match input.source_id {
            Some(source_id) => {
                self.tx.execute(
                    "INSERT INTO sources (
                         source_id,
                         source_class,
                         authority,
                         identity_key,
                         display_name,
                         medium_label,
                         is_user_visible,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                    params![
                        source_id,
                        input.source_class,
                        input.authority,
                        identity_key,
                        input.display_name,
                        input.medium_label,
                        if input.is_user_visible { 1 } else { 0 },
                        input.changed_at,
                    ],
                )?;
                source_id
            }
            None => {
                self.tx.execute(
                    "INSERT INTO sources (
                         source_class,
                         authority,
                         identity_key,
                         display_name,
                         medium_label,
                         is_user_visible,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                    params![
                        input.source_class,
                        input.authority,
                        identity_key,
                        input.display_name,
                        input.medium_label,
                        if input.is_user_visible { 1 } else { 0 },
                        input.changed_at,
                    ],
                )?;
                self.tx.last_insert_rowid()
            }
        };
        self.upsert_source_order(source_id, input.browser_order_ordinal, input.changed_at)?;
        Ok(source_id)
    }

    fn upsert_source_order(
        &self,
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
                 VALUES ('source', ?1, NULL, ?2, ?3, ?3)
                 ON CONFLICT(node_domain, node_id)
                 WHERE node_domain = 'source'
                   AND parent_scope IS NULL
                 DO UPDATE
                 SET ordinal = excluded.ordinal,
                     updated_at = excluded.updated_at",
                params![source_id.to_string(), ordinal, changed_at],
            )?;
        }
        Ok(())
    }

    fn resolve_existing_id(&self, input: &UpsertSourceInput) -> LibrarySqliteResult<Option<i64>> {
        if let Some(source_id) = input.source_id
            && self.tx.query_row(
                "SELECT EXISTS(
                         SELECT 1
                         FROM sources
                         WHERE source_id = ?1
                     )",
                [source_id],
                |row| row.get::<_, i64>(0),
            )? != 0
        {
            return Ok(Some(source_id));
        }

        self.tx
            .query_row(
                "SELECT source_id
                 FROM sources
                 WHERE identity_key = ?1",
                [format_source_identity_key(
                    &input.identity_kind,
                    &input.identity_value,
                )],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
}

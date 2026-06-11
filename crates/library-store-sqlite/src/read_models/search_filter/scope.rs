use rusqlite::types::Value;

use super::types::{StoreSearchRecursion, StoreSearchScope};
use crate::LibrarySqliteResult;

pub(crate) fn append_scope_predicate(
    predicates: &mut Vec<String>,
    values: &mut Vec<Value>,
    scope: &StoreSearchScope,
    recursion: StoreSearchRecursion,
) -> LibrarySqliteResult<()> {
    match scope {
        StoreSearchScope::Library => {}
        StoreSearchScope::Source { source_id } => {
            predicates.push("source_id = ?".to_string());
            values.push(Value::Integer(*source_id));
        }
        StoreSearchScope::SourceLocation { source_location_id } => {
            if recursion == StoreSearchRecursion::Recursive {
                predicates.push(
                    "(source_location_id = ? OR EXISTS (
                        SELECT 1 FROM source_locations location
                        WHERE location.source_location_id = ?
                          AND location.is_user_visible = 1
                          AND search_rows.source_id = location.source_id
                          AND (
                              search_rows.relative_path = location.relative_path
                              OR search_rows.relative_path LIKE location.relative_path || '/%'
                          )
                    ))"
                    .to_string(),
                );
                values.push(Value::Integer(*source_location_id));
                values.push(Value::Integer(*source_location_id));
            } else {
                predicates.push(
                    "(source_location_id = ? OR parent_source_directory_id = (
                        SELECT directory.source_directory_id
                        FROM source_locations location
                        JOIN source_directories directory
                          ON directory.source_id = location.source_id
                         AND directory.relative_path = location.relative_path
                        WHERE location.source_location_id = ?
                          AND location.is_user_visible = 1
                    ))"
                    .to_string(),
                );
                values.push(Value::Integer(*source_location_id));
                values.push(Value::Integer(*source_location_id));
            }
        }
        StoreSearchScope::Directory {
            source_id,
            source_directory_id,
        } => {
            if recursion == StoreSearchRecursion::Recursive {
                predicates.push(
                    "(source_id = ? AND (
                        source_directory_id = ?
                        OR EXISTS (
                            SELECT 1 FROM source_directories scoped
                            WHERE scoped.source_directory_id = ?
                              AND scoped.source_id = ?
                              AND search_rows.relative_path IS NOT NULL
                              AND (
                                  search_rows.relative_path = scoped.relative_path
                                  OR search_rows.relative_path LIKE scoped.relative_path || '/%'
                              )
                        )
                    ))"
                    .to_string(),
                );
                values.push(Value::Integer(*source_id));
                values.push(Value::Integer(*source_directory_id));
                values.push(Value::Integer(*source_directory_id));
                values.push(Value::Integer(*source_id));
            } else {
                predicates.push(
                    "(source_id = ? AND EXISTS (
                         SELECT 1
                         FROM source_directories scoped
                         WHERE scoped.source_id = ?
                           AND scoped.source_directory_id = ?
                     ) AND (source_directory_id = ? OR parent_source_directory_id = ?))"
                        .to_string(),
                );
                values.push(Value::Integer(*source_id));
                values.push(Value::Integer(*source_id));
                values.push(Value::Integer(*source_directory_id));
                values.push(Value::Integer(*source_directory_id));
                values.push(Value::Integer(*source_directory_id));
            }
        }
    }
    Ok(())
}

use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::ProjectionDomain;

use super::{SqliteDurableStore, bootstrap::open_connection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MaintainedReadModelScope {
    NavigationRows,
    LibraryBrowser,
}

impl MaintainedReadModelScope {
    const ALL: [Self; 2] = [Self::NavigationRows, Self::LibraryBrowser];

    const fn projection_domain(self) -> ProjectionDomain {
        match self {
            Self::NavigationRows => ProjectionDomain::Navigation,
            Self::LibraryBrowser => ProjectionDomain::LibraryBrowser,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaintainedReadModelRevision {
    pub scope: MaintainedReadModelScope,
    pub revision: u64,
}

impl SqliteDurableStore {
    pub fn read_maintained_read_model_revisions(
        &self,
    ) -> LibrarySqliteResult<Vec<MaintainedReadModelRevision>> {
        let connection = open_connection(&self.path)?;
        let mut revisions = Vec::with_capacity(MaintainedReadModelScope::ALL.len());

        for scope in MaintainedReadModelScope::ALL {
            let domain = scope.projection_domain();
            let revision = connection.query_row(
                "SELECT MAX(change_sequence)
                 FROM ProjectionChangeLog
                 WHERE projection_domain = ?1",
                [domain.as_str()],
                |row| row.get::<_, Option<i64>>(0),
            )?;
            let revision = revision.unwrap_or(0);
            let revision = u64::try_from(revision).map_err(|error| {
                LibrarySqliteError::MalformedSchemaState(format!(
                    "maintained read model revision for {} is invalid: {error}",
                    domain.as_str()
                ))
            })?;

            revisions.push(MaintainedReadModelRevision { scope, revision });
        }

        Ok(revisions)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionDomain {
    LibraryBrowser,
    Navigation,
}

impl ProjectionDomain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LibraryBrowser => "library_browser",
            Self::Navigation => "navigation",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "library_browser" => Some(Self::LibraryBrowser),
            "navigation" => Some(Self::Navigation),
            _ => None,
        }
    }
}

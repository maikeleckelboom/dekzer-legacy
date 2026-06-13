#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionDomain {
    Navigation,
}

impl ProjectionDomain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Navigation => "navigation",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "navigation" => Some(Self::Navigation),
            _ => None,
        }
    }
}

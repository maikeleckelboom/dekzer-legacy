macro_rules! define_id_type {
    ($name:ident) => {
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(i64);

        impl $name {
            pub fn new(value: i64) -> Option<Self> {
                if value > 0 { Some(Self(value)) } else { None }
            }

            pub const fn get(self) -> i64 {
                self.0
            }
        }
    };
}

define_id_type!(SourceId);
define_id_type!(SourceLocationId);
define_id_type!(SourceDirectoryId);
define_id_type!(SourceFileId);
define_id_type!(SourceSegmentSetId);
define_id_type!(SourceSegmentId);
define_id_type!(LibraryAssetId);
define_id_type!(PlaylistId);
define_id_type!(WorkItemId);
define_id_type!(WorkRunId);
define_id_type!(ArtifactId);
define_id_type!(PrepPolicyId);
define_id_type!(ProjectionSubscriberId);

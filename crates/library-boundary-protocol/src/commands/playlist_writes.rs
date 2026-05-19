#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CreatePlaylistRequest {
    pub display_name: String,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CreatePlaylistReply {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playlist_id: i64,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RenamePlaylistRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playlist_id: i64,
    pub display_name: String,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RenamePlaylistReply {
    pub renamed: bool,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DeletePlaylistRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playlist_id: i64,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DeletePlaylistReply {
    pub deleted: bool,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppendLibraryAssetToPlaylistRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playlist_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub library_asset_id: i64,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppendLibraryAssetToPlaylistReply {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playlist_entry_id: i64,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RemoveLibraryAssetFromPlaylistRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playlist_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub library_asset_id: i64,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RemoveLibraryAssetFromPlaylistReply {
    pub removed: bool,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MovePlaylistEntryRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playlist_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playlist_entry_id: i64,
    pub new_position: i64,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MovePlaylistEntryReply {
    pub moved: bool,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum PlaylistWriteCommand {
    CreatePlaylist(CreatePlaylistRequest),
    RenamePlaylist(RenamePlaylistRequest),
    DeletePlaylist(DeletePlaylistRequest),
    AppendLibraryAssetToPlaylist(AppendLibraryAssetToPlaylistRequest),
    RemoveLibraryAssetFromPlaylist(RemoveLibraryAssetFromPlaylistRequest),
    MovePlaylistEntry(MovePlaylistEntryRequest),
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum PlaylistWriteReply {
    CreatePlaylist(CreatePlaylistReply),
    RenamePlaylist(RenamePlaylistReply),
    DeletePlaylist(DeletePlaylistReply),
    AppendLibraryAssetToPlaylist(AppendLibraryAssetToPlaylistReply),
    RemoveLibraryAssetFromPlaylist(RemoveLibraryAssetFromPlaylistReply),
    MovePlaylistEntry(MovePlaylistEntryReply),
}

#[cfg(test)]
mod tests {
    use super::{
        AppendLibraryAssetToPlaylistReply, AppendLibraryAssetToPlaylistRequest,
        CreatePlaylistReply, CreatePlaylistRequest, DeletePlaylistReply, DeletePlaylistRequest,
        MovePlaylistEntryReply, MovePlaylistEntryRequest, PlaylistWriteCommand, PlaylistWriteReply,
        RemoveLibraryAssetFromPlaylistReply, RemoveLibraryAssetFromPlaylistRequest,
        RenamePlaylistReply, RenamePlaylistRequest,
    };
    use serde_json::json;

    #[test]
    fn playlist_write_commands_carry_typed_mutation_inputs() {
        let commands = [
            PlaylistWriteCommand::CreatePlaylist(CreatePlaylistRequest {
                display_name: "Set".to_string(),
            }),
            PlaylistWriteCommand::RenamePlaylist(RenamePlaylistRequest {
                playlist_id: 1,
                display_name: "Renamed Set".to_string(),
            }),
            PlaylistWriteCommand::DeletePlaylist(DeletePlaylistRequest { playlist_id: 1 }),
            PlaylistWriteCommand::AppendLibraryAssetToPlaylist(
                AppendLibraryAssetToPlaylistRequest {
                    playlist_id: 1,
                    library_asset_id: 2,
                },
            ),
            PlaylistWriteCommand::RemoveLibraryAssetFromPlaylist(
                RemoveLibraryAssetFromPlaylistRequest {
                    playlist_id: 1,
                    library_asset_id: 2,
                },
            ),
            PlaylistWriteCommand::MovePlaylistEntry(MovePlaylistEntryRequest {
                playlist_id: 1,
                playlist_entry_id: 3,
                new_position: 0,
            }),
        ];

        assert!(matches!(
            &commands[0],
            PlaylistWriteCommand::CreatePlaylist(CreatePlaylistRequest { display_name })
                if display_name == "Set"
        ));
        assert!(matches!(
            &commands[1],
            PlaylistWriteCommand::RenamePlaylist(RenamePlaylistRequest { playlist_id: 1, .. })
        ));
        assert!(matches!(
            &commands[2],
            PlaylistWriteCommand::DeletePlaylist(DeletePlaylistRequest { playlist_id: 1 })
        ));
        assert!(matches!(
            &commands[3],
            PlaylistWriteCommand::AppendLibraryAssetToPlaylist(
                AppendLibraryAssetToPlaylistRequest {
                    playlist_id: 1,
                    library_asset_id: 2,
                },
            )
        ));
        assert!(matches!(
            &commands[4],
            PlaylistWriteCommand::RemoveLibraryAssetFromPlaylist(
                RemoveLibraryAssetFromPlaylistRequest {
                    playlist_id: 1,
                    library_asset_id: 2,
                },
            )
        ));
        assert!(matches!(
            &commands[5],
            PlaylistWriteCommand::MovePlaylistEntry(MovePlaylistEntryRequest {
                playlist_id: 1,
                playlist_entry_id: 3,
                new_position: 0,
            })
        ));
    }

    #[test]
    fn playlist_write_replies_carry_typed_mutation_results() {
        let replies = [
            PlaylistWriteReply::CreatePlaylist(CreatePlaylistReply { playlist_id: 1 }),
            PlaylistWriteReply::RenamePlaylist(RenamePlaylistReply { renamed: true }),
            PlaylistWriteReply::DeletePlaylist(DeletePlaylistReply { deleted: true }),
            PlaylistWriteReply::AppendLibraryAssetToPlaylist(AppendLibraryAssetToPlaylistReply {
                playlist_entry_id: 2,
            }),
            PlaylistWriteReply::RemoveLibraryAssetFromPlaylist(
                RemoveLibraryAssetFromPlaylistReply { removed: true },
            ),
            PlaylistWriteReply::MovePlaylistEntry(MovePlaylistEntryReply { moved: true }),
        ];

        assert!(matches!(
            &replies[0],
            PlaylistWriteReply::CreatePlaylist(CreatePlaylistReply { playlist_id: 1 })
        ));
        assert!(matches!(
            &replies[1],
            PlaylistWriteReply::RenamePlaylist(RenamePlaylistReply { renamed: true })
        ));
        assert!(matches!(
            &replies[2],
            PlaylistWriteReply::DeletePlaylist(DeletePlaylistReply { deleted: true })
        ));
        assert!(matches!(
            &replies[3],
            PlaylistWriteReply::AppendLibraryAssetToPlaylist(AppendLibraryAssetToPlaylistReply {
                playlist_entry_id: 2,
            },)
        ));
        assert!(matches!(
            &replies[4],
            PlaylistWriteReply::RemoveLibraryAssetFromPlaylist(
                RemoveLibraryAssetFromPlaylistReply { removed: true },
            )
        ));
        assert!(matches!(
            &replies[5],
            PlaylistWriteReply::MovePlaylistEntry(MovePlaylistEntryReply { moved: true })
        ));
    }

    #[test]
    fn playlist_write_command_serializes_as_tagged_camel_case_payload() {
        let command = PlaylistWriteCommand::AppendLibraryAssetToPlaylist(
            AppendLibraryAssetToPlaylistRequest {
                playlist_id: 1,
                library_asset_id: 2,
            },
        );

        let json = serde_json::to_value(&command).expect("serialize playlist write command");
        assert_eq!(
            json,
            json!({
                "type": "appendLibraryAssetToPlaylist",
                "payload": {
                    "playlistId": "1",
                    "libraryAssetId": "2"
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<PlaylistWriteCommand>(json)
                .expect("deserialize playlist write command"),
            command
        );
    }

    #[test]
    fn playlist_write_ids_serialize_as_strings() {
        let command = PlaylistWriteCommand::MovePlaylistEntry(MovePlaylistEntryRequest {
            playlist_id: 1,
            playlist_entry_id: 3,
            new_position: 0,
        });
        let reply =
            PlaylistWriteReply::AppendLibraryAssetToPlaylist(AppendLibraryAssetToPlaylistReply {
                playlist_entry_id: 2,
            });

        assert_eq!(
            serde_json::to_value(&command).expect("serialize command"),
            json!({
                "type": "movePlaylistEntry",
                "payload": {
                    "playlistId": "1",
                    "playlistEntryId": "3",
                    "newPosition": 0
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&reply).expect("serialize reply"),
            json!({
                "type": "appendLibraryAssetToPlaylist",
                "payload": {
                    "playlistEntryId": "2"
                }
            })
        );
    }
}

use std::path::{Path, PathBuf};

use crate::commands::asset::ImportAssetCmd;
use crate::editor_global::{push_command, push_toast};
use engine_core::assets::AssetKind;

/// Opens the shared asset importer, optionally constraining the asset kind and destination.
pub(crate) fn import_asset_from_dialog(
    expected_kind: Option<AssetKind>,
    destination_directory: Option<PathBuf>,
) {
    let supported_extensions = expected_kind.map_or_else(
        || {
            AssetKind::ALL
                .into_iter()
                .flat_map(AssetKind::supported_extensions)
                .copied()
                .collect::<Vec<_>>()
        },
        |kind| kind.supported_extensions().to_vec(),
    );

    let Some(path) = rfd::FileDialog::new()
        .add_filter("Supported game assets", &supported_extensions)
        .pick_file()
    else {
        return;
    };

    match asset_kind_for_import(expected_kind, &path) {
        Ok(kind) => {
            let command = if let Some(destination) = destination_directory {
                ImportAssetCmd::new_in_directory(kind, path, destination)
            } else {
                ImportAssetCmd::new(kind, path)
            };
            push_command(Box::new(command));
        }
        Err(AssetImportError::KindMismatch) => {
            push_toast("Selected file does not match this resource folder.", 3.0);
        }
        Err(AssetImportError::Unsupported) => {
            push_toast("Unsupported resource type.", 3.0);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssetImportError {
    KindMismatch,
    Unsupported,
}

fn asset_kind_for_import(
    expected_kind: Option<AssetKind>,
    path: &Path,
) -> Result<AssetKind, AssetImportError> {
    let kind = AssetKind::from_path(path).ok_or(AssetImportError::Unsupported)?;
    if expected_kind.is_some_and(|expected| expected != kind) {
        return Err(AssetImportError::KindMismatch);
    }
    Ok(kind)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use engine_core::assets::AssetKind;

    use super::{asset_kind_for_import, AssetImportError};

    #[test]
    fn importer_accepts_the_requested_font_kind() {
        assert_eq!(
            asset_kind_for_import(Some(AssetKind::Font), Path::new("fonts/game.ttf")),
            Ok(AssetKind::Font)
        );
    }

    #[test]
    fn importer_rejects_a_supported_asset_of_the_wrong_kind() {
        assert_eq!(
            asset_kind_for_import(Some(AssetKind::Font), Path::new("sprites/player.png")),
            Err(AssetImportError::KindMismatch)
        );
    }

    #[test]
    fn importer_rejects_unsupported_extensions() {
        assert_eq!(
            asset_kind_for_import(None, Path::new("fonts/license.txt")),
            Err(AssetImportError::Unsupported)
        );
    }
}

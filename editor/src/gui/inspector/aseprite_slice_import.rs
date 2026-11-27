#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AsepriteSliceImportToast {
    Success,
    Warning,
    Failure,
}

pub(super) fn format_aseprite_slice_import_toast(
    imported_count: usize,
    warning_count: usize,
    failure_count: usize,
    imported_item_label: &str,
    failure_message: &str,
    no_results_message: &str,
) -> (AsepriteSliceImportToast, String, f32) {
    if imported_count > 0 {
        if warning_count > 0 || failure_count > 0 {
            return (
                AsepriteSliceImportToast::Warning,
                format!(
                    "Imported {imported_count} {imported_item_label}, {warning_count} warning(s), {failure_count} failure(s)"
                ),
                3.0,
            );
        }

        return (
            AsepriteSliceImportToast::Success,
            format!("Imported {imported_count} {imported_item_label}"),
            2.0,
        );
    }

    if failure_count > 0 {
        return (
            AsepriteSliceImportToast::Failure,
            format!("{failure_message} for {failure_count} clip(s)"),
            3.0,
        );
    }

    (
        AsepriteSliceImportToast::Warning,
        no_results_message.to_string(),
        3.0,
    )
}

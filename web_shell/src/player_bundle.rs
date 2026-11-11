use std::collections::HashSet;

use engine_core::storage::{ResourceKey, web_game_bundle::WebGameBundle};

const MAX_IN_FLIGHT: usize = 4;

/// Parses the versioned authored resource manifest before requesting any game files.
pub(crate) fn parse_manifest(bytes: &[u8]) -> Result<WebGameBundle, String> {
    let source = std::str::from_utf8(bytes)
        .map_err(|error| format!("game manifest is not UTF-8: {error}"))?;
    let bundle: WebGameBundle = ron::from_str(source)
        .map_err(|error| format!("invalid web game manifest: {error}"))?;
    bundle.validate().map_err(|error| format!("invalid web game bundle: {error}"))?;
    Ok(bundle)
}

/// Resolves a validated resource path relative to the selected manifest URL.
pub(crate) fn resource_url(manifest_url: &str, relative: &ResourceKey) -> Result<String, String> {
    let (base, _) = manifest_url.rsplit_once('/')
        .ok_or_else(|| format!("invalid game manifest URL: {manifest_url}"))?;
    let mut url = format!("{base}/");
    for byte in relative.as_str().bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~') {
            url.push(char::from(byte));
        } else {
            url.push_str(&format!("%{byte:02X}"));
        }
    }
    Ok(url)
}

/// Tracks bounded, out-of-order browser downloads without making requests itself.
pub(crate) struct DownloadPlan {
    lengths: Vec<usize>,
    next_index: usize,
    in_flight: HashSet<usize>,
    completed_files: usize,
    loaded_bytes: usize,
    total_bytes: usize,
    failed: bool,
}

#[derive(Debug)]
pub(crate) struct DownloadProgress {
    pub completed_files: usize,
    pub loaded_bytes: usize,
    pub total_bytes: usize,
    pub total_files: usize,
}

impl DownloadProgress {
    pub(crate) fn is_complete(&self) -> bool {
        self.completed_files == self.total_files
    }
}

impl DownloadPlan {
    pub(crate) fn new(lengths: Vec<usize>) -> Self {
        let total_bytes: usize = lengths.iter().sum();
        Self {
            lengths, next_index: 0, in_flight: HashSet::new(),
            completed_files: 0, loaded_bytes: 0, total_bytes, failed: false,
        }
    }

    pub(crate) fn next(&mut self) -> Option<usize> {
        if self.failed || self.in_flight.len() >= MAX_IN_FLIGHT || self.next_index >= self.lengths.len() {
            return None;
        }
        let index = self.next_index;
        self.next_index += 1;
        self.in_flight.insert(index);
        Some(index)
    }

    pub(crate) fn fail(&mut self) {
        self.failed = true;
    }

    pub(crate) fn is_failed(&self) -> bool {
        self.failed
    }

    pub(crate) fn finish(&mut self, index: usize, fetched_bytes: usize) -> Result<DownloadProgress, String> {
        if self.failed {
            return Err("game resource download already failed".into());
        }
        if !self.in_flight.remove(&index) {
            self.fail();
            return Err(format!("unrequested game resource index: {index}"));
        }
        let expected = self.lengths[index];
        if fetched_bytes != expected {
            self.fail();
            return Err(format!("expected {expected} bytes, fetched {fetched_bytes}"));
        }
        self.completed_files += 1;
        self.loaded_bytes += fetched_bytes;
        Ok(DownloadProgress {
            completed_files: self.completed_files,
            loaded_bytes: self.loaded_bytes,
            total_bytes: self.total_bytes,
            total_files: self.lengths.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DownloadPlan, parse_manifest, resource_url};
    use engine_core::constants::paths;
    use engine_core::storage::web_game_bundle::{WebGameBundle, WEB_GAME_BUNDLE_VERSION};
    use engine_core::storage::{ResourceEntry, ResourceKey, ResourceManifest};

    fn bundle() -> WebGameBundle {
        WebGameBundle {
            format_version: WEB_GAME_BUNDLE_VERSION,
            resources: ResourceManifest {
                files: vec![ResourceEntry { path: ResourceKey::from_wire(paths::GAME_RON).unwrap(), byte_len: 1 }],
            },
        }
    }

    #[test]
    fn player_manifest_accepts_valid_versioned_bundle() {
        let bytes = ron::to_string(&bundle()).unwrap().into_bytes();
        assert_eq!(parse_manifest(&bytes).unwrap(), bundle());
    }

    #[test]
    fn player_manifest_rejects_unsupported_versions_before_download() {
        let mut bundle = bundle();
        bundle.format_version += 1;
        let bytes = ron::to_string(&bundle).unwrap().into_bytes();
        let error = parse_manifest(&bytes).unwrap_err();
        assert!(error.contains("unsupported web game bundle version"), "{error}");
    }

    #[test]
    fn player_asset_url_keeps_bundle_base_and_escapes_resource_bytes() {
        let url = resource_url(
            "https://example.test/site/games/Another/manifest.ron",
            &ResourceKey::from_wire("assets/sprite art/é.png").unwrap(),
        ).unwrap();
        assert_eq!(url, "https://example.test/site/games/Another/assets/sprite%20art/%C3%A9.png");
    }

    #[test]
    fn player_manifest_rejects_more_than_1024_declared_files() {
        let mut large = bundle();
        let mut files = (0..1024).map(|index| ResourceEntry {
            path: ResourceKey::from_wire(&format!("assets/file{index:04}.bin")).unwrap(), byte_len: 1,
        }).collect::<Vec<_>>();
        files.extend(large.resources.files);
        large.resources.files = files;
        let error = parse_manifest(ron::to_string(&large).unwrap().as_bytes()).unwrap_err();
        assert!(error.contains("1024"), "{error}");
    }

    #[test]
    fn player_manifest_rejects_more_than_32_mib_of_declared_game_bytes() {
        let mut large = bundle();
        large.resources.files[0].byte_len = 32 * 1024 * 1024 + 1;
        let error = parse_manifest(ron::to_string(&large).unwrap().as_bytes()).unwrap_err();
        assert!(error.contains("32 MiB"), "{error}");
    }

    #[test]
    fn player_download_plan_keeps_at_most_four_requests_in_flight() {
        let mut plan = DownloadPlan::new(vec![3, 4, 5, 6, 7, 8]);
        assert_eq!((0..4).map(|_| plan.next()).collect::<Vec<_>>(),
                   vec![Some(0), Some(1), Some(2), Some(3)]);
        assert_eq!(plan.next(), None);
        let progress = plan.finish(2, 5).unwrap();
        assert_eq!(progress.loaded_bytes, 5);
        assert_eq!(progress.completed_files, 1);
        assert_eq!(plan.next(), Some(4));
        assert_eq!(plan.next(), None);
        for (index, bytes) in [(0, 3), (1, 4), (3, 6), (4, 7)] {
            plan.finish(index, bytes).unwrap();
        }
        assert_eq!(plan.next(), Some(5));
        let done = plan.finish(5, 8).unwrap();
        assert!(done.is_complete());
        assert_eq!(done.loaded_bytes, done.total_bytes);
    }

    #[test]
    fn player_download_plan_rejects_size_mismatch_without_finishing() {
        let mut plan = DownloadPlan::new(vec![3, 4]);
        assert_eq!(plan.next(), Some(0));
        assert_eq!(plan.next(), Some(1));
        let error = plan.finish(0, 2).unwrap_err();
        assert!(error.contains("expected 3 bytes"), "{error}");
        assert!(plan.is_failed());
        assert_eq!(plan.next(), None);
        assert!(plan.finish(1, 4).is_err());
    }
}

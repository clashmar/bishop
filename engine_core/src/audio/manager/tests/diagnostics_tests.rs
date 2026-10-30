use super::{seeded_manager, AudioDiagnosticsEntry, Frames};

#[test]
fn diagnostics_snapshot_includes_cached_only_entries() {
    let manager = seeded_manager();

    let snapshot = manager.diagnostics_snapshot();

    assert_eq!(snapshot.cached_sound_count, 3);
    assert_eq!(snapshot.loading_sound_count, 0);
    assert_eq!(snapshot.ref_count_entry_count, 0);
    assert_eq!(snapshot.entries.len(), 3);
    assert_eq!(
        snapshot.entries,
        vec![
            AudioDiagnosticsEntry {
                id: "music/intro".to_string(),
                cached: true,
                loading: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "music/next".to_string(),
                cached: true,
                loading: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "preview/click".to_string(),
                cached: true,
                loading: false,
                ref_count: 0,
            },
        ]
    );
}

#[test]
fn diagnostics_snapshot_includes_ref_counted_entries() {
    let mut manager = seeded_manager();
    manager.ref_counts.insert("ref/only".to_string(), 2);
    manager.ref_counts.insert("shared".to_string(), 1);
    manager.sound_cache.insert(
        "shared".to_string(),
        Frames::from_slice(44_100, &[[0.0, 0.0]]),
    );

    let snapshot = manager.diagnostics_snapshot();

    assert_eq!(snapshot.cached_sound_count, 4);
    assert_eq!(snapshot.loading_sound_count, 0);
    assert_eq!(snapshot.ref_count_entry_count, 2);
    assert_eq!(snapshot.entries.len(), 5);
    assert_eq!(
        snapshot.entries,
        vec![
            AudioDiagnosticsEntry {
                id: "music/intro".to_string(),
                cached: true,
                loading: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "music/next".to_string(),
                cached: true,
                loading: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "preview/click".to_string(),
                cached: true,
                loading: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "ref/only".to_string(),
                cached: false,
                loading: false,
                ref_count: 2,
            },
            AudioDiagnosticsEntry {
                id: "shared".to_string(),
                cached: true,
                loading: false,
                ref_count: 1,
            },
        ]
    );
}

#[test]
fn diagnostics_snapshot_entries_are_sorted_by_sound_id() {
    let mut manager = seeded_manager();
    manager.sound_cache.clear();
    manager.sound_cache.insert(
        "zeta".to_string(),
        Frames::from_slice(44_100, &[[0.0, 0.0]]),
    );
    manager.sound_cache.insert(
        "alpha".to_string(),
        Frames::from_slice(44_100, &[[0.0, 0.0]]),
    );
    manager.ref_counts.insert("middle".to_string(), 1);
    manager.ref_counts.insert("beta".to_string(), 1);

    let snapshot = manager.diagnostics_snapshot();

    let ids: Vec<String> = snapshot.entries.into_iter().map(|entry| entry.id).collect();
    assert_eq!(
        ids,
        vec![
            "alpha".to_string(),
            "beta".to_string(),
            "middle".to_string(),
            "zeta".to_string(),
        ]
    );
}

#[test]
fn release_claimed_sound_decrements_and_evicts_unrefd_audio() {
    let mut manager = seeded_manager();
    manager.claim_sound("music/intro");
    assert_eq!(manager.diagnostics_snapshot().ref_count_entry_count, 1);

    manager.release_claimed_sound("music/intro");
    let snapshot = manager.diagnostics_snapshot();
    assert_eq!(snapshot.ref_count_entry_count, 0);
    assert_eq!(snapshot.cached_sound_count, 2);
}

#[test]
fn diagnostics_snapshot_includes_loading_entries() {
    let mut manager = seeded_manager();
    manager.sound_cache.remove("music/next");
    manager.queue_sound_load("music/next");

    let snapshot = manager.diagnostics_snapshot();

    assert_eq!(snapshot.cached_sound_count, 2);
    assert_eq!(snapshot.loading_sound_count, 1);
    assert_eq!(snapshot.ref_count_entry_count, 0);
    assert_eq!(
        snapshot.entries,
        vec![
            AudioDiagnosticsEntry {
                id: "music/intro".to_string(),
                cached: true,
                loading: false,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "music/next".to_string(),
                cached: false,
                loading: true,
                ref_count: 0,
            },
            AudioDiagnosticsEntry {
                id: "preview/click".to_string(),
                cached: true,
                loading: false,
                ref_count: 0,
            },
        ]
    );
}


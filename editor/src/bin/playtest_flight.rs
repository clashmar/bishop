use editor::playtest_shared::{resolve_playtest_binary, write_playtest_payload};
use editor::storage_shared::most_recent_game_name;
use engine_core::constants::agents;
use engine_core::engine_global::{set_engine_mode, set_game_name, EngineMode};
use engine_core::playtest::{
    PlaytestControlProfileRef, PlaytestControlRequest, BUILTIN_PROFILE_CAMERA_FOLLOW_TOGGLE,
    BUILTIN_PROFILE_CAMERA_PAN_SWEEP, BUILTIN_PROFILE_GROUNDED_WALK_LEFT,
    BUILTIN_PROFILE_GROUNDED_WALK_RIGHT, BUILTIN_PROFILE_MOVEMENT_EVENNESS_LEFT_RIGHT,
};
use engine_core::playtest::{PlaytestSessionManifest, PlaytestSnapshot};
use engine_core::prelude::RoomId;
use engine_core::storage::load_game_from_folder;
use engine_core::storage::path_utils::resources_folder;
use serde::{Deserialize, Deserializer, Serialize};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const COMMAND_LAUNCH: &str = "launch";
const COMMAND_PROFILES: &str = "profiles";
const COMMAND_PILOT: &str = "pilot";
const COMMAND_SEQUENCE: &str = "sequence";
const COMMAND_INSPECT: &str = "inspect";
const COMMAND_CLEANUP: &str = "cleanup";
const FLAG_GAME: &str = "--game";
const FLAG_ROOM: &str = "--room";
const FLAG_LAUNCH: &str = "--launch";
const FLAG_SESSION: &str = "--session";
const FLAG_PROFILE: &str = "--profile";
const FLAG_PROFILES: &str = "--profiles";
const FLAG_WAIT: &str = "--wait";
const FLAG_PID: &str = "--pid";
const HELP_LONG: &str = "--help";
const HELP_SHORT: &str = "-h";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    set_engine_mode(EngineMode::Editor);

    match CommandArgs::parse(std::env::args_os().skip(1))? {
        CommandArgs::Launch(args) => run_launch(args),
        CommandArgs::Profiles(args) => run_profiles(args),
        CommandArgs::Pilot(args) => run_pilot(args),
        CommandArgs::Sequence(args) => run_sequence(args),
        CommandArgs::Inspect(args) => run_inspect(args),
        CommandArgs::Cleanup(args) => run_cleanup(args),
    }
}

#[derive(Clone)]
enum CommandArgs {
    Launch(LaunchArgs),
    Profiles(ProfilesArgs),
    Pilot(PilotArgs),
    Sequence(SequenceArgs),
    Inspect(InspectArgs),
    Cleanup(CleanupArgs),
}

impl CommandArgs {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut iter = args.into_iter();
        let Some(command) = iter.next() else {
            return Err(usage());
        };

        match command.to_string_lossy().as_ref() {
            COMMAND_LAUNCH => Ok(Self::Launch(LaunchArgs::parse(iter)?)),
            COMMAND_PROFILES => Ok(Self::Profiles(ProfilesArgs::parse(iter)?)),
            COMMAND_PILOT => Ok(Self::Pilot(PilotArgs::parse(iter)?)),
            COMMAND_SEQUENCE => Ok(Self::Sequence(SequenceArgs::parse(iter)?)),
            COMMAND_INSPECT => Ok(Self::Inspect(InspectArgs::parse(iter)?)),
            COMMAND_CLEANUP => Ok(Self::Cleanup(CleanupArgs::parse(iter)?)),
            HELP_LONG | HELP_SHORT => Err(usage()),
            _ => Err(usage()),
        }
    }
}

#[derive(Clone)]
struct LaunchArgs {
    game: Option<String>,
    room_id: usize,
    launch: bool,
}

impl LaunchArgs {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut game = None;
        let mut room_id = None;
        let mut launch = false;
        let mut iter = args.into_iter();

        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                FLAG_GAME => game = Some(next_string(&mut iter)?),
                FLAG_ROOM => room_id = Some(next_parsed(&mut iter)?),
                FLAG_LAUNCH => launch = true,
                _ => return Err(usage()),
            }
        }

        Ok(Self {
            game,
            room_id: room_id.ok_or_else(usage)?,
            launch,
        })
    }
}

#[derive(Clone)]
struct ProfilesArgs;

impl ProfilesArgs {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        if args.into_iter().next().is_some() {
            return Err(usage());
        }
        Ok(Self)
    }
}

#[derive(Clone)]
struct PilotArgs {
    session_dir: PathBuf,
    profile: String,
    wait: bool,
}

impl PilotArgs {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut session_dir = None;
        let mut profile = None;
        let mut wait = false;
        let mut iter = args.into_iter();

        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                FLAG_SESSION => session_dir = Some(next_path_buf(&mut iter)?),
                FLAG_PROFILE => profile = Some(next_string(&mut iter)?),
                FLAG_WAIT => wait = true,
                _ => return Err(usage()),
            }
        }

        Ok(Self {
            session_dir: session_dir.ok_or_else(usage)?,
            profile: profile.ok_or_else(usage)?,
            wait,
        })
    }
}

#[derive(Clone)]
struct SequenceArgs {
    session_dir: PathBuf,
    profiles: Vec<String>,
    wait: bool,
}

impl SequenceArgs {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut session_dir = None;
        let mut profiles = None;
        let mut wait = false;
        let mut iter = args.into_iter();

        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                FLAG_SESSION => session_dir = Some(next_path_buf(&mut iter)?),
                FLAG_PROFILES => {
                    let parsed = next_string(&mut iter)?
                        .split(',')
                        .filter(|part| !part.is_empty())
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>();
                    profiles = Some(parsed);
                }
                FLAG_WAIT => wait = true,
                _ => return Err(usage()),
            }
        }

        let profiles = profiles.ok_or_else(usage)?;
        if profiles.is_empty() {
            return Err(usage());
        }

        Ok(Self {
            session_dir: session_dir.ok_or_else(usage)?,
            profiles,
            wait,
        })
    }
}

#[derive(Clone)]
struct InspectArgs {
    session_dir: PathBuf,
}

impl InspectArgs {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut session_dir = None;
        let mut iter = args.into_iter();

        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                FLAG_SESSION => session_dir = Some(next_path_buf(&mut iter)?),
                _ => return Err(usage()),
            }
        }

        Ok(Self {
            session_dir: session_dir.ok_or_else(usage)?,
        })
    }
}

#[derive(Clone)]
struct CleanupArgs {
    pid: Option<u32>,
    session_dir: Option<PathBuf>,
}

impl CleanupArgs {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut pid = None;
        let mut session_dir = None;
        let mut iter = args.into_iter();

        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                FLAG_PID => pid = Some(next_parsed(&mut iter)?),
                FLAG_SESSION => session_dir = Some(next_path_buf(&mut iter)?),
                _ => return Err(usage()),
            }
        }

        if pid.is_none() && session_dir.is_none() {
            return Err(usage());
        }

        Ok(Self { pid, session_dir })
    }
}

#[derive(Serialize)]
struct LaunchOutput {
    game: String,
    room_id: usize,
    payload_path: String,
    session_dir: String,
    pid: Option<u32>,
}

#[derive(Serialize)]
struct ProfilesOutput {
    profiles: Vec<&'static str>,
}

#[derive(Clone, Serialize)]
struct SessionSnapshotSummary {
    manifest_path: String,
    snapshot_path: String,
    payload_path: Option<String>,
    manifest_active_control_profile: Option<String>,
    frame_index: Option<u64>,
    active_control_profile: Option<String>,
    active_control_seed: Option<u64>,
    active_control_frame_index: Option<u64>,
    player_position_x: Option<f32>,
    player_position_y: Option<f32>,
    player_velocity_x: Option<f32>,
    camera_target: Option<(f32, f32)>,
    camera_zoom: Option<(f32, f32)>,
    camera_follow_enabled: Option<bool>,
    camera_override_active: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct RuntimeSnapshotPayloadView {
    #[serde(deserialize_with = "deserialize_plain_or_wrapped_option")]
    active_control_profile: Option<String>,
    #[serde(deserialize_with = "deserialize_plain_or_wrapped_option")]
    active_control_seed: Option<u64>,
    #[serde(deserialize_with = "deserialize_plain_or_wrapped_option")]
    active_control_frame_index: Option<u64>,
    #[serde(deserialize_with = "deserialize_plain_or_wrapped_option")]
    player_position_x: Option<f32>,
    #[serde(deserialize_with = "deserialize_plain_or_wrapped_option")]
    player_position_y: Option<f32>,
    #[serde(deserialize_with = "deserialize_plain_or_wrapped_option")]
    player_velocity_x: Option<f32>,
    #[serde(deserialize_with = "deserialize_plain_or_wrapped_option")]
    camera_target: Option<(f32, f32)>,
    #[serde(deserialize_with = "deserialize_plain_or_wrapped_option")]
    camera_zoom: Option<(f32, f32)>,
    #[serde(deserialize_with = "deserialize_plain_or_wrapped_option")]
    camera_follow_enabled: Option<bool>,
    #[serde(deserialize_with = "deserialize_plain_or_wrapped_option")]
    camera_override_active: Option<bool>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PlainOrWrappedOption<T> {
    Plain(T),
    Wrapped(Option<T>),
}

#[derive(Serialize)]
struct PilotOutput {
    profile: String,
    session_dir: String,
    control_request_path: String,
    before: SessionSnapshotSummary,
    after: SessionSnapshotSummary,
    accepted: bool,
    progressed: bool,
    completed: bool,
    request_consumed: bool,
}

#[derive(Serialize)]
struct SequenceOutput {
    session_dir: String,
    results: Vec<PilotOutput>,
}

#[derive(Serialize)]
struct CleanupOutput {
    pid: Option<u32>,
    session_dir: Option<String>,
    killed: bool,
    payload_path: Option<String>,
    payload_removed: bool,
    session_dir_removed: bool,
}

#[derive(Default)]
struct CleanupArtifactsOutcome {
    payload_path: Option<PathBuf>,
    payload_removed: bool,
    session_dir_removed: bool,
}

fn run_launch(args: LaunchArgs) -> Result<(), String> {
    let game_name = match args.game {
        Some(name) => name,
        None => most_recent_game_name().ok_or_else(|| {
            "No game provided and no recent game could be inferred. Pass --game <name>.".to_string()
        })?,
    };
    set_game_name(&game_name);

    let game = load_game_from_folder(&resources_folder(&game_name))
        .map_err(|error| format!("Failed to load game '{game_name}': {error}"))?;
    let room_id = RoomId(args.room_id);
    let room = game.current_world().get_room(room_id).ok_or_else(|| {
        format!(
            "Room {} not found in current world for game '{game_name}'",
            args.room_id
        )
    })?;

    let payload_path = write_playtest_payload(room, &game)
        .map_err(|error| format!("Failed to write playtest payload: {error}"))?;
    let session_dir = session_dir_for_payload(&payload_path);

    let pid = if args.launch {
        let exe_path = resolve_playtest_binary()
            .map_err(|error| format!("Failed to resolve playtest binary: {error}"))?;
        let child = Command::new(&exe_path)
            .arg(&payload_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Failed to launch playtest: {error}"))?;
        Some(child.id())
    } else {
        None
    };

    let out = LaunchOutput {
        game: game_name,
        room_id: args.room_id,
        payload_path: payload_path.display().to_string(),
        session_dir: session_dir.display().to_string(),
        pid,
    };
    print_ron(&out)
}

fn run_profiles(_args: ProfilesArgs) -> Result<(), String> {
    let out = ProfilesOutput {
        profiles: vec![
            BUILTIN_PROFILE_GROUNDED_WALK_LEFT,
            BUILTIN_PROFILE_GROUNDED_WALK_RIGHT,
            BUILTIN_PROFILE_MOVEMENT_EVENNESS_LEFT_RIGHT,
            BUILTIN_PROFILE_CAMERA_PAN_SWEEP,
            BUILTIN_PROFILE_CAMERA_FOLLOW_TOGGLE,
        ],
    };
    print_ron(&out)
}

fn run_pilot(args: PilotArgs) -> Result<(), String> {
    let result = run_single_pilot(&args.session_dir, &args.profile, args.wait)?;
    print_ron(&result)
}

fn run_sequence(args: SequenceArgs) -> Result<(), String> {
    let mut results = Vec::new();
    for profile in &args.profiles {
        results.push(run_single_pilot(&args.session_dir, profile, args.wait)?);
    }
    let out = SequenceOutput {
        session_dir: args.session_dir.display().to_string(),
        results,
    };
    print_ron(&out)
}

fn run_inspect(args: InspectArgs) -> Result<(), String> {
    let summary = read_snapshot_summary(&args.session_dir)?;
    print_ron(&summary)
}

fn run_cleanup(args: CleanupArgs) -> Result<(), String> {
    let session_dir = args
        .session_dir
        .clone()
        .or_else(|| args.pid.and_then(find_session_dir_by_game_playtest_pid));
    let pid = if let Some(pid) = args.pid {
        Some(pid)
    } else if let Some(session_dir) = session_dir.as_ref() {
        find_game_playtest_pid_by_session_dir(session_dir)
    } else {
        None
    };

    let mut killed = false;
    if let Some(pid) = pid {
        let _ = Command::new("kill").arg(pid.to_string()).status();
        thread::sleep(Duration::from_millis(200));
        if is_pid_running(pid) {
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
            thread::sleep(Duration::from_millis(200));
        }
        killed = !is_pid_running(pid);
    }

    let cleanup = cleanup_playtest_artifacts(session_dir.as_deref());

    let out = CleanupOutput {
        pid,
        session_dir: session_dir.map(|path| path.display().to_string()),
        killed,
        payload_path: cleanup
            .payload_path
            .as_ref()
            .map(|path| path.display().to_string()),
        payload_removed: cleanup.payload_removed,
        session_dir_removed: cleanup.session_dir_removed,
    };
    print_ron(&out)
}

fn cleanup_playtest_artifacts(session_dir: Option<&Path>) -> CleanupArtifactsOutcome {
    let payload_path = session_dir.and_then(payload_path_for_session_dir);
    let payload_removed = payload_path
        .as_ref()
        .is_some_and(|path| remove_file_if_exists(path));
    let session_dir_removed = session_dir.is_some_and(remove_dir_if_exists);

    CleanupArtifactsOutcome {
        payload_path,
        payload_removed,
        session_dir_removed,
    }
}

fn run_single_pilot(session_dir: &Path, profile: &str, wait: bool) -> Result<PilotOutput, String> {
    let before = read_snapshot_summary(session_dir)?;
    let request = PlaytestControlRequest::named(profile);
    let control_request_path = session_dir.join(agents::CONTROL_REQUEST_FILENAME);
    let request_ron = ron::ser::to_string_pretty(&request, ron::ser::PrettyConfig::default())
        .map_err(|error| format!("Failed to serialize control request: {error}"))?;
    fs::write(&control_request_path, request_ron)
        .map_err(|error| format!("Failed to write control request: {error}"))?;

    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    let mut accepted = false;
    let mut progressed = false;
    let mut completed = false;
    let mut request_consumed = false;
    let mut after = before.clone();

    while start.elapsed() < timeout {
        thread::sleep(Duration::from_millis(100));
        request_consumed = !control_request_path.exists();
        after = read_snapshot_summary(session_dir)?;
        let accepted_now = after.manifest_active_control_profile.as_deref() == Some(profile)
            || after.active_control_profile.as_deref() == Some(profile);
        accepted |= accepted_now;

        let active_control_progress = after
            .active_control_frame_index
            .zip(before.active_control_frame_index)
            .is_some_and(|(after_idx, before_idx)| after_idx > before_idx)
            || after.active_control_frame_index.is_some();
        let state_changed = float_changed(before.player_position_x, after.player_position_x)
            || float_changed(before.player_position_y, after.player_position_y)
            || float_changed(before.player_velocity_x, after.player_velocity_x);
        let active_control_still_present = after.manifest_active_control_profile.is_some()
            || after.active_control_profile.is_some();

        completed = accepted && request_consumed && !active_control_still_present;
        progressed |= active_control_progress || (accepted && (state_changed || completed));

        if !wait && accepted {
            break;
        }
        if wait && completed {
            break;
        }
    }

    Ok(PilotOutput {
        profile: profile.to_string(),
        session_dir: session_dir.display().to_string(),
        control_request_path: control_request_path.display().to_string(),
        before,
        after,
        accepted,
        progressed,
        completed,
        request_consumed,
    })
}

fn read_snapshot_summary(session_dir: &Path) -> Result<SessionSnapshotSummary, String> {
    let manifest_path = session_dir.join(agents::MANIFEST_FILENAME);
    let snapshot_path = session_dir.join(agents::SNAPSHOT_FILENAME);
    let manifest_ron = fs::read_to_string(&manifest_path).map_err(|error| {
        format!(
            "Failed to read manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    let snapshot_ron = fs::read_to_string(&snapshot_path).map_err(|error| {
        format!(
            "Failed to read snapshot {}: {error}",
            snapshot_path.display()
        )
    })?;

    let manifest: PlaytestSessionManifest = ron::from_str(&manifest_ron).map_err(|error| {
        format!(
            "Failed to parse manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    let snapshot: PlaytestSnapshot = ron::from_str(&snapshot_ron).map_err(|error| {
        format!(
            "Failed to parse snapshot {}: {error}",
            snapshot_path.display()
        )
    })?;

    let payload = snapshot
        .payload
        .as_ref()
        .map(decode_snapshot_payload)
        .transpose()?;

    Ok(SessionSnapshotSummary {
        manifest_path: manifest_path.display().to_string(),
        snapshot_path: snapshot_path.display().to_string(),
        payload_path: manifest.payload_path,
        manifest_active_control_profile: manifest
            .active_control
            .as_ref()
            .map(|active_control| control_profile_label(&active_control.request.profile)),
        frame_index: snapshot.frame_index,
        active_control_profile: payload
            .as_ref()
            .and_then(|payload| payload.active_control_profile.clone()),
        active_control_seed: payload
            .as_ref()
            .and_then(|payload| payload.active_control_seed),
        active_control_frame_index: payload
            .as_ref()
            .and_then(|payload| payload.active_control_frame_index),
        player_position_x: payload
            .as_ref()
            .and_then(|payload| payload.player_position_x),
        player_position_y: payload
            .as_ref()
            .and_then(|payload| payload.player_position_y),
        player_velocity_x: payload
            .as_ref()
            .and_then(|payload| payload.player_velocity_x),
        camera_target: payload.as_ref().and_then(|payload| payload.camera_target),
        camera_zoom: payload.as_ref().and_then(|payload| payload.camera_zoom),
        camera_follow_enabled: payload
            .as_ref()
            .and_then(|payload| payload.camera_follow_enabled),
        camera_override_active: payload
            .as_ref()
            .and_then(|payload| payload.camera_override_active),
    })
}

fn decode_snapshot_payload(value: &ron::Value) -> Result<RuntimeSnapshotPayloadView, String> {
    RuntimeSnapshotPayloadView::deserialize(value.clone())
        .map_err(|error| format!("Failed to decode snapshot payload: {error}"))
}

fn deserialize_plain_or_wrapped_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    match PlainOrWrappedOption::<T>::deserialize(deserializer)? {
        PlainOrWrappedOption::Plain(value) => Ok(Some(value)),
        PlainOrWrappedOption::Wrapped(value) => Ok(value),
    }
}

fn next_os_string<I>(iter: &mut I) -> Result<OsString, String>
where
    I: Iterator<Item = OsString>,
{
    iter.next().ok_or_else(usage)
}

fn next_string<I>(iter: &mut I) -> Result<String, String>
where
    I: Iterator<Item = OsString>,
{
    Ok(next_os_string(iter)?.to_string_lossy().into_owned())
}

fn next_path_buf<I>(iter: &mut I) -> Result<PathBuf, String>
where
    I: Iterator<Item = OsString>,
{
    Ok(PathBuf::from(next_os_string(iter)?))
}

fn next_parsed<I, T>(iter: &mut I) -> Result<T, String>
where
    I: Iterator<Item = OsString>,
    T: std::str::FromStr,
{
    next_string(iter)?.parse::<T>().map_err(|_| usage())
}

fn control_profile_label(profile: &PlaytestControlProfileRef) -> String {
    match profile {
        PlaytestControlProfileRef::Named(name) => name.clone(),
        PlaytestControlProfileRef::Inline(_) => "inline".to_string(),
    }
}

fn float_changed(before: Option<f32>, after: Option<f32>) -> bool {
    before
        .zip(after)
        .is_some_and(|(before, after)| (before - after).abs() > f32::EPSILON)
}

fn print_ron<T: Serialize>(value: &T) -> Result<(), String> {
    let output = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
        .map_err(|error| format!("Failed to serialize RON output: {error}"))?;
    println!("{output}");
    Ok(())
}

fn find_game_playtest_pid_by_session_dir(session_dir: &Path) -> Option<u32> {
    let output = Command::new("pgrep")
        .arg("-fl")
        .arg("game-playtest")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let mut parts = line.splitn(2, ' ');
        let pid = parts.next()?.parse::<u32>().ok()?;
        let Some(line_session_dir) = session_dir_from_pgrep_line(line) else {
            continue;
        };
        if line_session_dir == session_dir {
            return Some(pid);
        }
    }
    None
}

fn find_session_dir_by_game_playtest_pid(pid: u32) -> Option<PathBuf> {
    let output = Command::new("pgrep")
        .arg("-fl")
        .arg("game-playtest")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let mut parts = line.splitn(2, ' ');
        let line_pid = parts.next()?.parse::<u32>().ok()?;
        if line_pid != pid {
            continue;
        }
        return session_dir_from_pgrep_line(line);
    }
    None
}

fn is_pid_running(pid: u32) -> bool {
    Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn usage() -> String {
    format!(
        "Usage: playtest-flight <{COMMAND_LAUNCH}|{COMMAND_PROFILES}|{COMMAND_PILOT}|{COMMAND_SEQUENCE}|{COMMAND_INSPECT}|{COMMAND_CLEANUP}> ..."
    )
}

fn session_dir_for_payload(payload_path: &Path) -> PathBuf {
    let session_dir_name = payload_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| format!("{stem}_agent"))
        .unwrap_or_else(|| "agent_session".to_string());
    payload_path
        .parent()
        .map(|parent| parent.join(session_dir_name))
        .unwrap_or_else(|| PathBuf::from("agent_session"))
}

fn payload_path_for_session_dir(session_dir: &Path) -> Option<PathBuf> {
    Some(session_dir.parent()?.join(format!(
        "{}.ron",
        session_dir
            .file_name()?
            .to_string_lossy()
            .trim_end_matches("_agent")
    )))
}

fn session_dir_from_pgrep_line(line: &str) -> Option<PathBuf> {
    let command = line.split_once(' ')?.1;
    let payload = command
        .split_whitespace()
        .find(|part| part.ends_with(".ron"))?;
    Some(session_dir_for_payload(Path::new(payload)))
}

fn remove_file_if_exists(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    fs::remove_file(path).is_ok()
}

fn remove_dir_if_exists(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    fs::remove_dir_all(path).is_ok()
}

#[cfg(test)]
#[path = "playtest_flight/tests.rs"]
mod tests;

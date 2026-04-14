use engine_core::constants::{agents, PLAYTEST_PAYLOAD_RON};

/// Typed launch mode for the playtest binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaytestLaunchMode {
    EditorPayload { payload_path: String },
    SeededAgentPayload { payload_path: String },
    Headless,
}

/// Parsed launch arguments for the playtest binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaytestLaunchArgs {
    /// Selected playtest launch mode.
    pub mode: PlaytestLaunchMode,
}

impl PlaytestLaunchArgs {
    /// Parses `game-playtest` arguments.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let usage = format!(
            "Usage: {} [{}] [{} {}] [{}]",
            args.first().map(String::as_str).unwrap_or("game-playtest"),
            agents::HEADLESS_FLAG,
            agents::PAYLOAD_FLAG,
            agents::PAYLOAD_FILENAME,
            PLAYTEST_PAYLOAD_RON
        );
        let mut payload_path = None;
        let mut agent_payload_path = None;
        let mut headless = false;
        let mut saw_payload_flag_before_headless = false;
        let mut iter = args[1..].iter().peekable();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                agents::HEADLESS_FLAG => {
                    if headless {
                        return Err(usage);
                    }
                    headless = true;
                }
                agents::PAYLOAD_FLAG => {
                    if agent_payload_path.is_some() {
                        return Err(usage);
                    }
                    if !headless {
                        saw_payload_flag_before_headless = true;
                    }

                    let Some(path) = iter.next() else {
                        return Err(usage);
                    };
                    agent_payload_path = Some(path.clone());
                }
                _ => {
                    if payload_path.replace(arg.clone()).is_some() {
                        return Err(usage);
                    }
                }
            }
        }

        if headless && payload_path.is_some() && agent_payload_path.is_none() {
            return Err(usage);
        }

        if agent_payload_path.is_some() && !headless {
            return Err(usage);
        }

        if saw_payload_flag_before_headless {
            return Err(usage);
        }

        if !headless && payload_path.is_none() {
            return Err(usage);
        }

        let mode = match (headless, payload_path, agent_payload_path) {
            (false, Some(payload_path), None) => PlaytestLaunchMode::EditorPayload { payload_path },
            (true, None, None) => PlaytestLaunchMode::Headless,
            (true, None, Some(payload_path)) => {
                PlaytestLaunchMode::SeededAgentPayload { payload_path }
            }
            _ => return Err(usage),
        };

        Ok(Self { mode })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_editor_payload_mode() {
        let args = vec!["game-playtest".to_string(), "payload.ron".to_string()];

        let parsed = PlaytestLaunchArgs::parse(&args).unwrap();

        assert_eq!(
            parsed.mode,
            PlaytestLaunchMode::EditorPayload {
                payload_path: "payload.ron".to_string(),
            }
        );
    }

    #[test]
    fn parse_accepts_headless_only() {
        let args = vec![
            "game-playtest".to_string(),
            agents::HEADLESS_FLAG.to_string(),
        ];

        let parsed = PlaytestLaunchArgs::parse(&args).unwrap();

        assert_eq!(parsed.mode, PlaytestLaunchMode::Headless);
    }

    #[test]
    fn parse_accepts_seeded_agent_payload_mode() {
        let args = vec![
            "game-playtest".to_string(),
            agents::HEADLESS_FLAG.to_string(),
            agents::PAYLOAD_FLAG.to_string(),
            agents::PAYLOAD_FILENAME.to_string(),
        ];

        let parsed = PlaytestLaunchArgs::parse(&args).unwrap();

        assert_eq!(
            parsed.mode,
            PlaytestLaunchMode::SeededAgentPayload {
                payload_path: agents::PAYLOAD_FILENAME.to_string(),
            }
        );
    }

    #[test]
    fn parse_rejects_reversed_seeded_agent_flag_order() {
        let args = vec![
            "game-playtest".to_string(),
            agents::PAYLOAD_FLAG.to_string(),
            agents::PAYLOAD_FILENAME.to_string(),
            agents::HEADLESS_FLAG.to_string(),
        ];

        let error = PlaytestLaunchArgs::parse(&args).unwrap_err();

        assert_eq!(
            error,
            format!(
                "Usage: game-playtest [{}] [{} {}] [{}]",
                agents::HEADLESS_FLAG,
                agents::PAYLOAD_FLAG,
                agents::PAYLOAD_FILENAME,
                PLAYTEST_PAYLOAD_RON
            )
        );
    }

    #[test]
    fn parse_rejects_skip_flag() {
        let args = vec![
            "game-playtest".to_string(),
            "--skip-to-playing".to_string(),
            "payload.ron".to_string(),
        ];

        let error = PlaytestLaunchArgs::parse(&args).unwrap_err();

        assert_eq!(
            error,
            format!(
                "Usage: game-playtest [{}] [{} {}] [{}]",
                agents::HEADLESS_FLAG,
                agents::PAYLOAD_FLAG,
                agents::PAYLOAD_FILENAME,
                PLAYTEST_PAYLOAD_RON
            )
        );
    }

    #[test]
    fn parse_rejects_missing_payload_when_not_headless() {
        let args = vec!["game-playtest".to_string()];

        let error = PlaytestLaunchArgs::parse(&args).unwrap_err();

        assert_eq!(
            error,
            format!(
                "Usage: game-playtest [{}] [{} {}] [{}]",
                agents::HEADLESS_FLAG,
                agents::PAYLOAD_FLAG,
                agents::PAYLOAD_FILENAME,
                PLAYTEST_PAYLOAD_RON
            )
        );
    }
}

use engine_core::constants::PLAYTEST_PAYLOAD_RON;

/// Parsed launch arguments for the playtest binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaytestLaunchArgs {
    /// Path to the editor-authored playtest payload.
    pub payload_path: String,
}

impl PlaytestLaunchArgs {
    /// Parses `game-playtest` arguments.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let usage = format!(
            "Usage: {} [{}]",
            args.first().map(String::as_str).unwrap_or("game-playtest"),
            PLAYTEST_PAYLOAD_RON
        );
        let mut payload_path = None;
        let mut iter = args[1..].iter().peekable();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                value if value.starts_with('-') => return Err(usage),
                _ => {
                    if payload_path.replace(arg.clone()).is_some() {
                        return Err(usage);
                    }
                }
            }
        }

        if payload_path.is_none() {
            return Err(usage);
        }

        let payload_path = match payload_path {
            Some(payload_path) => payload_path,
            None => return Err(usage),
        };

        Ok(Self { payload_path })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_editor_payload_mode() {
        let args = vec!["game-playtest".to_string(), "payload.ron".to_string()];

        let parsed = PlaytestLaunchArgs::parse(&args).unwrap();

        assert_eq!(parsed.payload_path, "payload.ron".to_string());
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
            format!("Usage: game-playtest [{}]", PLAYTEST_PAYLOAD_RON)
        );
    }

    #[test]
    fn parse_rejects_missing_payload() {
        let args = vec!["game-playtest".to_string()];

        let error = PlaytestLaunchArgs::parse(&args).unwrap_err();

        assert_eq!(
            error,
            format!("Usage: game-playtest [{}]", PLAYTEST_PAYLOAD_RON)
        );
    }
}

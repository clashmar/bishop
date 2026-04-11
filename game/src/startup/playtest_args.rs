/// Parsed launch arguments for the playtest binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaytestLaunchArgs {
    /// Path to the serialized playtest payload.
    pub payload_path: Option<String>,

    /// Whether the playtest should start without an editor payload.
    pub headless: bool,
}

impl PlaytestLaunchArgs {
    /// Parses `game-playtest` arguments.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let usage = format!(
            "Usage: {} [--headless] [playtest_payload.ron]",
            args.first().map(String::as_str).unwrap_or("game-playtest")
        );
        let mut payload_path = None;
        let mut headless = false;

        for arg in &args[1..] {
            match arg.as_str() {
                "--headless" => {
                    if headless {
                        return Err(usage);
                    }
                    headless = true;
                }
                _ => {
                    if payload_path.replace(arg.clone()).is_some() {
                        return Err(usage);
                    }
                }
            }
        }

        if headless && payload_path.is_some() {
            return Err(usage);
        }

        if !headless && payload_path.is_none() {
            return Err(usage);
        }

        Ok(Self {
            payload_path,
            headless,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_payload_only() {
        let args = vec!["game-playtest".to_string(), "payload.ron".to_string()];

        let parsed = PlaytestLaunchArgs::parse(&args).unwrap();

        assert_eq!(
            parsed,
            PlaytestLaunchArgs {
                payload_path: Some("payload.ron".to_string()),
                headless: false,
            }
        );
    }

    #[test]
    fn parse_accepts_headless_only() {
        let args = vec!["game-playtest".to_string(), "--headless".to_string()];

        let parsed = PlaytestLaunchArgs::parse(&args).unwrap();

        assert_eq!(
            parsed,
            PlaytestLaunchArgs {
                payload_path: None,
                headless: true,
            }
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
            "Usage: game-playtest [--headless] [playtest_payload.ron]"
        );
    }

    #[test]
    fn parse_rejects_missing_payload_when_not_headless() {
        let args = vec!["game-playtest".to_string()];

        let error = PlaytestLaunchArgs::parse(&args).unwrap_err();

        assert_eq!(
            error,
            "Usage: game-playtest [--headless] [playtest_payload.ron]"
        );
    }
}

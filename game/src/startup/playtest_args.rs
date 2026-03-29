/// Parsed launch arguments for the playtest binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaytestLaunchArgs {
    /// Path to the serialized playtest payload.
    pub payload_path: String,
}

impl PlaytestLaunchArgs {
    /// Parses `game-playtest` arguments.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        if args.len() < 2 {
            return Err(format!(
                "Usage: {} <playtest_payload.ron>",
                args.first().map(String::as_str).unwrap_or("game-playtest")
            ));
        }

        let mut payload_path = None;

        for arg in &args[1..] {
            if payload_path.replace(arg.clone()).is_some() {
                return Err(format!("Usage: {} <playtest_payload.ron>", args[0]));
            }
        }

        let Some(payload_path) = payload_path else {
            return Err(format!("Usage: {} <playtest_payload.ron>", args[0]));
        };

        Ok(Self { payload_path })
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
                payload_path: "payload.ron".to_string(),
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

        assert_eq!(error, "Usage: game-playtest <playtest_payload.ron>");
    }
}

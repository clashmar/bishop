use super::*;
use bishop::text::FontId;

#[test]
fn game_text_settings_round_trip_default_font() -> Result<(), Box<dyn std::error::Error>> {
    let mut game = Game::default();
    assert_eq!(game.text_settings.default_font, None);
    game.text_settings.default_font = Some(FontId(17));

    let serialized = ron::to_string(&game)?;
    let decoded: Game = ron::from_str(&serialized)?;

    assert_eq!(decoded.text_settings.default_font, Some(FontId(17)));
    Ok(())
}

use super::*;

#[test]
fn agent_payload_builder_rejects_unknown_component_type() {
    let payload = AgentPayloadSpec::synthetic("TestGame");
    let result = payload
        .add_entity("Player")
        .attach_component("Player", "NoSuchComponent", "()")
        .build();

    assert!(result.is_err());
}

#[test]
fn agent_payload_builder_can_build_seeded_and_synthetic_payloads() {
    let synthetic = AgentPayloadSpec::synthetic("TestGame")
        .add_room("TestRoom")
        .add_entity("Player")
        .attach_component(
            "Player",
            "Transform",
            "(visible:true, position:(x:0.0,y:0.0), pivot:())",
        )
        .attach_script("Player", "player_main")
        .build()
        .unwrap();

    assert_eq!(synthetic.game.name, "TestGame");
    assert_eq!(synthetic.room.name, "TestRoom");
}

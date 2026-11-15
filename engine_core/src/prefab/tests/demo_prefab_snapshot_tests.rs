use crate::ecs::component::comp_type_name;
use crate::ecs::Script;
use crate::prefab::PrefabAsset;
use std::fs;
use std::path::Path;

#[test]
fn demo_prefab_when_script_component_snapshots_are_loaded_deserializes_them() {
    let prefab_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../games/Demo/Resources/prefabs");
    let prefab_paths = fs::read_dir(&prefab_dir)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", prefab_dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "prefab"));

    for path in prefab_paths {
        let prefab_ron = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
        let prefab: PrefabAsset = ron::from_str(&prefab_ron)
            .unwrap_or_else(|error| panic!("could not parse {}: {error}", path.display()));

        for component in prefab
            .nodes
            .iter()
            .flat_map(|node| &node.components)
            .filter(|component| component.type_name == comp_type_name::<Script>())
        {
            ron::from_str::<Script>(&component.ron).unwrap_or_else(|error| {
                panic!("could not deserialize Script component in {}: {error}", path.display())
            });
        }
    }
}

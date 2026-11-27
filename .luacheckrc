std = "lua54"

globals = {
    "engine",
    "InputAction",
    "Color",
    "Widget",
    "private",
    "local",
    "Input",
    "Direction",
    "Components",
    "Animations",
    "Prefabs",
    "Sounds",
    "Menus",
    "Worlds",
    "Entries",
    "Script",
    "Entity",
}

unused_args = false

files["games/Demo/Resources/scripts/_engine/**/*.lua"] = {
    globals = {
        "engine",
        "InputAction",
        "Color",
        "Widget",
        "private",
        "local",
        "Input",
        "Direction",
        "Components",
        "Animations",
        "Prefabs",
        "Sounds",
        "Menus",
        "Worlds",
        "Entries",
        "Script",
        "Entity",
    },
    unused_args = false,
    ignore = { "211", "631" },
}

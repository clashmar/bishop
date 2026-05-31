std = "lua54"

globals = {
    "engine",
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
    "Script",
    "Entity",
}

unused_args = false

files["Resources/scripts/_engine/**/*.lua"] = {
    globals = {
        "engine",
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
        "Script",
        "Entity",
    },
    unused_args = false,
    ignore = { "211", "631" },
}

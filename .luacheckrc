std = "lua54"

globals = {
    "engine",
    "Color",
    "Widget",
    "private",
    "local",
}

unused_args = false

files["games/Demo/Resources/scripts/_engine/**/*.lua"] = {
    globals = {
        "engine",
        "Color",
        "Widget",
        "private",
        "local",
    },
    unused_args = false,
    ignore = { "211", "631" },
}

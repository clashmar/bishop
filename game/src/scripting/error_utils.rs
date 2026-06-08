use std::io;

/// Converts an `mlua::Error` to an `io::Error`.
///
/// `mlua::Error` does not implement `Send + Sync`, so the error is
/// converted to a display string before wrapping.
pub fn lua_io_error(err: mlua::Error) -> io::Error {
    io::Error::other(err.to_string())
}

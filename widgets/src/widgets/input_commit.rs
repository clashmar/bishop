/// Signals the commit state of an input widget.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputCommit {
    /// Widget is not focused and value has not changed.
    Unchanged,
    /// Widget has focus and the user is actively editing (live preview).
    Previewing,
    /// The value was committed (Enter, Tab, or click-away).
    Committed,
}

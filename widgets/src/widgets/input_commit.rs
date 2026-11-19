/// Signals the commit state of an input widget.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputCommit {
    /// Widget is not focused and value has not changed.
    Unchanged,
    /// Widget has focus and the user is actively editing (live preview).
    Previewing,
    /// The value was committed (Enter, Tab, click-away, or clear-button activation).
    Committed,
}

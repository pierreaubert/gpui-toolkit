/// Timer state
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimerState {
    /// Timer is active and will fire
    Active,
    /// Timer has been stopped
    Stopped,
}

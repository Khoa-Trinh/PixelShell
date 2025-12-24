#[derive(Debug, Clone, PartialEq)]
pub enum RunnerMode {
    Watchdog, // Keeps restarting the process (Normal/Silent)
    Detach,   // Starts process and exits (Fire & Forget)
}

#[derive(Debug, Clone)]
pub enum RunnerStatus {
    Starting(String),
    Running(u32), // PID
    Restarting,
    Detached,
    Error(String),
}

pub struct RunArgs {
    pub target: Option<String>,
    pub silent: bool,
    pub detach: bool,
}

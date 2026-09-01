use std::fs;
use std::path::Path;

const STATE_DIR: &str = "/var/lib/rulmee";
const STATE_FILE: &str = "/var/lib/rulmee/state";

pub struct LaunchState {
    pub username: String,
    pub session_opt: String,
}

pub fn read_launch_state() -> Option<LaunchState> {
    let content = fs::read_to_string(STATE_FILE).ok()?;
    let mut lines = content.lines();
    let username = lines.next()?.to_string();
    let session_opt = lines.next()?.to_string();
    Some(LaunchState {
        username,
        session_opt,
    })
}

pub fn write_launch_state(state: &LaunchState) -> std::io::Result<()> {
    if !Path::new(STATE_DIR).exists() {
        fs::create_dir_all(STATE_DIR)?;
    }
    let content = format!("{}\n{}\n", state.username, state.session_opt);
    fs::write(STATE_FILE, content)
}

use crate::exec::get_active_child_pgid;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::thread;

pub fn setup_signal_handler() -> Result<(), String> {
    let mut signals = Signals::new([SIGTERM, SIGINT])
        .map_err(|e| format!("Failed to register signal handlers: {}", e))?;

    thread::spawn(move || {
        for sig in signals.forever() {
            let pgid = get_active_child_pgid();
            if pgid > 0 {
                let _ = kill(Pid::from_raw(-pgid), Signal::SIGTERM);
            }
            if (sig == SIGTERM || sig == SIGINT) && pgid == 0 {
                std::process::exit(0);
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_signal_handler_runs_without_error() {
        let res = setup_signal_handler();
        assert!(res.is_ok());
    }
}

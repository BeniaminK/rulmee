use std::process::exit;

/// Triggers system poweroff via libc reboot.
pub fn poweroff() -> ! {
    unsafe {
        libc::reboot(libc::RB_POWER_OFF);
    }
    exit(0);
}

/// Triggers system reboot via libc reboot.
pub fn reboot() -> ! {
    unsafe {
        libc::reboot(libc::RB_AUTOBOOT);
    }
    exit(0);
}

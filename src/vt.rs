use nix::fcntl::{OFlag, open};
use nix::ioctl_write_int_bad;
use nix::sys::stat::Mode;
use std::ffi::c_int;
use std::io;
use std::os::unix::io::AsRawFd;

// Constants from <linux/vt.h> and <linux/kd.h>
const VT_ACTIVATE: u64 = 0x5606;
const VT_WAITACTIVE: u64 = 0x5607;
const KDGKBTYPE: u64 = 0x4B33;

ioctl_write_int_bad!(vt_activate, VT_ACTIVATE);
ioctl_write_int_bad!(vt_waitactive, VT_WAITACTIVE);
ioctl_write_int_bad!(kd_gkbtype, KDGKBTYPE);

const VTERMS: &[&str] = &[
    "/dev/tty",
    "/dev/tty0",
    "/dev/vc/0",
    "/dev/systty",
    "/dev/console",
];

pub fn chvt(n: c_int) -> io::Result<()> {
    for &vterm in VTERMS {
        if let Ok(fd) = open(vterm, OFlag::O_RDWR, Mode::empty()) {
            let mut kbtype: i32 = 0;
            // KDGKBTYPE returns the keyboard type. We check if it's < 3 as in the C version.
            unsafe {
                if kd_gkbtype(fd.as_raw_fd(), &mut kbtype as *mut i32 as i32).is_ok() && kbtype < 3
                {
                    if vt_activate(fd.as_raw_fd(), n).is_ok()
                        && vt_waitactive(fd.as_raw_fd(), n).is_ok()
                    {
                        return Ok(());
                    }
                }
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::Other,
        "Could not activate VT",
    ))
}

use std::io::{Error, ErrorKind};
use std::os::unix::io::AsRawFd;

use udev::MonitorBuilder;

use super::Media;

pub struct Drives;

impl Media for Drives {
    fn watch(&self, mut react: impl FnMut()) -> Result<(), String> {
        let unwatchable = |error: Error| format!("the drives could not be watched: {error}");

        let socket = MonitorBuilder::new()
            .and_then(|monitor| monitor.match_subsystem_devtype("block", "disk"))
            .and_then(MonitorBuilder::listen)
            .map_err(unwatchable)?;

        let mut waiting = libc::pollfd {
            fd: socket.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };

        loop {
            if unsafe { libc::poll(&mut waiting, 1, -1) } < 0 {
                let failure = Error::last_os_error();

                if failure.kind() == ErrorKind::Interrupted {
                    continue;
                }

                return Err(unwatchable(failure));
            }

            for _ in socket.iter() {
                react();
            }
        }
    }
}

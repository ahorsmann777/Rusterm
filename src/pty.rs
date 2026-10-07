use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::io::{Read, Write};
use std::thread;
use std::sync::Arc;
use crossbeam_channel::{Receiver, unbounded};
use winit::window::Window;

pub struct Pty {
    pub _pair: PtyPair,
    pub writer: Box<dyn Write + Send>,
}

impl Pty {
    pub fn new(shell: &str, rows: u16, cols: u16, window: Arc<Window>) -> (Self, Receiver<Vec<u8>>) {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("failed to open pty");

        let cmd = CommandBuilder::new(shell);
        let _child = pair.slave.spawn_command(cmd).expect("failed to spawn shell");

        let mut reader = pair.master.try_clone_reader().expect("failed to get reader");
        let writer = pair.master.take_writer().expect("failed to get writer");

        let (tx, rx) = unbounded();

        thread::spawn(move || {
            let mut buf = [0u8; 65536]; // Maximize buffer to 64KB
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                // Send data immediately
                if tx.send(buf[..n].to_vec()).is_ok() {
                    window.request_redraw();
                } else {
                    break;
                }
            }
        });

        (Self { _pair: pair, writer }, rx)
    }

    pub fn write(&mut self, data: &[u8]) {
        self.writer.write_all(data).ok();
        self.writer.flush().ok();
    }

    pub fn resize(&self, rows: u16, cols: u16) {
        self._pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }).ok();
    }
}

//! Single-instance coordination and path forwarding.
//!
//! The first process listens on a local socket. Later processes connect, send the path they were
//! asked to open, and exit. Namespaced sockets are used where the platform supports them;
//! otherwise a temporary filesystem socket is used.

use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, Name, Stream, ToFsName, ToNsName,
};

/// A running primary instance's listener.
pub struct Primary {
    incoming: Receiver<PathBuf>,
    _thread: thread::JoinHandle<()>,
}

impl Primary {
    /// Take all paths forwarded since the last call.
    pub fn take_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        while let Ok(path) = self.incoming.try_recv() {
            paths.push(path);
        }
        paths
    }
}

/// The result of acquiring the single-instance role.
pub enum Acquired {
    /// This process is the primary instance.
    Primary(Primary),
    /// Another instance received the path; this process should exit.
    Forwarded,
}

/// Acquire the single-instance role, forwarding `path` to an existing instance when there is one.
pub fn acquire(path: Option<&Path>, wake: impl Fn() + Send + Sync + 'static) -> io::Result<Acquired> {
    let name = socket_name()?;
    if forward(&name, path)? {
        return Ok(Acquired::Forwarded);
    }

    let listener = match ListenerOptions::new().name(name.clone()).create_sync() {
        Ok(listener) => listener,
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => {
            if forward(&name, path)? {
                return Ok(Acquired::Forwarded);
            }
            return Err(error);
        }
        Err(error) => return Err(error),
    };

    let (sender, incoming) = mpsc::channel();
    let thread = thread::spawn(move || {
        for connection in listener.incoming().flatten() {
            let mut line = String::new();
            if BufReader::new(connection).read_line(&mut line).is_ok() {
                let path = PathBuf::from(line.trim_end_matches(['\r', '\n']));
                if sender.send(path).is_ok() {
                    wake();
                }
            }
        }
    });

    Ok(Acquired::Primary(Primary {
        incoming,
        _thread: thread,
    }))
}

fn forward(name: &Name<'_>, path: Option<&Path>) -> io::Result<bool> {
    let Ok(mut stream) = Stream::connect(name.borrow()) else {
        return Ok(false);
    };
    let line = path
        .map(|path| path.to_string_lossy().replace('\n', " "))
        .unwrap_or_default();
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(true)
}

fn socket_name() -> io::Result<Name<'static>> {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "default".to_owned());
    let user = user
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if GenericNamespaced::is_supported() {
        format!("mdview-{user}").to_ns_name::<GenericNamespaced>()
    } else {
        std::env::temp_dir()
            .join(format!("mdview-{user}.sock"))
            .to_fs_name::<GenericFilePath>()
    }
}

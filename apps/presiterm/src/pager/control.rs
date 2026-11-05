//! The client of the control socket: `presiterm ctl` talks to a running presenter.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::deck;

/// Talk to a running presenter: one command line out, one reply line back.
pub fn ctl(control: Option<&Path>, command: &[String]) -> Result<String> {
    if command.is_empty() {
        bail!(
            "no command given: goto N | next | prev | reload | status | quit | font <pt|+N|-N|reset> (see `presiterm ctl --help`)"
        );
    }
    let mut stream = match control {
        Some(p) => UnixStream::connect(p).with_context(|| format!("connect to {}", p.display()))?,
        None => newest_presenter()?,
    };
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    writeln!(stream, "{}", command.join(" "))?;
    let mut reply = String::new();
    BufReader::new(&stream).read_line(&mut reply)?;
    if reply.starts_with("error") {
        bail!("{}", reply.trim());
    }
    Ok(reply.trim().to_string())
}

pub(super) fn newest_presenter() -> Result<UnixStream> {
    let mut socks: Vec<_> = std::fs::read_dir(std::env::temp_dir())?
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.starts_with("presiterm-") && deck::extension_is(&n, &["sock"])
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    socks.sort_by_key(|s| std::cmp::Reverse(s.0));
    for (_, path) in socks {
        match UnixStream::connect(&path) {
            Ok(s) => return Ok(s),
            Err(_) => {
                let _ = std::fs::remove_file(&path);
            },
        }
    }
    bail!("no running presenter found (start one, or pass --control <path>)")
}

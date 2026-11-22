//! The client of the control socket: `presiterm ctl` talks to a running presenter.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::deck;

/// The commands the control socket takes, as the errors of both ends name them.
pub(super) const COMMANDS: &str =
    "goto N | next | prev | reload | status | quit | font <pt|+N|-N|reset>";

/// Talk to a running presenter: one command line out, one reply line back.
pub fn ctl(control: Option<&Path>, command: &[String]) -> Result<String> {
    if command.is_empty() {
        bail!("no command given: {COMMANDS}; see `presiterm ctl --help`");
    }
    let mut stream = match control {
        Some(p) => UnixStream::connect(p).with_context(|| format!("connect to {}", p.display()))?,
        None => newest_presenter()?,
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .context("set the reply timeout")?;
    writeln!(stream, "{}", command.join(" ")).context("send to the presenter")?;
    let mut reply = String::new();
    BufReader::new(&stream)
        .read_line(&mut reply)
        .context("read the presenter's reply")?;
    // The presenter marks a refusal `error: ...`; the binary adds its own prefix, so drop this one.
    if let Some(refusal) = reply.trim().strip_prefix("error:") {
        bail!("{}", refusal.trim_start());
    }
    Ok(reply.trim().to_string())
}

pub(super) fn newest_presenter() -> Result<UnixStream> {
    let dir = std::env::temp_dir();
    let mut socks: Vec<_> = std::fs::read_dir(&dir)
        .with_context(|| format!("read {}", dir.display()))?
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
    bail!("no running presenter found; start one, or pass --control PATH")
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;

    use super::*;

    #[test]
    fn a_refusal_comes_back_without_the_error_prefix_of_the_reply() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("p.sock");
        let listener = UnixListener::bind(&path).expect("bind");
        let presenter = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut command = String::new();
            BufReader::new(&stream)
                .read_line(&mut command)
                .expect("read");
            writeln!(stream, "error: slide must be 1..9").expect("reply");
        });
        let message = format!(
            "{:#}",
            ctl(Some(&path), &["goto".to_string(), "12".to_string()]).expect_err("refused")
        );
        presenter.join().expect("presenter");
        assert_eq!(message, "slide must be 1..9");
    }
}

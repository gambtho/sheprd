//! Herdr 1 metadata events, separate from the terminal's binary endpoint.
use crate::{ConnectTarget, Result, Transport, ssh, wsl};
use serde_json::Value;
use std::{
    io::{
        self, BufRead, BufReader,
        ErrorKind::{Interrupted, InvalidData, TimedOut, WouldBlock},
        Read, Write,
    },
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// Dropping cancels only this reader; its worker reaps any owned API bridge.
pub struct Watcher(Arc<AtomicBool>, Instant);
impl Drop for Watcher {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
impl Watcher {
    pub fn retry_ready(&self) -> bool {
        self.0.load(Ordering::Acquire) && self.1.elapsed().as_secs() >= 60
    }

    pub fn new(
        target: ConnectTarget,
        mut received: impl FnMut(String, Instant) + Send + 'static,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let cancelled = stop.clone();
        let started = std::thread::Builder::new().name("herdr-metadata".into()).spawn(move || {
            let mut run = || -> Result<()> {
                let mut transport = open(&target, &cancelled)?;
                let stream = match &mut transport { Transport::Local(s) => s, Transport::Bridge(b) => b.stream_mut() };
                stream.set_read_timeout(Some(Duration::from_millis(200)))?;
                stream.set_write_timeout(Some(Duration::from_secs(1)))?;
                stream.write_all(b"{\"id\":\"metadata\",\"method\":\"events.subscribe\",\"params\":{\"subscriptions\":[{\"type\":\"workspace.metadata_updated\"}]}}\n")?;
                stream.flush()?;
                read_events(stream, &cancelled, &mut received).map_err(Into::into)
            };
            if let Err(error) = run() { tracing::debug!(kind = ?error.kind(), "metadata subscription unavailable"); }
            cancelled.store(true, Ordering::Release);
        });
        if let Err(error) = started {
            stop.store(true, Ordering::Release);
            tracing::warn!(kind = ?error.kind(), "metadata reader could not start");
        }
        Self(stop, Instant::now())
    }
}

fn open(target: &ConnectTarget, stop: &AtomicBool) -> Result<Transport> {
    match target {
        ConnectTarget::Wsl { distro, session } => Ok(wsl::wsl_api_bridge(distro, session)?.into()),
        ConnectTarget::Ssh { target, session } => {
            let command = ssh::script_command(target, &wsl::api_bridge_script(session))?;
            let (stream, child) = ssh::start_bridge(command, stop, |_, _| Ok(()))?
                .map_err(|closed| ssh::Error::Refused(closed.failure))?;
            Ok(ssh::Bridge { stream, child }.into())
        }
        _ => {
            let client = target.socket_path()?;
            let stem = client
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix("-client.sock"))
                .filter(|s| !s.is_empty())
                .ok_or(crate::Error::NoLocalSocket)?;
            Ok(crate::Stream::connect(client.with_file_name(format!("{stem}.sock")))?.into())
        }
    }
}

fn workspace(event: &Value) -> Option<&str> {
    (event["event"] == "workspace_metadata_updated").then_some(())?;
    event["data"]["workspace"]["workspace_id"]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 1024)
}

fn read_events(
    stream: impl Read,
    stop: &AtomicBool,
    mut received: impl FnMut(String, Instant),
) -> io::Result<()> {
    let mut reader = BufReader::with_capacity(4096, stream);
    let mut line = Vec::new();
    while !stop.load(Ordering::Acquire) {
        match (&mut reader)
            .take((65_536 - line.len()) as u64)
            .read_until(b'\n', &mut line)
        {
            Err(e) if matches!(e.kind(), TimedOut | WouldBlock | Interrupted) => continue,
            Err(e) => return Err(e),
            Ok(0) if line.is_empty() => return Ok(()),
            Ok(_) => {}
        }
        if line.last() != Some(&b'\n') {
            return Err(io::Error::new(InvalidData, "invalid metadata line"));
        }
        let value: Value =
            serde_json::from_slice(&line).map_err(|e| io::Error::new(InvalidData, e))?;
        if value.get("error").is_some_and(|e| !e.is_null()) {
            return Err(io::Error::new(InvalidData, "subscription rejected"));
        }
        if let Some(id) = workspace(&value) {
            received(id.to_owned(), Instant::now());
        }
        line.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests;

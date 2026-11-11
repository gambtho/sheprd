#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;
use std::{
    io::{self, Cursor, Read},
    sync::atomic::AtomicBool,
    time::Instant,
};

#[test]
fn failed_observers_retry_after_backoff_but_not_while_alive_or_just_started() {
    use std::time::Duration;
    let done = Arc::new(AtomicBool::new(true));
    let failed = Watcher(done.clone(), Instant::now() - Duration::from_secs(61));
    assert!(failed.retry_ready());
    let recent = Watcher(done.clone(), Instant::now());
    assert!(!recent.retry_ready());
    done.store(false, Ordering::Release);
    assert!(
        !failed.retry_ready(),
        "a healthy observer does not resubscribe"
    );
}

#[test]
fn partial_lines_survive_timeouts_and_same_value_reports_each_refresh() {
    struct Paused {
        bytes: Cursor<Vec<u8>>,
        paused: bool,
    }
    impl Read for Paused {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.bytes.position() == 17 && !self.paused {
                self.paused = true;
                return Err(TimedOut.into());
            }
            let end = if self.bytes.position() < 17 {
                17 - self.bytes.position() as usize
            } else {
                out.len()
            };
            let count = end.min(out.len());
            self.bytes.read(&mut out[..count])
        }
    }
    let event = b"{\"event\":\"workspace_metadata_updated\",\"data\":{\"workspace\":{\"workspace_id\":\"lane-7\",\"tokens\":[]}}}\n";
    let mut bytes =
        b"{\"id\":\"metadata\",\"result\":{\"type\":\"subscription_started\"}}\n".to_vec();
    bytes.extend(event);
    bytes.extend(event);
    let before = Instant::now();
    let mut updates = Vec::new();
    read_events(
        Paused {
            bytes: Cursor::new(bytes),
            paused: false,
        },
        &AtomicBool::new(false),
        |id, at| updates.push((id, at)),
    )
    .unwrap();
    assert_eq!(
        updates
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        ["lane-7", "lane-7"]
    );
    assert!(
        updates
            .iter()
            .all(|(_, at)| before <= *at && *at <= Instant::now())
    );
}

#[test]
fn oversized_and_rejected_subscriptions_are_not_accepted_as_updates() {
    let mut bytes = Cursor::new(vec![b'x'; 65_537]);
    let stop = AtomicBool::new(false);
    assert!(read_events(&mut bytes, &stop, |_, _| panic!("no metadata event")).is_err());
    assert!(bytes.position() <= 65_536);
    assert!(
        read_events(
            Cursor::new(b"{\"error\":{\"code\":\"unsupported\"}}\n"),
            &stop,
            |_, _| panic!("no metadata event")
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn an_api_eof_publishes_completion_so_the_owner_can_resubscribe() {
    use std::{
        io::{BufRead, BufReader},
        os::unix::net::UnixListener,
        time::Duration,
    };
    let dir = tempfile::tempdir().unwrap();
    let server = UnixListener::bind(dir.path().join("demo.sock")).unwrap();
    let mut watcher = Watcher::new(
        ConnectTarget::Socket(dir.path().join("demo-client.sock")),
        |_, _| panic!("no metadata report"),
    );
    server.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let (stream, _) = loop {
        match server.accept() {
            Ok(pair) => break pair,
            Err(e) if e.kind() == WouldBlock && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(5))
            }
            Err(e) => panic!("metadata reader did not connect: {e}"),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut request = String::new();
    BufReader::new(&stream).read_line(&mut request).unwrap();
    drop(stream);
    let deadline = Instant::now() + Duration::from_secs(3);
    while !watcher.0.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        watcher.0.load(Ordering::Acquire),
        "EOF must not leave the observer apparently alive"
    );
    assert!(!watcher.retry_ready(), "repeated failures cannot spin");
    watcher.1 = Instant::now() - Duration::from_secs(61);
    assert!(watcher.retry_ready());
}

#[cfg(unix)]
#[test]
fn local_api_subscription_is_read_only_and_drop_closes_only_its_socket() {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::net::UnixListener,
        sync::mpsc,
        time::Duration,
    };
    let dir = tempfile::tempdir().unwrap();
    let listener = UnixListener::bind(dir.path().join("demo.sock")).unwrap();
    let (sent, received) = mpsc::sync_channel(1);
    let watcher = Watcher::new(
        ConnectTarget::Socket(dir.path().join("demo-client.sock")),
        move |id, at| sent.send((id, at)).unwrap(),
    );
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line).unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["method"], "events.subscribe");
        assert_eq!(
            request["params"],
            json!({"subscriptions":[{"type":"workspace.metadata_updated"}]})
        );
        stream.write_all(b"{\"event\":\"workspace_metadata_updated\",\"data\":{\"workspace\":{\"workspace_id\":\"demo-7\"}}}\n").unwrap();
        let mut end = [0];
        assert_eq!(
            stream.read(&mut end).unwrap(),
            0,
            "watcher drop closes its connection"
        );
    });
    assert_eq!(
        received.recv_timeout(Duration::from_secs(3)).unwrap().0,
        "demo-7"
    );
    drop(watcher);
    server.join().unwrap();
}

#[test]
fn only_the_metadata_event_with_a_bounded_workspace_id_can_refresh_an_age() {
    let event = json!({"event":"workspace_metadata_updated","data":{
        "type":"workspace_metadata_updated", "workspace":{"workspace_id":"lane-7","tokens":[]}
    }});
    assert_eq!(workspace(&event), Some("lane-7"));
    let mut other = event.clone();
    other["event"] = json!("workspace_focused");
    assert_eq!(workspace(&other), None);
    for id in [json!(""), json!("x".repeat(1025)), json!(23), json!(null)] {
        let mut malformed = event.clone();
        malformed["data"]["workspace"]["workspace_id"] = id;
        assert_eq!(workspace(&malformed), None);
    }
    assert_eq!(
        workspace(&json!({"result":{"type":"subscription_started"}})),
        None
    );
}

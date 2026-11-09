use std::cell::RefCell;

use system_info::reader::Host;
use system_info::{SystemInfo, declaration};
use system_info::payload::{SCHEMA, Snapshot, Uptime};
use yoke_sdk::base::Error;
use yoke_sdk::plugin::{Activated, Event, Question};

/// A Session that records what the Plugin does on it.
#[derive(Default)]
struct Session {
    answers: RefCell<Vec<(String, Vec<u8>)>>,
    frames: RefCell<Vec<(String, Vec<u8>)>>,
}

impl system_info::Acts for Session {
    async fn answer(&self, q: &Question, payload: Vec<u8>) -> Result<(), Error> {
        self.answers.borrow_mut().push((q.id.clone(), payload));
        Ok(())
    }
    async fn emit(&self, stream: &str, payload: Vec<u8>) -> Result<(), Error> {
        self.frames.borrow_mut().push((stream.into(), payload));
        Ok(())
    }
}

fn bench() -> Host {
    Host {
        hostname: "bench".into(),
        os_name: "Arch Linux".into(),
        os_version: "rolling".into(),
        kernel_version: "6.18.57-1-lts".into(),
        cpu_brand: "AMD Ryzen".into(),
        physical_cores: 8,
        logical_cpus: 16,
        total_memory_bytes: 1 << 34,
    }
}

fn question(r#type: &str) -> Event {
    Event::Question(Question {
        id: "q1".into(),
        r#type: r#type.into(),
        payload: Vec::new(),
    })
}

fn activated(stream: &str) -> Event {
    Event::Activated(Activated {
        stream: stream.into(),
        transport: String::new(),
        address: String::new(),
    })
}

#[test]
fn the_manifest_is_the_one_the_declaration_generates() {
    let committed = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/com.yoke.system-info.yaml"))
        .expect("the Manifest is committed beside the crate");
    let generated = declaration().manifest();
    assert_eq!(
        committed, generated,
        "the Manifest is stale; run `cargo run -- --manifest > com.yoke.system-info.yaml`"
    );
}

#[tokio::test]
async fn the_question_is_answered_with_a_snapshot_of_the_host() {
    let s = Session::default();
    let mut p = SystemInfo::new(bench());
    p.handle(&s, &question("system.info"), || 42).await.unwrap();
    let answers = s.answers.borrow();
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].0, "q1");
    let snap = Snapshot::from_bytes(&answers[0].1).unwrap();
    assert_eq!(snap.schema, SCHEMA);
    assert_eq!(snap.hostname, "bench");
    assert_eq!(snap.logical_cpus, 16);
    assert_eq!(snap.uptime_seconds, 42);
}

#[tokio::test]
async fn nothing_is_emitted_before_the_core_activates_the_stream() {
    let s = Session::default();
    let p = SystemInfo::new(bench());
    p.tick(&s, || 1).await.unwrap();
    assert!(s.frames.borrow().is_empty());
}

#[tokio::test]
async fn the_uptime_flows_from_its_activation_to_its_stop() {
    let s = Session::default();
    let mut p = SystemInfo::new(bench());
    p.handle(&s, &activated("system.uptime"), || 10)
        .await
        .unwrap();
    p.tick(&s, || 11).await.unwrap();
    p.handle(
        &s,
        &Event::Stopped {
            stream: "system.uptime".into(),
        },
        || 12,
    )
    .await
    .unwrap();
    p.tick(&s, || 13).await.unwrap();
    let frames: Vec<u64> = s
        .frames
        .borrow()
        .iter()
        .map(|(stream, bytes)| {
            assert_eq!(stream, "system.uptime");
            Uptime::from_bytes(bytes).unwrap().uptime_seconds
        })
        .collect();
    assert_eq!(frames, vec![10, 11]);
}

#[tokio::test]
async fn an_undeclared_question_is_left_alone() {
    let s = Session::default();
    let mut p = SystemInfo::new(bench());
    p.handle(&s, &question("system.other"), || 0).await.unwrap();
    assert!(s.answers.borrow().is_empty());
}

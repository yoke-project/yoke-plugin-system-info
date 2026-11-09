//! system-info: a Plugin that tells what host it runs on.
//!
//! It answers one question, `system.info`, with a snapshot of the host, and publishes one stream,
//! `system.uptime`, a frame a second while the Core has it flowing. Stopping and restarting it are the
//! operator's (`yoke unit stop`, `yoke unit restart`), so it declares no command for either.

pub mod payload;
pub mod reader;

use crate::payload::{INFO_QUERY, UPTIME_STREAM, Uptime};
use yoke_sdk::base::Error;
use yoke_sdk::plugin::{Capability, Declaration, Event, Object, Question, Stream, Unit};

use reader::Host;

/// The Plugin's identity: what the Registry authorises, and the directory its Manifest is found in.
pub const ID: &str = "com.yoke.system-info";

/// What system-info says about itself: one question, one stream, and the capability governing each.
pub fn declaration() -> Declaration {
    Declaration {
        id: ID.into(),
        queries: vec![INFO_QUERY.into()],
        // A panel showing the uptime loses nothing worth having when a second is dropped.
        streams: vec![Stream {
            id: UPTIME_STREAM.into(),
            tolerates_loss: true,
            tolerates_reorder: false,
        }],
        capabilities: vec![
            Capability {
                name: "query.info.answer".into(),
                governs: Object::Query(INFO_QUERY.into()),
            },
            Capability {
                name: "stream.uptime.publish".into(),
                governs: Object::Stream(UPTIME_STREAM.into()),
            },
        ],
        ..Default::default()
    }
}

/// What system-info does on its Session. A started unit of the library does both.
pub trait Acts {
    fn answer(&self, q: &Question, payload: Vec<u8>) -> impl Future<Output = Result<(), Error>>;
    fn emit(&self, stream: &str, payload: Vec<u8>) -> impl Future<Output = Result<(), Error>>;
}

impl Acts for Unit {
    async fn answer(&self, q: &Question, payload: Vec<u8>) -> Result<(), Error> {
        Unit::answer(self, q, payload).await
    }
    async fn emit(&self, stream: &str, payload: Vec<u8>) -> Result<(), Error> {
        Unit::emit(self, stream, payload).await
    }
}

/// The Plugin's state: the host it read at start, and whether the uptime is flowing.
pub struct SystemInfo {
    host: Host,
    flowing: bool,
}

impl SystemInfo {
    pub fn new(host: Host) -> Self {
        SystemInfo {
            host,
            flowing: false,
        }
    }

    pub fn host(&self) -> &Host {
        &self.host
    }

    pub fn flowing(&self) -> bool {
        self.flowing
    }

    /// Does what one event of the Session asks, reading the uptime from `uptime`.
    pub async fn handle(
        &mut self,
        u: &impl Acts,
        event: &Event,
        uptime: impl Fn() -> u64,
    ) -> Result<(), Error> {
        match event {
            Event::Question(q) if q.r#type == INFO_QUERY => {
                u.answer(q, self.host.snapshot(uptime()).to_bytes()).await
            }
            // Only the Core activates a stream; the first frame goes at once, so a panel that started
            // it does not wait a second for the uptime.
            Event::Activated(a) if a.stream == UPTIME_STREAM => {
                self.flowing = true;
                self.tick(u, uptime).await
            }
            Event::Stopped { stream } if stream == UPTIME_STREAM => {
                self.flowing = false;
                Ok(())
            }
            // Nothing else is declared, so nothing else is granted, and the Core sends nothing else.
            _ => Ok(()),
        }
    }

    /// Emits the uptime, while the stream flows.
    pub async fn tick(&self, u: &impl Acts, uptime: impl Fn() -> u64) -> Result<(), Error> {
        if !self.flowing {
            return Ok(());
        }
        let frame = Uptime {
            uptime_seconds: uptime(),
        };
        u.emit(UPTIME_STREAM, frame.to_bytes()).await
    }
}

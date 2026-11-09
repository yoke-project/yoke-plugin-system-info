use std::time::Duration;

use system_info::reader::{Host, uptime};
use system_info::{SystemInfo, declaration};
use yoke_sdk::plugin::{Event, start};

#[tokio::main]
async fn main() {
    // `system-info --manifest` prints the Manifest the declaration generates, which is installed beside the
    // binary.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && args[1] == "--manifest" {
        print!("{}", declaration().manifest());
        return;
    }
    // The host is read before registering: what the Plugin answers with is ready once it is admitted.
    let mut plugin = SystemInfo::new(Host::read());
    let unit = match start(&declaration()).await {
        Ok(unit) => unit,
        Err(err) => {
            eprintln!("system-info: {err}");
            std::process::exit(1);
        }
    };
    // The library beats only once the Plugin has said how it is, so it says so as soon as it may.
    let line = format!("reading {}", plugin.host().hostname);
    if let Err(err) = unit.health(0, &line).await {
        eprintln!("system-info: {err}");
    }
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("system-info listens for SIGTERM");
    let mut second = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            event = unit.next() => match event {
                None => return,
                Some(Event::Ended(ended)) => {
                    if let Some(cause) = ended.cause {
                        eprintln!("system-info: the Session was revoked: {cause}");
                    }
                    return;
                }
                Some(event) => {
                    if let Err(err) = plugin.handle(&unit, &event, uptime).await {
                        eprintln!("system-info: {err}");
                    }
                }
            },
            _ = second.tick() => {
                if let Err(err) = plugin.tick(&unit, uptime).await {
                    eprintln!("system-info: {err}");
                }
            },
            _ = term.recv() => {
                // Asked to stop: close the Session in order, and leave once it has ended.
                let _ = unit.close().await;
                while let Some(event) = unit.next().await {
                    if matches!(event, Event::Ended(_)) {
                        break;
                    }
                }
                return;
            }
        }
    }
}

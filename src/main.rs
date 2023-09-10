#[macro_use]
extern crate rocket;

use std::fs::OpenOptions;
use std::io::{self, Read};
use std::path::Path;
use std::mem;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use rocket::fs::TempFile;
use rocket::tokio::{spawn, select};
use rocket::tokio::sync::mpsc;
use rocket::tokio::time::sleep;
use rocket::State;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    queue_dir: String,
    exec: String,
    debounce: Option<u64>,
}

async fn process_queue(mut queue: Vec<String>, config: Arc<Config>) {
    queue.sort();
    queue.dedup();

    let exec: Vec<_> = config.exec.split_whitespace().flat_map(|v| {
        if v == "%a" {
            queue.iter().map(|v| v.as_str()).collect::<Vec<_>>()
        } else {
            vec![v]
        }
    }).collect();

    println!("exec {:?}", exec);
    let (cmd, args) = exec.split_first().expect("no command to execute");
    let status = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .status();
    println!("command finished: {:?}", status);
}

async fn processor(mut msgq: mpsc::Receiver<String>, mut config: Arc<Config>) -> Result<(), rocket::Error> {
    let mut queued_files = vec![];
    let debounce_secs = Duration::from_secs(config.debounce.unwrap_or(5));

    loop {
        select! {
            msg = msgq.recv() => {
                match msg {
                    Some(s) => {
                        let out_path = Path::new(&config.queue_dir).join(s);
                        queued_files.push(format!("{}", out_path.display()));
                    }
                    None => return Ok(()),
                }
            }
            _ = sleep(debounce_secs) => {
                if !queued_files.is_empty() {
                    let queue = mem::take(&mut queued_files);
                    process_queue(queue, Arc::clone(&mut config)).await;
                }
            }
        }
    }
}

struct RocketState {
    msg_tx: mpsc::Sender<String>,
    config: Arc<Config>,
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[post("/<name>", data = "<file>")]
async fn index_post(
    state: &State<RocketState>,
    name: &str,
    mut file: TempFile<'_>,
) -> std::io::Result<()> {
    let out_path = Path::new(&state.config.queue_dir).join(name);
    file.persist_to(out_path).await?;
    state.msg_tx.send(name.to_owned()).await.unwrap();
    Ok(())
}

fn read_config() -> Result<Arc<Config>, io::Error> {
    let mut config_file = OpenOptions::new().read(true).open("config.toml")?;
    let mut config_vec = vec![];
    config_file.read_to_end(&mut config_vec)?;
    let config_str = std::str::from_utf8(&config_vec).expect("invalid utf-8");
    let config = toml::from_str(config_str).expect("invalid config");
    Ok(Arc::new(config))
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let mut config = read_config().expect("config");
    let (msg_tx, msg_rx) = mpsc::channel(10);

    spawn(processor(msg_rx, Arc::clone(&mut config)));

    let state = RocketState { msg_tx, config };

    rocket::build()
        .manage(state)
        .mount("/", routes![index, index_post])
        .launch()
        .await?;

    Ok(())
}

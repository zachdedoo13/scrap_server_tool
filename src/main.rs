use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use sysinfo::{Process, System};

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Settings {
    pub steam_game_id: String,
    pub save_location: String,
    pub backup_output_path: String,
    pub set_args: bool,
}


const SAVE_INTERVAL: Duration = Duration::from_secs(60*5);

fn main() -> Result<()> {
    println!("Starting server tool");

    let settings_json = std::fs::read("settings.json")?;
    let settings: Settings = serde_json::from_slice(&settings_json)?;
    println!("{settings:#?}");

    let mut sys = System::new_all();

    let scrap = is_open(&sys);
    if let Some(s) = scrap {
        println!("Killed already running processes");
        s.kill();
        sleep(Duration::from_secs(5));
    };

    println!("Starting event loop");
    let mut save_timer = Instant::now();
    loop {
        sys.refresh_all();
        if is_open(&sys).is_none() {
            println!("Not open");
            open_game(&settings)?;
            sleep(Duration::from_secs(30));
        } else {
            // println!("Open");
            if save_timer.elapsed() > SAVE_INTERVAL {
                save_timer = Instant::now();
                save_backup(&settings)?;
            }

            sleep(Duration::from_secs(10));
        }
    }
}

fn is_open(sys: &System) -> Option<&Process> {
    let p = sys.processes_by_exact_name(&OsStr::new("ScrapMechanic.exe")).next();
    p
}

fn save_backup(settings: &Settings) -> Result<()> {
    println!("Saving backup");

    let path = PathBuf::from_str(&settings.save_location)?;
    let file = std::fs::read(&path)?;

    let tse = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let time_hint = format!("{}", tse.as_millis());

    let o_name = path.file_name().unwrap().to_str().unwrap();
    let n_name = format!("{time_hint}#{o_name}");

    let end_path = PathBuf::from_str(&settings.backup_output_path)?;
    let end_path = end_path.join(&n_name);

    std::fs::write(&end_path, &file)?;

    Ok(())
}

fn open_game(settings: &Settings) -> Result<()> {
    println!("Opening scrap mechanic");

    let args = format!("-open \"{}\"", settings.save_location);
    println!("In property's set launch args to ={args}");

    if !settings.set_args {
        println!("Set launch args and swap set args in settings");
        sleep(Duration::from_secs(20));
        std::process::exit(1);
    }

    Command::new("cmd")
        .arg("/C")
        .arg(&format!("start steam://rungameid/{}", settings.steam_game_id))
        .spawn()?;

    Ok(())
}

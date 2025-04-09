
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use serenity::all::{EventHandler};
use sysinfo::{Process, System};

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Settings {
    pub steam_exe_location: String,
    pub steam_game_id: String,
    pub save_location: String,
    pub backup_output_path: String,
    pub auto_save_interval_sec: u64,
    pub discord_bot_token: String,
    pub full_restart_timer_min: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting server tool");

    let settings_json: Vec<u8> = std::fs::read("settings.json").unwrap();
    let settings: Settings = serde_json::from_slice(&settings_json).unwrap();
    println!("{settings:#?}");


    let bot = start_bot(&settings.discord_bot_token).await.unwrap();

    let _ = bot.send_msg(&format!("Bot online, starting server, settings = {settings:#?}")).await;

    let kill_intervel = Duration::from_secs(settings.full_restart_timer_min * 60);
    let save_interval = Duration::from_secs(settings.auto_save_interval_sec);
    let mut sys = System::new_all();

    let scrap: Option<&Process> = is_open(&sys);
    if let Some(s) = scrap {
        println!("Killed already running processes");
        s.kill();
        sleep(Duration::from_secs(5)).await;
    };

    println!("Starting event loop");
    let mut save_timer: Instant = Instant::now();
    let mut kill_timer: Instant = Instant::now();
    loop {
        sys.refresh_all();
        if is_open(&sys).is_none() {
            println!("Not open");
            let _ = bot.send_msg("Starting scrap mechanic").await;
            open_game(&settings).unwrap();
            sleep(Duration::from_secs(30)).await;
        } else {
            // println!("Open");
            if save_timer.elapsed() > save_interval {
                save_timer = Instant::now();
                if let Err(e) = save_backup(&settings, &bot).await {
                    println!("{e:?}");
                }
            }

            let c = FORCE_KILL.load(Ordering::SeqCst);

            if (kill_timer.elapsed() > kill_intervel) | c {
                if c {
                    FORCE_KILL.store(false, Ordering::SeqCst);
                    println!("Killing server from forced restart");
                    let _ = bot.send_msg("Triggerd forced restart").await;
                }
                println!("Running periodic scrap machanic kill");
                kill_timer = Instant::now();
                if let Err(e) = save_backup(&settings, &bot).await {
                    println!("{e:?}");
                }

                let scrap: Option<&Process> = is_open(&sys);
                if let Some(s) = scrap {
                    println!("Periodic scrap machanic kill");
                    s.kill();
                    let _ = bot.send_msg("Killing server, will restart momentarily").await;
                    sleep(Duration::from_secs(10)).await;
                };
            }

            sleep(Duration::from_secs(1)).await;
        }
    }
}

static FORCE_KILL: AtomicBool = AtomicBool::new(false);
pub fn trigger_force_restart() {
    println!("Triggering forced restart");
    FORCE_KILL.store(true, Ordering::SeqCst);
}

fn is_open(sys: &System) -> Option<&Process> {
    let p = sys.processes_by_exact_name(&OsStr::new("ScrapMechanic.exe")).next();
    p
}

async fn save_backup(settings: &Settings, bot: &Bot) -> Result<()> {
    println!("Saving backup");

    let path = PathBuf::from_str(&settings.save_location)?;
    let file = std::fs::read(&path)?;

    let tse = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let time_hint = format!("{}", tse.as_millis());

    let o_name = path.file_name().unwrap().to_str().unwrap();
    let n_name = format!("{time_hint}#{o_name}");

    let end_path = PathBuf::from_str(&settings.backup_output_path)?;
    let end_path = end_path.join(&n_name);

    let _ = std::fs::write(&end_path, &file);

    bot.send_file(&n_name, &file).await?;

    Ok(())
}

fn open_game(settings: &Settings) -> Result<()> {
    println!("Opening scrap mechanic");

    Command::new(&settings.steam_exe_location)
        .arg("-applaunch")
        .arg(&settings.steam_game_id)
        .arg("-open")
        .arg(&settings.save_location)
        .spawn().unwrap();

    Ok(())
}


pub use bot::*;
mod bot {
    use std::sync::{Arc, Mutex};
    use std::sync::mpsc::{channel, Sender};
    use serenity::all::{ChannelId, Context, EventHandler, GatewayIntents, GuildId, Http, Message, Ready};
    use serenity::{async_trait, Client};
    use tokio::spawn;
    use anyhow::Result;
    use serenity::builder::{CreateAttachment, CreateMessage};
    use crate::trigger_force_restart;

    const MAX_ATTACH_BYTES: usize = 5_000_000;
    pub struct Bot {
        pub http: Arc<Http>,
        pub server: GuildId,
        pub channel: ChannelId,
    }
    impl Bot {
        pub async fn send_msg(&self, msg: &str) -> Result<()> {
            let msg = CreateMessage::new().content(msg);

            self.channel.send_message(&self.http, msg).await?;

            Ok(())
        }

        pub async fn send_file(&self, name: &str, data: &[u8]) -> Result<()> {
            let init_msg = CreateMessage::new().content(&format!("New file upload [{name}]"));
            self.channel.send_message(&self.http, init_msg).await?;

            for (i, chunk) in data.chunks(MAX_ATTACH_BYTES).into_iter().enumerate() {
                println!("Sending attachment chunk {i}");
                let attach = CreateAttachment::bytes(chunk, format!("Chunk{i}.chunk"));
                let msg = CreateMessage::new().add_file(attach);
                self.channel.send_message(&self.http, msg).await?;
            };

            Ok(())
        }
    }

    struct BotHandler {
        channel_id: Mutex<Option<ChannelId>>,
        sender: Sender<(Arc<Http>, GuildId, ChannelId)>,
    }
    #[async_trait]
    impl EventHandler for BotHandler {
        async fn message(&self, ctx: Context, new_message: Message) {
            println!("{}", &new_message.content);
            let c = self.channel_id.lock().unwrap().unwrap();
            if new_message.channel_id == c {
                println!("{}", &new_message.content);
                if &new_message.content == "ForceRestartServer" {
                    trigger_force_restart()
                }
            }
        }
        async fn ready(&self, ctx: Context, data_about_bot: Ready) {
            println!("Starting bot event handler");
            let guilds = data_about_bot.guilds;
            // for gid in guilds {
            //             //     // let name = ctx.http.get_guild(gid.id).await.unwrap().name;
            //             //     // println!("{name:?}")
            //             // }

            let server = guilds.first().expect("Add the bot to only one server with admin").id;
            let channel = find_general_channel(&ctx, server).await.expect("No server_backup channel");

            *self.channel_id.lock().unwrap() = Some(channel);


            self.sender.send((ctx.http.clone(), server, channel)).unwrap()
        }
    }

    pub async fn find_general_channel(
        ctx: &Context,
        guild_id: GuildId,
    ) -> Option<ChannelId> {
        match ctx.http.get_channels(guild_id).await {
            Ok(channels) => {
                for channel in channels {
                    if channel.name == "server_backup" {
                        return Some(channel.id);
                    }
                }
                None
            }
            Err(e) => {
                eprintln!("Error fetching channels for guild {}: {:?}", guild_id, e);
                None
            }
        }
    }


    pub async fn start_bot(token: &str) -> Result<Bot> {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::GUILDS | GatewayIntents::MESSAGE_CONTENT; // Add the GUILDS intent

        let (s, r) = channel();


        let mut client = Client::builder(token, intents)
            .event_handler(BotHandler {
                channel_id: Mutex::new(None),
                sender: s,
            })
            .await?;

        spawn(async move {
            client.start().await.expect("Bot killed itself");
        });


        let (http, server, channel) = r.recv()?;

        let bot = Bot {
            http,
            server,
            channel,
        };

        Ok(bot)
    }

}

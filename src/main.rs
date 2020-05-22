use futures::StreamExt;
use nanoid;
use nanoid::generate;
use serde::{Deserialize, Serialize};
use std::convert::From;
use std::fs::read_to_string;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use telegram_bot::*;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub admins: Vec<i64>,
    pub bot_token: String,
    pub monolith_path: String,
    pub output_path: String,
    pub index_path: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub owner: String,
    pub author: String,
    pub title: String,
    pub description: Option<String>,
    pub date: Option<String>,
    pub path: String,
}

#[derive(Debug)]
enum ArchiveError {
    TelegramError(Error),
    InternalError(std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), ArchiveError> {
    let config_str = read_to_string("config.yml");
    if config_str.is_err() {
        panic!("No config file. Please add `config.yml`.");
    }
    let config: Config = serde_yaml::from_str(&config_str.unwrap()).unwrap();
    let api = Api::new(config.bot_token);
    let mut stream = api.stream();

    while let Some(update) = stream.next().await {
        let update = update?;
        match update.kind {
            UpdateKind::Message(message) => {
                if let MessageKind::Text {
                    ref data,
                    ref entities,
                } = message.kind
                {
                    if config
                        .admins
                        .iter()
                        .any(|&i| UserId::new(i) == message.from.id)
                    {
                        let text = data.as_str();
                        if text.starts_with('/') {
                            match text.split(' ').next() {
                                Some(command) => match command {
                                    "/update" => {
                                        Command::new("/usr/bin/hugo")
                                            .args(&[
                                                "-s",
                                                "hugo",
                                                "-d",
                                                &format!("../{}", config.output_path),
                                            ])
                                            .output()?;
                                        api.send(message.text_reply("Page updated.")).await?;
                                    }
                                    _ => {
                                        api.send(message.text_reply("Invalid command.")).await?;
                                    }
                                },
                                None => {
                                    api.send(message.text_reply("Invalid command.")).await?;
                                }
                            }
                        } else {
                            for e in entities.iter() {
                                if e.kind == MessageEntityKind::Url {
                                    let url = text[(e.offset as usize)
                                        ..(e.offset as usize + e.length as usize)]
                                        .to_string();
                                    let mut output =
                                        format!("{}/{}.html", config.output_path, generate(10));
                                    while Path::new(&output).exists() {
                                        output =
                                            format!("{}/{}.html", config.output_path, generate(10));
                                    }
                                    match Command::new(&config.monolith_path)
                                        .args(&[&url, "-s", "-o", &output])
                                        .output()
                                    {
                                        Err(e) => {
                                            api.send(message.text_reply(format!(
                                                "Internal error {:?} when trying to archive {}.",
                                                e, url
                                            )))
                                            .await?;
                                        }
                                        Ok(_) => {
                                            post_process(&output, &config.index_path).await?;
                                            api.send(message.text_reply("Url archived.")).await?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

async fn post_process(output: &str, dest: &str) -> Result<(), ArchiveError> {
    let file = File::open(output).unwrap();
    let mut item = Item {
        owner: String::default(),
        title: String::default(),
        author: String::default(),
        description: None,
        date: None,
        path: output.split("/").last().unwrap().to_string(),
    };
    let mut owner_flag = false;
    let mut prev = String::default();
    for line in BufReader::new(file).lines() {
        if let Ok(content) = line {
            if content.contains("property=\"og:description\"") {
                item.description = Some(content.split('\"').nth(3).unwrap().trim().to_string());
                continue;
            } else if content.contains("property=\"og:article:author\"") {
                item.author = content.split('\"').nth(3).unwrap().trim().to_string();
                continue;
            } else if content.contains("property=\"og:title\"") {
                item.title = content.split('\"').nth(3).unwrap().trim().to_string();
                continue;
            } else if content.contains("js_name") {
                owner_flag = true;
                continue;
            } else if owner_flag {
                item.owner = content.split('<').next().unwrap().trim().to_string();
                owner_flag = false;
                continue;
            } else if content.contains("getElementById(\"publish_time\")") {
                // time things
                item.date = Some(prev.split('\"').nth(5).unwrap().trim().to_string());
            }
            prev = content.to_string();
        }
    }
    let mut index_file = OpenOptions::new().append(true).open(dest).await.unwrap();
    index_file
        .write_all(
            &(format!(
                "
- owner: '{}'
  author: '{}'
  title: '{}'
  description: '{}'
  date: '{}'
  path: '{}'",
                &item.owner,
                &item.author,
                &item.title,
                &item.description.unwrap_or("".into()),
                &item.date.unwrap_or("".into()),
                &item.path
            )
            .bytes()
            .collect::<Vec<u8>>()),
        )
        .await?;
    Ok(())
}

impl From<Error> for ArchiveError {
    fn from(error: Error) -> Self {
        return ArchiveError::TelegramError(error);
    }
}

impl From<std::io::Error> for ArchiveError {
    fn from(error: std::io::Error) -> Self {
        return ArchiveError::InternalError(error);
    }
}

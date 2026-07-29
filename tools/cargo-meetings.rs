#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "cargo-meetings"
version = "0.1.0"
edition = "2024"
license = "MIT"

[dependencies]
chrono = "0.4"
chrono-tz = "0.10"
clap = { version = "4.5.59", features = ["derive"] }
reqwest = { version = "0.13.2", features = ["json", "form"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
---

use chrono::TimeZone;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use clap::{Args, Parser, Subcommand};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::json;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

macro_rules! error {
    ($($arg:tt)*) => {
        eprintln!("error: {}", format!($($arg)*));
        std::process::exit(1);
    };
}

#[derive(Parser)]
#[command(name = "cargo-meetings")]
#[command(about = "A tool for managing cargo meetings", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new meeting
    NewMeeting,
    /// Announce a meeting
    AnnounceMeeting,
    /// Save meeting notes
    SaveNotes(SaveNotesArgs),
}

#[derive(Args)]
struct SaveNotesArgs {
    /// Path to the local git clone of the cargo team repository
    #[arg(long)]
    cargo_team_repo: PathBuf,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::NewMeeting => new_meeting().await,
        Commands::AnnounceMeeting => announce_meeting().await,
        Commands::SaveNotes(args) => save_notes(args).await,
    }
}

fn create_hackmd_client() -> reqwest::Client {
    let Ok(token) = env::var("HMD_API_ACCESS_TOKEN") else {
        error!("HMD_API_ACCESS_TOKEN environment variable not set");
    };

    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    let auth_value = HeaderValue::from_str(&format!("Bearer {token}"))
        .expect("Invalid authorization header value");
    headers.insert(AUTHORIZATION, auth_value);
    reqwest::Client::builder()
        .default_headers(headers)
        .user_agent("cargo-meetings/1.0")
        .build()
        .expect("Failed to build HTTP client")
}

#[derive(Clone, Copy)]
enum MeetingDate {
    NextTuesday,
    Today,
    MostRecent,
}

fn note_title(which: MeetingDate) -> (String, String) {
    let today = chrono::Local::now().date_naive();
    let when = match which {
        MeetingDate::NextTuesday => get_next_tuesday(today),
        MeetingDate::Today => today,
        MeetingDate::MostRecent => get_most_recent_tuesday(today),
    };
    let date_str = when.format("%Y-%m-%d").to_string();
    (format!("Cargo team meeting notes {date_str}"), date_str)
}

async fn new_meeting() {
    let (title, date_str) = note_title(MeetingDate::NextTuesday);

    let body = json!({
        "parentFolderId": "a075b2d8-461a-4a76-b8c4-e3885bc59965",
        "suggestEditPermission": "disabled",
        "commentPermission": "disabled",
        "writePermission": "owner",
        "readPermission": "signed_in",
        "content": format!("---\nrobots: noindex, nofollow\ntags: meetings\n---\n\n# {title}\n\nhttps://meet.jit.si/CargoTeamMeeting\n\nOut:\nAttendees:\n\n## FCP Reminders\n\n## New RFCs\n\n"),
        "description": title,
        "tags": ["meetings"],
        "title": title
    });

    let client = create_hackmd_client();
    let response = client
        .post("https://api.hackmd.io/v1/teams/rust-cargo-team/notes")
        .json(&body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Successfully created meeting note for {date_str}");
                if let Ok(json_value) = resp.json::<serde_json::Value>().await {
                    if let Ok(pretty) = serde_json::to_string_pretty(&json_value) {
                        println!("{pretty}");
                    }

                    if let Some(publish_link) = json_value.get("publishLink") {
                        println!("\npublishLink: {publish_link}");
                    }
                } else {
                    error!("Failed to parse response as JSON");
                }
            } else {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                error!("Request failed with status {status}: {text}");
            }
        }
        Err(e) => {
            error!("Failed to send request: {e}");
        }
    }
}

fn get_next_tuesday(from_date: NaiveDate) -> NaiveDate {
    let current_weekday = from_date.weekday();
    let days_until_tuesday = match current_weekday {
        Weekday::Tue => 7, // If today is Tuesday, get next Tuesday
        Weekday::Wed => 6,
        Weekday::Thu => 5,
        Weekday::Fri => 4,
        Weekday::Sat => 3,
        Weekday::Sun => 2,
        Weekday::Mon => 1,
    };
    from_date + Duration::days(days_until_tuesday)
}

fn get_most_recent_tuesday(from_date: NaiveDate) -> NaiveDate {
    let current_weekday = from_date.weekday();
    let days_from_tuesday = match current_weekday {
        Weekday::Tue => 0, // If today is Tuesday, assume this is after the meeting
        Weekday::Wed => 1,
        Weekday::Thu => 2,
        Weekday::Fri => 3,
        Weekday::Sat => 4,
        Weekday::Sun => 5,
        Weekday::Mon => 6,
    };
    from_date - Duration::days(days_from_tuesday)
}

async fn fetch_notes(client: &reqwest::Client) -> Vec<serde_json::Value> {
    let response = client
        .get("https://api.hackmd.io/v1/teams/rust-cargo-team/notes")
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<Vec<serde_json::Value>>().await {
                    Ok(notes) => notes,
                    Err(e) => {
                        error!("Failed to parse notes as JSON: {e}");
                    }
                }
            } else {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                error!("Request failed with status {status}: {text}");
            }
        }
        Err(e) => {
            error!("Failed to send request: {e}");
        }
    }
}

fn find_note_by_title<'a>(notes: &'a [serde_json::Value], title: &str) -> &'a serde_json::Value {
    match notes.iter().find(|note| {
        note.get("title")
            .and_then(|t| t.as_str())
            .map(|t| t == title)
            .unwrap_or(false)
    }) {
        Some(n) => n,
        None => {
            error!("Could not find note with title '{title}'");
        }
    }
}

async fn announce_meeting() {
    let (title, date_str) = note_title(MeetingDate::Today);

    // Fetch notes and find today's
    println!("Fetching notes from HackMD...");
    let client = create_hackmd_client();
    let notes = fetch_notes(&client).await;
    let note = find_note_by_title(&notes, &title);

    let publish_link = match note.get("publishLink").and_then(|v| v.as_str()) {
        Some(link) => link.to_string(),
        None => {
            error!("Note does not have a 'publishLink' field");
        }
    };

    let naive_dt =
        chrono::NaiveDateTime::parse_from_str(&format!("{date_str}T07:00:00"), "%Y-%m-%dT%H:%M:%S")
            .expect("Failed to parse date");
    let tz = chrono_tz::America::Los_Angeles;
    let local_dt = tz
        .from_local_datetime(&naive_dt)
        .single()
        .expect("Failed to convert to local time");
    let time_tag = local_dt.format("%Y-%m-%dT07:00:00%:z").to_string();
    let now = chrono::Utc::now().with_timezone(&tz);
    let minutes_until = (local_dt - now).num_minutes();
    let message = format!(
        "@*T-cargo* Meeting will start in about {minutes_until} minutes at <time:{time_tag}>\n\
         Meeting info: https://doc.crates.io/contrib/team.html#meetings\nAgenda: {publish_link}"
    );

    let zulip_login = match env::var("ZULIP_LOGIN") {
        Ok(val) => val,
        Err(_) => {
            error!("ZULIP_LOGIN environment variable not set");
        }
    };
    let zulip_api_token = match env::var("ZULIP_API_TOKEN") {
        Ok(val) => val,
        Err(_) => {
            error!("ZULIP_API_TOKEN environment variable not set");
        }
    };

    let zulip_client = reqwest::Client::builder()
        .user_agent("cargo-meetings/1.0")
        .build()
        .expect("Failed to build Zulip HTTP client");

    #[derive(serde::Serialize)]
    struct ZulipSendMessage<'a> {
        #[serde(rename = "type")]
        type_: &'static str,
        to: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        topic: Option<&'a str>,
        content: &'a str,
    }

    let payload = ZulipSendMessage {
        type_: "stream",
        to: "t-cargo".to_string(),
        topic: Some("Cargo meeting"),
        content: &message,
    };

    let response = zulip_client
        .post("https://rust-lang.zulipchat.com/api/v1/messages")
        .basic_auth(&zulip_login, Some(&zulip_api_token))
        .form(&payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if let Ok(json_value) = resp.json::<serde_json::Value>().await {
                if let Ok(pretty) = serde_json::to_string_pretty(&json_value) {
                    println!("{pretty}");
                }
            } else {
                error!("Failed to parse Zulip response as JSON");
            }
        }
        Err(e) => {
            error!("Failed to send Zulip message: {e}");
        }
    }
}

async fn save_notes(args: SaveNotesArgs) {
    let cargo_team_repo = match fs::canonicalize(&args.cargo_team_repo) {
        Ok(p) => p,
        Err(e) => {
            error!(
                "Failed to canonicalize {}: {e}",
                args.cargo_team_repo.display()
            );
        }
    };

    let (note_title, date_str) = note_title(MeetingDate::MostRecent);

    println!("Fetching notes from HackMD...");
    let client = create_hackmd_client();
    let notes = fetch_notes(&client).await;
    let note = find_note_by_title(&notes, &note_title);

    let Some(content) = note.get("content").and_then(|c| c.as_str()) else {
        error!("Note does not have a 'content' field");
    };

    let mut content_without_frontmatter = strip_frontmatter(content);
    if !content_without_frontmatter.ends_with('\n') {
        content_without_frontmatter.push('\n');
    }

    let meetings_dir = cargo_team_repo.join("meetings").join("sync-meeting");
    if !meetings_dir.exists() {
        error!("Directory {meetings_dir:?} does not exist");
    }

    let file_path = meetings_dir.join(format!("{date_str}.md"));
    println!("Writing to file: {file_path:?}");

    if let Err(e) = fs::write(&file_path, content_without_frontmatter) {
        error!("Failed to write file: {e}");
    }

    println!("Running git add...");
    let output = Command::new("git")
        .arg("add")
        .arg(&file_path)
        .current_dir(&cargo_team_repo)
        .output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                error!(
                    "git add failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Err(e) => {
            error!("Failed to run git add: {e}");
        }
    }

    print!("Ready to commit and push. Press Return to continue: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    println!("Running git commit...");
    let commit_message = format!("Add {date_str}");
    let output = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(&commit_message)
        .current_dir(&cargo_team_repo)
        .output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                error!(
                    "git commit failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        }
        Err(e) => {
            error!("Failed to run git commit: {e}");
        }
    }

    println!("Running git push...");
    let output = Command::new("git")
        .arg("push")
        .current_dir(&cargo_team_repo)
        .output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                error!(
                    "git push failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            println!("Successfully pushed changes!");
        }
        Err(e) => {
            error!("Failed to run git push: {e}");
        }
    }
}

fn strip_frontmatter(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() || lines[0] != "---" {
        return content.to_string();
    }

    for (i, line) in lines.iter().enumerate().skip(1) {
        if *line == "---" {
            return lines[i + 1..].join("\n").trim_start().to_string();
        }
    }

    content.to_string()
}

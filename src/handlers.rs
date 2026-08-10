use matrix_sdk::{
    Client, Room, RoomState,
    config::SyncSettings,
    ruma::{
        OwnedRoomId, RoomId,
        events::room::{
            member::StrippedRoomMemberEvent,
            message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
        }
    }
};
use tokio::time::{Duration, sleep};
use chrono::{Local, Days, NaiveDateTime, NaiveDate, TimeDelta};
use tokio_rusqlite::Connection;

use regex::Regex;
use std::sync::OnceLock;
use rust_i18n::t;

use pulldown_cmark::{Parser, html};

// app crates
use crate::config::BotConfig;

// Compile regex only once
static REMINDER_REGEX: OnceLock<Regex> = OnceLock::new();

/// Default command
pub const DEFAULT_BOT_COMMAND: &str = "remind";

/// Day options in natural language.
enum NaturalDay {
    Today,
    Tomorrow,
}

impl NaturalDay {
    fn from_str(text: &str, i18n_today: &str, i18n_tomorrow: &str) -> Option<Self> {
        if text == i18n_today {
            Some(NaturalDay::Today)
        } else if text == i18n_tomorrow {
            Some(NaturalDay::Tomorrow)
        } else {
            None
        }
    }
}

/// Time options in natural language.
enum NaturalTime {
    Morning,
    Afternoon,
    Evening,
}

impl NaturalTime {
    fn from_str(text: &str, i18n_morning: &str, i18n_afternoon: &str, i18n_evening: &str) -> Option<Self> {
        if text == i18n_morning {
            Some(NaturalTime::Morning)
        } else if text == i18n_afternoon {
            Some(NaturalTime::Afternoon)
        } else if text == i18n_evening {
            Some(NaturalTime::Evening)
        } else {
            None
        }
    }
}

/// Reply to incoming message
pub async fn on_room_message(
    event: OriginalSyncRoomMessageEvent, 
    room: Room, 
    client: Client, 
    db: Connection,
    bot_config: BotConfig,
) {
    // We only want to log text messages in joined rooms.
    if room.state() != RoomState::Joined {
        return;
    }
    let MessageType::Text(text_content) = &event.content.msgtype else { return };
    let body = text_content.body.trim();

    // Vec of commands to activate bot
    let mut commands = vec![DEFAULT_BOT_COMMAND];
    let i18n_command = t!("reminder.command");
    commands.push(&i18n_command);

    // Command for regular expression
    let mut regex_str = String::with_capacity(256); 
    regex_str.push_str(&format!("^(?i)!(?:{}|", &commands.join("|")));
    
    if let Some(lang_cmd) = &bot_config.command {
        commands.push(&lang_cmd);
        regex_str.push_str(&regex::escape(&lang_cmd));
    };
    regex_str.push_str(r")\s+(?:(?P<datetime>(?P<day>\d{1,2})(?:\s+|\.|\|-)(?P<month>.+|\d{2})(?:\s+|\.|\|-)?(?P<year>\d{4})?)|(?P<day_natural>");

    // natural language expressions
    // date
    let i18n_today = t!("dates.today");
    let i18n_tomorrow = t!("dates.tomorrow");
    let i18n_days = vec![i18n_today.as_ref(), &i18n_tomorrow.as_ref()];
    regex_str.push_str(&i18n_days.join("|"));
    // time
    let i18n_morning = t!("dates.morning");
    let i18n_afternoon = t!("dates.afternoon");
    let i18n_evening = t!("dates.evening");
    let i18n_times = vec![i18n_morning.as_ref(), i18n_afternoon.as_ref(), i18n_evening.as_ref()];

    // natural time for regex
    regex_str.push_str(r"))(?:(?:\s+(?:в|at))?\s+((?P<hour>\d{2}):(?P<min>\d{2})|(?P<time_natural>");
    regex_str.push_str(&i18n_times.join("|"));
    regex_str.push_str(r")))?\s+(?P<text>.+)$");

    // Regular expression
    let re = REMINDER_REGEX.get_or_init(|| {
        Regex::new(&regex_str).unwrap()
    });

    // Check command word
    if let Some(caps) = re.captures(body) {
        let mut day: &str = &(Local::now().format("%d").to_string());
        let mut month_str: String = Local::now().format("%m").to_string();

        let today = Local::now().date_naive();
        let tomorrow_date = today + Days::new(1);
        let tomorrow = tomorrow_date.format("%d").to_string();

        // Day and month
        if let (Some(day_match), Some(month_match)) = (caps.name("day"), caps.name("month")) {
            day = day_match.as_str();
            // to_lowercase() returns String
            month_str = month_match.as_str().to_lowercase(); 
        } else if let Some(day_natural_match) = caps.name("day_natural") {
            let day_natural = day_natural_match.as_str();

            let natural_day = match NaturalDay::from_str(day_natural, &i18n_today, &i18n_tomorrow) {
                Some(d) => d,
                None => {
                    tracing::error!("day_natural error 1");
                    return;
                }
            };

            day = match natural_day {
                NaturalDay::Today => &day,
                NaturalDay::Tomorrow => &tomorrow,
                _ => {
                    tracing::error!("day_natural error 2");
                    return;
                }
            };            
        } else {
            tracing::error!("Error matching date");
            return;
        };

        let year_string = Local::now().format("%Y").to_string();
        let year_slice: &str = &year_string;
        let year = caps.name("year").map_or(year_slice, |m| m.as_str());
        
        // Time
        let mut hour: &str;
        let mut min: &str = "00";
        // Todo: change default time via .env or even special for user
        if let (Some(hour_match), Some(min_match)) = (caps.name("hour"), caps.name("min")) {
            hour = hour_match.as_str();
            min = min_match.as_str();
        } else if let Some(time_natural_match) = caps.name("time_natural") {
            let time_natural = time_natural_match.as_str().to_lowercase();

            let natural_time = match NaturalTime::from_str(&time_natural, &i18n_morning, &i18n_afternoon, &i18n_afternoon) {
                Some(d) => d,
                None => {
                    tracing::error!("time_natural error 1");
                    return;
                }
            };

            hour = match natural_time {
                NaturalTime::Morning => &bot_config.morning_time,
                NaturalTime::Afternoon => &bot_config.afternoon_time,
                NaturalTime::Evening => &bot_config.evening_time,
                _ => {
                    tracing::error!("time_natural error 2");
                    return;
                }
            };  

        } else {
            tracing::error!("Error matching time");
            return;
        };

        // let hour = caps.name("hour").map_or("09", |m| m.as_str());
        // let min = caps.name("min").map_or("00", |m| m.as_str());
        
        let reminder_text = &caps["text"];
        // shadowing
        let reminder_text = reminder_text.to_owned();

        // Vec of months names and Vec for months numbers matching
        let i18n_months_str = t!("dates.months");
        let i18n_months: Vec<&str> = i18n_months_str.split_whitespace().collect();
        let month_numbers = ["01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12"];

        // &* - reborrow; like Deref
        // https://stackoverflow.com/questions/41273041/what-does-combined-together-do-in-rust
        let month = match &*month_str {
            "01" | "1" => "01", "02" | "2" => "02", "03" | "3" => "03", "04" | "4" => "04", 
            "05" | "5" => "05", "06" | "6" => "06", "07" | "7" => "07", "08" | "8" => "08", 
            "09" | "9" => "09", "10" => "10", "11" => "11", "12" => "12",
            _ => {
                if let Some(m) = i18n_months.iter().position(|&name| name == month_str)
                    .map(|idx| month_numbers[idx]) {
                    m
                } else {
                    let _ = room.send(RoomMessageEventContent::text_plain("Unrecognizable month!")).await.unwrap();
                    tracing::error!("Month error");
                    return;
                }
            }
        };

        // Final date string
        let datetime_str = format!("{}-{}-{} {}:{}:00", year, month, day, hour, min);

        // Parse date
        if let Ok(target_time) = NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S") {
            
            let room_id_str = room.room_id().to_string();
            let text_for_db = reminder_text.clone().to_string();
            let time_for_db = datetime_str.clone();
            // chenge &RoomId to OwnedRoomId,
            // to_owned(): &str to String, Path to PathBuf
            let _owned_room_id: OwnedRoomId = room.room_id().to_owned();

            // 1. Save to SQLite
            let new_reminder = db.call(move |c| -> Result<super::reminder::Reminder, tokio_rusqlite::Error> {
                c.execute(
                    "INSERT INTO reminders (room_id, text, target_time) VALUES (?1, ?2, ?3)",
                    [&room_id_str, &text_for_db, &time_for_db],
                )?;
                
                // Reminder ID
                let reminder_id = c.last_insert_rowid();

                let room_id = RoomId::parse(&room_id_str)
                    .map_err(|err| tokio_rusqlite::rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
                
                let target_time = NaiveDateTime::parse_from_str(&time_for_db, "%Y-%m-%d %H:%M:%S")
                    .map_err(|err| tokio_rusqlite::rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;

                /*
                let reminder = super::reminder::Reminder {
                    id: reminder_id,
                    room_id: room_id,
                    text: text_for_db,
                    target_time: target_time,
                };
                Ok::<_, tokio_rusqlite::Error>(reminder)
                */
                /*
                Ok::<_, tokio_rusqlite::Error>(super::reminder::Reminder {
                    id: reminder_id,
                    room_id: room_id,
                    text: text_for_db,
                    target_time: target_time,
                })
                */
                Ok(super::reminder::Reminder {
                    id: reminder_id,
                    room_id: room_id,
                    text: text_for_db,
                    target_time: target_time,
                    status: super::reminder::ReminderStatus::Pending,
                })
            }).await;

            // 2. Tokio timer
            match new_reminder {
                Ok(new_reminder) => {
                    super::reminder::schedule_reminder(
                        client,
                        db.clone(),
                        new_reminder,
                    ).await;
                }
                Err(err) => {
                    println!("SQLite error: {:?}", err);
                    let _ = room.send(RoomMessageEventContent::text_plain("DB error in handlers.rs, sorry.")).await.unwrap();
                }
            }

            let date_str = format!("{}.{}.{}", day, month, year);
            let reminder_mes = t!("reminder.saved", date = date_str, hour = hour, min = min);
            let _ = room.send(RoomMessageEventContent::text_plain(reminder_mes)).await.unwrap();
        } else {
            let _ = room.send(RoomMessageEventContent::text_plain("Format error.")).await.unwrap();
        }
    } else if body.starts_with("!") {
        let body_content = body.strip_prefix("!").unwrap_or(body);
        let found = commands.iter().any(|&cmd| body_content.starts_with(cmd));
        if found {
            let tomorrow = Local::now().date_naive() + Days::new(1);
            let date = tomorrow.format("%d.%m.%Y").to_string();

            let welcome_msg = t!("welcome", date = date);
            let welcome_msg_html = markdown_to_html(&welcome_msg).await;
            let _ = room.send(RoomMessageEventContent::text_html(welcome_msg, welcome_msg_html.as_str())).await.unwrap();
        }
    }
}

/// Auto-join
pub async fn on_stripped_state_member(
    room_member: StrippedRoomMemberEvent,
    client: Client,
    room: Room,
) {
    if room_member.state_key != client.user_id().unwrap() {
        return;
    }

    tokio::spawn(async move {
        println!("Autojoining room {}", room.room_id());
        let mut delay = 2;

        while let Err(err) = room.join().await {
            // retry autojoin due to synapse sending invites, before the
            // invited user can join for more information see
            // https://github.com/matrix-org/synapse/issues/4345
            eprintln!("Failed to join room {} ({err:?}), retrying in {delay}s", room.room_id());

            sleep(Duration::from_secs(delay)).await;
            delay *= 2;

            if delay > 3600 {
                eprintln!("Can't join room {} ({err:?})", room.room_id());
                break;
            }
        }
        println!("Successfully joined room {}", room.room_id());
    });
}

/// Build regular expression
/*
async fn build_reminder_regex(i18n_today: &str, i18n_tomorrow: &str) -> Regex {
    // Vec of commands to activate bot
    let mut commands = vec![DEFAULT_BOT_COMMAND];
    let i18n_command = t!("reminder.command");
    commands.push(&i18n_command);

    // Command for regular expression
    let mut regex_str = String::with_capacity(256); 
    regex_str.push_str(&format!("^(?i)!(?:{}|", &commands.join("|")));
    
    if let Some(lang_cmd) = &bot_config.command {
        commands.push(&lang_cmd);
        regex_str.push_str(&regex::escape(&lang_cmd));
    };
    regex_str.push_str(r")\s+(?:(?P<datetime>(?P<day>\d{1,2})(?:\s+|\.|\|-)(?P<month>.+|\d{2})(?:\s+|\.|\|-)?(?P<year>\d{4})?)|(?P<day_natural>");

    // natural language expressions
    // date
    let i18n_today = t!("dates.today");
    let i18n_tomorrow = t!("dates.tomorrow");
    let i18n_days = vec![i18n_today.as_ref(), &i18n_tomorrow.as_ref()];
    regex_str.push_str(&i18n_days.join("|"));
    // time
    let i18n_morning = t!("dates.morning");
    let i18n_afternoon = t!("dates.afternoon");
    let i18n_evening = t!("dates.evening");
    let i18n_times = vec![i18n_morning.as_ref(), i18n_afternoon.as_ref(), i18n_evening.as_ref()];

    // natural time for regex
    regex_str.push_str(r"))(?:(?:\s+(?:в|at))?\s+((?P<hour>\d{2}):(?P<min>\d{2})|(?P<time_natural>");
    regex_str.push_str(&i18n_times.join("|"));
    regex_str.push_str(r")))?\s+(?P<text>.+)$");

    // Regular expression
    let re = REMINDER_REGEX.get_or_init(|| {
        Regex::new(&regex_str).unwrap()
    });
    return re
}
*/

/// Parse markdown i18n str to html String for matrix format
async fn markdown_to_html(markdown: &str) -> String {
    let parser = Parser::new(&markdown);
    let mut buffer = String::new();
    html::push_html(&mut buffer, parser);
    buffer
}
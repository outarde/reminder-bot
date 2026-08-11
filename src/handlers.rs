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
use crate::reminder::{Reminder, ReminderStatus};
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

/// Errors for build_datetime_str
#[derive(Debug)]
enum ReminderDateError {
    InvalidMonth,
    TimeInPast,
}

/// i18n keys
impl ReminderDateError {
    fn as_i18n_key(&self) -> &'static str {
        match self {
            ReminderDateError::InvalidMonth => "reminder.date-error",
            ReminderDateError::TimeInPast => "reminder.time-past-error",
        }
    }
}

/// Parsed data of user message for newly reminder
 #[derive(Debug)]
struct ParsedReminder {
    text: String,
    year: String,
    month: String,
    day: String,
    hour: String,
    min: String,
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

    // i18n
    let i18n_command = t!("reminder.command");
    let i18n_today = t!("dates.today");
    let i18n_tomorrow = t!("dates.tomorrow");
    let i18n_morning = t!("dates.morning");
    let i18n_afternoon = t!("dates.afternoon");
    let i18n_evening = t!("dates.evening");

    // Vecs for i18n
    let mut commands = vec![DEFAULT_BOT_COMMAND, &i18n_command];
    if let Some(lang_cmd) = &bot_config.command {
        commands.push(lang_cmd);
    }

    let i18n_days = vec![i18n_today.as_ref(), i18n_tomorrow.as_ref()];
    let i18n_times = vec![i18n_morning.as_ref(), i18n_afternoon.as_ref(), i18n_evening.as_ref()];

    // Make regular expression
    let re = build_reminder_regex(&commands, &i18n_days, &i18n_times);

    if let Some(caps) = re.captures(body) {
        let reminder_data = match parse_reminder_data(
            &caps, 
            &bot_config, 
            &i18n_today, 
            &i18n_tomorrow, 
            &i18n_morning, 
            &i18n_afternoon, 
            &i18n_evening
        ) {
            Some(data) => data,
            None => {
                tracing::error!("Error parsing regex: {:?}", caps);
                return;
            }
        };

        let datetime_str = match build_datetime_str(&reminder_data) {
            Ok(dt) => dt,
            Err(err) => {
                let err_msg = t!(err.as_i18n_key()); 
                let _ = room.send(RoomMessageEventContent::text_plain(err_msg)).await;
                
                tracing::error!("Date and time validation error: {:?} for {:?}", err, reminder_data);
                return;
            }
        };

        match save_reminder_to_db(&db, room.room_id(), reminder_data.text, datetime_str.clone()).await {
            Ok(new_reminder) => {
                super::reminder::schedule_reminder(client, db.clone(), new_reminder).await;
                
                let date_str = format!("{}.{}.{}", reminder_data.day, reminder_data.month, reminder_data.year);
                let reminder_mes = t!("reminder.saved", date = date_str, hour = reminder_data.hour, min = reminder_data.min);
                let _ = room.send(RoomMessageEventContent::text_plain(reminder_mes)).await;
            }
            Err(err) => {
                tracing::error!("SQLite error: {:?}", err);
            }
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
fn build_reminder_regex(
    commands: &[&str],
    i18n_days: &[&str],
    i18n_times: &[&str]
) -> Regex {
    let mut regex_str = String::with_capacity(256); 
    regex_str.push_str(&format!("^(?i)!(?:{}", &commands.join("|")));
    
    regex_str.push_str(r")\s+(?:(?P<datetime>(?P<day>\d{1,2})(?:\s+|\.|\|-)(?P<month>.+|\d{2})(?:\s+|\.|\|-)?(?P<year>\d{4})?)|(?P<day_natural>");
    regex_str.push_str(&i18n_days.join("|"));

    regex_str.push_str(r"))(?:(?:\s+(?:в|at))?\s+((?P<hour>\d{2}):(?P<min>\d{2})|(?P<time_natural>");
    regex_str.push_str(&i18n_times.join("|"));
    regex_str.push_str(r")))?\s+(?P<text>.+)$");

    Regex::new(&regex_str).unwrap()
}

/// Parse regex captions to ParsedReminder
fn parse_reminder_data(
    caps: &regex::Captures,
    bot_config: &BotConfig,
    i18n_today: &str,
    i18n_tomorrow: &str,
    i18n_morning: &str,
    i18n_afternoon: &str,
    i18n_evening: &str,
) -> Option<ParsedReminder> {
    let today_date = Local::now().date_naive();
    
    // Day and month
    let (day, month) = if let (Some(d), Some(m)) = (caps.name("day"), caps.name("month")) {
        (d.as_str().to_string(), m.as_str().to_lowercase())
    } else if let Some(d_nat) = caps.name("day_natural") {
        let natural_day = NaturalDay::from_str(d_nat.as_str(), i18n_today, i18n_tomorrow)?;
        match natural_day {
            NaturalDay::Today => (today_date.format("%d").to_string(), today_date.format("%m").to_string()),
            NaturalDay::Tomorrow => {
                let tomorrow = today_date + Days::new(1);
                (tomorrow.format("%d").to_string(), tomorrow.format("%m").to_string())
            }
        }
    } else {
        return None;
    };

    // Year
    let year = caps.name("year")
        .map(|y| y.as_str().to_string())
        .unwrap_or_else(|| today_date.format("%Y").to_string());

    // Time
    let (hour, min) = if let (Some(h), Some(m)) = (caps.name("hour"), caps.name("min")) {
        (h.as_str().to_string(), m.as_str().to_string())
    } else if let Some(t_nat) = caps.name("time_natural") {
        let natural_time = NaturalTime::from_str(&t_nat.as_str().to_lowercase(), i18n_morning, i18n_afternoon, i18n_evening)?;
        let h = match natural_time {
            NaturalTime::Morning => &bot_config.morning_time,
            NaturalTime::Afternoon => &bot_config.afternoon_time,
            NaturalTime::Evening => &bot_config.evening_time,
        };
        (h.to_string(), "00".to_string())
    } else {
        return None;
    };

    // Reminder's text
    let text = caps.name("text")?.as_str().to_string();

    Some(ParsedReminder { text, year, month, day, hour, min })
}

/// Build final string for DB and validate its time in the future
fn build_datetime_str(data: &ParsedReminder) -> Result<String, ReminderDateError> {
    let i18n_months_str = t!("dates.months");
    let i18n_months: Vec<&str> = i18n_months_str.split_whitespace().collect();
    let month_numbers = ["01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12"];

    let month = match data.month.as_str() {
        "01" | "1" => "01", "02" | "2" => "02", "03" | "3" => "03", "04" | "4" => "04", 
        "05" | "5" => "05", "06" | "6" => "06", "07" | "7" => "07", "08" | "8" => "08", 
        "09" | "9" => "09", "10" => "10", "11" => "11", "12" => "12",
        _ => {
            let idx = i18n_months.iter().position(|&name| name == data.month)
                .ok_or(ReminderDateError::InvalidMonth)?;
            month_numbers[idx]
        }
    };

    let datetime_string = format!("{}-{}-{} {}:{}:00", data.year, month, data.day, data.hour, data.min);
    
    // Check if time is in the future
    if let Ok(target_time) = NaiveDateTime::parse_from_str(&datetime_string, "%Y-%m-%d %H:%M:%S") {
        if target_time <= Local::now().naive_local() {
            return Err(ReminderDateError::TimeInPast); 
        }
    }

    Ok(datetime_string)
}

/// Save reminder to DB
// TODO: move to reminder.rs; use it in reminder.rs
async fn save_reminder_to_db(
    db: &Connection,
    room_id: &RoomId,
    text: String,
    datetime_str: String,
) -> Result<super::reminder::Reminder, tokio_rusqlite::Error> {
    let room_id_str = room_id.to_string();
    
    db.call(move |c| {
        c.execute(
            "INSERT INTO reminders (room_id, text, target_time) VALUES (?1, ?2, ?3)",
            [&room_id_str, &text, &datetime_str],
        )?;
        
        let reminder_id = c.last_insert_rowid();
        let parsed_room_id = RoomId::parse(&room_id_str)
            .map_err(|err| tokio_rusqlite::rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
        
        let target_time = NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|err| tokio_rusqlite::rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;

        Ok(Reminder {
            id: reminder_id,
            room_id: parsed_room_id,
            text,
            target_time,
            status: ReminderStatus::Pending,
        })
    }).await
}

/// Parse markdown i18n str to html String for matrix format
async fn markdown_to_html(markdown: &str) -> String {
    let parser = Parser::new(&markdown);
    let mut buffer = String::new();
    html::push_html(&mut buffer, parser);
    buffer
}

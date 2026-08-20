use matrix_sdk::{
    Client, Room, RoomState,
    ruma::{
        RoomId,
        events::room::{
            member::StrippedRoomMemberEvent,
            message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
        }
    }
};
use tokio::time::{Duration, sleep};
use chrono::{Local, Days, NaiveDateTime};
use tokio_rusqlite::Connection;

use regex::Regex;
use std::sync::{OnceLock, Arc};
use rust_i18n::t;

// app crates
use crate::reminder::{Reminder, ReminderStatus};
use crate::config::BotConfig;

// Compile regex only once
static REMINDER_REGEX: OnceLock<Regex> = OnceLock::new();
static MENTION_REGEX: OnceLock<Regex> = OnceLock::new();

enum BotCommand {
    Remind,
    List,
    Tz,
}

impl BotCommand {
    fn parse(text: &str, bot_config: BotConfig) -> Option<(Self, String)> {
        if !text.starts_with('/') {
            // Return None if bot can be activated only with command
            // and return "remind" command if can be activated without command
            if bot_config.on_command {
                return None;
            } else {
                return Some((BotCommand::Remind, text.to_string()))
            }
        }
        
        let remaining: String = text.chars().skip(1).collect();
        let mut parts = remaining.splitn(2, ' ');

        // command and args from text
        let cmd_str = parts.next()?.to_lowercase();
        let args = parts.next().unwrap_or("").to_string();

        // i18n aliases for commands
        let i18n_command = t!("reminder.command");

        if bot_config.remind_commands.contains(&cmd_str) || &i18n_command == &cmd_str {
            Some((BotCommand::Remind, args))
        } else if bot_config.list_commands.contains(&cmd_str) {
            Some((BotCommand::List, args))
        } else if bot_config.tz_commands.contains(&cmd_str) {
            Some((BotCommand::Tz, args))
        } else {
            None
        }
    }
}

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
    InvalidTime,
}

/// i18n keys
impl ReminderDateError {
    fn as_i18n_key(&self) -> &'static str {
        match self {
            ReminderDateError::InvalidMonth => "reminder.error.month",
            ReminderDateError::TimeInPast => "reminder.error.past-time",
            ReminderDateError::InvalidTime => "reminder.error.time",
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
    ctx: Arc<super::BotContext>
) {
    // check if we joined the room
    if room.state() != RoomState::Joined { return; }
    // check if message is not from bot account
    // can be changed if multi-step interaction with bot will be presented
    if event.sender == ctx.bot_id { return; }

    // check for text type of message
    let MessageType::Text(text_content) = &event.content.msgtype else { return };
    let mut body = text_content.body.trim().to_string();

    // if ctx.bot_config.on_mention is true, 
    // check for if bot was mentioned in public rooms (where more than 2 members joined or invited)
    if ctx.bot_config.on_mention && room.active_members_count() > 2 {
        if let Some(mentions) = &event.content.mentions {
            // clean body from makrdown if there is mention
            if mentions.user_ids.contains(&ctx.bot_id) {
                body = clean_from_mention(&body);
            }
            else {
                return;
            }
        } else {
            return;
        }
    }

    // Parse command
    let Some((command, args)) = BotCommand::parse(&body, ctx.bot_config.clone()) else { 
        return;
    };

    match command {
        BotCommand::Remind => {
            handle_remind(&args, room, &ctx.client, &ctx.db_conn, &ctx.bot_config).await;
        }
        BotCommand::List => {
            // handle_list(&room, &db).await;
            return;
        }
        BotCommand::Tz => {
            // handle_settings(&room, &bot_config).await;
            return;
        }
    }
}

/*
/// Reply to incoming message
pub async fn on_room_message_re(
    event: OriginalSyncRoomMessageEvent, 
    room: Room, 
    client: Client, 
    db: Connection,
    bot_config: BotConfig,
) {
    // check if we joined the room
    if room.state() != RoomState::Joined { return; }
    // check if message is not from bot account
    // can be changed if multi-step interaction with bot will be presented
    let sender = event.sender;
    let bot_id = client.user_id().unwrap();
    // let bot_id = owned_user_id!("@example:localhost");
    if sender == bot_id { return; }


    let MessageType::Text(text_content) = &event.content.msgtype else { return };
    let body = text_content.body.trim();

    // Parse command
    let Some((command, args)) = BotCommand::parse(body, bot_config.clone()) else { 
        return;
    };

    match command {
        BotCommand::Remind => {
            handle_remind(&args, room, client, db, bot_config).await;
        }
        BotCommand::List => {
            // handle_list(&room, &db).await;
            return;
        }
        BotCommand::Tz => {
            // handle_settings(&room, &bot_config).await;
            return;
        }
    }
}
*/

/// New reminder
pub async fn handle_remind(
    body: &str, 
    room: Room, 
    client: &Client, 
    db: &Connection,
    bot_config: &BotConfig,
) {
    // i18n
    let i18n_today = t!("dates.today");
    let i18n_tomorrow = t!("dates.tomorrow");
    let i18n_morning = t!("times.morning");
    let i18n_afternoon = t!("times.afternoon");
    let i18n_evening = t!("times.evening");
    let i18n_prepositions_str = t!("prepositions");

    let i18n_days = vec![i18n_today.as_ref(), i18n_tomorrow.as_ref()];
    let i18n_times = vec![i18n_morning.as_ref(), i18n_afternoon.as_ref(), i18n_evening.as_ref()];
    let i18n_prepositions: Vec<&str> = i18n_prepositions_str.split_whitespace().collect();

    // Make regular expression
    let re = build_reminder_regex(&i18n_days, &i18n_times, &i18n_prepositions);

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

        let target_time = match build_datetime_str(&reminder_data) {
            Ok(dt) => dt,
            Err(err) => {
                let err_msg = t!(err.as_i18n_key()); 
                let _ = room.send(RoomMessageEventContent::text_plain(err_msg)).await;
                
                tracing::error!("Date and time validation error: {:?} for {:?}", err, reminder_data);
                return;
            }
        };

        match save_reminder_to_db(&db, room.room_id(), reminder_data.text, target_time.clone()).await {
            Ok(new_reminder) => {
                super::reminder::schedule_reminder(client.clone(), db.clone(), new_reminder).await;

                let date_str = target_time.format("%d.%m.%Y");
                let reminder_mes = t!("reminder.saved", date = date_str, hour = reminder_data.hour, min = reminder_data.min);
                let _ = room.send(RoomMessageEventContent::text_plain(reminder_mes)).await;
            }
            Err(err) => {
                tracing::error!("SQLite error: {:?}", err);
            }
        }
    } else {
        let tomorrow = Local::now().date_naive() + Days::new(1);
        let date = tomorrow.format("%d.%m.%Y").to_string();

        let welcome_msg = t!("welcome", date = date);
        // for markdonw to text_html: use pulldown_cmark::{Parser, html};
        // let welcome_msg_html = markdown_to_html(&welcome_msg).await;

        let _ = room.send(RoomMessageEventContent::text_markdown(welcome_msg)).await.unwrap();
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
            tracing::error!("Failed to join room {} ({err:?}), retrying in {delay}s", room.room_id());

            sleep(Duration::from_secs(delay)).await;
            delay *= 2;

            if delay > 3600 {
                tracing::error!("Can't join room {} ({err:?})", room.room_id());
                break;
            }
        }
        tracing::info!("Successfully joined room {}", room.room_id());
    });
}

/// Build regular expression
fn build_reminder_regex(
    i18n_days: &[&str],
    i18n_times: &[&str],
    i18n_prepositions: &[&str]
) -> &'static Regex {
    REMINDER_REGEX.get_or_init(|| {
        let mut regex_str = String::with_capacity(256); 
        
        // [^\.\-\s]{1,15} in ?P<month> can be replaced with white list of months names
        regex_str.push_str(r"^(?i)(?:(?P<datetime>(?P<day>\d{1,2})(?:\s+|\.|\/|-)(?P<month>[^\.\-\s]{1,15}|\d{2})(?:\s|\.|\/|-)?(?P<year>\d{4})?)|(?P<day_natural>");
        regex_str.push_str(&i18n_days.join("|"));

        regex_str.push_str(r"))(?:(?:\s+(?<prep>at|");
        regex_str.push_str(&i18n_prepositions.join("|"));
        regex_str.push_str(r"))?\s+(((?P<hour>\d{2}):(?P<min>\d{2}))|(?P<time_natural>");
        regex_str.push_str(&i18n_times.join("|"));
        regex_str.push_str(r")))?\s+(?P<text>.+)$");
        //regex_str.push_str(r")|(?P<time_interval>(?<gap>\d{1,2})\s(?<step>минут|часов)) ))?\s+(?P<text>.+)$");

        Regex::new(&regex_str).unwrap()
    })
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
        let (h, m) = match natural_time {
            NaturalTime::Morning => &bot_config.morning.split_once(":")?,
            NaturalTime::Afternoon => &bot_config.afternoon.split_once(":")?,
            NaturalTime::Evening => &bot_config.evening.split_once(":")?,
        };
        (h.to_string(), m.to_string())
    } else {
        // TODO: Return +1 hour if day, month are today
        // let f_t = Local::now().checked_add_signed(TimeDelta::hours(1)).unwrap();
        // (f_t.format("%H").to_string(), f_t.format("%M").to_string())
        (super::config::DEFAULT_MORNING_TIME.to_string(), "00".to_string())
    };

    // Reminder's text
    let text = caps.name("text")?.as_str().to_string();

    Some(ParsedReminder { text, year, month, day, hour, min })
}

/// Build final string for DB and validate its time in the future
fn build_datetime_str(data: &ParsedReminder) -> Result<NaiveDateTime, ReminderDateError> {
    let mut month: String = String::from("");

    // Check if number
    if let Ok(m) = data.month.as_str().parse::<u32>() {
        if (1..=12).contains(&m) {
            month = format!("{:02}", m);
        }
    }
    // If word
    else {
        let i18n_months_str = t!("dates.months");
        let i18n_months: Vec<&str> = i18n_months_str.split_whitespace().collect();
        let month_numbers = ["01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12"];

        let idx = i18n_months.iter().position(|&name| {
            (name == data.month || name.starts_with(&data.month)) && data.month.len() >= 3
        }).ok_or(ReminderDateError::InvalidMonth)?;
        month = month_numbers[idx].to_string();
    }

    let datetime_string = format!("{}-{}-{} {}:{}:00", data.year, month.as_str(), data.day, data.hour, data.min);
    
    // Check if time can be parsed and in the future
    let target_time = NaiveDateTime::parse_from_str(&datetime_string, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| ReminderDateError::InvalidTime)?;
        
    if target_time <= Local::now().naive_local() {
        return Err(ReminderDateError::TimeInPast); 
    }

    Ok(target_time)
}

/// Save reminder to DB
// TODO: move to reminder.rs; use it in reminder.rs
async fn save_reminder_to_db(
    db: &Connection,
    room_id: &RoomId,
    text: String,
    target_time: NaiveDateTime,
) -> Result<super::reminder::Reminder, tokio_rusqlite::Error> {
    let room_id_str = room_id.to_string();
    let datetime_str = target_time.format("%Y-%m-%d %H:%M:%S").to_string();
    
    db.call(move |c| {
        c.execute(
            "INSERT INTO reminders (room_id, text, target_time) VALUES (?1, ?2, ?3)",
            [&room_id_str, &text, &datetime_str],
        )?;
        
        let reminder_id = c.last_insert_rowid();
        let parsed_room_id = RoomId::parse(&room_id_str)
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

/*
/// Parse markdown i18n str to html String for matrix format
async fn markdown_to_html(markdown: &str) -> String {
    let parser = Parser::new(&markdown);
    let mut buffer = String::new();
    html::push_html(&mut buffer, parser);
    buffer
}
*/

/// Clean message from makrdown link with user mention
// or can be implemented with input.strip_prefix()
fn clean_from_mention(text: &str) -> String {
    let re = MENTION_REGEX.get_or_init(|| {
        Regex::new(r"\[@[^\]]+\]\(https://matrix\.to/#/[^)]+\)").unwrap()
    });

    re.replace(text, "").trim().to_string()
}

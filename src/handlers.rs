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
use chrono::{Local, NaiveDateTime, NaiveDate, TimeDelta};
use tokio_rusqlite::Connection;

use regex::Regex;
use std::sync::OnceLock;
use rust_i18n::t;

// Compile regex only once
static REMINDER_REGEX: OnceLock<Regex> = OnceLock::new();

///
pub async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: Room, client: Client, db: Connection) {
    // We only want to log text messages in joined rooms.
    if room.state() != RoomState::Joined {
        return;
    }
    let MessageType::Text(text_content) = &event.content.msgtype else { return };
    let body = text_content.body.trim();


    // Todo: command word modification support 
    let re = REMINDER_REGEX.get_or_init(|| {
        Regex::new(
            r"^!(?:напомни|remind){1}\s+(?:(?P<datetime>(?P<day>\d{1,2})(?:\s+|\.|\\)(?P<month>[а-яёa-z]+|\d{2})(?:\s+|\.|\\)?(?P<year>\d{4})?)|(?P<day_natural>сегодня|завтра|today|tomorrow))(?:\s+(?:в|at)?\s+(?P<hour>\d{2}):(?P<min>\d{2}))?\s+(?P<text>.+)$"
        ).unwrap()
    }); 

    // Check command word
    if let Some(caps) = re.captures(body) {
        let mut day: &str = &(Local::now().format("%d").to_string());
        let mut month_str: &str = &(Local::now().format("%m").to_string());

        let today = Local::now().date_naive();
        let tomorrow: NaiveDate = today + TimeDelta::try_days(1).unwrap();
        let tomorrow = tomorrow.format("%d").to_string();

        // Named groups
        if let (Some(day_match), Some(month_match)) = (caps.name("day"), caps.name("month")) {
            day = day_match.as_str();
            month_str = month_match.as_str();
            //month_str = &caps["month"];
        } else if let Some(day_natural_match) = caps.name("day_natural") {
            let day_natural = day_natural_match.as_str();

            day = match day_natural {
                "сегодня" | "today" => &day, 
                "завтра" | "tomorrow" => &tomorrow,
                _ => {
                    room.send(RoomMessageEventContent::text_plain("Day error, sorry")).await.unwrap();
                    println!("day_natural error");
                    return;
                }
            };

            //month_str = "07";
        } else {
            room.send(RoomMessageEventContent::text_plain("Date error, sorry")).await.unwrap();
            println!("day_natural 2 error");
            return;
        };
        //let day = &caps["day"];
        //let month_str = &caps["month"];
        //let year = &caps["year"];

        let year_string = Local::now().format("%Y").to_string();
        let year_slice: &str = &year_string;
        let year = caps.name("year").map_or(year_slice, |m| m.as_str());
        
        // Time or default 09:00
        // Todo: change default time via .env or even special for user
        let hour = caps.name("hour").map_or("09", |m| m.as_str());
        let min = caps.name("min").map_or("00", |m| m.as_str());
        
        let reminder_text = &caps["text"];
        // shadowing
        let reminder_text = reminder_text.to_owned();

        // Todo: change
        let month = match month_str {
            "января" | "january" => "01", "февраля" | "february" => "02", "марта" | "march" => "03", "апреля" | "april" => "04",
            "мая" | "may" => "05", "июня" | "june" => "06", "июля" | "july" => "07", "августа" | "august" => "08",
            "сентября" | "september" => "09", "октября" | "october" => "10", "ноября" | "november" => "11", "декабря" | "december" => "12",
            "01" => "01", "02" => "02", "03" => "03", "04" => "04", "05" => "05", "06" => "06", 
            "07" => "07", "08" => "08", "09" => "09", "10" => "10", "11" => "11", "12" => "12",
            _ => {
                let _ = room.send(RoomMessageEventContent::text_plain("Unrecognizable month!")).await.unwrap();
                return;
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
            let reminder_mes = t!("saved", date = date_str, hour = hour, min = min);
            let _ = room.send(RoomMessageEventContent::text_plain(reminder_mes)).await.unwrap();
        } else {
            let _ = room.send(RoomMessageEventContent::text_plain("Format error.")).await.unwrap();
        }
    } else if body.starts_with("!напомни") | body.starts_with("!remind") {
        let date = Local::now().format("%Y.%m.%d").to_string();
        let mes = t!("welcome", date = date);
        let _ = room.send(RoomMessageEventContent::text_plain(mes)).await.unwrap();
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

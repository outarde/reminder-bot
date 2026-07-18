use matrix_sdk::{
    Client, Error, LoopCtrl, Room, RoomState,
    config::SyncSettings,
    ruma::{
        OwnedRoomId, RoomId,
        api::client::filter::FilterDefinition,
        events::room::message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
    },
};
use tokio_rusqlite::Connection;
use chrono::{Local, TimeZone, NaiveDateTime};
use std::{
    //sync::Arc, 
    collections::HashMap
};
use rust_i18n::t;

/// Reminder
#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: i64,
    pub room_id: OwnedRoomId,
    pub text: String,
    pub target_time: NaiveDateTime,
    pub status: ReminderStatus,
}

/// Statuses of Reminder
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum ReminderStatus {
    Pending = 0,
    Sent = 1,
    // missed can be when bot was offline
    Missed = 2,
    Recurring = 3,
    Cancelled = 4,
}

/*
impl TryFrom<i64> for ReminderStatus {
    type Error = String;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ReminderStatus::Pending),
            1 => Ok(ReminderStatus::Sent),
            2 => Ok(ReminderStatus::Missed),
            3 => Ok(ReminderStatus::Recurring),
            4 => Ok(ReminderStatus::Cancelled),
            _ => Err(format!("Unknown status: {}", value)),
        }
    }
}
*/
impl From<i64> for ReminderStatus {
    fn from(value: i64) -> Self {
        // change expect()
        ReminderStatus::try_from(value).expect("Invalid status in database")
    }
}

//ReminderStatus::try_from(db_value).unwrap_or(ReminderStatus::Pending);
//let status = ReminderStatus::from(db_value);

/// Database initialization.
pub async fn init_db() -> anyhow::Result<Connection> {
    // test
    /*
        let data_dir = dirs::data_dir()
            .context("No data_dir")?
            .join("persist_session");
        */

        //let birthday = parse_to_datetime("15-07-2026 at 14:16", Language::English)?;
        //println!("{:?}", birthday);

        //let now = Local::now().naive_local();
        //let date = from_human_time("15-07-2026 at 19:45", now).unwrap();
        //println!("{date}");

    // Path for DB file
    let path = dirs::data_dir().expect("No data_dir directory found").join(super::APP_FOLDER).join("reminders.db");
    let path_string: String = path.to_string_lossy().into_owned();

    // Open or create DB file
    let conn = Connection::open(&path_string).await?;
    
    // Create table
    // todo: table for rooms to customize timezone
    conn.call(|c| -> Result<(), tokio_rusqlite::Error> {
        let _ = c.execute(
            "CREATE TABLE IF NOT EXISTS reminders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                room_id TEXT NOT NULL,
                text TEXT NOT NULL,
                target_time TEXT NOT NULL,
                status INTEGER DEFAULT 0
            )",
            [],
        );
        let _ = c.execute(
            "CREATE INDEX IF NOT EXISTS idx_reminders_status ON reminders(status)",
            [],
        );
        //Ok::<_, tokio_rusqlite::Error>(())
        Ok(())
    }).await?;
    
    Ok(conn)
}

/// Scheduling reminder.
pub async fn schedule_reminder(
    client: Client,
    db: Connection,
    reminder: Reminder,
) {
    // NaiveDateTime to Local time
    let local_target = Local.from_local_datetime(&reminder.target_time).unwrap();
    let now = Local::now();

    // Calculate duration to remind
    let duration_to_wait = local_target.signed_duration_since(now);
    
    if duration_to_wait.num_seconds() <= 0 {
        tracing::error!("The reminder time has already passed!");
        return;
    }

    let std_duration = std::time::Duration::from_secs(duration_to_wait.num_seconds() as u64);

    // Tokio
    tokio::spawn(async move {
        tracing::info!("New reminder in {} sec", std_duration.as_secs());
        
        // Asynchronic sleep
        tokio::time::sleep(std_duration).await;

        // After sleep
        if let Some(room) = client.get_room(&reminder.room_id) {
            let reminder_text = t!("reminder.new", text = reminder.text);
            let _ = room.send(RoomMessageEventContent::text_plain(reminder_text)).await;
            // If the message was sent successfully, update the status!
            let _ = db.call(move |c| {
                c.execute("UPDATE reminders SET status = ?1 WHERE id = ?2", [ReminderStatus::Sent as i64, reminder.id])
            }).await;
            tracing::info!("Reminder #{} was sent", reminder.id);
        }
    });
}

/// Restores all future (or missed) reminders from the database.
pub async fn restore_reminders(client: Client, db: Connection) -> anyhow::Result<()> {
    // Get reminders

    /*
    let rows: Vec<(i64, String, String, String)> = db.call(|c| {
        let mut stmt = c.prepare("SELECT id, room_id, text, target_time FROM reminders")?;
        
        let mapped_rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut res = Vec::new();
        for row in mapped_rows {
            res.push(row?);
        }
        
        // Required type via Ok::<_, tokio_rusqlite::Error>
        Ok::<_, tokio_rusqlite::Error>(res)
    }).await?;
    */

    let reminders: Vec<Reminder> = db.call(|c| {
        let mut stmt = c.prepare("SELECT id, room_id, text, target_time FROM reminders WHERE status = 0")?;
        
        let mapped_rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let room_id_str: String = row.get(1)?;
            let text: String = row.get(2)?;
            let time_str: String = row.get(3)?;
            let status = ReminderStatus::Pending;

            // Parse strings to matrix RoomId and NativeDateTime
            let room_id = RoomId::parse(&room_id_str)
                .map_err(|err| tokio_rusqlite::rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
                
            let target_time = NaiveDateTime::parse_from_str(&time_str, "%Y-%m-%d %H:%M:%S")
                .map_err(|err| tokio_rusqlite::rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;

            Ok(Reminder {
                id,
                room_id,
                text,
                target_time,
                status,
            })
        })?;

        let mut res = Vec::new();
        for row in mapped_rows {
            res.push(row?);
        }
        
        Ok::<_, tokio_rusqlite::Error>(res)
    }).await?;

    let now = Local::now().naive_local();

    //let mut missed_by_room: HashMap<String, Vec<String>> = HashMap::new();
    let mut missed_by_room: HashMap<OwnedRoomId, Vec<Reminder>> = HashMap::new();

    for reminder in reminders {
        if reminder.target_time > now {
            schedule_reminder(
                client.clone(), 
                db.clone(), 
                reminder.clone()
            ).await;
        } else {
            missed_by_room
                .entry(reminder.room_id.clone())
                .or_default()
                .push(reminder);
        }
    }
    //let client_clone = client.clone();
    //let db_clone = db.clone();

    if !missed_by_room.is_empty() {
        tracing::info!("Sending missed reminders by room numbers: {}", missed_by_room.len());
        summary_missed(client.clone(), db.clone(), missed_by_room).await?;
    } else {
        tracing::info!("No missed reminders");
    }

    Ok(())
}

/// Sending a summary of missed reminders for each room. 
/// Currently, this is only needed if the bot was offline and unable to send reminders.
async fn summary_missed(
    client: Client, 
    db: Connection, 
    missed_by_room: HashMap<OwnedRoomId, Vec<Reminder>>
) -> anyhow::Result<()> {
    for (room_id, reminders) in missed_by_room {
        //let room_id_cloned = RoomId::parse(&room_id).clone()?;
        let client_clone = client.clone();
        let db_clone = db.clone();

        tokio::spawn(async move {
            if let Some(room) = client_clone.get_room(&room_id) {
                // Sorting by time and combining into a summary, can also be numbered.
                let mut sorted = reminders.clone();
                sorted.sort_by_key(|r| r.target_time);
                let summary = sorted
                    .iter()
                    .map(|r| format!("⚠️ {} (⏲️: {})", r.text, r.target_time))
                    .collect::<Vec<_>>()
                    .join("\n");

                let message = t!("reminder.summary", sum = summary);

                // todo urgent: change to UPDATE! one method!
                if room.send(RoomMessageEventContent::text_plain(message)).await.is_ok() {
                    let ids: Vec<i64> = reminders.iter().map(|r| r.id).collect();
                    let _ = db_clone.call(move |c| -> Result<(), tokio_rusqlite::Error> {
                        for id in ids {
                            c.execute("DELETE FROM reminders WHERE id = ?1", [id])?;
                            tracing::info!("Reminder #{} deleted from DB", id);
                        }
                        Ok(())
                    }).await;
                }
            }
        });
    }

    Ok(())
}

/// Remove reminder
// just reminder id? or array of ids?
async fn _remove_reminders(db: Connection, reminder: Vec<Reminder>) -> anyhow::Result<()> {
    Ok(())
}

//
/*
fn bulk_insert(conn: &Connection, items: &[(String, i32)]) -> Result<()> {
    // 1. Create transaction
    let tx = conn.transaction()?;

    // 2. Statement prepare
    let mut stmt = tx.prepare("INSERT INTO users (name, age) VALUES (?1, ?2)")?;

    // 3. Insert in cycle
    for item in items {
        stmt.execute(params![item.0, item.1])?;
    }

    // 4. Commit transaction
    tx.commit()?;

    Ok(())
}
*/
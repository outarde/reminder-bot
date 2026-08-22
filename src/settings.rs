use std::{
    sync::Arc
};
use anyhow::Result;
use matrix_sdk::{
	Room,
	ruma::events::{EmptyStateKey, macros::EventContent}
};
use serde::{Deserialize, Serialize};
use chrono_tz::Tz;

#[derive(Clone, Debug, Deserialize, Serialize, EventContent)]
#[ruma_event(type = "com.reminder-bot.room_timezone", kind = State, state_key_type = EmptyStateKey)]
pub struct RoomTimezoneContent {
    pub timezone: String,
}

pub async fn get_room_tz() {
	return;
}


pub async fn set_room_tz(
	room: Room,
	ctx: Arc<super::BotContext>,
	tz: Tz,
) -> Result<()> {
	let content = RoomTimezoneContent {
	    timezone: tz.to_string(),
	};

	// let state_key = client.user_id().unwrap().to_string(); 

	room.send_state_event(content).await?;

	Ok(())
}

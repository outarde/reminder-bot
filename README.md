<p align="center">
	<img src="docs/assets/logo.png" alt="Logo" width="400px">
</p>
<p align="center">
	<i>
		Logo credits:  <a href="https://www.flaticon.com/free-stickers/alarm-clock" title="alarm clock stickers">Alarm clock stickers created by Stickers - Flaticon</a>
	</i>
</p>
<br>

[![GitHub License](https://img.shields.io/github/license/outarde/reminder-bot)](https://github.com/outarde/reminder-bot/blob/main/LICENSE) [![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/outarde/reminder-bot/docker-publish.yml)](https://github.com/outarde/reminder-bot/actions) [![GitHub Tag](https://img.shields.io/github/v/tag/outarde/reminder-bot)](https://github.com/outarde/reminder-bot/releases) [![GitHub commit activity](https://img.shields.io/github/commit-activity/m/outarde/reminder-bot)](https://github.com/outarde/reminder-bot/commits/main/)

# Reminder Bot
A lightweight chatbot for reminders on Matrix servers with multilingual support. Schedule reminders without leaving the messenger in personal or group rooms.
## Key Features
- ⏲️ Create reminders with the `!remind` command
- 📆 Basic date variability with the words 'today', 'tomorrow', omitting the year and time
- 🔤 Multilingual support for commands and service messages
- 📋 Send a summary of missed reminders in each chat and room
- 🎹 Alternative command to create reminder
## Matrix Account Features
- Login to the bot's Matrix account with a password and token, automatic device verification and backup if this is the first device for the account, and receiving a recovery key
- Manual verification with a recovery key if the bot account has been logged in to before, and backup enabled via the command line (CLI)
- Verification of other devices on which the bot is authorized
- Reset all verification settings with the ability to save or delete the backup and receive a new recovery key
## Feature Roadmap
- [x] Alternative text for the bot activation command
- [ ] Sending a summary of sent and scheduled room or chat reminders on user command
- [ ] Deleting reminders
- [ ] Recurring reminders
- [ ] More advanced parsing of reminder date and time from user messages
- [ ] Adding European languages
- [ ] Adding other languages
- [ ] Time zone settings. Currently, reminders are sent according to a single time zone, set in the bot or server settings. It is expected that each user will be able to set their own alternative time zone, which will be taken into account when sending reminders.
- [ ] Creating reminders for one user for another
- [ ] Administrative module for cleaning up the reminder database
## Quick Start
### Docker
```
docker run -d --name reminder-bot --restart unless-stopped \
  -v reminder-bot:/app/data \
  -e MATRIX_HOMESERVER=homeserver-url \
  -e MATRIX_TOKEN=your-token \
  ghcr.io/outarde/reminder-bot:latest
```

For more persistent setup use [docker-compose.yml](https://github.com/outarde/reminder-bot/blob/main/docker/docker-compose.yml).

Set the environment variables as shown in [example.env](https://github.com/outarde/reminder-bot/blob/main/docker/example.env):
1. `MATRIX_HOMESERVER` — Matrix server address
2. `MATRIX_USERNAME` and `MATRIX_PASSWORD` — bot's username and password. Create a user via Matrix Authentication Service (MAS): `docker exec matrix-auth mas-cli manage register-user USERNAME --password PASSWORD`.

> [!IMPORTANT]
>  Make sure the bot's data folder is forwarded to the host in `volumes` section. Otherwise, the bot will create a new session each time it's started.
### Optional Variables

| Variable            | Description                                                                                                                                | Default                             |
| :------------------ | :----------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------- |
| `MATRIX_HOMESERVER` | Matrix homeserver address.                                                                                                                 | `Required`                          |
| `MATRIX_USERNAME`   | Bot's account username.                                                                                                                    | `Required`, or `MATRIX_TOKEN` instead |
| `MATRIX_PASSWORD`   | Bot's account password.                                                                                                                    | `Required`, or `MATRIX_TOKEN` instead |
| `MATRIX_TOKEN`      | Authentication token instead of a username and password. Generate a token via Element Admin or other service.                              | `None`                              |
| `MATRIX_DEVICE`     | An arbitrary name for the bot's device, which will be visible in the server's admin panel and in the bot's device list.                    | `reminder-bot-device`               |
| `TZ`                | Timezone (e.g. `Europe/Paris`). Use values from the [list](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones#List).             | `UTC`                               |
| `LANG_APP`          | Language for bot messages. As of version 0.2.2, the following languages are available: en — English, ru — Russian. Applies to all users. | `en`                                |

## Usage
### Start a Chat
Create a conversation with the bot or add it to a room. Send the `!remind` command to get help:
>I'm a reminder bot. Send me a reminder in this format: !remind me \<date and time\> \<reminder text\> You can replace the month number with its name, use words "today" or "tomorrow". If no time was specified the reminder will arrive at 09:00. I'll send it to you in a shared or private chat.

> [!TIP]
> You can omit the preposition "at" between the date and time.
### Create a Reminder
Example commands:
- `!remind 19.08.2026 at 10:00 Buy milk`
- `!remind 19-08 Pet a cactus` — creates a reminder for August 19th of this year at 9am.
- `!remind 19 August 21:30 plant a tree`
- `!remind tomorrow be kind with people`

> [!WARNING]
> Currently, the American format of writing the month and then the day are not supported, as is the 12-hour system.

### Summary
After the container with bot or bot itself restarts, all reminders are restored from the local database. Reminders that weren't sent are sent to the user or room as a *summary* of missed reminders. Soon, it will be possible to request the summary manually.
### Deletion
After reminders are sent, they are not deleted from the database but marked as sent. To delete all sent reminders, the server administrator must use the `cleanup` command (in development).

> [!IMPORTANT]
> Reminders are stored unencrypted.

## Matrix Verification
Without verification, every bot message will be marked with an exclamation point in most clients.

<p>
	<img src="docs/assets/ElementX-Screenshot1.jpg" alt="ElementX Screenshot" width="480px">
</p>

For example, the Element X will warn: 
>Encrypted by a device not verified by its owner.

Users will also receive a warning before sending their first message to the bot.
### Enabling a New Account
The easiest way is to create a new account for the bot. The account will be backed up and verified automatically. You'll then see the *recovery key* in the bot logs, which you'll need to save. The key will also be saved in the `recovery.json` file in the bot's session folder (which should have been forwarded to the host in step 1 of Quick Start).

> [!IMPORTANT]
> Keep your recovery key in a safe place!
### Enabling a Previously Used Account
To verify a new device, you will need a *recovery key* you received when logging in through another device or new matrix account. Recovery by *passphrase* is not supported and will likely never be supported, as it encrypts the same recovery key.
#### Step 1. Set your Recovery Key
First, pass the recovery key. Here are the methods for passing the recovery key, in descending order of priority:
1. Write it as a command flag: `recover --recovery-key=your-recovery-key`.
2. Write the recovery key in the `.env` file: `MATRIX_RECOVERY=your-recovery-key`.
3. Move the `recovery.json` file to the bot session folder if you have already run the container on another machine and obtained a recovery key or recovered your account using it.
#### Step 2. Run the Command
Then run the recovery command. In `docker-compose.yml` add:
```yaml
command: ["recover"]
```
Or, if you want to specify the key directly:
```yaml
command: ["recover", "--recovery-key", "your-recovery-key"]
```

If verification is successful, you will see a corresponding message in the logs and the recovery key will be written to the `recovery.json` file. After this, remove `command` from `docker-compose.yml` and restart the bot.

## Matrix Recovery
### Verification with Re-creation of Backup
You can use the `recover` command with the `--fix-backup` flag to automatically create a new backup if the previous one is missing chat encryption keys (key backup), as described in the [Matrix Rust SDK documentation](https://docs.rs/matrix-sdk/latest/matrix_sdk/encryption/recovery/struct.Recovery.html#method.recover_and_fix_backup). This is likely useful if the bot started some chats on a new device before verification. The recovery key will also be required. Remove the `--recovery-key` or `-r` flag from the commands below if you don't need to pass the key directly.

Example for Docker:
```yaml
command: ["recover", "--recovery-key=your-recovery-key", "--fix-backup"]
```
Same as:
```yaml
command: recover -r your-recovery-key --fix-backup
```
Or in a list:
```yaml
command:
- recover
- -r
- your-recovery-key
- --fix-backup
```

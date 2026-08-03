# Reminder Bot for Matrix

# Key Features
- Create reminders with the `!remind` command in private and public rooms
- Basic command variability: use of the words `today`, `tomorrow`, omitting the year and time
- Basic multilingual support: English and Russian for commands and service messages
- Send a summary of missed reminders in each chat and room
## Matrix Account Features
- Login to the bot's Matrix account with a password and token, automatic device verification and backup if this is the first device for the account, and receiving a recovery key
- Manual verification with a recovery key if the bot account has been logged in to before, and backup enabled via the command line
- Verification of other devices on which the bot is authorized
- Reset all verification settings with the ability to save or delete the backup and receive a new recovery key
# Feature Roadmap
- [ ] Alternative text for the bot activation command, set in the bot settings
- [ ] Sending a summary of sent and scheduled room or chat reminders on user command
- [ ] Deleting reminders
- [ ] Recurring reminders
- [ ] More advanced parsing of reminder date and time from user messages
- [ ] Adding European languages
- [ ] Adding other languages
- [ ] Time zone settings. Currently, reminders are sent according to a single time zone, set in the bot or server settings. It is expected that each user will be able to set their own alternative time zone, which will be taken into account when sending reminders
- [ ] Creating reminders for one user for another
- [ ] Administrative module for cleaning up the reminder database
# Quick Start
## Docker
```
docker run -d --name reminder-bot --restart unless-stopped \
  -v reminder-bot:/app/data \
  -e MATRIX_HOMESERVER=homeserver-url \
  -e MATRIX_TOKEN=your-token \
  ghcr.io/outarde/reminder-bot:latest
```
### Docker Compose

1. Use [docker-compose.yml](https://github.com/outarde/reminder-bot/blob/main/docker/docker-compose.yml). Make sure the bot's data folder is forwarded to the host in `volumes` section. Otherwise, the bot will create a new session each time it's started.
2. Set the environment variables as shown in [example.env](https://github.com/outarde/reminder-bot/blob/main/docker/example.env):
	1. `MATRIX_HOMESERVER` — Matrix server address
	2. `MATRIX_USERNAME` and `MATRIX_PASSWORD` — Bot username and password. Create a user via Matrix Authentication Service (MAS): `docker exec matrix-auth mas-cli manage register-user USERNAME --password PASSWORD --yes`

Optional variables:
- `MATRIX_TOKEN` — you can use a token instead of a username and password. Generate a login token via Element Admin.
- `MATRIX_DEVICE` — an arbitrary name for the bot's device, which will be visible in the server's admin panel and in the bot's device list.
- `TZ` — specify a time zone. Use values from the [list](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones#List).
- `LANG_APP` — select a language. As of version 0.2.1, the following languages ​​are available: en — English, ru — Russian.

## Verification
If you've logged into the bot account before, you need to verify the new device that appears when the bot is activated in the Docker environment. This requires the *recovery key* you received when logging in through another device or new matrix account.
### Enabling a New Account
The easiest way is to create a new account for the bot. The account will be backed up and verified automatically. You'll then see the *recovery key* in the bot logs, which you'll need to save. The key will also be saved in the `recovery.json` file in the bot's session folder (which should have been forwarded to the host in step 1 of Quick Start).

Keep your recovery key in a safe place!
### Enabling a Previously Used Account
To verify a new device, you will need a *recovery key*. Recovery by *passphrase* is not supported and will likely never be supported, as it encrypts the same recovery key.

First, pass the recovery key. Here are the methods for passing the recovery key, in descending order of priority:
1. Write it as a command flag: `recover --your-recovery-key`.
2. Write the recovery key in the `.env` file: `MATRIX_RECOVERY=your-recovery-key`.
3. Move the `recovery.json` file to the bot session folder if you have already run the container on another machine and obtained a recovery key or recovered your account using it.

Then run the recovery command. In `docker-compose.yml` add:
```yaml
command: ["recover"]
```
Or, if you want to specify the key directly:
```yaml
command: ["recover", "--recovery-key", "your-recovery-key"]
```

If verification is successful, you will see a corresponding message in the logs and the recovery key will be written to the `recovery.json` file. After this, remove `command` from `docker-compose.yml` and restart the bot.
#### Verification with re-creation of backup
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

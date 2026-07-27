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
You will most likely want to configure the bot in the same way as you configured the Matrix server in Docker.
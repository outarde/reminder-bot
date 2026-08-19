# Installation and Configuration
## Required Settings
Required settings are stored as environment variables. These are the parameters responsible for authorization. Without these, you can only run commands that do not connect to the Matrix server.

| Variable            | Description                                                                                                   | Example                    |
| :------------------ | :------------------------------------------------------------------------------------------------------------ | :------------------------- |
| `MATRIX_HOMESERVER` | Matrix homeserver address.                                                                                    | `https://matrix.org`       |
| `MATRIX_USERNAME`   | Bot's account username.                                                                                       | `@reminder-bot:matrix.org` |
| `MATRIX_PASSWORD`   | Bot's account password.                                                                                       | `mypassword`               |
| `MATRIX_TOKEN`      | Authentication token instead of a username and password. Generate a token via [Element Admin](https://github.com/element-hq/element-admin) or other service. | `mpt_mytoken`              |

You can set variables in the [docker-compose.yml](https://github.com/outarde/reminder-bot/blob/main/docker/docker-compose.yml):
```yaml
environment:
  - MATRIX_HOMESERVER=https://matrix.org
  - MATRIX_TOKEN=mpt_mytoken
```
Or in the [.env file](https://github.com/outarde/reminder-bot/blob/main/docker/example.env) in the root of the Docker container folder:
```
MATRIX_HOMESERVER=https://matrix.org
MATRIX_USERNAME=@reminder-bot:matrix.org
MATRIX_PASSWORD=mypassword
```
---
The bot also has an optional variable `MATRIX_DEVICE` that specifies the device name. This name is displayed on the user's device list page and in the admin panel and does not affect the bot's use. Its default value is `reminder-bot-device`.

## Bot’s Optional Settings
Additional settings are stored in the `config.yml`/`config.yaml` file in a folder or volume that you have bound to the `/app/data/reminder_bot` folder inside the container. The repository has default [config.example.yml](https://github.com/outarde/reminder-bot/blob/main/docker/config.example.yml).

<details>
<summary>Interactive configurator</summary>

The interactive configurator walks you through a series of questions to create a settings file and saves it to disk. Run it via the command line:
```
docker run --rm -it -v ~/bot-data:/app/data/reminder_bot ghcr.io/outarde/reminder-bot:latest setup-config
```
The settings file will be saved in `~/bot-data` - in this case, in the folder in the user's home directory.

If you have already created a bot container, run the command where `reminder-bot` is the name of your container:

`docker exec -it reminder-bot setup-config`

Confirm that you want to create or overwrite a settings file.
</details>

| Field             | Description                                                                                                                                                                                          | Default        |
| :---------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------- |
| `lang`            | Language for bot commands and messages. See below for a list of available languages. Applies to all users.                                                                                           | `en`           |
| `remind_commands` | Aliases that override the standard bot invocation command. These are formatted as a list. The command for the selected language is available regardless of this variable.                            | `[ "remind" ]` |
| `list_commands`   | Same for the reminders list command.                                                                                                                                                                 | `[ "list" ]`   |
| `tz_commands`     | Same for the timezone set up command.                                                                                                                                                                | `[ "tz" ]`     |
| `on_command`      | Activate the bot only when a command is presented at the beginning of a message: `!remind ...`. If disabled, any text sent to the bot, except for other commands, will be treated as a new reminder. | `true`         |
| `on_mention`      | Activation of the bot in chats with more than two active or invited participants only when it is mentioned: `@reminder-bot:matrix.org ...`.                                                          | `false`        |
| `morning`         | The time that is considered morning. Please follow the format `%H:%M`, otherwise you will see a general error `Error parsing regex` only when the bot tries to access the variables.                 | `09:00`        |
| `afternoon`       | The time that is considered afternoon.                                                                                                                                                               | `14:00`        |
| `evening`         | The time that is considered evening.                                                                                                                                                                 | `19:00`        |

> [!NOTE]
>  New versions may contain incompatible configuration changes. We'll reflect these in the releases notes.

## Language
### List of Available Languages
**v0.4.0:**
- `en` English 🇬🇧,
- `de` German 🇩🇪,
- `fr` French 🇫🇷,
- `it` Italian 🇮🇹,
- `es` Spanish 🇪🇸,
- `sv` Swedish, aka IKEAish 🇸🇪,
- `pl` Polish 🇵🇱,
- `cs` Czech 🇨🇿,
- `fi` Finnish 🇫🇮,
- `ja` Japanese 🇯🇵,
- `zh` Chinese Simplified 🇨🇳,
- `ru` Russian 🇷🇺,
- `uk` Ukrainian 🇺🇦.

If you notice an incorrect translation or would like to request an other language, please [report it.](https://github.com/outarde/reminder-bot/issues)
### Using a Custom Translation File
A custom translation file is a great way to add a language that isn't yet in the bot, or to customize an existing translation to suit your needs, for a themed homeserver or special occasion 🎃!

**Step one.** Create a `locales` folder in the folder already bound to `/app/data/reminder_bot`.

**Step two.** Create an `app.yml` file inside it or download alternative translation files from the `/docs/locales` folder of this repository. File [app.flat.yml](https://github.com/outarde/reminder-bot/blob/main/docs/locales/app.flat.yml) contains default translation in a proper flat structure.

**Step three.** Add your translations, checking the keys from the [standard localization file](https://github.com/outarde/reminder-bot/blob/main/locales/app.yml).

> [!IMPORTANT]
>  Use the format of a custom translation file, where **the language code comes first**, and then the keys. For the language code, use the [standard language codes](https://en.wikipedia.org/wiki/List_of_ISO_639_language_codes).

Example:
```yaml
en:
  welcome: >
    I'm a reminder bot. Creating a reminder is easy: 
    `!remind %{date} [at] 15:30 <reminder text>`. 
    I'll take care of the rest!
  reminder.command: remind
  reminder.saved: ⏲️ Remind you on %{date} at %{hour}:%{min}
  reminder.list: >
    %{text} on %{date} at %{time}
  reminder.new: >
    🟢 Don't forget: %{text}
  reminder.missed: |
    ⚠️ You missed something: 
    %{sum}
  reminder.error.date: The thirteenth month!
  reminder.error.time: Oh, the times!
  reminder.error.time-past: Time is in the past, no reminder needed!

  dates.today: today
  dates.tomorrow: tomorrow
  times.morning: morning
  times.afternoon: afternoon
  times.evening: evening

  # we want to use default English month names
  # months: january february march april may june july august september october november december
de:
  welcome: ...
```
> [!NOTE]
>  The `>` symbol means to remove all line breaks, the `|` symbol means to keep line breaks.

The bot will first search for a translation in your file, and then in the standard one.

If you've created a translation file that you'd like to share with the community, please make a pull request.

## What's Next
Read about the tools for working [with Matrix account](https://github.com/outarde/reminder-bot/tree/main/docs/matrix.md) (including device verification) or skip straight to the page about the intricacies [of using the bot](https://github.com/outarde/reminder-bot/tree/main/docs/usage.md).

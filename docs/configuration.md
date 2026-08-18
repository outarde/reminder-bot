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
Additional settings are stored in the `config.yml`/`config.yaml` file in a folder or volume that you have binded to the `/app/data/reminder_bot` folder inside the container. The repository has default [config.yml](https://github.com/outarde/reminder-bot/blob/main/docker/config.example.yml).

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

| Variable     | Description                                                                                                                                                                          | Default |
| :----------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------ |
| `lang`       | Language for bot commands and messages. See below for a list of available languages. Applies to all users.                                                                           | `en`    |
| `command`    | An alternative command to call the bot in rooms in addition to default and localized remind commands that will always work. It should be specified without the exclamation point.    | `None`  |
| `commands`   |                                                                                                                                                                                      |         |
| `on_command` |                                                                                                                                                                                      | `true`  |
| `on_mention` |                                                                                                                                                                                      | `false` |
| `morning`    | The time that is considered morning. Please follow the format `%H:%M`, otherwise you will see a general error `Error parsing regex` only when the bot tries to access the variables. | `09:00` |
| `afternoon`  | The time that is considered afternoon.                                                                                                                                               | `14:00` |
| `evening`    | The time that is considered evening.                                                                                                                                                 | `19:00` |

### Language
List of available languages

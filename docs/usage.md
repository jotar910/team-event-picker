USAGE:
    `/picker` [SUBCOMMAND] [ARGS]

SUBCOMMANDS:
    `add`         Adds an entity
    `del`         Deletes an entity
    `edit`        Edits an entity
    `help`        Prints this message or the help of the given subcommand(s)
    `list`        Lists entities
    `pick`        Picks an event
    `repick`      Repicks an event
    `show`        Shows an entity

For more information on a specific command, use `/picker help <command>`

COMMANDS:
    `pick`    Picks a participant for an event
    USAGE:
        /picker pick <id>

    ARGS:
        <id>       The ID of the event

    `repick`  Repicks a participant for an event
        USAGE:
            /picker repick <id>

        ARGS:
            <id>       The ID of the event

    `add`     Adds an entity
        USAGE:
            /picker add event <event-data>

        ARGS:
            <event-data>          Event JSON object with the event creation data

            PROPERTIES:
                <name>          The name of the event
                <date>          The date of the event (in format yyyy-mm-dd)
                <repeat>        Sets if the event should be repeated daily, weekly, bi-weekly, monthly or yearly [possible values: daily, weekly, weekly_two, monthly, yearly]
                <participants>  The participants of the event (multiple values allowed)

            EXAMPLE:
                ```
                {
                    "name": "event name",
                    "date": "2023-02-10",
                    "repeat": "daily",
                    "participants": [
                        "user1",
                        "user2",
                        "user3"
                    ]
                }
                ```

    `edit`    Edits an entity
        USAGE:
            /picker edit event <id> <event-data>
            /picker edit participants <id> <participants-data>

            ARGS:
                <event-data>            Event JSON object with the event creation data - must also include the id
                <participants-data>     Participants JSON array with the name of the participants to be added in an event

    `del`     Deletes an entity
        USAGE:
            /picker del <event> <id>
            /picker del <participants> <id> <participants-data>

        ARGS:
            <id>                    The ID of the event to delete or change
            <participants>          The participants of the event to remove (multiple values allowed)

    `list`    Lists entities
        USAGE:
            /picker list channels
            /picker list events

    `show`    Shows an entity
        USAGE:
            /picker show event <id>

        ARGS:
            <id>       The ID of the event to show

    `help`    Prints this message or the help of the given subcommand(s)

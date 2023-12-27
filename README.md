# Team Event Picker

## Overview

Team Event Picker, available on the Slack App Directory, is an intuitive Slack app designed for efficient team member selection in event moderation. It enhances fairness and efficiency in team meetings, with features for creating events, automatic/manual participant picking, and event management via Slack commands. This app streamlines team organization, allowing teams to concentrate on their goals.

For more details, visit the [Slack App Directory page](https://slack.com/apps/A04PZBPAXKN-team-picker) and the [Team Event Picker website](https://team-event-picker.vercel.app/).

## Getting Started

### Prerequisites

- Ensure Rust is installed.
- Have a running MongoDB instance.
- Obtain Slack App Credentials.

### Installation

-  Clone the Repository
```bash
git clone https://github.com/jotar910/team-event-picker.git
cd team-event-picker
```

Set up the `.env` file in the root directory with the necessary Slack credentials and MongoDB URI.

Install dependencies:
```bash
cargo build
```

Run the application:
```bash
cargo run
```

*Ensure the Slack app and MongoDB are properly configured to allow the application to function correctly.*

## Usage

Use Slack commands to interact with the app for creating events, selecting participants, and managing team meetings.

## Features

- Event creation and management in Slack
- Automated and manual participant selection
- MongoDB for data storage
- Slack integration using blocks, WebClient, OAuth, actions, and commands
- Auth0 for secure authentication

## Contributing

Contributions are welcome to help evolve the application. Please adhere to standard Rust practices and include tests for new features.

## License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/jotar910/team-event-picker/blob/main/LICENSE) file for details.


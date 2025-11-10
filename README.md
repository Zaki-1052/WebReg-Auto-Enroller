# WebReg Auto-Enroller

> Automated course enrollment monitoring system for UCSD WebReg

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)

An intelligent, automated tool that monitors UCSD WebReg course availability and automatically enrolls you when seats become available. Built with Rust for reliability and performance, featuring both a command-line interface and a modern web-based dashboard.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [How It Works](#how-it-works)
- [Requirements](#requirements)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
  - [Web Interface](#web-interface)
  - [Command Line Interface](#command-line-interface)
- [Monitoring Modes](#monitoring-modes)
- [Notifications](#notifications)
- [Architecture](#architecture)
- [Security](#security)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [License](#license)
- [Disclaimer](#disclaimer)

## Overview

WebReg Auto-Enroller continuously monitors UCSD's WebReg system for course availability and automatically attempts enrollment when seats open up. It supports complex enrollment scenarios including multiple lecture sections, discussion sections, and intelligent seat threshold monitoring.

**Key Highlights:**
- **Dual Interface**: Choose between a user-friendly web interface or powerful CLI
- **Intelligent Monitoring**: Configurable polling with double-verification to prevent false positives
- **Multi-Course Support**: Monitor and enroll in multiple courses simultaneously
- **Smart Notifications**: Get instant alerts via email and Discord
- **Robust Error Handling**: Automatic retries, cookie refresh, and comprehensive logging
- **Performance**: Built in Rust for minimal resource usage and maximum reliability

## Features

### Core Functionality
- ✅ **Real-time Course Monitoring** - Continuously checks WebReg for seat availability
- ✅ **Automatic Enrollment** - Attempts enrollment immediately when seats become available
- ✅ **Multi-Section Support** - Handle complex course structures with multiple lectures and discussions
- ✅ **Double Verification** - Rechecks availability before attempting enrollment to avoid race conditions
- ✅ **Cookie Auto-Refresh** - Maintains session validity with periodic cookie renewal

### Monitoring Modes
- ✅ **Include Mode** - Enroll when ANY seats become available (best for high-demand courses)
- ✅ **Exclude Mode** - Enroll only when seats are at or below a threshold (strategic enrollment)

### Notification System
- ✅ **Email Notifications** - Gmail integration with app password support
- ✅ **Discord Webhooks** - Real-time notifications to Discord channels
- ✅ **Multiple Recipients** - Send alerts to multiple email addresses

### Interfaces
- ✅ **Web Dashboard** - Modern, responsive web interface with live status updates
- ✅ **CLI Mode** - Lightweight command-line interface for headless/server deployments
- ✅ **REST API** - Full API for programmatic control and integration

### Logging & Analytics
- ✅ **Comprehensive Logging** - Detailed logs of all monitoring and enrollment activities
- ✅ **Enrollment Statistics** - Track success rates and system performance
- ✅ **Section Details Logging** - Complete audit trail of seat availability changes

## How It Works

1. **Authentication**: Uses your WebReg session cookie to authenticate with UCSD's system
2. **Monitoring Loop**:
   - Polls WebReg at configured intervals (default: 30 seconds)
   - Checks seat availability for all configured sections
   - Logs detailed information about each section
3. **Smart Detection**:
   - Initial check finds potential availability
   - Immediate recheck verifies seats are still available
   - Only proceeds if both checks confirm availability
4. **Enrollment**:
   - Attempts to enroll in the section
   - Retries on failure with exponential backoff
   - Sends notifications on success or failure
5. **Session Maintenance**:
   - Automatically refreshes cookies to maintain session validity
   - Handles authentication errors gracefully

## Requirements

- **Rust** (1.70 or later) - [Install Rust](https://rustup.rs/)
- **UCSD WebReg Account** - Valid credentials with enrollment access
- **Internet Connection** - For WebReg API access
- **Optional**: Gmail account for email notifications
- **Optional**: Discord server with webhook access for Discord notifications

## Installation

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/WebReg-Auto-Enroller.git
cd WebReg-Auto-Enroller
```

### 2. Build the Project

Build both CLI and web binaries:

```bash
# Build in release mode for optimal performance
cargo build --release

# Or build specific binaries
cargo build --release --bin webreg-cli   # CLI only
cargo build --release --bin webreg-web   # Web interface only
```

Binaries will be available in `target/release/`:
- `webreg-cli` - Command-line interface
- `webreg-web` - Web server

### 3. Configure the Application

Copy the example configuration and edit with your details:

```bash
cp config.toml.example config.toml
nano config.toml  # or use your preferred editor
```

## Configuration

The `config.toml` file contains all application settings:

### WebReg Settings

```toml
[webreg]
term = "WI25"              # Quarter code (WI25, SP25, FA24, etc.)
polling_interval = 30      # Seconds between checks
cookie = "YOUR_COOKIE"     # WebReg session cookie
```

**Getting Your Cookie:**
1. Log in to [WebReg](https://act.ucsd.edu/webreg2/start)
2. Open Developer Tools (F12)
3. Navigate to Application → Cookies → `https://act.ucsd.edu`
4. Copy the entire cookie string (all name=value pairs)
5. Paste into the `cookie` field

### Course Configuration

**New Format (Recommended):**
```toml
[courses.chem]
department = "CHEM"
course_code = "6B"
sections = [
    { lecture = "A00", discussions = ["A01", "A02"] },
    { lecture = "B00", discussions = ["B01"] }
]
```

**Legacy Format:**
```toml
[courses.bild]
department = "BILD"
course_code = "1"
lecture_section = "A00"
discussion_sections = ["A01", "A02"]
```

### Notification Settings

```toml
[notifications]
gmail_address = "your.email@gmail.com"
gmail_app_password = "your_app_password"  # Generate at myaccount.google.com/apppasswords
email_recipients = ["recipient1@ucsd.edu", "recipient2@ucsd.edu"]
discord_webhook_url = "https://discord.com/api/webhooks/YOUR_WEBHOOK_URL"
```

### Monitoring Settings

```toml
[monitoring]
log_file = "webreg_monitor.log"
stats_file = "enrollment_stats.json"
cookie_refresh_interval = 480    # Seconds (8 minutes)
max_retries = 3                  # Retry attempts for failed operations
retry_delay = 1000               # Milliseconds between retries
seat_threshold = 0               # 0 = include mode, >0 = exclude mode
```

## Usage

### Web Interface

The web interface provides an intuitive dashboard for managing monitoring jobs.

#### 1. Start the Web Server

```bash
cargo run --bin webreg-web
# or use the release binary
./target/release/webreg-web
```

The server will start on `http://localhost:3000`

#### 2. Access the Dashboard

Open your browser and navigate to `http://localhost:3000`

#### 3. Configure via Web UI

The web interface allows you to:
- Set term, polling interval, and cookie
- Add/remove courses and sections
- Configure notification settings
- Choose monitoring mode (Include/Exclude)
- Start/stop monitoring jobs
- View real-time status and statistics

#### 4. Monitor Status

The dashboard displays:
- ✅ Connection status
- ⏱️ Last check timestamp
- 📊 Enrollment statistics
- 🎯 Active courses and sections
- 📬 Notification configuration

#### API Endpoints

The web server exposes a REST API:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Health check |
| `/api/status` | GET | Current monitoring status and stats |
| `/api/config` | GET | Current configuration |
| `/api/jobs` | POST | Create/update job configuration |
| `/api/jobs/start` | POST | Start monitoring |
| `/api/jobs/stop` | POST | Stop monitoring |
| `/api/notifications` | POST | Update notification settings |

### Command Line Interface

The CLI provides a lightweight option for server deployments or automation.

#### 1. Run the CLI

```bash
cargo run --bin webreg-cli
# or use the release binary
./target/release/webreg-cli
```

#### 2. Monitor the Logs

The CLI logs all activity to both the console and `webreg_monitor.log`:

```bash
# Follow the log file
tail -f webreg_monitor.log

# View section details
tail -f section_details.log

# Check enrollment statistics
cat enrollment_stats.json
```

#### 3. Stop Monitoring

Press `Ctrl+C` to gracefully shutdown the monitoring process.

## Monitoring Modes

### Include Mode (seat_threshold = 0)

**When to use:** High-demand courses where any opening is valuable

- Monitors for **any** seat availability
- Attempts enrollment as soon as `available_seats > 0`
- Best for competitive courses that fill instantly
- Maximizes chances of getting into the course

**Example:**
```toml
[monitoring]
seat_threshold = 0  # Include mode
```

### Exclude Mode (seat_threshold > 0)

**When to use:** Strategic enrollment or less competitive sections

- Monitors for **limited** seat availability
- Only attempts enrollment when `0 < available_seats ≤ threshold`
- Useful for avoiding full classes or waiting for specific conditions
- Example: `threshold = 3` means enroll only when 1-3 seats remain

**Example:**
```toml
[monitoring]
seat_threshold = 3  # Only enroll when 3 or fewer seats available
```

**Use Cases:**
- Avoid enrolling in sections that will definitely get full
- Wait for less competitive enrollment windows
- Strategic timing for discussion sections
- Coordinate with friends by waiting for limited availability

## Notifications

### Email Notifications (Gmail)

**Setup:**

1. **Enable 2-Factor Authentication** on your Gmail account
2. **Generate App Password**:
   - Visit [Google App Passwords](https://myaccount.google.com/apppasswords)
   - Select "Mail" and your device
   - Copy the generated password
3. **Configure in config.toml**:
   ```toml
   [notifications]
   gmail_address = "your.email@gmail.com"
   gmail_app_password = "your_app_password"
   email_recipients = ["recipient1@ucsd.edu"]
   ```

**What you'll receive:**
- ✅ Successful enrollment confirmations
- ❌ Enrollment failure notifications
- ⚠️ System warnings and errors

### Discord Notifications

**Setup:**

1. **Create a Discord Webhook**:
   - Go to Server Settings → Integrations → Webhooks
   - Click "New Webhook"
   - Copy the webhook URL
2. **Configure in config.toml**:
   ```toml
   [notifications]
   discord_webhook_url = "https://discord.com/api/webhooks/YOUR_WEBHOOK_URL"
   ```

**What you'll receive:**
- Real-time enrollment status updates
- System health notifications
- Error alerts

## Architecture

### Technology Stack

- **Language**: Rust (2021 Edition)
- **Async Runtime**: Tokio (full features)
- **Web Framework**: Axum 0.7
- **HTTP Client**: Reqwest with JSON support
- **Email**: Lettre with Tokio async support
- **Logging**: log + env_logger
- **Serialization**: Serde + serde_json + toml
- **WebReg API**: webweg crate

### Project Structure

```
WebReg-Auto-Enroller/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── web_main.rs       # Web server entry point
│   ├── config.rs         # Configuration parsing
│   ├── monitor.rs        # Course monitoring logic
│   ├── enroll.rs         # Enrollment logic
│   ├── notifier.rs       # Notification system
│   ├── state.rs          # Application state management
│   ├── stats.rs          # Statistics tracking
│   ├── web_server.rs     # Web server routes
│   ├── job_manager.rs    # Job lifecycle management
│   ├── webreg.rs         # WebReg API wrapper
│   └── utils.rs          # Utility functions
├── static/               # Web interface assets
│   ├── index.html
│   ├── styles.css
│   └── app.js
├── config.toml          # Configuration file
├── Cargo.toml           # Rust dependencies
└── README.md            # This file
```

### State Management

The application uses `Arc<Mutex<AppState>>` for thread-safe state sharing between async tasks:

- **Connection Status**: Tracks WebReg connectivity
- **Statistics**: Enrollment attempts and success rates
- **Job Management**: Active monitoring jobs
- **Configuration**: Runtime configuration updates

### Async Architecture

- **Tokio Runtime**: Handles concurrent monitoring tasks
- **Job Manager**: Spawns and manages monitoring tasks
- **Graceful Shutdown**: Clean shutdown on Ctrl+C or stop command
- **Cookie Refresh**: Periodic background task maintains session validity

## Security

### Important Security Considerations

⚠️ **Keep Sensitive Data Secure**

1. **WebReg Cookie**: Contains your authentication session
   - Never share or commit to version control
   - Expires after inactivity (typically 24 hours)
   - Auto-refreshes during active monitoring

2. **Gmail App Password**: Grants access to send emails
   - Use app-specific passwords, NEVER your main Gmail password
   - Revoke immediately if compromised
   - Rotate periodically for security

3. **Discord Webhooks**: Can post to your Discord server
   - Keep URLs private
   - Regenerate if exposed
   - Consider using a dedicated channel

4. **Configuration File**: Contains all sensitive credentials
   - Add `config.toml` to `.gitignore`
   - Use restrictive file permissions: `chmod 600 config.toml`
   - Never commit real credentials to git

### Best Practices

- ✅ Run the web interface on localhost only (not exposed to internet)
- ✅ Use app passwords instead of main account passwords
- ✅ Review logs regularly for suspicious activity
- ✅ Keep the application updated
- ✅ Use HTTPS when possible (proxy through nginx/caddy if needed)

## Troubleshooting

### Common Issues

#### Port Already in Use

```bash
Error: Address already in use (os error 98)
```

**Solution**: Change the port in `src/web_main.rs`:
```rust
let port = 3001; // Change to desired port
```

#### Cookie Expired

```bash
Error: Failed to fetch course info: Unauthorized
```

**Solution**:
1. Get a fresh cookie from WebReg (see [Configuration](#configuration))
2. Update `config.toml` with the new cookie
3. If using web interface, update via the UI

#### Connection Errors

```bash
Error: Connection refused
```

**Possible Causes:**
- UCSD network restrictions
- VPN required
- WebReg maintenance
- Rate limiting

**Solution**:
- Connect to UCSD network or VPN
- Increase polling interval to avoid rate limits
- Check WebReg status page

#### Build Errors

```bash
Error: failed to compile
```

**Solution**:
```bash
# Clean and rebuild
cargo clean
cargo update
cargo build --release

# Check Rust version
rustc --version  # Should be 1.70 or later
```

#### Enrollment Fails Despite Available Seats

**Possible Causes:**
- Requisite not met
- Time conflict with existing classes
- Maximum units exceeded
- Section restricted by college/major

**Solution**:
- Check WebReg error messages in logs
- Verify prerequisites are met
- Check for time conflicts
- Review enrollment restrictions

### Debug Mode

Enable detailed logging:

```bash
RUST_LOG=debug cargo run --bin webreg-cli
```

This will show:
- Detailed HTTP requests/responses
- Cookie refresh attempts
- State transitions
- Retry logic execution

### Log Files

Check these files for troubleshooting:

- `webreg_monitor.log` - Main application log
- `section_details.log` - Detailed section availability history
- `enrollment_stats.json` - Statistics and metrics

## Contributing

Contributions are welcome! Here's how you can help:

### Reporting Bugs

1. Check existing issues first
2. Provide detailed reproduction steps
3. Include relevant log excerpts
4. Specify your environment (OS, Rust version, etc.)

### Suggesting Features

1. Open an issue with the `enhancement` label
2. Describe the use case
3. Explain expected behavior
4. Consider implementation approach

### Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Add tests if applicable
5. Ensure code compiles: `cargo build`
6. Run tests: `cargo test`
7. Format code: `cargo fmt`
8. Lint code: `cargo clippy`
9. Commit with clear messages
10. Push and create a pull request

### Development Setup

```bash
# Clone your fork
git clone https://github.com/yourusername/WebReg-Auto-Enroller.git
cd WebReg-Auto-Enroller

# Create a development config
cp config.toml.example config.toml

# Run in development mode
cargo run --bin webreg-web

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Disclaimer

⚠️ **Educational and Personal Use Only**

This tool is provided for educational purposes and personal use. Users are responsible for:

- Complying with UCSD's terms of service
- Following university enrollment policies
- Using the tool responsibly and ethically
- Understanding that automated enrollment may violate university policies

**The authors assume no responsibility for:**
- Violations of university policies
- Failed enrollments or missed classes
- Account suspensions or penalties
- Any damages resulting from use of this software

**Use at your own risk.** Always verify enrollment through official channels and be aware that automated systems may be prohibited by your institution.

---

## Acknowledgments

- Built with [webweg](https://crates.io/crates/webweg) - UCSD WebReg API wrapper
- Powered by the [Tokio](https://tokio.rs/) async runtime
- Web framework by [Axum](https://github.com/tokio-rs/axum)

## Support

For questions, issues, or suggestions:
- Open an issue on GitHub
- Check the [Troubleshooting](#troubleshooting) section
- Review the logs for error details

---

**Happy Enrolling! 🎓**

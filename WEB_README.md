# WebReg Auto-Enroller - Web Interface

A modern web-based frontend for managing and monitoring your UCSD WebReg course enrollments.

## Features

- **Interactive Configuration**: Easy-to-use web interface for setting up monitoring jobs
- **Real-time Status**: Live updates of enrollment attempts and system health
- **Flexible Course Management**: Add multiple courses with different section configurations
- **Dual Monitoring Modes**:
  - **Include Mode**: Enroll when any seats become available
  - **Exclude Mode**: Enroll only when seats are at or below a threshold (useful for less competitive sections)
- **Notification Integration**: Configure email and Discord notifications
- **Multi-section Support**: Monitor multiple lecture and discussion sections per course

## Quick Start

### 1. Build the Web Server

```bash
cargo build --release --bin webreg-web
```

### 2. Start the Web Server

```bash
cargo run --bin webreg-web
```

The web interface will be available at: `http://localhost:3000`

### 3. Configure Your Monitoring Job

1. Open your browser and navigate to `http://localhost:3000`
2. Fill in the configuration form:
   - **Term**: Quarter/term code (e.g., WI25, SP25, FA24)
   - **Polling Interval**: How often to check for availability (in seconds)
   - **Cookie**: Your WebReg session cookie (see below)
   - **Monitoring Mode**:
     - **Include**: Enroll when seats become available
     - **Exclude**: Enroll when seats are limited (set threshold)

### 4. Add Courses

1. Click "Add Course" to add a new course
2. Enter department and course code (e.g., CHEM 6B)
3. Click "Add Section Group" to add lecture/discussion combinations
4. Enter lecture section code (e.g., A00, B00)
5. Add discussion sections if needed (e.g., A01, A02)

### 5. Set Up Notifications (Optional)

Configure email and/or Discord notifications:
- **Gmail**: Requires app password (generate at myaccount.google.com/apppasswords)
- **Discord**: Provide webhook URL from your Discord server settings

### 6. Start Monitoring

1. Click "Save Configuration" to save your settings
2. Click "Start Monitoring" to begin watching for openings
3. Monitor the status dashboard for real-time updates

## Getting Your WebReg Cookie

1. Log in to WebReg at https://act.ucsd.edu/webreg2/start
2. Open browser Developer Tools (F12)
3. Go to the "Application" or "Storage" tab
4. Navigate to "Cookies" → "https://act.ucsd.edu"
5. Copy the entire cookie string (all name=value pairs)
6. Paste into the "WebReg Cookie" field in the web interface

## Monitoring Modes Explained

### Include Mode (seat_threshold = 0)
- Monitors for **any** seat availability
- Attempts enrollment as soon as seats open up
- Best for high-demand courses where any opening is competitive

### Exclude Mode (seat_threshold > 0)
- Monitors for **limited** seat availability
- Only attempts enrollment when seats are at or below the threshold
- Example: threshold = 3 means enroll only when 1-3 seats remain
- Useful for avoiding full classes or strategic enrollment

## API Endpoints

The web server provides REST API endpoints:

- `GET /api/health` - Health check
- `GET /api/status` - Current monitoring status and statistics
- `GET /api/config` - Current configuration
- `POST /api/jobs` - Create/update job configuration
- `POST /api/jobs/start` - Start monitoring
- `POST /api/jobs/stop` - Stop monitoring
- `POST /api/notifications` - Update notification settings

## Running Both CLI and Web Modes

The project includes two binaries:

- **CLI Mode**: `cargo run --bin webreg-cli` (original command-line interface)
- **Web Mode**: `cargo run --bin webreg-web` (new web interface)

## Troubleshooting

### Port Already in Use
If port 3000 is already in use, you can modify the port in `src/web_main.rs`:
```rust
let port = 3001; // Change to desired port
```

### Cookie Expires
WebReg sessions expire after a period of inactivity. The system automatically attempts to refresh the cookie periodically, but if you see connection errors, try updating the cookie in the web interface.

### Build Errors
Make sure all dependencies are up to date:
```bash
cargo clean
cargo build --bin webreg-web
```

## Architecture

- **Backend**: Rust with Axum web framework
- **Frontend**: Vanilla JavaScript with responsive CSS
- **State Management**: Tokio async runtime with Arc/Mutex for thread-safe state
- **Job Management**: Custom job manager for controlling monitoring tasks

## Security Notes

- Keep your WebReg cookie secure and never share it
- Use app passwords for Gmail (never your main password)
- Discord webhooks should be kept private
- The web interface is intended for local use only (localhost)

## Contributing

Feel free to submit issues or pull requests to improve the web interface!

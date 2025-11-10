# WebReg Auto-Enroller - Multi-User Setup Guide

This guide will help you set up the multi-user version of WebReg Auto-Enroller with Clerk authentication, PostgreSQL database, and encrypted credential storage.

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Running the Server](#running-the-server)
- [Using the Application](#using-the-application)
- [Troubleshooting](#troubleshooting)

## Features

✅ **Multi-User Support**: Multiple users can create and manage their own monitoring jobs
✅ **Clerk Authentication**: Secure authentication with Clerk (OAuth, email/password, etc.)
✅ **PostgreSQL Database**: Persistent storage for jobs, courses, and statistics
✅ **Encrypted Credentials**: WebReg cookies and Gmail passwords are encrypted at rest
✅ **Concurrent Monitoring**: Each user's jobs run independently and concurrently
✅ **User-Scoped Data**: Users can only access their own jobs and settings
✅ **RESTful API**: Clean API with JWT authentication

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (latest stable version)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **PostgreSQL** (version 12 or higher)
  ```bash
  # Ubuntu/Debian
  sudo apt-get install postgresql postgresql-contrib

  # macOS
  brew install postgresql

  # Start PostgreSQL service
  sudo service postgresql start  # Linux
  brew services start postgresql  # macOS
  ```

- **SQLx CLI** (for database migrations)
  ```bash
  cargo install sqlx-cli --no-default-features --features postgres
  ```

- **OpenSSL** (for generating encryption keys)
  ```bash
  # Usually pre-installed, but if needed:
  # Ubuntu/Debian
  sudo apt-get install openssl

  # macOS
  brew install openssl
  ```

## Installation

### 1. Clone the Repository

```bash
git clone https://github.com/your-username/WebReg-Auto-Enroller.git
cd WebReg-Auto-Enroller
```

### 2. Set Up PostgreSQL Database

Run the database setup script:

```bash
chmod +x scripts/setup_database.sh
./scripts/setup_database.sh
```

This will create:
- A database user: `webreg_user`
- A database: `webreg_auto_enroller`

**Note**: The script will prompt for a password. Use a secure password or set the `POSTGRES_PASSWORD` environment variable.

### 3. Generate Encryption Key

Generate a secure encryption key for storing sensitive data:

```bash
chmod +x scripts/generate_encryption_key.sh
./scripts/generate_encryption_key.sh
```

Save the generated key - you'll need it in the next step.

### 4. Set Up Clerk Authentication

1. Go to [Clerk Dashboard](https://dashboard.clerk.com)
2. Create a new application
3. Get your publishable key and secret key from the dashboard
4. For JWT verification, you'll need the public key:
   - Go to **JWT Templates** in Clerk dashboard
   - Create a new template or use the default
   - Copy the public key (PEM format)

### 5. Configure Environment Variables

Copy the example environment file and fill in your values:

```bash
cp .env.example .env
```

Edit `.env` with your configuration:

```env
# Database Configuration
DATABASE_URL=postgresql://webreg_user:YOUR_PASSWORD@localhost:5432/webreg_auto_enroller

# Encryption Key (from step 3)
ENCRYPTION_KEY=your_base64_encoded_32_byte_key_here

# Clerk Authentication
CLERK_PUBLIC_KEY=-----BEGIN PUBLIC KEY-----
your_clerk_public_key_here
-----END PUBLIC KEY-----

# Alternative: Use Clerk Secret Key
CLERK_SECRET_KEY=sk_test_your_clerk_secret_key_here

# Server Configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# Logging
RUST_LOG=info
```

### 6. Run Database Migrations

Apply the database schema:

```bash
sqlx migrate run
```

This will create all necessary tables:
- `users`
- `jobs`
- `courses`
- `sections`
- `enrollment_stats`
- `notification_settings`

### 7. Update Frontend Configuration

Edit `static/multiuser.html` and update the Clerk configuration:

```html
<script
    async
    crossorigin="anonymous"
    data-clerk-publishable-key="pk_test_YOUR_CLERK_PUBLISHABLE_KEY"
    src="https://[your-clerk-frontend-api].clerk.accounts.dev/npm/@clerk/clerk-js@latest/dist/clerk.browser.js"
    type="text/javascript"
></script>
```

Replace:
- `YOUR_CLERK_PUBLISHABLE_KEY` with your actual Clerk publishable key
- `[your-clerk-frontend-api]` with your Clerk frontend API domain

### 8. Build the Application

```bash
cargo build --release --bin webreg-web-multiuser
```

## Running the Server

### Development Mode

```bash
cargo run --bin webreg-web-multiuser
```

### Production Mode

```bash
cargo run --release --bin webreg-web-multiuser
```

The server will start on `http://0.0.0.0:3000` (or the port specified in your `.env` file).

## Using the Application

### 1. Access the Web Interface

Open your browser and navigate to:
```
http://localhost:3000/multiuser.html
```

### 2. Sign Up / Sign In

- Click **Sign Up** to create a new account
- Or click **Sign In** if you already have an account
- Clerk will handle the authentication flow

### 3. Create a Monitoring Job

1. Click **+ Create New Job**
2. Fill in the job details:
   - **Term**: The academic quarter (e.g., `WI25`, `SP25`)
   - **WebReg Cookie**: Your WebReg session cookie (see below)
   - **Polling Interval**: How often to check for seats (seconds)
   - **Seat Threshold**: Number of seats to trigger enrollment
   - **Monitoring Mode**:
     - **Include**: Enroll when seats > threshold
     - **Exclude**: Enroll when seats ≤ threshold
3. Add courses and sections
4. Click **Create Job**

### 4. Getting Your WebReg Cookie

1. Log in to WebReg in your browser
2. Open Developer Tools (F12)
3. Go to **Application** → **Cookies** → `https://act.ucsd.edu`
4. Find the session cookie (usually named `connect.sid` or similar)
5. Copy the entire cookie value

### 5. Set Up Notifications (Optional)

1. Scroll to the **Notification Settings** section
2. Configure email notifications:
   - **Gmail Address**: Your Gmail account
   - **Gmail App Password**: [Create an app password](https://support.google.com/accounts/answer/185833)
   - **Email Recipients**: Who should receive notifications
3. (Optional) Add a Discord webhook URL
4. Click **Save Notifications**

### 6. Start Monitoring

- Click the **Start** button on your job card
- The system will begin monitoring courses in the background
- You'll receive notifications when seats become available and enrollment is attempted

## API Endpoints

The multi-user API provides the following endpoints (all require authentication):

### Authentication

All API requests must include a Bearer token in the Authorization header:

```
Authorization: Bearer <clerk_session_token>
```

### Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/health` | Health check (no auth required) |
| GET | `/api/user` | Get current user profile |
| POST | `/api/jobs` | Create a new monitoring job |
| GET | `/api/jobs` | Get all jobs for current user |
| GET | `/api/jobs/:id` | Get job details |
| POST | `/api/jobs/:id/start` | Start a job |
| POST | `/api/jobs/:id/stop` | Stop a job |
| DELETE | `/api/jobs/:id` | Delete a job |
| GET | `/api/notifications` | Get notification settings |
| POST | `/api/notifications` | Update notification settings |

### Example API Request

```bash
# Get all jobs
curl -X GET http://localhost:3000/api/jobs \
  -H "Authorization: Bearer YOUR_CLERK_TOKEN"
```

## Security Considerations

1. **Environment Variables**: Never commit `.env` to version control
2. **Encryption Key**: Keep your encryption key secure and back it up
3. **Database Credentials**: Use strong passwords for PostgreSQL
4. **HTTPS**: In production, always use HTTPS (consider using a reverse proxy like nginx)
5. **Clerk Keys**: Keep your Clerk secret key private
6. **WebReg Cookies**: Cookies are encrypted at rest in the database

## Database Schema

### Tables

- **users**: User accounts (linked to Clerk)
- **jobs**: Monitoring job configurations
- **courses**: Courses associated with jobs
- **sections**: Section groups (lecture + discussions)
- **enrollment_stats**: Statistics per job
- **notification_settings**: User notification preferences

### Relationships

```
users (1) → (*) jobs
jobs (1) → (*) courses
courses (1) → (*) sections
jobs (1) → (1) enrollment_stats
users (1) → (1) notification_settings
```

## Troubleshooting

### Database Connection Errors

**Error**: `Connection refused` or `could not connect to server`

**Solution**:
```bash
# Check if PostgreSQL is running
sudo service postgresql status  # Linux
brew services list  # macOS

# Start PostgreSQL if needed
sudo service postgresql start  # Linux
brew services start postgresql  # macOS
```

### Migration Errors

**Error**: `migration X has already been applied`

**Solution**:
```bash
# Revert the last migration
sqlx migrate revert

# Or reset all migrations (WARNING: deletes all data)
sqlx database drop
sqlx database create
sqlx migrate run
```

### Encryption Errors

**Error**: `ENCRYPTION_KEY environment variable not set`

**Solution**: Ensure your `.env` file is in the project root and contains a valid encryption key.

### Authentication Errors

**Error**: `Invalid token` or `Missing Authorization header`

**Solution**:
- Verify Clerk public key is correct in `.env`
- Ensure the frontend is configured with the correct Clerk publishable key
- Check that the token hasn't expired

### Compilation Errors

**Error**: Missing dependencies or trait implementations

**Solution**:
```bash
# Clean and rebuild
cargo clean
cargo build --release --bin webreg-web-multiuser
```

### Port Already in Use

**Error**: `Address already in use`

**Solution**:
```bash
# Find process using port 3000
lsof -i :3000

# Kill the process
kill -9 <PID>

# Or change the port in .env
SERVER_PORT=3001
```

## Performance Tips

1. **PostgreSQL Optimization**: Adjust PostgreSQL settings for your workload
2. **Connection Pooling**: The application uses connection pooling (max 5 connections by default)
3. **Polling Interval**: Don't set polling interval too low (minimum recommended: 30 seconds)
4. **Database Indexes**: Already created on frequently queried columns
5. **Monitoring**: Use `RUST_LOG=debug` for detailed logging

## Migration from Single-User Version

If you're migrating from the single-user version:

1. Export your configuration from `config.toml`
2. Create a user account in the multi-user system
3. Create a new job with your configuration
4. Set up notification settings
5. The old file-based system can coexist with the new system

## Development

### Running Tests

```bash
cargo test
```

### Database Migrations

Create a new migration:

```bash
sqlx migrate add <migration_name>
```

### Code Structure

```
src/
├── models.rs              # Database models
├── db.rs                  # Database queries
├── encryption.rs          # Encryption utilities
├── auth.rs                # Clerk authentication
├── multi_user_state.rs    # Multi-user job management
├── multi_user_api.rs      # API endpoints
└── web_main_multiuser.rs  # Server entry point
```

## Support

For issues, please open a GitHub issue with:
- Your OS and Rust version (`rustc --version`)
- PostgreSQL version (`psql --version`)
- Error messages and logs
- Steps to reproduce

## License

[Add your license here]

## Contributors

[Add contributors here]

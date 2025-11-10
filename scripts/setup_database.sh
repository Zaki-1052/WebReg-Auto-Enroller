#!/bin/bash
# Setup PostgreSQL database for WebReg Auto Enroller

set -e

DB_NAME="webreg_auto_enroller"
DB_USER="webreg_user"
DB_PASSWORD="${POSTGRES_PASSWORD:-webreg_password}"

echo "Setting up PostgreSQL database..."

# Check if PostgreSQL is installed
if ! command -v psql &> /dev/null; then
    echo "Error: PostgreSQL is not installed"
    echo "Please install PostgreSQL first:"
    echo "  Ubuntu/Debian: sudo apt-get install postgresql"
    echo "  macOS: brew install postgresql"
    exit 1
fi

# Create user
echo "Creating database user..."
sudo -u postgres psql -c "CREATE USER $DB_USER WITH PASSWORD '$DB_PASSWORD';" || echo "User may already exist"

# Create database
echo "Creating database..."
sudo -u postgres psql -c "CREATE DATABASE $DB_NAME OWNER $DB_USER;" || echo "Database may already exist"

# Grant privileges
echo "Granting privileges..."
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $DB_USER;"

echo ""
echo "Database setup complete!"
echo ""
echo "Add this to your .env file:"
echo "DATABASE_URL=postgresql://$DB_USER:$DB_PASSWORD@localhost:5432/$DB_NAME"
echo ""
echo "To run migrations, execute:"
echo "cargo install sqlx-cli"
echo "sqlx migrate run"

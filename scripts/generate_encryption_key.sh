#!/bin/bash
# Generate a secure encryption key for the application

echo "Generating 256-bit encryption key..."
KEY=$(openssl rand -base64 32)

echo ""
echo "Your encryption key (add this to .env):"
echo "ENCRYPTION_KEY=$KEY"
echo ""
echo "IMPORTANT: Keep this key secure and never commit it to version control!"

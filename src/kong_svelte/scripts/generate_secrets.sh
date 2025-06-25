#!/bin/bash

# Get the project root directory (3 levels up from scripts folder)
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")"/../../.. && pwd)"

# Source the .env file from project root
source "${PROJECT_ROOT}/.env"

# Pull secrets from Doppler and save to .secrets. 
# Vite will automatically merge the secrets with the root .env file at build time.

# Skip Doppler entirely and just create empty .secrets file
touch .secrets
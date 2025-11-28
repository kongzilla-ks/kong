#!/bin/bash
# ==============================================================================
# KongSwap Docker Test Runner
# Single command to start full testing environment
# ==============================================================================

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "========================================="
echo " KongSwap Docker Testing Environment"
echo "========================================="
echo ""
echo "This will:"
echo "  1. Build all containers (~15 min first time)"
echo "  2. Start PostgreSQL database"
echo "  3. Start DFX replica and deploy canisters"
echo "  4. Start kong_rpc proxy"
echo "  5. Run ping-pong cross-chain tests"
echo ""

# Check Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "ERROR: Docker is not running. Please start Docker Desktop."
    exit 1
fi

# Create logs directory
mkdir -p logs

# Parse arguments
BUILD_FLAG=""
case "${1:-}" in
    --build|-b)
        BUILD_FLAG="--build"
        echo "Building images..."
        ;;
    --clean|-c)
        echo "Cleaning up previous run..."
        docker-compose down -v 2>/dev/null || true
        BUILD_FLAG="--build"
        ;;
    --down|-d)
        echo "Stopping containers..."
        docker-compose down
        exit 0
        ;;
    --logs|-l)
        docker-compose logs -f
        exit 0
        ;;
    --help|-h)
        echo "Usage: $0 [option]"
        echo ""
        echo "Options:"
        echo "  --build, -b   Rebuild all images"
        echo "  --clean, -c   Remove all volumes and rebuild"
        echo "  --down, -d    Stop all containers"
        echo "  --logs, -l    Follow logs from all containers"
        echo "  --help, -h    Show this help"
        echo ""
        echo "Examples:"
        echo "  $0            # Start with existing images"
        echo "  $0 --build    # Rebuild and start"
        echo "  $0 --clean    # Full reset and rebuild"
        exit 0
        ;;
esac

# Start the stack
echo "Starting containers..."
docker-compose up $BUILD_FLAG

# This will block until Ctrl+C

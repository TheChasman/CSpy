#!/bin/bash
# Development script: runs frontend Vite dev server in background
# and cargo-watch for Rust backend. Both use file system events (not polling).

set -e

echo "CSpy dev mode — FSEvents-based watching (not polling)"
echo ""

# Start Vite frontend dev server in background
echo "Starting Vite (frontend)..."
npm run dev &
VITE_PID=$!

# Give Vite a moment to start
sleep 2

# Trap to clean up both processes on exit
trap 'kill $VITE_PID 2>/dev/null; exit' EXIT INT TERM

# Start cargo-watch for Rust backend
# cargo-watch uses file system events (FSEvents on macOS, inotify on Linux)
# -x runs the command on file changes
echo "Starting cargo-watch (backend)..."
cd src-tauri
cargo watch -x 'build --lib' -i '../src/**' -i '../.svelte-kit/**' -w src

# Note: This keeps the script running. Vite runs in background.
# Press Ctrl+C to stop both.

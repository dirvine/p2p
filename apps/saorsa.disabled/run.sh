#!/bin/bash

# Kill any existing Python servers on port 8080
lsof -ti:8080 | xargs kill -9 2>/dev/null

# Start the Python server in the background
cd src && python3 -m http.server 8080 &
SERVER_PID=$!

# Give the server a moment to start
sleep 2

# Run Tauri
cd .. && cargo tauri dev

# Clean up the server when done
kill $SERVER_PID 2>/dev/null
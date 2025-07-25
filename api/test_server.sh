#!/bin/bash

# Simple manual test for the server
echo "Starting server on port 9876..."

# Start server in background with specific port and capture logs
APP_SERVER__PORT=9876 APP_SERVER__BIND=127.0.0.1 cargo run > server.log 2>&1 &
SERVER_PID=$!

# Wait for server to start
sleep 1

echo "Testing health endpoints..."

# Test liveness endpoint
echo -n "Liveness: "
curl -s -w "%{http_code}" -o /tmp/liveness.txt http://127.0.0.1:9876/health/liveness
echo " ($(cat /tmp/liveness.txt))"

# Test readiness endpoint
echo -n "Readiness: "
curl -s -w "%{http_code}" -o /tmp/readiness.txt http://127.0.0.1:9876/health/readiness
echo " ($(cat /tmp/readiness.txt))"

# Test trace ID header
echo -n "Trace ID header: "
curl -s -I http://127.0.0.1:9876/health/liveness | grep -i x-trace-id || echo "Not found"

# Cleanup
echo "Stopping server..."
kill $SERVER_PID 2>/dev/null

echo "Done!"
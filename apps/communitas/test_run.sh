#!/bin/bash
# Test run Communitas
../../target/debug/communitas &
PID=$!
sleep 5
kill $PID 2>/dev/null || true
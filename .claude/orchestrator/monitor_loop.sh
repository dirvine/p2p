#!/bin/bash
# Orchestrator Active Monitoring Loop

PROJECT_DIR="/Users/davidirvine/Desktop/Devel/projects/p2p"
ORCHESTRATOR_DIR="$PROJECT_DIR/.claude/orchestrator"
CHECK_INTERVAL=15

echo "🎯 ORCHESTRATOR MONITORING ACTIVE"
echo "================================="
echo "Project: P2P Foundation Production Readiness"
echo "Check Interval: ${CHECK_INTERVAL}s"
echo ""

while true; do
    # Get current time
    TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    # Read current state
    CURRENT_TASK=$(cat "$ORCHESTRATOR_DIR/current_task.txt" 2>/dev/null || echo "0")
    
    # Check for completion signals
    if [ -f "$ORCHESTRATOR_DIR/signals/task_${CURRENT_TASK}_complete" ]; then
        echo "[$TIMESTAMP] ✅ Task $CURRENT_TASK completion signal detected!"
        
        # Progress to next task
        NEXT_TASK=$((CURRENT_TASK + 1))
        echo "$NEXT_TASK" > "$ORCHESTRATOR_DIR/current_task.txt"
        
        # Clean up signal
        rm -f "$ORCHESTRATOR_DIR/signals/task_${CURRENT_TASK}_complete"
        
        echo "[$TIMESTAMP] 🚀 Progressing to Task $NEXT_TASK"
    fi
    
    # Check for test results
    if [ -f "$PROJECT_DIR/crates/p2p-core/test_results.txt" ]; then
        TESTS_PASSED=$(grep -c "test result: ok" "$PROJECT_DIR/crates/p2p-core/test_results.txt" 2>/dev/null || echo "0")
        echo "[$TIMESTAMP] 🧪 Tests passed: $TESTS_PASSED"
    fi
    
    # Monitor for stuck tasks (no activity for 30 minutes)
    LAST_MODIFIED=$(stat -f %m "$ORCHESTRATOR_DIR/state.json" 2>/dev/null || echo "0")
    CURRENT_TIME=$(date +%s)
    TIME_SINCE_UPDATE=$((CURRENT_TIME - LAST_MODIFIED))
    
    if [ $TIME_SINCE_UPDATE -gt 1800 ]; then
        echo "[$TIMESTAMP] ⚠️  Task appears stuck (no updates for 30+ minutes)"
    fi
    
    # Brief status
    echo "[$TIMESTAMP] 📊 Task $CURRENT_TASK in progress..."
    
    sleep $CHECK_INTERVAL
done
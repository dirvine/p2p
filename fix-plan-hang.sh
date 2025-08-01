#!/bin/bash
# Fix for /plan hanging in p2p project due to active orchestrator

echo "🔧 Fixing /plan hang in p2p project"
echo "===================================="
echo ""

# Get the project directory
PROJECT_DIR="/Users/davidirvine/Desktop/Devel/projects/p2p"

echo "🎯 Found active orchestrator in p2p project!"
echo ""
echo "Current state:"
echo "- Status: active"
echo "- Current task: 1 (Error Handling Framework)"
echo "- Total tasks: 15"
echo ""

echo "The /plan command is hanging because there's already an active workflow."
echo ""

echo "Choose an option:"
echo "1) Pause current workflow and allow new planning"
echo "2) Reset orchestrator completely (lose current progress)"
echo "3) Archive current tasks and start fresh"
echo "4) Exit without changes"
echo ""

read -p "Enter choice (1-4): " -n 1 -r
echo ""

case $REPLY in
    1)
        echo "Pausing current workflow..."
        # Update local orchestrator state to paused
        cat > "$PROJECT_DIR/.claude/orchestrator/state.json" << 'EOF'
{
  "project": "P2P Foundation Production Readiness",
  "status": "paused",
  "current_task": {
    "number": 1,
    "file": "task-001-error-handling-framework.md",
    "started": "2025-01-29T15:40:00Z",
    "iterations": 1,
    "last_check": "2025-01-29T15:45:00Z",
    "status": "paused",
    "notes": "Paused to allow new planning"
  },
  "completed_tasks": [],
  "total_tasks": 15,
  "metrics": {
    "total_time": "5m",
    "average_per_task": "0m",
    "total_iterations": 1,
    "compilation_errors": 449
  },
  "signals": {
    "pending_progression": false,
    "last_signal": null
  }
}
EOF
        echo "✅ Workflow paused. You can now use /plan"
        echo ""
        echo "To resume the current tasks later, use: /orchestrate resume"
        ;;
    
    2)
        echo "Resetting orchestrator..."
        # Reset to inactive state
        cat > "$PROJECT_DIR/.claude/orchestrator/state.json" << 'EOF'
{
  "project": "none",
  "status": "inactive",
  "current_task": null,
  "completed_tasks": [],
  "total_tasks": 0,
  "metrics": {},
  "signals": {}
}
EOF
        # Clean up any signals
        rm -f "$PROJECT_DIR/.claude/orchestrator/signals/*"
        echo "✅ Orchestrator reset. You can now use /plan"
        echo ""
        echo "Note: Previous tasks are still in .claude/tasks/"
        ;;
    
    3)
        echo "Archiving current tasks..."
        # Create archive with timestamp
        TIMESTAMP=$(date +%Y%m%d_%H%M%S)
        ARCHIVE_DIR="$PROJECT_DIR/.claude/tasks/archive/production_readiness_$TIMESTAMP"
        mkdir -p "$ARCHIVE_DIR"
        
        # Move all task files
        mv "$PROJECT_DIR/.claude/tasks"/task-*.md "$ARCHIVE_DIR/" 2>/dev/null || true
        
        # Reset orchestrator
        cat > "$PROJECT_DIR/.claude/orchestrator/state.json" << 'EOF'
{
  "project": "none",
  "status": "inactive",
  "current_task": null,
  "completed_tasks": [],
  "total_tasks": 0,
  "metrics": {},
  "signals": {}
}
EOF
        
        echo "✅ Tasks archived to: $ARCHIVE_DIR"
        echo "✅ Orchestrator reset. You can now use /plan"
        ;;
    
    4)
        echo "No changes made."
        echo ""
        echo "To work with existing tasks, use:"
        echo "- /orchestrate status - Check current progress"
        echo "- /continue - Continue current task"
        echo "- /next-task - Move to next task"
        exit 0
        ;;
    
    *)
        echo "Invalid choice. No changes made."
        exit 1
        ;;
esac

# Clear any global planning flags too
rm -f ~/.claude/orchestrator/.planning_active 2>/dev/null || true

echo ""
echo "You can now use: /plan <your new project description>"

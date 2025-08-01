#!/bin/bash
# Quick fix for /plan hanging in p2p project

echo "🚀 Quick fix for /plan hang in p2p project"
echo ""

# Pause the current orchestrator
PROJECT_DIR="/Users/davidirvine/Desktop/Devel/projects/p2p"

# Update state to paused
cat > "$PROJECT_DIR/.claude/orchestrator/state.json" << 'EOF'
{
  "project": "P2P Foundation Production Readiness - PAUSED",
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

# Clear any planning flags
rm -f ~/.claude/orchestrator/.planning_active 2>/dev/null || true
rm -f "$PROJECT_DIR/.claude/orchestrator/.planning_active" 2>/dev/null || true

# Clear any active signals
rm -f "$PROJECT_DIR/.claude/orchestrator/signals/*" 2>/dev/null || true

echo "✅ Fixed! The orchestrator is now paused."
echo ""
echo "You can now use: /plan <your description>"
echo ""
echo "Your existing production readiness tasks are preserved."
echo "To resume them later, use: /orchestrate resume"

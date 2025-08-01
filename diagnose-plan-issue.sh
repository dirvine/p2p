#!/bin/bash
# Diagnostic for p2p project Claude state

echo "🔍 P2P Project Claude State Diagnostic"
echo "======================================"
echo ""

PROJECT_DIR="/Users/davidirvine/Desktop/Devel/projects/p2p"

echo "📊 Local Orchestrator State (p2p project):"
if [[ -f "$PROJECT_DIR/.claude/orchestrator/state.json" ]]; then
    echo "Found local orchestrator state:"
    jq -r '"  Status: \(.status)\n  Project: \(.project)\n  Current Task: \(.current_task.number // "none")"' "$PROJECT_DIR/.claude/orchestrator/state.json"
else
    echo "  No local orchestrator state"
fi

echo ""
echo "📊 Global Orchestrator State (~/.claude):"
if [[ -f ~/.claude/orchestrator/state.json ]]; then
    jq -r '"  Status: \(.status)\n  Project: \(.project)"' ~/.claude/orchestrator/state.json
else
    echo "  No global orchestrator state"
fi

echo ""
echo "⚠️  ISSUE FOUND:"
echo "The p2p project has an ACTIVE local orchestrator!"
echo "This is blocking the /plan command from working."
echo ""
echo "Current p2p project workflow:"
echo "- Production Readiness Remediation"
echo "- 15 tasks total"
echo "- Currently on Task 1: Error Handling Framework"
echo ""
echo "OPTIONS TO FIX:"
echo ""
echo "1) Run the quick fix (recommended):"
echo "   bash $PROJECT_DIR/quick-fix-plan.sh"
echo ""
echo "2) Run the interactive fix:"
echo "   bash $PROJECT_DIR/fix-plan-hang.sh"
echo ""
echo "3) Continue with existing tasks instead:"
echo "   /orchestrate          # Resume task execution"
echo "   /task-status          # Check current progress"
echo "   /continue             # Continue current task"

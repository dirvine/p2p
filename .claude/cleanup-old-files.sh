#!/bin/bash
# Quick cleanup script to archive old documentation

echo "Archiving old documentation files..."

# Create archive directory if needed
mkdir -p ~/.claude/archived/old-docs
mkdir -p ~/.claude/archived/old-scripts

# Move old documentation
docs_to_archive=(
    "CONTINUE-WITH-ORCHESTRATOR.md"
    "ENHANCEMENT-WORKFLOW.md"
    "FIX-ORCHESTRATION.md"
    "INTERACTIVE-TDD-GUIDE.md"
    "ORCHESTRATION-QUICKSTART.md"
    "ORCHESTRATOR-READY.md"
    "PLAN-MANAGEMENT-GUIDE.md"
    "SUB-AGENT-WORKFLOW.md"
    "SUB-AGENTS-WORKFLOW.md"
    "WORKFLOW-COMPLETE-GUIDE.md"
    "orchestrator-quick-ref.md"
)

for doc in "${docs_to_archive[@]}"; do
    if [ -f ~/.claude/"$doc" ]; then
        mv ~/.claude/"$doc" ~/.claude/archived/old-docs/
        echo "Archived: $doc"
    fi
done

# Move old scripts
scripts_to_archive=(
    "enable-orchestrator.sh"
    "setup-enhanced-plans.sh"
    "setup-plan-management.sh"
    "test-orchestrator.sh"
    "make-executable.sh"
    "make-scripts-executable.sh"
    "check-rust-safety.sh"
    "check-structure.sh"
)

for script in "${scripts_to_archive[@]}"; do
    if [ -f ~/.claude/"$script" ]; then
        mv ~/.claude/"$script" ~/.claude/archived/old-scripts/
        echo "Archived: $script"
    fi
done

echo ""
echo "✅ Cleanup complete!"
echo ""
echo "Essential files remaining:"
echo "- AUTONOMOUS-WORKFLOW.md (main guide)"
echo "- COMMANDS-OVERVIEW.md (command reference)"
echo "- WORKFLOW-QUICK-REF.md (quick reference)"
echo "- SETUP-COMPLETE.md (setup summary)"
echo ""
echo "All old files moved to ~/.claude/archived/"

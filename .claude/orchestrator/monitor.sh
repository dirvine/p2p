#\!/bin/bash
# Orchestrator Active Monitor

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Config
STATE_FILE=".claude/orchestrator/state.json"
CHECK_INTERVAL=10
PROJECT_NAME="P2P Foundation Production Readiness"

# Function to display progress
display_status() {
    clear
    echo -e "${BLUE}🎯 ORCHESTRATOR ACTIVE MONITOR${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    # Parse state
    local status=$(jq -r '.status' "$STATE_FILE")
    local current_task=$(jq -r '.current_task.number' "$STATE_FILE")
    local total_tasks=$(jq -r '.total_tasks' "$STATE_FILE")
    local completed_count=$(jq -r '.completed_tasks | length' "$STATE_FILE")
    local iterations=$(jq -r '.current_task.iterations' "$STATE_FILE")
    
    # Calculate progress
    local progress=$((completed_count * 100 / total_tasks))
    
    echo -e "Project: ${PROJECT_NAME}"
    echo -e "Status: ${GREEN}🟢 Active${NC} | Task ${current_task}/${total_tasks}"
    echo ""
    
    # Current task info
    echo -e "Current Task: Error Handling Framework"
    echo -e "├─ Started: $(date -r $(jq -r '.current_task.started' "$STATE_FILE" | xargs date -j -f "%Y-%m-%dT%H:%M:%SZ" +%s) +"%M mins ago")"
    echo -e "├─ Iterations: ${iterations}"
    echo -e "└─ Status: Starting implementation"
    echo ""
    
    # Progress bar
    echo "Progress Overview:"
    printf "["
    local filled=$((progress * 30 / 100))
    for ((i=0; i<filled; i++)); do printf "█"; done
    for ((i=filled; i<30; i++)); do printf "░"; done
    printf "] ${progress}%% (${completed_count}/${total_tasks})\n"
    echo ""
    
    echo "Next Check: ${CHECK_INTERVAL} seconds"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Main monitoring loop
echo -e "${GREEN}Starting orchestrator monitor...${NC}"
sleep 2

while true; do
    display_status
    sleep $CHECK_INTERVAL
done

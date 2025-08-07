#\!/bin/bash
# Production Readiness Sprint - Panic Detection Script
# Part of Task 001: Error Handling Framework

echo "🔍 P2P FOUNDATION PANIC SCAN REPORT"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Timestamp: $(date '+%Y-%m-%d %H:%M:%S')"
echo "Target: Rust source files (excluding tests and benchmarks)"
echo ""

# Check if ripgrep is available
if \! command -v rg &> /dev/null; then
    echo "❌ Error: ripgrep (rg) is not installed"
    echo "Install with: cargo install ripgrep"
    exit 1
fi

# Count patterns
unwrap_count=0
expect_count=0 
panic_count=0
unreachable_count=0
todo_count=0

# Count unwrap() calls
echo "Scanning for unwrap() calls..."
if rg "\.unwrap\(\)" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' >/dev/null 2>&1; then
    unwrap_count=$(rg "\.unwrap\(\)" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' 2>/dev/null | wc -l | tr -d ' ')
fi

# Count expect() calls  
echo "Scanning for expect() calls..."
if rg "\.expect\(" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' >/dev/null 2>&1; then
    expect_count=$(rg "\.expect\(" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' 2>/dev/null | wc -l | tr -d ' ')
fi

# Count panic\! macros
echo "Scanning for panic\! macros..."
if rg "panic\!\(" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' >/dev/null 2>&1; then
    panic_count=$(rg "panic\!\(" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' 2>/dev/null | wc -l | tr -d ' ')
fi

# Count unreachable\! macros
echo "Scanning for unreachable\! macros..."
if rg "unreachable\!\(" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' >/dev/null 2>&1; then
    unreachable_count=$(rg "unreachable\!\(" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' 2>/dev/null | wc -l | tr -d ' ')
fi

# Count todo\! macros
echo "Scanning for todo\! macros..."
if rg "todo\!\(" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' >/dev/null 2>&1; then
    todo_count=$(rg "todo\!\(" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' 2>/dev/null | wc -l | tr -d ' ')
fi

# Calculate totals
total_panics=$((unwrap_count + expect_count + panic_count + unreachable_count + todo_count))

echo "📊 PANIC DETECTION SUMMARY"
echo "┌─────────────────────┬───────┐"
echo "│ Type                │ Count │"
echo "├─────────────────────┼───────┤"
printf "│ %-19s │ %5d │\n" "unwrap() calls" "$unwrap_count"
printf "│ %-19s │ %5d │\n" "expect() calls" "$expect_count" 
printf "│ %-19s │ %5d │\n" "panic\! macros" "$panic_count"
printf "│ %-19s │ %5d │\n" "unreachable\! macros" "$unreachable_count"
printf "│ %-19s │ %5d │\n" "todo\! macros" "$todo_count"
echo "├─────────────────────┼───────┤"
printf "│ %-19s │ %5d │\n" "TOTAL PANICS" "$total_panics"
echo "└─────────────────────┴───────┘"

echo ""

# Show detailed breakdown by file (top 10 worst offenders)
echo "🎯 TOP 10 FILES BY PANIC COUNT"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
{
    rg "\.unwrap\(\)|\.expect\(|panic\!\(|unreachable\!\(|todo\!\(" --type rust -g '\!tests/' -g '\!**/tests/**' -g '\!*test*' -g '\!benches/' -c 2>/dev/null | sort -t: -k2 -nr | head -10
} || echo "No files with panic patterns found\!"

echo ""

# Production readiness assessment
if [ "$total_panics" -eq 0 ]; then
    echo "🎉 PRODUCTION READY: No panic-inducing patterns detected\!"
    exit 0
elif [ "$total_panics" -lt 10 ]; then
    echo "⚠️  CAUTION: $total_panics panic patterns detected - review recommended"
    exit 1
elif [ "$total_panics" -lt 50 ]; then
    echo "❌ NEEDS WORK: $total_panics panic patterns detected - significant cleanup required"
    exit 1
else
    echo "🚨 CRITICAL: $total_panics panic patterns detected - major refactoring needed before production"
    exit 1
fi
EOF < /dev/null
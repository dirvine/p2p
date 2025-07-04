#!/bin/bash
# Fix existing app bundles with a more robust launcher

echo "🔧 Fixing App Bundle Launchers"
echo "=============================="
echo ""

fix_app_launcher() {
    local APP_NAME="$1"
    local BINARY_NAME="$2"
    
    if [ ! -d "$APP_NAME.app" ]; then
        echo "❌ $APP_NAME.app not found"
        return 1
    fi
    
    echo "Fixing $APP_NAME.app..."
    
    # Create a new launcher that's more robust
    cat > "$APP_NAME.app/Contents/MacOS/launcher" << 'EOF'
#!/bin/bash
# Robust launcher

# Get the full path to this script's directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BINARY_NAME="'$BINARY_NAME'"

# Create a command file that Terminal will execute
COMMAND_FILE="/tmp/saorsa_launcher_$$.command"
cat > "$COMMAND_FILE" << SCRIPT_END
#!/bin/bash
clear
echo "Starting '$APP_NAME'..."
echo ""
echo "Working directory: \$(pwd)"
echo "Binary path: $SCRIPT_DIR/$BINARY_NAME"
echo ""

# Check if binary exists and is executable
if [ ! -f "$SCRIPT_DIR/$BINARY_NAME" ]; then
    echo "ERROR: Binary not found at: $SCRIPT_DIR/$BINARY_NAME"
    echo ""
    ls -la "$SCRIPT_DIR/"
    echo ""
    echo "Press any key to exit..."
    read -n 1
    exit 1
fi

if [ ! -x "$SCRIPT_DIR/$BINARY_NAME" ]; then
    echo "ERROR: Binary is not executable"
    echo "Fixing permissions..."
    chmod +x "$SCRIPT_DIR/$BINARY_NAME"
fi

# Run the binary
"$SCRIPT_DIR/$BINARY_NAME"
EXIT_CODE=\$?

echo ""
if [ \$EXIT_CODE -eq 0 ]; then
    echo "Application exited normally."
else
    echo "Application exited with code: \$EXIT_CODE"
fi
echo ""
echo "Press any key to close this window..."
read -n 1
rm -f "$COMMAND_FILE"
exit
SCRIPT_END

chmod +x "$COMMAND_FILE"

# Open the command file with Terminal
open -a Terminal "$COMMAND_FILE"
EOF
    
    chmod +x "$APP_NAME.app/Contents/MacOS/launcher"
    
    # Ensure binary is executable
    if [ -f "$APP_NAME.app/Contents/MacOS/$BINARY_NAME" ]; then
        chmod +x "$APP_NAME.app/Contents/MacOS/$BINARY_NAME"
        echo "✅ Fixed $APP_NAME.app"
    else
        echo "⚠️  Binary missing: $BINARY_NAME"
    fi
}

# Fix both apps
fix_app_launcher "Saorsa Terminal Chat" "saorsa-terminal-chat"
fix_app_launcher "Saorsa Network Tester" "saorsa-network-tester"

echo ""
echo "✅ Launchers fixed!"
echo ""
echo "Try double-clicking the apps again."
echo "If you still see issues, check Console.app for error messages."
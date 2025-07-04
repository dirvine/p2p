#!/bin/bash
# Create simple icons for the macOS apps using ImageMagick or sips

echo "🎨 Creating App Icons"
echo "===================="
echo ""

# Function to create icon using macOS sips
create_icon_with_sips() {
    local TEXT="$1"
    local COLOR="$2"
    local OUTPUT="$3"
    
    # Create a temporary image with colored background
    # Using sips is limited, so we'll create a simple colored square
    # For better icons, use a proper icon editor or ImageMagick
    
    echo "Note: For better icons, consider using:"
    echo "  - Icon Set Creator (Mac App Store)"
    echo "  - makeicns command line tool"
    echo "  - Professional icon design tools"
}

# Check if ImageMagick is installed
if command -v convert &> /dev/null; then
    echo "✅ ImageMagick found - creating custom icons..."
    
    # Create icon for Terminal Chat
    convert -size 512x512 xc:'#4A90E2' \
        -gravity center -pointsize 200 -fill white \
        -annotate +0+0 "💬" \
        -bordercolor '#4A90E2' -border 0 \
        "Saorsa Terminal Chat.app/Contents/Resources/AppIcon.png" 2>/dev/null
    
    # Create icon for Network Tester  
    convert -size 512x512 xc:'#50C878' \
        -gravity center -pointsize 200 -fill white \
        -annotate +0+0 "🔍" \
        -bordercolor '#50C878' -border 0 \
        "Saorsa Network Tester.app/Contents/Resources/AppIcon.png" 2>/dev/null
    
    # Convert to icns format if possible
    if command -v makeicns &> /dev/null; then
        makeicns -in "Saorsa Terminal Chat.app/Contents/Resources/AppIcon.png" \
                 -out "Saorsa Terminal Chat.app/Contents/Resources/AppIcon.icns" 2>/dev/null
        makeicns -in "Saorsa Network Tester.app/Contents/Resources/AppIcon.png" \
                 -out "Saorsa Network Tester.app/Contents/Resources/AppIcon.icns" 2>/dev/null
    fi
    
    echo "✅ Icons created!"
else
    echo "ℹ️  ImageMagick not found. Icons will use system defaults."
    echo ""
    echo "To create custom icons, install ImageMagick:"
    echo "  brew install imagemagick"
    echo ""
    echo "Or create icons manually:"
    echo "  1. Create a 512x512 PNG image"
    echo "  2. Save as AppIcon.png in the app's Resources folder"
    echo "  3. Convert to .icns format using makeicns or Icon Set Creator"
fi

echo ""
echo "💡 Tips for better icons:"
echo "  • Use Icon Set Creator from Mac App Store"
echo "  • Create 1024x1024 artwork and let it generate all sizes"
echo "  • Include @2x versions for Retina displays"
echo "  • Follow Apple's Human Interface Guidelines"
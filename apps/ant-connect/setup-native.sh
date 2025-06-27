#!/bin/bash
# Setup script for Ant Connect native app builds

set -e  # Exit on any error

echo "🚀 Setting up Ant Connect for native app builds..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "pubspec.yaml" ]; then
    echo -e "${RED}❌ Error: Please run this script from the Flutter project root directory${NC}"
    exit 1
fi

echo -e "${BLUE}📱 Configuring Flutter project...${NC}"

# Get Flutter dependencies
flutter pub get

echo -e "${BLUE}💎 Installing Ruby dependencies...${NC}"

# Install Bundler if not present
if ! command -v bundle &> /dev/null; then
    echo -e "${YELLOW}⚠️  Installing Bundler...${NC}"
    gem install bundler
fi

# Install Fastlane and dependencies
bundle install

echo -e "${BLUE}🍎 Setting up iOS configuration...${NC}"

# Configure iOS bundle identifier in Xcode project
if [ -f "ios/Runner.xcodeproj/project.pbxproj" ]; then
    # Update bundle identifier in Xcode project
    sed -i '' 's/PRODUCT_BUNDLE_IDENTIFIER = .*/PRODUCT_BUNDLE_IDENTIFIER = org.p2pfoundation.antconnect;/g' ios/Runner.xcodeproj/project.pbxproj
    echo -e "${GREEN}✅ Updated iOS bundle identifier${NC}"
else
    echo -e "${YELLOW}⚠️  iOS project not found - skipping iOS setup${NC}"
fi

echo -e "${BLUE}🤖 Setting up Android configuration...${NC}"

# Generate Android keystore for release builds (if not exists)
if [ ! -f "android/app/upload-keystore.jks" ]; then
    echo -e "${YELLOW}🔑 Generating Android upload keystore...${NC}"
    echo -e "${YELLOW}Please provide the following information for your keystore:${NC}"
    
    keytool -genkey -v -keystore android/app/upload-keystore.jks \
        -keyalg RSA -keysize 2048 -validity 10000 \
        -alias upload \
        -storepass android \
        -keypass android \
        -dname "CN=P2P Foundation, OU=Development, O=P2P Foundation, L=Unknown, S=Unknown, C=US"
    
    echo -e "${GREEN}✅ Generated Android keystore${NC}"
    
    # Create key.properties file
    cat > android/key.properties << EOF
storePassword=android
keyPassword=android
keyAlias=upload
storeFile=upload-keystore.jks
EOF
    
    echo -e "${GREEN}✅ Created Android key.properties${NC}"
else
    echo -e "${GREEN}✅ Android keystore already exists${NC}"
fi

# Update Android build.gradle for release signing
if ! grep -q "signingConfigs" android/app/build.gradle.kts; then
    echo -e "${YELLOW}📝 Updating Android build configuration for release signing...${NC}"
    # Note: Manual update required for Kotlin DSL - instructions provided
    echo -e "${BLUE}📋 Manual step required:${NC}"
    echo -e "Please add the following to your android/app/build.gradle.kts file:"
    echo -e "1. Add at the top: val keystoreProperties = Properties()"
    echo -e "2. Add release signing config in the signingConfigs block"
    echo -e "3. Reference the signing config in the release buildType"
    echo -e "See Fastlane documentation for details."
fi

echo -e "${BLUE}🌐 Building Flutter web for PWA...${NC}"

# Build Flutter web app
flutter build web --release

echo -e "${GREEN}✅ Flutter web build complete${NC}"

echo -e "${BLUE}📋 Next steps:${NC}"
echo -e "1. ${YELLOW}iOS Setup:${NC}"
echo -e "   - Set up Apple Developer account and certificates"
echo -e "   - Configure App Store Connect app with bundle ID: org.p2pfoundation.antconnect"
echo -e "   - Run: cd ios && bundle exec fastlane certificates"
echo -e "   - Run: cd ios && bundle exec fastlane beta (for TestFlight)"
echo -e ""
echo -e "2. ${YELLOW}Android Setup:${NC}"
echo -e "   - Set up Google Play Console app with package: org.p2pfoundation.antconnect"
echo -e "   - Upload your upload-keystore.jks to Play Console"
echo -e "   - Create service account and download JSON key"
echo -e "   - Run: cd android && bundle exec fastlane internal (for internal testing)"
echo -e ""
echo -e "3. ${YELLOW}Development Testing:${NC}"
echo -e "   - PWA: Open build/web/index.html in browser"
echo -e "   - iOS: flutter run -d ios"
echo -e "   - Android: flutter run -d android"
echo -e ""
echo -e "4. ${YELLOW}Production Builds:${NC}"
echo -e "   - iOS TestFlight: cd ios && bundle exec fastlane beta"
echo -e "   - Android Internal: cd android && bundle exec fastlane internal"
echo -e "   - Web PWA: Serve build/web/ directory"

echo -e "${GREEN}🎉 Native app setup complete!${NC}"
echo -e "${BLUE}📱 You now have both PWA and native app builds configured.${NC}"
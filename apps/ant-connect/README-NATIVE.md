# Ant Connect - Native App Setup

This document covers setting up Ant Connect for both PWA and native app distribution using Fastlane.

## Overview

Ant Connect supports three distribution methods:

1. **PWA (Progressive Web App)** - Immediate cross-platform access via browsers
2. **iOS Native App** - App Store distribution with Fastlane automation
3. **Android Native App** - Google Play Store distribution with Fastlane automation

## Quick Start

```bash
# Install and run the setup script
./setup-native.sh

# Or run individual steps manually (see below)
```

## PWA Setup (Already Complete)

✅ **Status: Ready to use**

The PWA is automatically built and embedded in the `ant-connect` Rust binary:

```bash
# Install and run PWA
cargo install ant-connect
ant-connect  # Opens PWA in browser
```

Features:
- Installable via browser
- Offline capability
- Cross-platform compatibility
- Three-word address system
- Network diagnostics

## iOS Native App Setup

### Prerequisites

1. **Apple Developer Account** ($99/year)
2. **Xcode** (latest version)
3. **Ruby and Bundler** (for Fastlane)

### Setup Steps

1. **Install Dependencies**
   ```bash
   bundle install
   flutter pub get
   ```

2. **Configure Bundle Identifier**
   - Bundle ID: `org.p2pfoundation.antconnect`
   - Already configured in project files

3. **Set up Certificates**
   ```bash
   cd ios
   bundle exec fastlane certificates
   ```

4. **App Store Connect Setup**
   - Create app with bundle ID: `org.p2pfoundation.antconnect`
   - Configure app metadata (automated via Deliverfile)

### iOS Build Commands

```bash
cd ios

# Build for development
bundle exec fastlane build_debug

# Build for release
bundle exec fastlane build_release

# Deploy to TestFlight
bundle exec fastlane beta

# Deploy to App Store (draft)
bundle exec fastlane release

# Automated production release
bundle exec fastlane auto_release
```

### iOS Configuration Files

- `ios/fastlane/Fastfile` - Build and deployment automation
- `ios/fastlane/Appfile` - Apple ID and team configuration
- `ios/fastlane/Deliverfile` - App Store metadata
- `ios/Runner/Info.plist` - App bundle configuration

## Android Native App Setup

### Prerequisites

1. **Google Play Console Account** ($25 one-time fee)
2. **Android Studio** (latest version)
3. **Java 11+** (for Android builds)

### Setup Steps

1. **Install Dependencies**
   ```bash
   bundle install
   flutter pub get
   ```

2. **Configure Package Name**
   - Package: `org.p2pfoundation.antconnect`
   - Already configured in project files

3. **Generate Upload Keystore**
   ```bash
   # Automated by setup script, or manually:
   keytool -genkey -v -keystore android/app/upload-keystore.jks \
     -keyalg RSA -keysize 2048 -validity 10000 \
     -alias upload
   ```

4. **Google Play Console Setup**
   - Create app with package: `org.p2pfoundation.antconnect`
   - Upload upload keystore
   - Create service account and download JSON key

### Android Build Commands

```bash
cd android

# Build debug APK
bundle exec fastlane build_debug

# Build release APK
bundle exec fastlane build_apk

# Build release AAB (App Bundle)
bundle exec fastlane build_aab

# Deploy to internal testing
bundle exec fastlane internal

# Deploy to alpha (closed testing)
bundle exec fastlane alpha

# Deploy to beta (open testing)
bundle exec fastlane beta

# Deploy to production (draft)
bundle exec fastlane release
```

### Android Configuration Files

- `android/fastlane/Fastfile` - Build and deployment automation
- `android/fastlane/Appfile` - Google Play configuration
- `android/fastlane/metadata/` - Play Store metadata
- `android/app/build.gradle.kts` - App build configuration

## Development Workflow

### Local Development

```bash
# Web development
flutter run -d chrome

# iOS development
flutter run -d ios

# Android development  
flutter run -d android

# Build PWA for embedding
flutter build web --release
```

### Testing Builds

```bash
# Test iOS on TestFlight
cd ios && bundle exec fastlane beta

# Test Android internal
cd android && bundle exec fastlane internal

# Test PWA
cargo build && ./target/debug/ant-connect
```

### Production Releases

```bash
# iOS App Store
cd ios && bundle exec fastlane release

# Android Play Store
cd android && bundle exec fastlane release

# Update embedded PWA
flutter build web --release && cargo build --release
```

## Configuration Reference

### Package/Bundle Identifiers

- iOS Bundle ID: `org.p2pfoundation.antconnect`
- Android Package: `org.p2pfoundation.antconnect`
- PWA Start URL: `/`

### App Names

- Display Name: "Ant Connect"
- Short Name: "Ant Connect" 
- Internal Name: `ant_connect`

### Version Management

- Flutter: `pubspec.yaml` version field
- iOS: Uses Flutter version automatically
- Android: Uses Flutter version automatically
- Rust binary: `Cargo.toml` version field

## Troubleshooting

### Common Issues

1. **iOS Certificate Problems**
   ```bash
   cd ios
   bundle exec fastlane match nuke development
   bundle exec fastlane match nuke distribution
   bundle exec fastlane certificates
   ```

2. **Android Signing Issues**
   - Verify `android/key.properties` exists
   - Check keystore passwords
   - Ensure upload certificate matches Play Console

3. **Flutter Build Errors**
   ```bash
   flutter clean
   flutter pub get
   flutter doctor
   ```

4. **Fastlane Ruby Issues**
   ```bash
   bundle update
   gem cleanup
   ```

### Logs and Debugging

- iOS logs: `cd ios && bundle exec fastlane build_debug --verbose`
- Android logs: `cd android && bundle exec fastlane build_debug --verbose`
- Flutter logs: `flutter run --verbose`

## Security Notes

⚠️ **Important Security Considerations:**

1. **Never commit secrets to git:**
   - `ios/fastlane/AuthKey.p8`
   - `android/fastlane/google-play-service-account.json`
   - `android/app/upload-keystore.jks`
   - `android/key.properties`

2. **Use environment variables for CI/CD:**
   - `FASTLANE_APPLE_ID`
   - `FASTLANE_PASSWORD`
   - `MATCH_PASSWORD`

3. **Store certificates securely:**
   - Use Fastlane Match for iOS certificates
   - Store Android keystore in secure location

## Resources

- [Fastlane Documentation](https://docs.fastlane.tools/)
- [Flutter Deployment Guide](https://flutter.dev/docs/deployment)
- [Apple Developer Documentation](https://developer.apple.com/documentation/)
- [Google Play Console Help](https://support.google.com/googleplay/android-developer/)
- [PWA Developer Guide](https://web.dev/progressive-web-apps/)

## Support

For issues with:
- App functionality: [GitHub Issues](https://github.com/dirvine/p2p/issues)
- Build/deployment: Check logs and troubleshooting section above
- Store submission: Consult platform-specific documentation
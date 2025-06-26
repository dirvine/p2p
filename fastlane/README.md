# P2P Fastlane Automation Setup

This directory contains the Fastlane automation setup for the P2P monorepo, enabling automated building and deployment of multiple Flutter apps with a shared Rust backend.

## 🚀 Quick Start

### Prerequisites
1. **Ruby** (3.0+) - Install via `brew install ruby` or use rbenv
2. **Bundler** - Install via `gem install bundler`
3. **Flutter** (3.19.0+) - [Install Flutter](https://flutter.dev/docs/get-started/install)
4. **Rust** (stable) - [Install Rust](https://rustup.rs/)
5. **Xcode** (for iOS) - Download from Mac App Store
6. **Android Studio** (for Android) - [Download Android Studio](https://developer.android.com/studio)

### Initial Setup
```bash
# Install Ruby dependencies
bundle install

# Setup development environment (installs Rust targets, Flutter deps)
fastlane setup

# Test with a development build
fastlane build_dev app:ant-connect
```

## 🏗️ Available Commands

### Development & Testing
```bash
# Setup development environment
fastlane setup

# Build development versions (no signing)
fastlane build_dev                    # All apps
fastlane build_dev app:ant-connect    # Specific app

# Build Rust backend only
fastlane build_rust
```

### Beta Deployment
```bash
# Deploy all apps to beta channels (TestFlight + Play Internal Testing)
fastlane beta_all

# Deploy specific app to beta
fastlane deploy_app app:ant-connect platform:ios
fastlane deploy_app app:ant-connect platform:android
```

### Production Deployment
```bash
# Deploy all apps to production
fastlane deploy_all_apps

# Deploy specific app to production
fastlane deploy_app app:ant-connect
```

### Utilities
```bash
# Generate screenshots for all apps
fastlane screenshots_all

# View available lanes
fastlane lanes
```

## 📱 Apps Configuration

Currently configured apps:
- **ant-connect**: Main P2P connection app
  - iOS Bundle ID: `net.maidsafe.antconnect`
  - Android Package: `net.maidsafe.antconnect`

To add a new app, edit `fastlane/Fastfile` and add to the `APPS` hash:
```ruby
APPS = {
  "your-new-app" => {
    ios_scheme: "YourNewApp",
    android_package: "net.maidsafe.yournewapp",
    bundle_id: "net.maidsafe.yournewapp",
    display_name: "Your New App"
  }
}
```

## 🔐 Secrets & Configuration

### Required Secrets (for CI/CD)
Set these in GitHub repository settings under Secrets and Variables:

#### iOS Deployment
- `FASTLANE_USER`: Your Apple Developer account email
- `FASTLANE_PASSWORD`: Your Apple Developer account password
- `FASTLANE_APPLE_APPLICATION_SPECIFIC_PASSWORD`: App-specific password for 2FA
- `APP_STORE_CONNECT_API_KEY_ID`: App Store Connect API key ID
- `APP_STORE_CONNECT_API_ISSUER_ID`: App Store Connect API issuer ID
- `APP_STORE_CONNECT_API_KEY`: App Store Connect API key content (.p8 file)
- `MATCH_PASSWORD`: Password for certificate encryption
- `MATCH_GIT_BASIC_AUTHORIZATION`: Base64 encoded Git credentials for certificate repo

#### Android Deployment
- `ANDROID_KEYSTORE`: Base64 encoded Android keystore file
- `ANDROID_KEY_ALIAS`: Android key alias
- `ANDROID_STORE_PASSWORD`: Android keystore password
- `ANDROID_KEY_PASSWORD`: Android key password
- `GOOGLE_PLAY_JSON_KEY`: Google Play Service Account JSON key

### Local Configuration Files
Update these files with your specific details:

1. **`fastlane/Appfile`**: Apple Developer account details
2. **`fastlane/Matchfile`**: Certificate management (create a private repo for certificates)
3. **`fastlane/Deliverfile`**: App Store metadata

## 🤖 Automated Deployments

### GitHub Actions Triggers
- **Production**: Push a version tag (`git tag v1.0.0 && git push --tags`)
- **Beta**: Push to `main` branch
- **Manual**: Use GitHub Actions "Run workflow" button

### Deployment Flow
1. **Change Detection**: Only builds/deploys changed apps
2. **Rust Backend**: Built once, shared by all apps
3. **Parallel Deployment**: iOS and Android deploy simultaneously
4. **Multi-App Support**: All apps in the monorepo deploy together

## 📋 Manual Deployment Steps

### First-Time iOS Setup
1. Create certificates repository:
   ```bash
   # Create a private repo: dirvine/p2p-certificates
   # Then run:
   fastlane match init
   ```

2. Generate certificates:
   ```bash
   fastlane match development
   fastlane match appstore
   ```

### First-Time Android Setup
1. Generate upload keystore:
   ```bash
   keytool -genkey -v -keystore upload-keystore.jks -keyalg RSA -keysize 2048 -validity 10000 -alias upload
   ```

2. Configure `android/key.properties` in each app

## 🔧 Troubleshooting

### Common Issues

**"Flutter not found"**
```bash
export PATH="$PATH:`pwd`/flutter/bin"
flutter doctor
```

**"Rust targets missing"**
```bash
rustup target add aarch64-apple-ios x86_64-apple-ios
rustup target add aarch64-linux-android armv7-linux-androideabi
```

**"Certificate issues on iOS"**
```bash
fastlane match nuke development  # Reset certificates
fastlane match development       # Regenerate
```

**"Android signing issues"**
- Verify keystore path in `android/key.properties`
- Check that keystore passwords are correct

### Debug Mode
Add `--verbose` to any fastlane command for detailed output:
```bash
fastlane deploy_app app:ant-connect --verbose
```

## 📚 Documentation Links
- [Fastlane Documentation](https://docs.fastlane.tools/)
- [Flutter CI/CD Guide](https://flutter.dev/docs/deployment/cd)
- [App Store Connect API](https://developer.apple.com/documentation/appstoreconnectapi)
- [Google Play Console API](https://developers.google.com/android-publisher)

## 🆘 Getting Help
1. Check the logs in the terminal output
2. Review the GitHub Actions logs for CI/CD issues
3. Consult the Fastlane documentation
4. Open an issue in the repository if needed
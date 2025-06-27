# GitHub Actions Workflows for P2P Desktop App

This directory contains GitHub Actions workflows for building and releasing the Tauri desktop application.

## Available Workflows

### 1. `release.yml` - Basic Release Workflow
Simple workflow that builds and releases the desktop app when pushing to the `release` branch.

**Triggers:**
- Push to `release` branch
- Manual workflow dispatch

**Features:**
- Cross-platform builds (Windows, macOS Intel/Apple Silicon, Linux)
- Automatic GitHub release creation
- Draft releases for review

### 2. `release-desktop.yml` - Advanced Release Workflow
More comprehensive workflow with better release management and artifact handling.

**Triggers:**
- Push to `release` branch
- Manual workflow dispatch with optional version input

**Features:**
- Checks for existing releases to avoid duplicates
- Better release notes formatting
- Separate release creation and building steps
- Automatic .app bundle upload for macOS
- Publishes release after all builds complete

### 3. `test-desktop-build.yml` - CI Build Testing
Continuous integration workflow for testing builds on pull requests and main branch.

**Triggers:**
- Pull requests affecting `apps/desktop-tauri/**`
- Pushes to `main` or `develop` branches
- Manual workflow dispatch

**Features:**
- Debug builds for faster CI
- Frontend linting
- Rust tests
- Artifact upload on failure for debugging

### 4. `release-signed.yml` - Production Release with Code Signing
Advanced workflow for creating signed production releases (requires certificates).

**Triggers:**
- Manual workflow dispatch only (with version input)

**Features:**
- macOS code signing and notarization
- Windows code signing
- Universal binary for macOS
- Enhanced security notices in release notes

## Setup Instructions

### Basic Setup (No Code Signing)

1. **Enable GitHub Actions** in your repository settings

2. **Configure Permissions**:
   - Go to Settings → Actions → General
   - Under "Workflow permissions", select "Read and write permissions"
   - Check "Allow GitHub Actions to create and approve pull requests"

3. **Create a Release Branch**:
   ```bash
   git checkout -b release
   git push -u origin release
   ```

4. **Trigger a Release**:
   - Push to the `release` branch, or
   - Go to Actions tab → Select workflow → Run workflow

### Code Signing Setup (Optional)

For production releases with code signing, you'll need to set up the following secrets:

#### macOS Signing
- `APPLE_CERTIFICATE`: Base64 encoded .p12 certificate
- `APPLE_CERTIFICATE_PASSWORD`: Certificate password
- `APPLE_SIGNING_IDENTITY`: Certificate common name (e.g., "Developer ID Application: Your Name")
- `APPLE_ID`: Your Apple ID email
- `APPLE_PASSWORD`: App-specific password (not your regular password)
- `APPLE_TEAM_ID`: Your Apple Developer Team ID

#### Windows Signing
- `WINDOWS_CERTIFICATE`: Base64 encoded .pfx certificate
- `WINDOWS_CERTIFICATE_PASSWORD`: Certificate password

#### Tauri Updater (Optional)
- `TAURI_PRIVATE_KEY`: Private key for update signatures
- `TAURI_KEY_PASSWORD`: Password for the private key

### Converting Certificates to Base64

**macOS (.p12):**
```bash
base64 -i certificate.p12 -o certificate_base64.txt
# Copy contents to GitHub secret
```

**Windows (.pfx):**
```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("certificate.pfx")) | Out-File certificate_base64.txt
# Copy contents to GitHub secret
```

## Workflow Configuration

### Modifying Package Manager

If using yarn or pnpm instead of npm, update the following in each workflow:

1. Change the cache setting in `setup-node`:
   ```yaml
   - uses: actions/setup-node@v4
     with:
       node-version: lts/*
       cache: 'yarn'  # or 'pnpm'
   ```

2. Update install commands:
   ```yaml
   - name: Install frontend dependencies
     run: |
       cd apps/desktop-tauri
       yarn install --frozen-lockfile  # or pnpm install --frozen-lockfile
   ```

### Customizing Build Targets

To add or remove platform targets, modify the `matrix` section:

```yaml
matrix:
  include:
    - platform: 'ubuntu-20.04'  # For older Linux compatibility
      args: ''
      name: 'Linux (Ubuntu 20.04)'
```

### Version Management

The workflows use the version from `tauri.conf.json`. To override:

1. Use manual workflow dispatch with version input
2. Or modify the version extraction logic in the workflow

## Troubleshooting

### Common Issues

1. **Permission Denied**: Ensure workflow permissions are set to "Read and write"

2. **Missing Dependencies**: The Linux build requires webkit2gtk. The workflow installs both 4.0 and 4.1 versions.

3. **Cache Issues**: Clear caches in Actions → Caches if builds are failing

4. **Release Already Exists**: The advanced workflow checks for existing releases to prevent duplicates

### Debug Tips

- Check workflow logs in the Actions tab
- Use `test-desktop-build.yml` to test changes without creating releases
- Add `RUST_BACKTRACE=full` to environment variables for detailed errors
- Failed builds upload artifacts for debugging (3-day retention)

## Best Practices

1. **Test First**: Use `test-desktop-build.yml` before releasing
2. **Version Bumping**: Update version in `tauri.conf.json` before releasing
3. **Release Notes**: Edit draft releases to add detailed changelog
4. **Branch Protection**: Consider protecting the `release` branch
5. **Secrets Security**: Rotate certificates periodically
6. **Cache Management**: Monitor cache usage in Actions settings

## Additional Resources

- [Tauri GitHub Action Documentation](https://github.com/tauri-apps/tauri-action)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Tauri Code Signing Guide](https://tauri.app/v1/guides/distribution/sign)
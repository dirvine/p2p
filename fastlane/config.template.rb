# Configuration template for Apple Developer settings
# Copy this to config.local.rb and fill in your actual values

# Apple Developer Account
APPLE_ID = "david@maidsafe.net"
TEAM_ID = "YOUR_TEAM_ID_HERE"          # Find this in Apple Developer portal
ITC_TEAM_ID = "YOUR_ITC_TEAM_ID_HERE"  # App Store Connect team ID

# Bundle IDs (update these to match your actual app IDs)
BUNDLE_IDS = {
  "ant-connect" => "net.maidsafe.antconnect"
}

# Git repository for certificate storage (create a private repo)
MATCH_GIT_URL = "git@github.com:dirvine/p2p-certificates.git"

# App Store Connect API (recommended for CI/CD)
# Generate these at: https://appstoreconnect.apple.com/access/api
ASC_API_KEY_ID = "YOUR_API_KEY_ID"
ASC_API_ISSUER_ID = "YOUR_ISSUER_ID"
ASC_API_KEY_PATH = "./AuthKey_YOUR_API_KEY_ID.p8"
#!/bin/bash

# Quick deployment script for P2P apps
# Usage: ./deploy.sh [app_name] [platform] [type]
# Examples:
#   ./deploy.sh ant-connect ios beta
#   ./deploy.sh all both production
#   ./deploy.sh

set -e

# Default values
APP_NAME=${1:-"ant-connect"}
PLATFORM=${2:-"both"}
DEPLOY_TYPE=${3:-"beta"}

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

echo "🚀 P2P App Deployment"
echo "======================="
echo "App: $APP_NAME"
echo "Platform: $PLATFORM"
echo "Type: $DEPLOY_TYPE"
echo

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "fastlane" ]; then
    echo "❌ Please run this script from the p2p repository root"
    exit 1
fi

# Check prerequisites
if ! command -v fastlane &> /dev/null; then
    echo "❌ Fastlane not found. Run ./setup.sh first"
    exit 1
fi

case $DEPLOY_TYPE in
    "dev"|"development")
        print_status "Building development version..."
        if [ "$APP_NAME" = "all" ]; then
            fastlane build_dev
        else
            fastlane build_dev app:$APP_NAME
        fi
        ;;
    "beta"|"testing")
        print_status "Deploying to beta channels..."
        if [ "$APP_NAME" = "all" ]; then
            fastlane beta_all
        else
            if [ "$PLATFORM" = "both" ]; then
                fastlane deploy_app app:$APP_NAME
            else
                fastlane deploy_app app:$APP_NAME platform:$PLATFORM
            fi
        fi
        ;;
    "prod"|"production"|"release")
        print_warning "Deploying to production stores..."
        read -p "Are you sure you want to deploy to production? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            if [ "$APP_NAME" = "all" ]; then
                fastlane deploy_all_apps
            else
                fastlane deploy_app app:$APP_NAME platform:$PLATFORM
            fi
        else
            echo "Deployment cancelled"
            exit 0
        fi
        ;;
    *)
        echo "❌ Unknown deployment type: $DEPLOY_TYPE"
        echo "Valid types: dev, beta, production"
        exit 1
        ;;
esac

print_success "🎉 Deployment complete!"
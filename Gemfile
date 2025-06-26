# Gemfile for Fastlane dependencies
source "https://rubygems.org"

gem "fastlane", "~> 2.217"
gem "cocoapods", "~> 1.15"

# Fastlane plugins
plugins_path = File.join(File.dirname(__FILE__), 'fastlane', 'Pluginfile')
eval_gemfile(plugins_path) if File.exist?(plugins_path)
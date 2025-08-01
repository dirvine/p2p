#!/bin/bash

# Fix error.rs to use Cow<'static, str> instead of String for better performance

cd /Users/davidirvine/Desktop/Devel/projects/p2p

# Replace String with Cow<'static, str> in error enum variants
sed -i '' 's/\(#\[error.*\]\)\(.*\)\(String)\)/\1\2Cow<'\''static, str>)/g' crates/p2p-core/src/error.rs

# Replace field types
sed -i '' 's/: String,/: Cow<'\''static, str>,/g' crates/p2p-core/src/error.rs
sed -i '' 's/: String }/: Cow<'\''static, str> }/g' crates/p2p-core/src/error.rs

# Update standalone String parameters
sed -i '' 's/(String)/(Cow<'\''static, str>)/g' crates/p2p-core/src/error.rs

echo "Fixed String to Cow<'static, str> conversions in error.rs"
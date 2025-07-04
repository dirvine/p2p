#!/bin/bash
echo "🔗 Connect to Friend's P2P Node"
echo "==============================="
echo ""
echo "Enter your friend's three-word address"
echo "(Example: ocean.swift.mountain)"
echo ""
read -p "Three-word address: " THREE_WORDS

if [ -z "$THREE_WORDS" ]; then
    echo "No address entered, exiting..."
    exit 1
fi

echo ""
echo "🚀 Connecting to $THREE_WORDS..."
./bin/p2p-chat --bootstrap-words "$THREE_WORDS"

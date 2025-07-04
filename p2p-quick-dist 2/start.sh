#!/bin/bash
echo "🐜 P2P Foundation - Quick Start"
echo "==============================="
echo ""
echo "1) Start P2P chat"
echo "2) Run connectivity test"
echo "3) Run local test network (5 nodes)"
echo ""
read -p "Enter choice (1-3): " choice

case $choice in
    1)
        ./bin/p2p-chat
        ;;
    2)
        ./bin/saorsa-test-suite connectivity --verbose
        ;;
    3)
        ./bin/saorsa-test-suite all --local-nodes 5
        ;;
    *)
        echo "Invalid choice"
        ;;
esac

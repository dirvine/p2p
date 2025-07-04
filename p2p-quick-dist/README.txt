P2P Foundation - Quick Distribution
===================================

This package contains essential P2P Foundation tools for testing.

QUICK START:
-----------
1. Run ./start.sh for interactive menu
2. Run ./connect-to-friend.sh to connect to a friend

MANUAL USAGE:
-------------
Chat:
  ./bin/p2p-chat                                    # Start with auto-discovery
  ./bin/p2p-chat --bootstrap-words friend.address   # Connect to specific node

Testing:
  ./bin/saorsa-test-suite --help                    # See all test options
  ./bin/saorsa-test-suite connectivity --verbose    # Test connectivity
  ./bin/saorsa-test-suite all --local-nodes 5       # Run full test suite

THREE-WORD ADDRESSES:
---------------------
When you start a chat node, you'll see your three-word address like:
"ocean.swift.mountain"

Share this address with friends so they can connect to you!

TROUBLESHOOTING:
----------------
- Ensure ports 9000-9010 are available
- Check firewall settings if connection fails
- Try different bootstrap nodes if auto-discovery fails

For more info: https://github.com/dirvine/p2p

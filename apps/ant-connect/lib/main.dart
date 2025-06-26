import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

void main() {
  runApp(const ConnectApp());
}

class ConnectApp extends StatelessWidget {
  const ConnectApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => NetworkProvider()),
        ChangeNotifierProvider(create: (_) => ChatProvider()),
      ],
      child: MaterialApp(
        title: 'Connect',
        theme: ThemeData(
          useMaterial3: true,
          colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
        ),
        home: const MainScreen(),
        routes: {
          '/connections': (context) => const ConnectionScreen(),
        },
      ),
    );
  }
}

class MainScreen extends StatefulWidget {
  const MainScreen({super.key});

  @override
  State<MainScreen> createState() => _MainScreenState();
}

class _MainScreenState extends State<MainScreen> {
  int _selectedIndex = 0;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: IndexedStack(
        index: _selectedIndex,
        children: const [
          ChatScreen(),
          ConnectionScreen(),
        ],
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _selectedIndex,
        onDestinationSelected: (index) {
          setState(() {
            _selectedIndex = index;
          });
        },
        destinations: const [
          NavigationDestination(
            icon: Icon(Icons.chat),
            label: 'Chat',
          ),
          NavigationDestination(
            icon: Icon(Icons.hub),
            label: 'Network',
          ),
        ],
      ),
    );
  }
}

class ChatScreen extends StatefulWidget {
  const ChatScreen({super.key});

  @override
  State<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends State<ChatScreen> {
  final TextEditingController _messageController = TextEditingController();

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Connect'),
        actions: [
          Consumer<NetworkProvider>(
            builder: (context, networkProvider, child) {
              return IconButton(
                icon: Badge(
                  label: Text('${networkProvider.peerCount}'),
                  isLabelVisible: networkProvider.peerCount > 0,
                  child: const Icon(Icons.people),
                ),
                onPressed: () {
                  Navigator.pushNamed(context, '/connections');
                },
                tooltip: '${networkProvider.peerCount} peers connected',
              );
            },
          ),
        ],
      ),
      body: Column(
        children: [
          // Connection status
          Consumer<NetworkProvider>(
            builder: (context, networkProvider, child) {
              if (networkProvider.peerCount == 0) {
                return Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(12),
                  color: Colors.orange.withOpacity(0.1),
                  child: Row(
                    children: [
                      const Icon(Icons.wifi_off, color: Colors.orange),
                      const SizedBox(width: 8),
                      const Text('Not connected to any peers'),
                      const Spacer(),
                      TextButton(
                        onPressed: () {
                          Navigator.pushNamed(context, '/connections');
                        },
                        child: const Text('Connect'),
                      ),
                    ],
                  ),
                );
              }
              return const SizedBox.shrink();
            },
          ),
          
          // Messages area
          Expanded(
            child: Consumer<ChatProvider>(
              builder: (context, chatProvider, child) {
                if (chatProvider.messages.isEmpty) {
                  return const Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(Icons.chat_bubble_outline, size: 64, color: Colors.grey),
                        SizedBox(height: 16),
                        Text('No messages yet', style: TextStyle(fontSize: 18, color: Colors.grey)),
                        Text('Connect to peers to start chatting', style: TextStyle(color: Colors.grey)),
                      ],
                    ),
                  );
                }
                
                return ListView.builder(
                  padding: const EdgeInsets.all(16),
                  itemCount: chatProvider.messages.length,
                  itemBuilder: (context, index) {
                    final message = chatProvider.messages[index];
                    return Padding(
                      padding: const EdgeInsets.symmetric(vertical: 4),
                      child: Align(
                        alignment: message.isFromMe ? Alignment.centerRight : Alignment.centerLeft,
                        child: Container(
                          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                          decoration: BoxDecoration(
                            color: message.isFromMe ? Colors.blue : Colors.grey[300],
                            borderRadius: BorderRadius.circular(18),
                          ),
                          child: Text(
                            message.content,
                            style: TextStyle(
                              color: message.isFromMe ? Colors.white : Colors.black,
                            ),
                          ),
                        ),
                      ),
                    );
                  },
                );
              },
            ),
          ),
          
          // Message input
          Container(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _messageController,
                    decoration: const InputDecoration(
                      hintText: 'Type a message...',
                      border: OutlineInputBorder(),
                    ),
                    onSubmitted: (text) => _sendMessage(),
                  ),
                ),
                const SizedBox(width: 8),
                FloatingActionButton.small(
                  onPressed: _sendMessage,
                  child: const Icon(Icons.send),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
  
  void _sendMessage() {
    final text = _messageController.text.trim();
    if (text.isNotEmpty) {
      Provider.of<ChatProvider>(context, listen: false).addMessage(text);
      _messageController.clear();
    }
  }
}

class ConnectionScreen extends StatefulWidget {
  const ConnectionScreen({super.key});

  @override
  State<ConnectionScreen> createState() => _ConnectionScreenState();
}

class _ConnectionScreenState extends State<ConnectionScreen> {
  final TextEditingController _addressController = TextEditingController();

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Network'),
      ),
      body: Consumer<NetworkProvider>(
        builder: (context, networkProvider, child) {
          return Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // My address card
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            const Icon(Icons.account_circle, color: Colors.blue),
                            const SizedBox(width: 8),
                            Text(
                              'My Connection Info',
                              style: Theme.of(context).textTheme.titleMedium,
                            ),
                          ],
                        ),
                        const SizedBox(height: 12),
                        const Text('Listen Address:', style: TextStyle(color: Colors.grey)),
                        const SizedBox(height: 4),
                        Text(
                          networkProvider.localAddress,
                          style: const TextStyle(fontFamily: 'monospace'),
                        ),
                        const SizedBox(height: 8),
                        const Text('Share this address with others to connect'),
                      ],
                    ),
                  ),
                ),
                
                const SizedBox(height: 16),
                
                // Connect to peer section
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Connect to Peer',
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                        const SizedBox(height: 12),
                        TextField(
                          controller: _addressController,
                          decoration: const InputDecoration(
                            labelText: 'Peer Address',
                            hintText: '/ip6/::1/udp/9001/quic',
                            border: OutlineInputBorder(),
                          ),
                        ),
                        const SizedBox(height: 12),
                        ElevatedButton.icon(
                          onPressed: () {
                            final address = _addressController.text.trim();
                            if (address.isNotEmpty) {
                              networkProvider.connectToPeer(address);
                              _addressController.clear();
                              ScaffoldMessenger.of(context).showSnackBar(
                                SnackBar(content: Text('Connecting to $address')),
                              );
                            }
                          },
                          icon: const Icon(Icons.add_link),
                          label: const Text('Connect'),
                        ),
                      ],
                    ),
                  ),
                ),
                
                const SizedBox(height: 16),
                
                // Quick Connect to Bootstrap Nodes
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            const Icon(Icons.flash_on, color: Colors.green),
                            const SizedBox(width: 8),
                            Text(
                              'Quick Connect',
                              style: Theme.of(context).textTheme.titleMedium,
                            ),
                          ],
                        ),
                        const SizedBox(height: 8),
                        Text(
                          'Connect to well-known bootstrap nodes instantly!',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: Colors.grey[600],
                          ),
                        ),
                        const SizedBox(height: 12),
                        Wrap(
                          spacing: 8,
                          runSpacing: 8,
                          children: [
                            ElevatedButton.icon(
                              onPressed: () {
                                networkProvider.connectToPeer('foundation.main.bootstrap');
                                ScaffoldMessenger.of(context).showSnackBar(
                                  const SnackBar(
                                    content: Text('Connecting to main bootstrap node...'),
                                    backgroundColor: Colors.green,
                                  ),
                                );
                              },
                              icon: const Icon(Icons.rocket_launch, size: 16),
                              label: const Text('Main Bootstrap'),
                              style: ElevatedButton.styleFrom(
                                backgroundColor: Colors.green,
                                foregroundColor: Colors.white,
                              ),
                            ),
                            ElevatedButton.icon(
                              onPressed: () {
                                networkProvider.connectToPeer('global.fast.eagle');
                                ScaffoldMessenger.of(context).showSnackBar(
                                  const SnackBar(
                                    content: Text('Connecting to global.fast.eagle...'),
                                    backgroundColor: Colors.blue,
                                  ),
                                );
                              },
                              icon: const Icon(Icons.public, size: 16),
                              label: const Text('IPv6 Node'),
                              style: ElevatedButton.styleFrom(
                                backgroundColor: Colors.blue,
                                foregroundColor: Colors.white,
                              ),
                            ),
                            OutlinedButton.icon(
                              onPressed: () {
                                networkProvider.autoConnectToBootstraps();
                                ScaffoldMessenger.of(context).showSnackBar(
                                  const SnackBar(
                                    content: Text('Auto-connecting to bootstrap network...'),
                                    backgroundColor: Colors.orange,
                                  ),
                                );
                              },
                              icon: const Icon(Icons.auto_fix_high, size: 16),
                              label: const Text('Auto Connect'),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
                
                const SizedBox(height: 16),
                
                // Connected peers
                Text(
                  'Connected Peers (${networkProvider.peerCount})',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                
                Expanded(
                  child: networkProvider.peers.isEmpty
                      ? const Center(
                          child: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              Icon(Icons.group_off, size: 48, color: Colors.grey),
                              SizedBox(height: 12),
                              Text('No peers connected', style: TextStyle(color: Colors.grey)),
                            ],
                          ),
                        )
                      : ListView.builder(
                          itemCount: networkProvider.peers.length,
                          itemBuilder: (context, index) {
                            final peer = networkProvider.peers[index];
                            return Card(
                              child: ListTile(
                                leading: const CircleAvatar(
                                  child: Icon(Icons.person),
                                ),
                                title: Text(peer.name),
                                subtitle: Text(peer.address),
                                trailing: IconButton(
                                  icon: const Icon(Icons.link_off),
                                  onPressed: () {
                                    networkProvider.disconnectFromPeer(peer.id);
                                  },
                                ),
                              ),
                            );
                          },
                        ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}

// Simple data models
class ChatMessage {
  final String id;
  final String content;
  final bool isFromMe;
  final DateTime timestamp;

  ChatMessage({
    required this.id,
    required this.content,
    required this.isFromMe,
    required this.timestamp,
  });
}

class PeerInfo {
  final String id;
  final String name;
  final String address;

  PeerInfo({
    required this.id,
    required this.name,
    required this.address,
  });
}

// Simple providers
class ChatProvider extends ChangeNotifier {
  final List<ChatMessage> _messages = [];
  
  List<ChatMessage> get messages => _messages;
  
  void addMessage(String content) {
    _messages.add(ChatMessage(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      content: content,
      isFromMe: true,
      timestamp: DateTime.now(),
    ));
    notifyListeners();
  }
}

class NetworkProvider extends ChangeNotifier {
  final List<PeerInfo> _peers = [];
  
  // Hardcoded well-known bootstrap nodes (will be updated with real Digital Ocean IPs)
  static const Map<String, String> _wellKnownBootstraps = {
    'foundation.main.bootstrap': '/dns4/bootstrap.p2pfoundation.org/udp/9000/quic',
    'foundation.backup.lighthouse': '/dns4/bootstrap2.p2pfoundation.org/udp/9000/quic',
    'global.fast.eagle': '/ip6/2001:19f0:7001:3259::1/udp/9000/quic',
    'reliable.sturdy.anchor': '/ip4/147.182.203.123/udp/9000/quic',
    // Demo addresses for local testing
    'local.swift.lighthouse': '/ip6/::1/udp/9000/quic',
    'quick.strong.sword': '/ip6/::1/tcp/9000',
    'demo.test.node': '/ip6/::1/udp/9001/quic',
  };
  
  List<PeerInfo> get peers => _peers;
  int get peerCount => _peers.length;
  String get localAddress => '/ip6/::1/udp/9000/quic';
  
  /// Get the three-word address for a given technical address
  String getThreeWordAddress(String technicalAddress) {
    // Look for exact match first
    for (final entry in _wellKnownBootstraps.entries) {
      if (entry.value == technicalAddress) {
        return entry.key;
      }
    }
    
    // Demo mapping for common local addresses
    final demoMappings = {
      '/ip6/::1/udp/9000/quic': 'local.swift.lighthouse',
      '/ip6/::1/tcp/9000': 'quick.strong.sword',
      '/ip6/::1/udp/9001/quic': 'global.fast.eagle',
    };
    
    return demoMappings[technicalAddress] ?? 'your.node.address';
  }
  
  /// Resolve a three-word address to a technical multiaddr
  String? resolveThreeWordAddress(String threeWordAddress) {
    return _wellKnownBootstraps[threeWordAddress];
  }
  
  /// Get all available well-known three-word addresses
  List<String> getWellKnownAddresses() {
    return _wellKnownBootstraps.keys.toList();
  }
  
  /// Connect to a peer using either three-word address or technical address
  void connectToPeer(String address) {
    String resolvedAddress = address;
    String displayName = 'Peer ${_peers.length + 1}';
    
    // Check if it's a three-word address
    if (_wellKnownBootstraps.containsKey(address)) {
      resolvedAddress = _wellKnownBootstraps[address]!;
      displayName = 'Bootstrap ($address)';
    } else if (address.contains('.') && !address.contains('/')) {
      // Might be a three-word address not in our registry
      displayName = 'Unknown ($address)';
    }
    
    final peer = PeerInfo(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      name: displayName,
      address: resolvedAddress,
    );
    _peers.add(peer);
    notifyListeners();
  }
  
  /// Auto-connect to well-known bootstrap nodes
  void autoConnectToBootstraps() {
    final primaryBootstraps = [
      'foundation.main.bootstrap',
      'global.fast.eagle',
    ];
    
    for (final bootstrap in primaryBootstraps) {
      connectToPeer(bootstrap);
    }
  }
  
  void disconnectFromPeer(String peerId) {
    _peers.removeWhere((peer) => peer.id == peerId);
    notifyListeners();
  }
}
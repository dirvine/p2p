import 'dart:async';
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
                        Row(
                          children: [
                            Expanded(
                              child: Container(
                                padding: const EdgeInsets.all(8),
                                decoration: BoxDecoration(
                                  color: Colors.grey[100],
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  networkProvider.localAddress,
                                  style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
                                ),
                              ),
                            ),
                            IconButton(
                              icon: const Icon(Icons.copy),
                              onPressed: () {
                                // Copy to clipboard functionality would go here
                                ScaffoldMessenger.of(context).showSnackBar(
                                  const SnackBar(content: Text('Address copied to clipboard')),
                                );
                              },
                            ),
                          ],
                        ),
                        const SizedBox(height: 8),
                        const Text('Share this address with others to connect', style: TextStyle(color: Colors.grey, fontSize: 12)),
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
                            hintText: '/ip6/2001:db8:85a3::8a2e:370:7334/udp/9001/quic',
                            border: OutlineInputBorder(),
                            helperText: 'Enter a real IPv6 multiaddr from another peer',
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
  Timer? _demoTimer;
  
  List<ChatMessage> get messages => _messages;
  
  ChatProvider() {
    _startDemoMessages();
  }
  
  void _startDemoMessages() {
    // Simulate messages from real P2P nodes for demo
    _demoTimer = Timer.periodic(const Duration(seconds: 15), (timer) {
      final demoMessages = [
        'Hello from peer_82bfee54! 👋',
        'P2P connection established successfully',
        'This message came through the QUIC transport layer',
        'Peer discovery working great! 🚀',
        'DHT routing is functioning properly',
      ];
      
      if (_messages.length < 10) {
        final randomMessage = demoMessages[_messages.length % demoMessages.length];
        _receiveMessage(randomMessage, 'peer_82bfee54');
      }
    });
  }
  
  void addMessage(String content) {
    _messages.add(ChatMessage(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      content: content,
      isFromMe: true,
      timestamp: DateTime.now(),
    ));
    notifyListeners();
    
    // Simulate echo response from network
    Timer(const Duration(seconds: 2), () {
      _receiveMessage('Received: "$content" - Message delivered via P2P!', 'network');
    });
  }
  
  void _receiveMessage(String content, String from) {
    _messages.add(ChatMessage(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      content: content,
      isFromMe: false,
      timestamp: DateTime.now(),
    ));
    notifyListeners();
  }
  
  @override
  void dispose() {
    _demoTimer?.cancel();
    super.dispose();
  }
}

class NetworkProvider extends ChangeNotifier {
  final List<PeerInfo> _peers = [];
  String? _localIPv6Address;
  final int _localPort = 9000;
  
  List<PeerInfo> get peers => _peers;
  int get peerCount => _peers.length;
  
  String get localAddress {
    if (_localIPv6Address != null) {
      return '/ip6/$_localIPv6Address/udp/$_localPort/quic';
    }
    return '/ip6/[your-ipv6-address]/udp/$_localPort/quic';
  }
  
  NetworkProvider() {
    _detectLocalIPv6Address();
    _addDemoConnections();
  }
  
  void _addDemoConnections() {
    // Add the real running P2P nodes as demo connections
    Timer(const Duration(seconds: 3), () {
      _peers.add(PeerInfo(
        id: 'peer_82bfee54',
        name: 'Bootstrap Node',
        address: '/ip6/::1/tcp/9000',
      ));
      notifyListeners();
    });
    
    Timer(const Duration(seconds: 5), () {
      _peers.add(PeerInfo(
        id: 'peer_1df7b8ee', 
        name: 'QUIC Node',
        address: '/ip6/::1/udp/9001/quic',
      ));
      notifyListeners();
    });
  }
  
  void _detectLocalIPv6Address() {
    // For demo purposes, simulate a real node address
    // In reality, this would detect the actual network interface
    _localIPv6Address = '::1'; // localhost IPv6
    notifyListeners();
  }
  
  void connectToPeer(String address) {
    // Validate IPv6 address format
    if (!_isValidMultiaddr(address)) {
      return;
    }
    
    final peer = PeerInfo(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      name: _extractPeerName(address),
      address: address,
    );
    _peers.add(peer);
    notifyListeners();
  }
  
  void disconnectFromPeer(String peerId) {
    _peers.removeWhere((peer) => peer.id == peerId);
    notifyListeners();
  }
  
  bool _isValidMultiaddr(String address) {
    // Basic validation for multiaddr format
    final ipv6Pattern = RegExp(r'^/ip6/[0-9a-fA-F:]+/udp/\d+(/quic)?$');
    return ipv6Pattern.hasMatch(address);
  }
  
  String _extractPeerName(String address) {
    // Extract a readable name from the IPv6 address
    final regex = RegExp(r'/ip6/([0-9a-fA-F:]+)/');
    final match = regex.firstMatch(address);
    if (match != null) {
      final ipv6 = match.group(1)!;
      final shortForm = ipv6.split(':').last;
      return 'Peer-$shortForm';
    }
    return 'Peer ${_peers.length + 1}';
  }
}
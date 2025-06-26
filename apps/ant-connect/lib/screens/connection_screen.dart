import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'package:ant_connect/providers/network_provider.dart';
import 'package:ant_connect/widgets/peer_card.dart';
import 'package:ant_connect/widgets/network_stats_card.dart';
import 'package:ant_connect/theme/app_theme.dart';

class ConnectionScreen extends StatefulWidget {
  const ConnectionScreen({super.key});

  @override
  State<ConnectionScreen> createState() => _ConnectionScreenState();
}

class _ConnectionScreenState extends State<ConnectionScreen> {
  final TextEditingController _addressController = TextEditingController();
  bool _isConnecting = false;

  @override
  void dispose() {
    _addressController.dispose();
    super.dispose();
  }

  Future<void> _connectToPeer() async {
    final address = _addressController.text.trim();
    if (address.isEmpty) return;

    setState(() {
      _isConnecting = true;
    });

    try {
      final networkProvider = Provider.of<NetworkProvider>(context, listen: false);
      final success = await networkProvider.connectToPeer(address);

      if (mounted) {
        if (success) {
          _addressController.clear();
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(
              content: Text('Connection initiated'),
              backgroundColor: Colors.green,
            ),
          );
        } else {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(
              content: Text('Failed to connect to peer'),
              backgroundColor: Colors.red,
            ),
          );
        }
      }
    } finally {
      if (mounted) {
        setState(() {
          _isConnecting = false;
        });
      }
    }
  }

  void _showAddPeerDialog() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Connect to Peer'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('Enter the peer address:'),
            const SizedBox(height: 8),
            TextField(
              controller: _addressController,
              decoration: const InputDecoration(
                hintText: '/ip6/::1/udp/9001/quic',
                labelText: 'Peer Address',
              ),
              autofocus: true,
            ),
            const SizedBox(height: 16),
            Text(
              'Example: /ip6/::1/udp/9001/quic',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.outline,
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () {
              Navigator.of(context).pop();
            },
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: _isConnecting ? null : () {
              Navigator.of(context).pop();
              _connectToPeer();
            },
            child: _isConnecting 
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Text('Connect'),
          ),
        ],
      ),
    );
  }

  void _copyMyAddress(String address) {
    Clipboard.setData(ClipboardData(text: address));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('Address copied to clipboard'),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Network'),
        actions: [
          IconButton(
            onPressed: () {
              Provider.of<NetworkProvider>(context, listen: false)
                  .refreshNetworkStats();
            },
            icon: const Icon(Icons.refresh),
            tooltip: 'Refresh',
          ),
        ],
      ),
      body: Consumer<NetworkProvider>(
        builder: (context, networkProvider, child) {
          return RefreshIndicator(
            onRefresh: () => networkProvider.refreshNetworkStats(),
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: [
                // My Address Section
                _buildMyAddressCard(networkProvider),
                const SizedBox(height: 16),
                
                // Network Stats
                const NetworkStatsCard(),
                const SizedBox(height: 16),
                
                // Connected Peers Section
                _buildPeersSection(networkProvider),
              ],
            ),
          );
        },
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: _showAddPeerDialog,
        tooltip: 'Connect to Peer',
        child: const Icon(Icons.add_link),
      ),
    );
  }

  Widget _buildMyAddressCard(NetworkProvider networkProvider) {
    final address = networkProvider.getShareableAddress();
    final peerId = networkProvider.localPeerId;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  Icons.account_circle,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  'My Connection Info',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            
            // Peer ID
            if (peerId != null) ...[
              Text(
                'Peer ID',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.outline,
                ),
              ),
              const SizedBox(height: 4),
              Row(
                children: [
                  Expanded(
                    child: Text(
                      peerId.length > 16 ? '${peerId.substring(0, 16)}...' : peerId,
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        fontFamily: 'monospace',
                      ),
                    ),
                  ),
                  IconButton(
                    onPressed: () => _copyMyAddress(peerId),
                    icon: const Icon(Icons.copy, size: 18),
                    tooltip: 'Copy Peer ID',
                  ),
                ],
              ),
              const SizedBox(height: 12),
            ],
            
            // Listen Address
            Text(
              'Listen Address',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.outline,
              ),
            ),
            const SizedBox(height: 4),
            Row(
              children: [
                Expanded(
                  child: Text(
                    address,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
                IconButton(
                  onPressed: () => _copyMyAddress(address),
                  icon: const Icon(Icons.copy, size: 18),
                  tooltip: 'Copy Address',
                ),
              ],
            ),
            
            const SizedBox(height: 12),
            Text(
              'Share this address with others to connect',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.outline,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildPeersSection(NetworkProvider networkProvider) {
    final peers = networkProvider.peers;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Icon(
              Icons.hub,
              color: Theme.of(context).colorScheme.primary,
            ),
            const SizedBox(width: 8),
            Text(
              'Connected Peers',
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const Spacer(),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              decoration: BoxDecoration(
                color: AppTheme.getStatusColor(networkProvider.connectionStatus)
                    .withOpacity(0.1),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(
                  color: AppTheme.getStatusColor(networkProvider.connectionStatus),
                  width: 1,
                ),
              ),
              child: Text(
                '${peers.length}',
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: AppTheme.getStatusColor(networkProvider.connectionStatus),
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        
        if (peers.isEmpty)
          Card(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                children: [
                  Icon(
                    Icons.group_off,
                    size: 48,
                    color: Theme.of(context).colorScheme.outline,
                  ),
                  const SizedBox(height: 12),
                  Text(
                    'No peers connected',
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      color: Theme.of(context).colorScheme.outline,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'Add a peer connection to start chatting',
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: Theme.of(context).colorScheme.outline,
                    ),
                    textAlign: TextAlign.center,
                  ),
                ],
              ),
            ),
          )
        else
          ...peers.map((peer) => Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: PeerCard(
              peer: peer,
              onDisconnect: () => networkProvider.disconnectFromPeer(peer.id),
            ),
          )),
      ],
    );
  }
}
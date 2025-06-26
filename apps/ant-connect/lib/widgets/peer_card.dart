import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:ant_connect/models/peer_info.dart';
import 'package:ant_connect/theme/app_theme.dart';

class PeerCard extends StatelessWidget {
  final PeerInfo peer;
  final VoidCallback? onDisconnect;

  const PeerCard({
    super.key,
    required this.peer,
    this.onDisconnect,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header row with name and status
            Row(
              children: [
                _buildStatusIndicator(),
                const SizedBox(width: 12),
                
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        peer.displayName,
                        style: Theme.of(context).textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      Text(
                        'ID: ${peer.shortId}',
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: Theme.of(context).colorScheme.outline,
                          fontFamily: 'monospace',
                        ),
                      ),
                    ],
                  ),
                ),
                
                // Connection status
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                  decoration: BoxDecoration(
                    color: _getStatusColor().withOpacity(0.1),
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(
                      color: _getStatusColor(),
                      width: 1,
                    ),
                  ),
                  child: Text(
                    peer.isOnline ? 'Online' : 'Offline',
                    style: Theme.of(context).textTheme.labelSmall?.copyWith(
                      color: _getStatusColor(),
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                
                // More options menu
                PopupMenuButton<String>(
                  icon: const Icon(Icons.more_vert),
                  onSelected: (value) => _handleMenuAction(context, value),
                  itemBuilder: (context) => [
                    const PopupMenuItem(
                      value: 'copy_id',
                      child: Row(
                        children: [
                          Icon(Icons.copy, size: 18),
                          SizedBox(width: 8),
                          Text('Copy ID'),
                        ],
                      ),
                    ),
                    const PopupMenuItem(
                      value: 'copy_address',
                      child: Row(
                        children: [
                          Icon(Icons.link, size: 18),
                          SizedBox(width: 8),
                          Text('Copy Address'),
                        ],
                      ),
                    ),
                    if (onDisconnect != null)
                      const PopupMenuItem(
                        value: 'disconnect',
                        child: Row(
                          children: [
                            Icon(Icons.link_off, size: 18, color: Colors.red),
                            SizedBox(width: 8),
                            Text('Disconnect', style: TextStyle(color: Colors.red)),
                          ],
                        ),
                      ),
                  ],
                ),
              ],
            ),
            
            const SizedBox(height: 12),
            
            // Connection info
            Row(
              children: [
                Expanded(
                  child: _buildInfoChip(
                    context,
                    icon: Icons.schedule,
                    label: 'Connected',
                    value: peer.connectionDuration,
                  ),
                ),
                
                if (peer.latency != null) ...[
                  const SizedBox(width: 8),
                  Expanded(
                    child: _buildInfoChip(
                      context,
                      icon: Icons.speed,
                      label: 'Latency',
                      value: '${peer.latency!.toInt()}ms',
                    ),
                  ),
                ],
                
                const SizedBox(width: 8),
                Expanded(
                  child: _buildInfoChip(
                    context,
                    icon: Icons.message,
                    label: 'Messages',
                    value: '${peer.messageCount}',
                  ),
                ),
              ],
            ),
            
            const SizedBox(height: 8),
            
            // Address (truncated)
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.surfaceVariant.withOpacity(0.5),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Text(
                _truncateAddress(peer.address),
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  fontFamily: 'monospace',
                  color: Theme.of(context).colorScheme.outline,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildStatusIndicator() {
    return Container(
      width: 48,
      height: 48,
      decoration: BoxDecoration(
        color: _getStatusColor().withOpacity(0.1),
        shape: BoxShape.circle,
        border: Border.all(
          color: _getStatusColor(),
          width: 2,
        ),
      ),
      child: Icon(
        peer.isOnline ? Icons.person : Icons.person_off,
        color: _getStatusColor(),
        size: 24,
      ),
    );
  }

  Widget _buildInfoChip(
    BuildContext context, {
    required IconData icon,
    required String label,
    required String value,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: Theme.of(context).colorScheme.outline.withOpacity(0.2),
        ),
      ),
      child: Column(
        children: [
          Icon(
            icon,
            size: 16,
            color: Theme.of(context).colorScheme.outline,
          ),
          const SizedBox(height: 2),
          Text(
            value,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
          Text(
            label,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: Theme.of(context).colorScheme.outline,
              fontSize: 10,
            ),
          ),
        ],
      ),
    );
  }

  Color _getStatusColor() {
    return peer.isOnline 
        ? AppTheme.connectedColor 
        : AppTheme.disconnectedColor;
  }

  String _truncateAddress(String address) {
    if (address.length <= 50) return address;
    return '${address.substring(0, 25)}...${address.substring(address.length - 15)}';
  }

  void _handleMenuAction(BuildContext context, String action) {
    switch (action) {
      case 'copy_id':
        Clipboard.setData(ClipboardData(text: peer.id));
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Peer ID copied to clipboard')),
        );
        break;
        
      case 'copy_address':
        Clipboard.setData(ClipboardData(text: peer.address));
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Address copied to clipboard')),
        );
        break;
        
      case 'disconnect':
        _showDisconnectDialog(context);
        break;
    }
  }

  void _showDisconnectDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Disconnect Peer'),
        content: Text('Are you sure you want to disconnect from ${peer.displayName}?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.of(context).pop();
              onDisconnect?.call();
            },
            style: ElevatedButton.styleFrom(
              backgroundColor: Colors.red,
              foregroundColor: Colors.white,
            ),
            child: const Text('Disconnect'),
          ),
        ],
      ),
    );
  }
}
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:ant_connect/providers/network_provider.dart';
import 'package:ant_connect/theme/app_theme.dart';

class ConnectionStatusBar extends StatelessWidget {
  const ConnectionStatusBar({super.key});

  @override
  Widget build(BuildContext context) {
    return Consumer<NetworkProvider>(
      builder: (context, networkProvider, child) {
        final status = networkProvider.connectionStatus;
        final message = networkProvider.statusMessage;
        final peerCount = networkProvider.peerCount;
        
        // Don't show status bar if connected and no special message
        if (status == ConnectionStatus.connected && peerCount > 0) {
          return const SizedBox.shrink();
        }
        
        return Container(
          width: double.infinity,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          decoration: BoxDecoration(
            color: _getStatusColor(status).withOpacity(0.1),
            border: Border(
              bottom: BorderSide(
                color: _getStatusColor(status).withOpacity(0.3),
                width: 1,
              ),
            ),
          ),
          child: Row(
            children: [
              _buildStatusIcon(status),
              const SizedBox(width: 12),
              
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      _getStatusTitle(status, peerCount),
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                        color: _getStatusColor(status),
                      ),
                    ),
                    
                    if (message.isNotEmpty) ...[
                      const SizedBox(height: 2),
                      Text(
                        message,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: _getStatusColor(status).withOpacity(0.8),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
              
              if (status == ConnectionStatus.disconnected) ...[
                TextButton.icon(
                  onPressed: () {
                    Navigator.pushNamed(context, '/connections');
                  },
                  icon: const Icon(Icons.add_link, size: 16),
                  label: const Text('Connect'),
                  style: TextButton.styleFrom(
                    foregroundColor: _getStatusColor(status),
                    textStyle: const TextStyle(fontSize: 12),
                  ),
                ),
              ],
              
              if (status == ConnectionStatus.connecting) ...[
                SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    valueColor: AlwaysStoppedAnimation<Color>(
                      _getStatusColor(status),
                    ),
                  ),
                ),
              ],
            ],
          ),
        );
      },
    );
  }

  Widget _buildStatusIcon(ConnectionStatus status) {
    Color color = _getStatusColor(status);
    IconData icon;
    
    switch (status) {
      case ConnectionStatus.connected:
        icon = Icons.wifi;
        break;
      case ConnectionStatus.connecting:
        icon = Icons.wifi_find;
        break;
      case ConnectionStatus.disconnected:
        icon = Icons.wifi_off;
        break;
    }
    
    return Container(
      padding: const EdgeInsets.all(6),
      decoration: BoxDecoration(
        color: color.withOpacity(0.2),
        shape: BoxShape.circle,
      ),
      child: Icon(
        icon,
        size: 16,
        color: color,
      ),
    );
  }

  Color _getStatusColor(ConnectionStatus status) {
    return AppTheme.getStatusColor(status);
  }

  String _getStatusTitle(ConnectionStatus status, int peerCount) {
    switch (status) {
      case ConnectionStatus.connected:
        return 'Connected to $peerCount peer${peerCount == 1 ? '' : 's'}';
      case ConnectionStatus.connecting:
        return 'Connecting...';
      case ConnectionStatus.disconnected:
        return 'Not Connected';
    }
  }
}
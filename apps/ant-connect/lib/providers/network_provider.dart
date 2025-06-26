import 'package:flutter/foundation.dart';
import 'package:ant_connect/models/peer_info.dart';
import 'package:ant_connect/services/p2p_service.dart';
import 'package:ant_connect/theme/app_theme.dart';
import 'dart:async';

class NetworkProvider with ChangeNotifier {
  final P2PService _p2pService;
  late StreamSubscription _peerConnectedSubscription;
  late StreamSubscription _peerDisconnectedSubscription;
  late StreamSubscription _networkStatusSubscription;
  
  final List<PeerInfo> _peers = [];
  ConnectionStatus _connectionStatus = ConnectionStatus.disconnected;
  String _statusMessage = 'Disconnected';
  Map<String, dynamic> _networkStats = {};

  NetworkProvider(this._p2pService) {
    _initialize();
  }

  // Getters
  List<PeerInfo> get peers => List.unmodifiable(_peers);
  ConnectionStatus get connectionStatus => _connectionStatus;
  String get statusMessage => _statusMessage;
  Map<String, dynamic> get networkStats => Map.unmodifiable(_networkStats);
  int get peerCount => _peers.length;
  String? get localPeerId => _p2pService.localPeerId;
  String? get listenAddress => _p2pService.listenAddress;
  
  // Get online peers only
  List<PeerInfo> get onlinePeers => _peers.where((p) => p.isOnline).toList();
  
  // Get peer by ID
  PeerInfo? getPeer(String peerId) {
    try {
      return _peers.firstWhere((p) => p.id == peerId);
    } catch (e) {
      return null;
    }
  }

  void _initialize() {
    // Listen to peer events
    _peerConnectedSubscription = _p2pService.peerConnectedStream.listen((peer) {
      _addPeer(peer);
      _updateConnectionStatus();
    });
    
    _peerDisconnectedSubscription = _p2pService.peerDisconnectedStream.listen((peerId) {
      _removePeer(peerId);
      _updateConnectionStatus();
    });
    
    _networkStatusSubscription = _p2pService.networkStatusStream.listen((status) {
      _updateNetworkStatus(status);
    });
    
    // Initial status update
    _updateConnectionStatus();
    _refreshNetworkStats();
    
    if (kDebugMode) {
      print('NetworkProvider initialized');
    }
  }

  void _addPeer(PeerInfo peer) {
    // Remove existing peer with same ID if any
    _peers.removeWhere((p) => p.id == peer.id);
    // Add new peer
    _peers.add(peer);
    notifyListeners();
    
    if (kDebugMode) {
      print('Peer connected: ${peer.displayName} (${peer.shortId})');
    }
  }

  void _removePeer(String peerId) {
    final removed = _peers.removeWhere((p) => p.id == peerId);
    if (removed > 0) {
      notifyListeners();
      
      if (kDebugMode) {
        print('Peer disconnected: $peerId');
      }
    }
  }

  void _updateConnectionStatus() {
    final previousStatus = _connectionStatus;
    
    if (_peers.isNotEmpty) {
      _connectionStatus = ConnectionStatus.connected;
      _statusMessage = '${_peers.length} peer${_peers.length == 1 ? '' : 's'} connected';
    } else {
      _connectionStatus = ConnectionStatus.disconnected;
      _statusMessage = 'No peers connected';
    }
    
    // Only notify if status actually changed
    if (previousStatus != _connectionStatus) {
      notifyListeners();
    }
  }

  void _updateNetworkStatus(String status) {
    _statusMessage = status;
    notifyListeners();
  }

  /// Connect to a peer by address
  Future<bool> connectToPeer(String address) async {
    if (address.trim().isEmpty) return false;
    
    try {
      _connectionStatus = ConnectionStatus.connecting;
      _statusMessage = 'Connecting to peer...';
      notifyListeners();
      
      final success = await _p2pService.connectToPeer(address.trim());
      
      if (!success) {
        _updateConnectionStatus(); // Reset status
      }
      
      return success;
    } catch (e) {
      _updateConnectionStatus(); // Reset status
      if (kDebugMode) {
        print('Error connecting to peer: $e');
      }
      return false;
    }
  }

  /// Disconnect from a specific peer
  Future<bool> disconnectFromPeer(String peerId) async {
    try {
      final success = await _p2pService.disconnectFromPeer(peerId);
      
      if (success) {
        _removePeer(peerId);
        _updateConnectionStatus();
      }
      
      return success;
    } catch (e) {
      if (kDebugMode) {
        print('Error disconnecting from peer: $e');
      }
      return false;
    }
  }

  /// Refresh network statistics
  Future<void> refreshNetworkStats() async {
    await _refreshNetworkStats();
  }

  Future<void> _refreshNetworkStats() async {
    try {
      final stats = await _p2pService.getNetworkStatus();
      if (stats != null) {
        _networkStats = stats;
        notifyListeners();
      }
    } catch (e) {
      if (kDebugMode) {
        print('Error refreshing network stats: $e');
      }
    }
  }

  /// Get formatted address for sharing
  String getShareableAddress() {
    final address = listenAddress;
    if (address != null && localPeerId != null) {
      return address;
    }
    return 'Address not available';
  }

  /// Update peer information (e.g., after receiving messages)
  void updatePeer(String peerId, {String? name, int? messageCount}) {
    final index = _peers.indexWhere((p) => p.id == peerId);
    if (index != -1) {
      _peers[index] = _peers[index].copyWith(
        name: name,
        messageCount: messageCount,
      );
      notifyListeners();
    }
  }

  /// Get connection quality info
  String getConnectionQuality(String peerId) {
    final peer = getPeer(peerId);
    if (peer?.latency != null) {
      final latency = peer!.latency!;
      if (latency < 50) return 'Excellent';
      if (latency < 100) return 'Good';
      if (latency < 200) return 'Fair';
      return 'Poor';
    }
    return 'Unknown';
  }

  @override
  void dispose() {
    _peerConnectedSubscription.cancel();
    _peerDisconnectedSubscription.cancel();
    _networkStatusSubscription.cancel();
    super.dispose();
  }
}
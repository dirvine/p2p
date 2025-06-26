import 'dart:async';
import 'dart:convert';
import 'package:flutter/services.dart';
import 'package:ant_connect/models/chat_message.dart';
import 'package:ant_connect/models/peer_info.dart';

/// Service for interfacing with the Rust P2P backend
class P2PService {
  static const MethodChannel _channel = MethodChannel('ant_connect/p2p');
  
  // Event streams
  final StreamController<ChatMessage> _messageController = StreamController.broadcast();
  final StreamController<PeerInfo> _peerConnectedController = StreamController.broadcast();
  final StreamController<String> _peerDisconnectedController = StreamController.broadcast();
  final StreamController<String> _networkStatusController = StreamController.broadcast();
  
  // Getters for streams
  Stream<ChatMessage> get messageStream => _messageController.stream;
  Stream<PeerInfo> get peerConnectedStream => _peerConnectedController.stream;
  Stream<String> get peerDisconnectedStream => _peerDisconnectedController.stream;
  Stream<String> get networkStatusStream => _networkStatusController.stream;
  
  String? _localPeerId;
  String? _listenAddress;
  final List<PeerInfo> _connectedPeers = [];
  
  // Getters
  String? get localPeerId => _localPeerId;
  String? get listenAddress => _listenAddress;
  List<PeerInfo> get connectedPeers => List.unmodifiable(_connectedPeers);

  /// Initialize the P2P service
  Future<void> initialize() async {
    try {
      // Set up method call handler for events from Rust
      _channel.setMethodCallHandler(_handleMethodCall);
      
      // Initialize the Rust P2P node
      final result = await _channel.invokeMethod('initialize', {
        'listen_address': '/ip6/::/udp/0/quic',
        'enable_ipv6': true,
      });
      
      _localPeerId = result['peer_id'];
      _listenAddress = result['listen_address'];
      
      print('P2P Service initialized - Peer ID: $_localPeerId');
      print('Listening on: $_listenAddress');
      
    } catch (e) {
      print('Failed to initialize P2P service: $e');
      rethrow;
    }
  }

  /// Handle method calls from Rust backend
  Future<dynamic> _handleMethodCall(MethodCall call) async {
    switch (call.method) {
      case 'on_message_received':
        _handleMessageReceived(call.arguments);
        break;
      case 'on_peer_connected':
        _handlePeerConnected(call.arguments);
        break;
      case 'on_peer_disconnected':
        _handlePeerDisconnected(call.arguments);
        break;
      case 'on_network_status':
        _handleNetworkStatus(call.arguments);
        break;
      default:
        print('Unknown method call: ${call.method}');
    }
  }

  void _handleMessageReceived(Map<String, dynamic> args) {
    final message = ChatMessage(
      id: args['id'] ?? DateTime.now().millisecondsSinceEpoch.toString(),
      senderId: args['sender_id'],
      senderName: args['sender_name'] ?? args['sender_id'],
      content: args['content'],
      timestamp: DateTime.fromMillisecondsSinceEpoch(args['timestamp'] ?? DateTime.now().millisecondsSinceEpoch),
      isFromMe: args['sender_id'] == _localPeerId,
    );
    _messageController.add(message);
  }

  void _handlePeerConnected(Map<String, dynamic> args) {
    final peer = PeerInfo(
      id: args['peer_id'],
      name: args['name'] ?? args['peer_id'],
      address: args['address'],
      connectionTime: DateTime.now(),
      isOnline: true,
    );
    
    _connectedPeers.add(peer);
    _peerConnectedController.add(peer);
  }

  void _handlePeerDisconnected(Map<String, dynamic> args) {
    final peerId = args['peer_id'];
    _connectedPeers.removeWhere((peer) => peer.id == peerId);
    _peerDisconnectedController.add(peerId);
  }

  void _handleNetworkStatus(Map<String, dynamic> args) {
    final status = args['status'];
    _networkStatusController.add(status);
  }

  /// Send a chat message to all connected peers
  Future<bool> sendMessage(String content) async {
    try {
      await _channel.invokeMethod('send_message', {
        'content': content,
        'sender_id': _localPeerId,
        'timestamp': DateTime.now().millisecondsSinceEpoch,
      });
      
      // Add to our own message stream as a sent message
      final message = ChatMessage(
        id: DateTime.now().millisecondsSinceEpoch.toString(),
        senderId: _localPeerId!,
        senderName: 'You',
        content: content,
        timestamp: DateTime.now(),
        isFromMe: true,
      );
      _messageController.add(message);
      
      return true;
    } catch (e) {
      print('Failed to send message: $e');
      return false;
    }
  }

  /// Connect to a peer by address
  Future<bool> connectToPeer(String address) async {
    try {
      await _channel.invokeMethod('connect_to_peer', {
        'address': address,
      });
      return true;
    } catch (e) {
      print('Failed to connect to peer: $e');
      return false;
    }
  }

  /// Disconnect from a peer
  Future<bool> disconnectFromPeer(String peerId) async {
    try {
      await _channel.invokeMethod('disconnect_from_peer', {
        'peer_id': peerId,
      });
      return true;
    } catch (e) {
      print('Failed to disconnect from peer: $e');
      return false;
    }
  }

  /// Get current network status
  Future<Map<String, dynamic>?> getNetworkStatus() async {
    try {
      final result = await _channel.invokeMethod('get_network_status');
      return Map<String, dynamic>.from(result);
    } catch (e) {
      print('Failed to get network status: $e');
      return null;
    }
  }

  /// Subscribe to chat topic
  Future<bool> subscribeToChat() async {
    try {
      await _channel.invokeMethod('subscribe_to_chat');
      return true;
    } catch (e) {
      print('Failed to subscribe to chat: $e');
      return false;
    }
  }

  /// Dispose resources
  void dispose() {
    _messageController.close();
    _peerConnectedController.close();
    _peerDisconnectedController.close();
    _networkStatusController.close();
  }
}
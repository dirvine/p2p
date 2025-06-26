import 'package:flutter/foundation.dart';
import 'package:ant_connect/models/chat_message.dart';
import 'package:ant_connect/services/p2p_service.dart';
import 'dart:async';

class ChatProvider with ChangeNotifier {
  final P2PService _p2pService;
  final List<ChatMessage> _messages = [];
  late StreamSubscription _messageSubscription;
  
  bool _isInitialized = false;
  String _currentInput = '';

  ChatProvider(this._p2pService) {
    _initialize();
  }

  // Getters
  List<ChatMessage> get messages => List.unmodifiable(_messages);
  bool get isInitialized => _isInitialized;
  String get currentInput => _currentInput;
  int get messageCount => _messages.length;
  
  // Get messages from a specific peer
  List<ChatMessage> getMessagesFromPeer(String peerId) {
    return _messages.where((msg) => msg.senderId == peerId).toList();
  }
  
  // Get recent messages (last 100)
  List<ChatMessage> get recentMessages {
    if (_messages.length <= 100) return _messages;
    return _messages.sublist(_messages.length - 100);
  }

  void _initialize() async {
    try {
      // Subscribe to chat in the P2P service
      await _p2pService.subscribeToChat();
      
      // Listen to incoming messages
      _messageSubscription = _p2pService.messageStream.listen((message) {
        _addMessage(message);
      });
      
      _isInitialized = true;
      notifyListeners();
      
      if (kDebugMode) {
        print('ChatProvider initialized');
      }
    } catch (e) {
      if (kDebugMode) {
        print('Failed to initialize ChatProvider: $e');
      }
    }
  }

  void _addMessage(ChatMessage message) {
    // Avoid duplicates
    if (!_messages.any((m) => m.id == message.id)) {
      _messages.add(message);
      // Keep only last 1000 messages to avoid memory issues
      if (_messages.length > 1000) {
        _messages.removeRange(0, _messages.length - 1000);
      }
      notifyListeners();
    }
  }

  /// Send a message to all connected peers
  Future<bool> sendMessage(String content) async {
    if (content.trim().isEmpty) return false;
    
    try {
      final success = await _p2pService.sendMessage(content.trim());
      
      if (!success) {
        // Add a failed message to show the user what happened
        final failedMessage = ChatMessage(
          id: DateTime.now().millisecondsSinceEpoch.toString(),
          senderId: _p2pService.localPeerId ?? 'unknown',
          senderName: 'You',
          content: content.trim(),
          timestamp: DateTime.now(),
          isFromMe: true,
          status: MessageStatus.failed,
        );
        _addMessage(failedMessage);
      }
      
      return success;
    } catch (e) {
      if (kDebugMode) {
        print('Error sending message: $e');
      }
      return false;
    }
  }

  /// Update the current input text
  void updateInput(String input) {
    _currentInput = input;
    // Don't notify listeners for every keystroke to avoid excessive rebuilds
  }

  /// Clear current input
  void clearInput() {
    _currentInput = '';
    notifyListeners();
  }

  /// Clear all messages
  void clearMessages() {
    _messages.clear();
    notifyListeners();
  }

  /// Mark message as failed
  void markMessageAsFailed(String messageId) {
    final index = _messages.indexWhere((m) => m.id == messageId);
    if (index != -1) {
      _messages[index] = _messages[index].copyWith(status: MessageStatus.failed);
      notifyListeners();
    }
  }

  /// Retry sending a failed message
  Future<bool> retryMessage(String messageId) async {
    final message = _messages.firstWhere((m) => m.id == messageId);
    if (message.status == MessageStatus.failed) {
      // Update status to sending
      final index = _messages.indexWhere((m) => m.id == messageId);
      _messages[index] = message.copyWith(status: MessageStatus.sending);
      notifyListeners();
      
      // Try to send again
      final success = await _p2pService.sendMessage(message.content);
      
      // Update status based on result
      _messages[index] = message.copyWith(
        status: success ? MessageStatus.delivered : MessageStatus.failed,
      );
      notifyListeners();
      
      return success;
    }
    return false;
  }

  @override
  void dispose() {
    _messageSubscription.cancel();
    super.dispose();
  }
}
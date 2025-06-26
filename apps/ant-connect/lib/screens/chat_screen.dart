import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:ant_connect/providers/chat_provider.dart';
import 'package:ant_connect/providers/network_provider.dart';
import 'package:ant_connect/widgets/message_bubble.dart';
import 'package:ant_connect/widgets/connection_status_bar.dart';
import 'package:ant_connect/theme/app_theme.dart';

class ChatScreen extends StatefulWidget {
  const ChatScreen({super.key});

  @override
  State<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends State<ChatScreen> {
  final TextEditingController _messageController = TextEditingController();
  final ScrollController _scrollController = ScrollController();
  final FocusNode _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    // Auto-scroll to bottom when new messages arrive
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scrollToBottom();
    });
  }

  @override
  void dispose() {
    _messageController.dispose();
    _scrollController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _scrollToBottom() {
    if (_scrollController.hasClients) {
      _scrollController.animateTo(
        _scrollController.position.maxScrollExtent,
        duration: const Duration(milliseconds: 300),
        curve: Curves.easeOut,
      );
    }
  }

  Future<void> _sendMessage() async {
    final content = _messageController.text.trim();
    if (content.isEmpty) return;

    final chatProvider = Provider.of<ChatProvider>(context, listen: false);
    final success = await chatProvider.sendMessage(content);

    if (success) {
      _messageController.clear();
      _scrollToBottom();
    } else {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('Failed to send message'),
            backgroundColor: Colors.red,
          ),
        );
      }
    }
  }

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
          // Connection status bar
          const ConnectionStatusBar(),
          
          // Messages list
          Expanded(
            child: Consumer<ChatProvider>(
              builder: (context, chatProvider, child) {
                final messages = chatProvider.recentMessages;
                
                if (!chatProvider.isInitialized) {
                  return const Center(
                    child: CircularProgressIndicator(),
                  );
                }
                
                if (messages.isEmpty) {
                  return _buildEmptyState();
                }
                
                return ListView.builder(
                  controller: _scrollController,
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  itemCount: messages.length,
                  itemBuilder: (context, index) {
                    final message = messages[index];
                    return MessageBubble(
                      message: message,
                      onRetry: message.status == MessageStatus.failed
                          ? () => chatProvider.retryMessage(message.id)
                          : null,
                    );
                  },
                );
              },
            ),
          ),
          
          // Message input
          _buildMessageInput(),
        ],
      ),
    );
  }

  Widget _buildEmptyState() {
    return Consumer<NetworkProvider>(
      builder: (context, networkProvider, child) {
        return Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.chat_bubble_outline,
                size: 64,
                color: Theme.of(context).colorScheme.outline,
              ),
              const SizedBox(height: 16),
              Text(
                'No messages yet',
                style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                  color: Theme.of(context).colorScheme.outline,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                networkProvider.peerCount > 0
                    ? 'Start a conversation with your peers'
                    : 'Connect to peers to start chatting',
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Theme.of(context).colorScheme.outline,
                ),
                textAlign: TextAlign.center,
              ),
              if (networkProvider.peerCount == 0) ...[
                const SizedBox(height: 16),
                ElevatedButton.icon(
                  onPressed: () {
                    Navigator.pushNamed(context, '/connections');
                  },
                  icon: const Icon(Icons.add_link),
                  label: const Text('Connect to Peers'),
                ),
              ],
            ],
          ),
        );
      },
    );
  }

  Widget _buildMessageInput() {
    return Consumer<NetworkProvider>(
      builder: (context, networkProvider, child) {
        final isDisabled = networkProvider.peerCount == 0;
        
        return Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surface,
            border: Border(
              top: BorderSide(
                color: Theme.of(context).colorScheme.outline.withOpacity(0.2),
              ),
            ),
          ),
          child: SafeArea(
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _messageController,
                    focusNode: _focusNode,
                    enabled: !isDisabled,
                    decoration: InputDecoration(
                      hintText: isDisabled 
                          ? 'Connect to peers to send messages'
                          : 'Type a message...',
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(24),
                      ),
                      contentPadding: const EdgeInsets.symmetric(
                        horizontal: 16,
                        vertical: 12,
                      ),
                    ),
                    textInputAction: TextInputAction.send,
                    onSubmitted: (_) => _sendMessage(),
                    maxLines: null,
                    minLines: 1,
                  ),
                ),
                const SizedBox(width: 8),
                FloatingActionButton.small(
                  onPressed: isDisabled ? null : _sendMessage,
                  child: const Icon(Icons.send),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
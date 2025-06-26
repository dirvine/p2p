import 'package:flutter/material.dart';
import 'package:ant_connect/models/chat_message.dart';

class MessageBubble extends StatelessWidget {
  final ChatMessage message;
  final VoidCallback? onRetry;

  const MessageBubble({
    super.key,
    required this.message,
    this.onRetry,
  });

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        mainAxisAlignment: message.isFromMe 
            ? MainAxisAlignment.end 
            : MainAxisAlignment.start,
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          if (!message.isFromMe) ...[
            _buildAvatar(context),
            const SizedBox(width: 8),
          ],
          
          Flexible(
            child: Container(
              constraints: BoxConstraints(
                maxWidth: MediaQuery.of(context).size.width * 0.75,
              ),
              child: Column(
                crossAxisAlignment: message.isFromMe 
                    ? CrossAxisAlignment.end 
                    : CrossAxisAlignment.start,
                children: [
                  if (!message.isFromMe)
                    Padding(
                      padding: const EdgeInsets.only(left: 12, bottom: 4),
                      child: Text(
                        message.senderName,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: Theme.of(context).colorScheme.outline,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ),
                  
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 12,
                    ),
                    decoration: BoxDecoration(
                      color: _getBubbleColor(context),
                      borderRadius: _getBorderRadius(),
                      border: message.status == MessageStatus.failed
                          ? Border.all(color: Colors.red.withOpacity(0.5))
                          : null,
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          message.content,
                          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                            color: message.isFromMe
                                ? Colors.white
                                : Theme.of(context).colorScheme.onSurface,
                          ),
                        ),
                        
                        const SizedBox(height: 4),
                        
                        Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                              _formatTime(message.timestamp),
                              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                color: message.isFromMe
                                    ? Colors.white.withOpacity(0.7)
                                    : Theme.of(context).colorScheme.outline,
                                fontSize: 11,
                              ),
                            ),
                            
                            if (message.isFromMe) ...[
                              const SizedBox(width: 4),
                              _buildStatusIcon(context),
                            ],
                          ],
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
          
          if (message.isFromMe) ...[
            const SizedBox(width: 8),
            _buildAvatar(context),
          ],
        ],
      ),
    );
  }

  Widget _buildAvatar(context) {
    return CircleAvatar(
      radius: 16,
      backgroundColor: message.isFromMe
          ? Theme.of(context).colorScheme.primary
          : Theme.of(context).colorScheme.secondary,
      child: Text(
        message.isFromMe 
            ? 'Me'[0]
            : message.senderName[0].toUpperCase(),
        style: const TextStyle(
          color: Colors.white,
          fontSize: 12,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }

  Color _getBubbleColor(BuildContext context) {
    if (message.status == MessageStatus.failed) {
      return Colors.red.withOpacity(0.1);
    }
    
    return message.isFromMe
        ? Theme.of(context).colorScheme.primary
        : Theme.of(context).colorScheme.surfaceVariant;
  }

  BorderRadius _getBorderRadius() {
    return BorderRadius.only(
      topLeft: const Radius.circular(18),
      topRight: const Radius.circular(18),
      bottomLeft: message.isFromMe 
          ? const Radius.circular(18) 
          : const Radius.circular(4),
      bottomRight: message.isFromMe 
          ? const Radius.circular(4) 
          : const Radius.circular(18),
    );
  }

  Widget _buildStatusIcon(BuildContext context) {
    switch (message.status) {
      case MessageStatus.sending:
        return SizedBox(
          width: 12,
          height: 12,
          child: CircularProgressIndicator(
            strokeWidth: 1.5,
            valueColor: AlwaysStoppedAnimation<Color>(
              Colors.white.withOpacity(0.7),
            ),
          ),
        );
      
      case MessageStatus.delivered:
        return Icon(
          Icons.check,
          size: 14,
          color: Colors.white.withOpacity(0.7),
        );
      
      case MessageStatus.failed:
        return GestureDetector(
          onTap: onRetry,
          child: Icon(
            Icons.error_outline,
            size: 14,
            color: Colors.red[300],
          ),
        );
    }
  }

  String _formatTime(DateTime timestamp) {
    final now = DateTime.now();
    final diff = now.difference(timestamp);
    
    if (diff.inDays > 0) {
      return '${diff.inDays}d ago';
    } else if (diff.inHours > 0) {
      return '${diff.inHours}h ago';
    } else if (diff.inMinutes > 0) {
      return '${diff.inMinutes}m ago';
    } else {
      return 'now';
    }
  }
}
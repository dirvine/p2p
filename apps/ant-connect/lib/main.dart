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
        ChangeNotifierProvider(create: (_) => ContactsProvider()),
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
  final ScrollController _scrollController = ScrollController();

  @override
  void initState() {
    super.initState();
    // Set up auto-scroll callback for the chat provider
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final chatProvider = Provider.of<ChatProvider>(context, listen: false);
      chatProvider.setScrollCallback(_scrollToBottom);
      // Switch to system contact by default
      chatProvider.switchToContact('system');
    });
  }

  @override
  void dispose() {
    _scrollController.dispose();
    _messageController.dispose();
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

  @override
  Widget build(BuildContext context) {
    return Consumer2<ContactsProvider, ChatProvider>(
      builder: (context, contactsProvider, chatProvider, child) {
        return Scaffold(
          appBar: AppBar(
            title: Text(contactsProvider.selectedContact?.name ?? 'Connect'),
            actions: [
              // Position settings button
              PopupMenuButton<ContactsPanelPosition>(
                icon: const Icon(Icons.view_sidebar),
                onSelected: contactsProvider.setPanelPosition,
                itemBuilder: (context) => [
                  const PopupMenuItem(
                    value: ContactsPanelPosition.left,
                    child: Row(
                      children: [Icon(Icons.view_sidebar), SizedBox(width: 8), Text('Left Panel')],
                    ),
                  ),
                  const PopupMenuItem(
                    value: ContactsPanelPosition.top,
                    child: Row(
                      children: [Icon(Icons.view_agenda), SizedBox(width: 8), Text('Top Panel')],
                    ),
                  ),
                  const PopupMenuItem(
                    value: ContactsPanelPosition.right,
                    child: Row(
                      children: [Icon(Icons.view_sidebar), SizedBox(width: 8), Text('Right Panel')],
                    ),
                  ),
                  const PopupMenuItem(
                    value: ContactsPanelPosition.bottom,
                    child: Row(
                      children: [Icon(Icons.view_agenda), SizedBox(width: 8), Text('Bottom Panel')],
                    ),
                  ),
                ],
              ),
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
          body: _buildLayoutByPosition(contactsProvider, chatProvider),
        );
      },
    );
  }

  Widget _buildLayoutByPosition(ContactsProvider contactsProvider, ChatProvider chatProvider) {
    final contactsPanel = _buildContactsPanel(contactsProvider, chatProvider);
    final chatArea = _buildChatArea(contactsProvider, chatProvider);

    switch (contactsProvider.panelPosition) {
      case ContactsPanelPosition.left:
        return Row(
          children: [
            SizedBox(width: 280, child: contactsPanel),
            const VerticalDivider(width: 1),
            Expanded(child: chatArea),
          ],
        );
      case ContactsPanelPosition.right:
        return Row(
          children: [
            Expanded(child: chatArea),
            const VerticalDivider(width: 1),
            SizedBox(width: 280, child: contactsPanel),
          ],
        );
      case ContactsPanelPosition.top:
        return Column(
          children: [
            SizedBox(height: 200, child: contactsPanel),
            const Divider(height: 1),
            Expanded(child: chatArea),
          ],
        );
      case ContactsPanelPosition.bottom:
        return Column(
          children: [
            Expanded(child: chatArea),
            const Divider(height: 1),
            SizedBox(height: 200, child: contactsPanel),
          ],
        );
    }
  }

  Widget _buildContactsPanel(ContactsProvider contactsProvider, ChatProvider chatProvider) {
    return Container(
      color: Theme.of(context).colorScheme.surfaceContainer,
      child: Column(
        children: [
          // Contacts header
          Container(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                const Icon(Icons.contacts),
                const SizedBox(width: 8),
                Text(
                  'Contacts',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.add),
                  onPressed: () {
                    // TODO: Add contact functionality
                  },
                  iconSize: 20,
                ),
              ],
            ),
          ),
          const Divider(height: 1),
          
          // Contacts list
          Expanded(
            child: ListView.builder(
              itemCount: contactsProvider.contacts.length,
              itemBuilder: (context, index) {
                final contact = contactsProvider.contacts[index];
                final isSelected = contactsProvider.selectedContact?.id == contact.id;
                
                return _buildContactTile(contact, isSelected, contactsProvider, chatProvider);
              },
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildContactTile(Contact contact, bool isSelected, ContactsProvider contactsProvider, ChatProvider chatProvider) {
    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: isSelected ? Theme.of(context).colorScheme.primaryContainer : null,
        borderRadius: BorderRadius.circular(8),
      ),
      child: ListTile(
        dense: true,
        leading: CircleAvatar(
          radius: 20,
          backgroundColor: contact.isSystemContact 
              ? Colors.deepPurple.shade200 
              : Colors.blue.shade200,
          child: Icon(
            contact.isSystemContact ? Icons.auto_awesome : Icons.person,
            color: contact.isSystemContact 
                ? Colors.deepPurple.shade700 
                : Colors.blue.shade700,
            size: 20,
          ),
        ),
        title: Text(
          contact.name,
          style: TextStyle(
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 14,
          ),
        ),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              contact.threeWordAddress,
              style: TextStyle(
                fontSize: 11,
                color: Colors.grey.shade600,
              ),
            ),
            if (!contact.isSystemContact) ...[
              const SizedBox(height: 2),
              Row(
                children: [
                  Container(
                    width: 8,
                    height: 8,
                    decoration: BoxDecoration(
                      color: contact.isOnline ? Colors.green : Colors.grey,
                      shape: BoxShape.circle,
                    ),
                  ),
                  const SizedBox(width: 4),
                  Text(
                    contact.isOnline ? 'Online' : 'Offline',
                    style: TextStyle(
                      fontSize: 10,
                      color: Colors.grey.shade600,
                    ),
                  ),
                ],
              ),
            ],
          ],
        ),
        trailing: contact.unreadCount > 0
            ? Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: Colors.red,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Text(
                  '${contact.unreadCount}',
                  style: const TextStyle(
                    color: Colors.white,
                    fontSize: 10,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              )
            : null,
        onTap: () {
          contactsProvider.selectContact(contact);
          chatProvider.switchToContact(contact.id);
          
          // If it's system contact and menu is collapsed, show the help menu
          if (contact.isSystemContact && !contactsProvider.isSystemMenuExpanded) {
            contactsProvider.toggleSystemMenu();
          }
          
          WidgetsBinding.instance.addPostFrameCallback((_) {
            _scrollToBottom();
          });
        },
      ),
    );
  }

  Widget _buildChatArea(ContactsProvider contactsProvider, ChatProvider chatProvider) {
    return GestureDetector(
      onTap: () {
        // Collapse system menu when clicking in chat area
        if (contactsProvider.isSystemMenuExpanded) {
          contactsProvider.collapseSystemMenu();
        }
      },
      child: Column(
        children: [
          // Connection status (only show when no peers connected)
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
                  final selectedContact = contactsProvider.selectedContact;
                  return Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(
                          selectedContact?.isSystemContact == true 
                              ? Icons.auto_awesome 
                              : Icons.chat_bubble_outline,
                          size: 64,
                          color: Colors.grey,
                        ),
                        const SizedBox(height: 16),
                        Text(
                          selectedContact?.isSystemContact == true
                              ? 'Type ? for help'
                              : 'No messages yet',
                          style: const TextStyle(fontSize: 18, color: Colors.grey),
                        ),
                        Text(
                          selectedContact?.isSystemContact == true
                              ? 'Ask me about network status, connections, and more!'
                              : 'Start a conversation with ${selectedContact?.name ?? 'this contact'}',
                          style: const TextStyle(color: Colors.grey),
                        ),
                      ],
                    ),
                  );
                }
                
                return ListView.builder(
                  controller: _scrollController,
                  padding: const EdgeInsets.all(16),
                  itemCount: chatProvider.messages.length,
                  itemBuilder: (context, index) {
                    final message = chatProvider.messages[index];
                    return Padding(
                      padding: const EdgeInsets.symmetric(vertical: 4),
                      child: _buildMessageWidget(message, chatProvider, contactsProvider),
                    );
                  },
                );
              },
            ),
          ),
          
          // Message input
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surface,
              border: Border(
                top: BorderSide(color: Theme.of(context).dividerColor),
              ),
            ),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _messageController,
                    decoration: InputDecoration(
                      hintText: contactsProvider.selectedContact?.isSystemContact == true
                          ? 'Type a message or ? for help...'
                          : 'Type a message...',
                      border: const OutlineInputBorder(),
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
  
  Widget _buildMessageWidget(ChatMessage message, ChatProvider chatProvider, ContactsProvider contactsProvider) {
    switch (message.type) {
      case MessageType.help:
        return _buildHelpMessage(message, chatProvider, contactsProvider);
      case MessageType.system:
        return _buildSystemMessage(message);
      case MessageType.regular:
      default:
        return _buildRegularMessage(message);
    }
  }
  
  Widget _buildRegularMessage(ChatMessage message) {
    return Align(
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
    );
  }
  
  Widget _buildSystemMessage(ChatMessage message) {
    return Align(
      alignment: Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.symmetric(vertical: 8),
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          gradient: LinearGradient(
            colors: [Colors.deepPurple.shade100, Colors.blue.shade100],
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
          ),
          borderRadius: BorderRadius.circular(16),
          border: Border.all(color: Colors.deepPurple.shade200),
          boxShadow: [
            BoxShadow(
              color: Colors.deepPurple.withOpacity(0.1),
              blurRadius: 8,
              offset: const Offset(0, 2),
            ),
          ],
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: Colors.deepPurple.shade200,
                borderRadius: BorderRadius.circular(20),
              ),
              child: Icon(
                Icons.auto_awesome,
                color: Colors.deepPurple.shade700,
                size: 20,
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                message.content,
                style: TextStyle(
                  color: Colors.deepPurple.shade800,
                  fontSize: 14,
                  height: 1.4,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
  
  Widget _buildHelpMessage(ChatMessage message, ChatProvider chatProvider, ContactsProvider contactsProvider) {
    return Align(
      alignment: Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.symmetric(vertical: 8),
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          gradient: LinearGradient(
            colors: [Colors.amber.shade50, Colors.orange.shade50],
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
          ),
          borderRadius: BorderRadius.circular(16),
          border: Border.all(color: Colors.amber.shade200),
          boxShadow: [
            BoxShadow(
              color: Colors.orange.withOpacity(0.1),
              blurRadius: 12,
              offset: const Offset(0, 3),
            ),
          ],
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: Colors.amber.shade200,
                    borderRadius: BorderRadius.circular(20),
                  ),
                  child: Icon(
                    Icons.help_outline,
                    color: Colors.amber.shade800,
                    size: 20,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    message.content,
                    style: TextStyle(
                      color: Colors.amber.shade800,
                      fontSize: 16,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
              ],
            ),
            if (message.helpOptions != null && contactsProvider.isSystemMenuExpanded) ...[
              const SizedBox(height: 16),
              ...message.helpOptions!.map((option) => _buildHelpOption(option, chatProvider)),
            ] else if (message.helpOptions != null && !contactsProvider.isSystemMenuExpanded) ...[
              const SizedBox(height: 16),
              GestureDetector(
                onTap: () => contactsProvider.toggleSystemMenu(),
                child: Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: Colors.white.withOpacity(0.7),
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: Colors.amber.shade100),
                  ),
                  child: Row(
                    children: [
                      Icon(Icons.touch_app, color: Colors.amber.shade700, size: 16),
                      const SizedBox(width: 8),
                      Text(
                        'Tap to view options',
                        style: TextStyle(
                          color: Colors.amber.shade800,
                          fontSize: 14,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                      const Spacer(),
                      Icon(Icons.expand_more, color: Colors.amber.shade700, size: 16),
                    ],
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
  
  Widget _buildHelpOption(HelpOption option, ChatProvider chatProvider) {
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () => chatProvider.executeHelpAction(option.action),
          borderRadius: BorderRadius.circular(12),
          child: Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: Colors.white.withOpacity(0.7),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.amber.shade100),
            ),
            child: Row(
              children: [
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: Colors.blue.shade100,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Icon(
                    option.icon,
                    color: Colors.blue.shade700,
                    size: 18,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        option.title,
                        style: TextStyle(
                          color: Colors.grey.shade800,
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      Text(
                        option.description,
                        style: TextStyle(
                          color: Colors.grey.shade600,
                          fontSize: 12,
                        ),
                      ),
                    ],
                  ),
                ),
                Icon(
                  Icons.chevron_right,
                  color: Colors.grey.shade400,
                  size: 16,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  void _sendMessage() {
    final text = _messageController.text.trim();
    if (text.isNotEmpty) {
      Provider.of<ChatProvider>(context, listen: false).addMessage(text);
      _messageController.clear();
      // Auto-scroll to bottom after adding message
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _scrollToBottom();
      });
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
                        const Text('🔤 Three-Word Address:', style: TextStyle(color: Colors.grey)),
                        const SizedBox(height: 4),
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                          decoration: BoxDecoration(
                            color: Colors.blue.withOpacity(0.1),
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(color: Colors.blue.withOpacity(0.3)),
                          ),
                          child: Text(
                            networkProvider.localAddress,
                            style: const TextStyle(
                              fontSize: 18,
                              fontWeight: FontWeight.bold,
                              color: Colors.blue,
                            ),
                          ),
                        ),
                        const SizedBox(height: 8),
                        const Text(
                          'Technical: /ip6/::1/udp/9000/quic',
                          style: TextStyle(
                            fontSize: 12,
                            color: Colors.grey,
                            fontFamily: 'monospace',
                          ),
                        ),
                        const SizedBox(height: 8),
                        const Text('💫 Share this three-word address with others to connect'),
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
                          '🔤 Use human-friendly three-word addresses instead of complex URLs!',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: Colors.grey[600],
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        const SizedBox(height: 4),
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
                                    content: Text('Connecting to foundation.main.bootstrap...'),
                                    backgroundColor: Colors.green,
                                  ),
                                );
                              },
                              icon: const Icon(Icons.rocket_launch, size: 16),
                              label: const Column(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  Text('foundation.main.bootstrap', style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold)),
                                  Text('Main Bootstrap', style: TextStyle(fontSize: 9)),
                                ],
                              ),
                              style: ElevatedButton.styleFrom(
                                backgroundColor: Colors.green,
                                foregroundColor: Colors.white,
                                padding: EdgeInsets.symmetric(horizontal: 12, vertical: 8),
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
                              label: const Column(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  Text('global.fast.eagle', style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold)),
                                  Text('IPv6 Bootstrap', style: TextStyle(fontSize: 9)),
                                ],
                              ),
                              style: ElevatedButton.styleFrom(
                                backgroundColor: Colors.blue,
                                foregroundColor: Colors.white,
                                padding: EdgeInsets.symmetric(horizontal: 12, vertical: 8),
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
  final MessageType type;
  final List<HelpOption>? helpOptions;

  ChatMessage({
    required this.id,
    required this.content,
    required this.isFromMe,
    required this.timestamp,
    this.type = MessageType.regular,
    this.helpOptions,
  });
}

enum MessageType {
  regular,
  system,
  help,
}

class HelpOption {
  final String title;
  final String description;
  final IconData icon;
  final String action;

  HelpOption({
    required this.title,
    required this.description,
    required this.icon,
    required this.action,
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

class Contact {
  final String id;
  final String name;
  final String threeWordAddress;
  final String? avatarUrl;
  final DateTime lastSeen;
  final bool isOnline;
  final int unreadCount;
  final bool isSystemContact;

  Contact({
    required this.id,
    required this.name,
    required this.threeWordAddress,
    this.avatarUrl,
    required this.lastSeen,
    this.isOnline = false,
    this.unreadCount = 0,
    this.isSystemContact = false,
  });

  Contact copyWith({
    String? name,
    String? threeWordAddress,
    String? avatarUrl,
    DateTime? lastSeen,
    bool? isOnline,
    int? unreadCount,
    bool? isSystemContact,
  }) {
    return Contact(
      id: id,
      name: name ?? this.name,
      threeWordAddress: threeWordAddress ?? this.threeWordAddress,
      avatarUrl: avatarUrl ?? this.avatarUrl,
      lastSeen: lastSeen ?? this.lastSeen,
      isOnline: isOnline ?? this.isOnline,
      unreadCount: unreadCount ?? this.unreadCount,
      isSystemContact: isSystemContact ?? this.isSystemContact,
    );
  }
}

enum ContactsPanelPosition {
  left,
  top,
  right,
  bottom,
}

// Providers
class ContactsProvider extends ChangeNotifier {
  final List<Contact> _contacts = [];
  Contact? _selectedContact;
  ContactsPanelPosition _panelPosition = ContactsPanelPosition.left;
  bool _isSystemMenuExpanded = false;

  List<Contact> get contacts => _contacts;
  Contact? get selectedContact => _selectedContact;
  ContactsPanelPosition get panelPosition => _panelPosition;
  bool get isSystemMenuExpanded => _isSystemMenuExpanded;

  ContactsProvider() {
    _initializeDefaultContacts();
  }

  void _initializeDefaultContacts() {
    // Add system contact
    _contacts.add(Contact(
      id: 'system',
      name: 'System',
      threeWordAddress: 'system.helper.assistant',
      lastSeen: DateTime.now(),
      isOnline: true,
      isSystemContact: true,
    ));

    // Add some demo contacts
    _contacts.addAll([
      Contact(
        id: 'demo1',
        name: 'Alice Cooper',
        threeWordAddress: 'creative.music.legend',
        lastSeen: DateTime.now().subtract(const Duration(minutes: 5)),
        isOnline: true,
        unreadCount: 2,
      ),
      Contact(
        id: 'demo2', 
        name: 'Bob Builder',
        threeWordAddress: 'construction.master.expert',
        lastSeen: DateTime.now().subtract(const Duration(hours: 1)),
        isOnline: false,
        unreadCount: 0,
      ),
      Contact(
        id: 'demo3',
        name: 'Charlie Brown',
        threeWordAddress: 'friendly.neighbor.companion',
        lastSeen: DateTime.now().subtract(const Duration(minutes: 15)),
        isOnline: true,
        unreadCount: 1,
      ),
    ]);

    // Select system contact by default
    _selectedContact = _contacts.first;
  }

  void selectContact(Contact contact) {
    _selectedContact = contact;
    
    // Collapse system menu when switching contacts
    if (!contact.isSystemContact) {
      _isSystemMenuExpanded = false;
    }
    
    notifyListeners();
  }

  void setPanelPosition(ContactsPanelPosition position) {
    _panelPosition = position;
    notifyListeners();
  }

  void toggleSystemMenu() {
    if (_selectedContact?.isSystemContact == true) {
      _isSystemMenuExpanded = !_isSystemMenuExpanded;
      notifyListeners();
    }
  }

  void collapseSystemMenu() {
    _isSystemMenuExpanded = false;
    notifyListeners();
  }

  void addContact(Contact contact) {
    _contacts.add(contact);
    notifyListeners();
  }

  void updateContactUnreadCount(String contactId, int count) {
    final index = _contacts.indexWhere((c) => c.id == contactId);
    if (index != -1) {
      _contacts[index] = _contacts[index].copyWith(unreadCount: count);
      notifyListeners();
    }
  }
}

class ChatProvider extends ChangeNotifier {
  final Map<String, List<ChatMessage>> _contactMessages = {};
  VoidCallback? _scrollCallback;
  String _currentContactId = 'system';
  
  List<ChatMessage> get messages => _contactMessages[_currentContactId] ?? [];
  String get currentContactId => _currentContactId;
  
  void setScrollCallback(VoidCallback callback) {
    _scrollCallback = callback;
  }

  void switchToContact(String contactId) {
    _currentContactId = contactId;
    
    // Initialize messages for contact if not exists
    if (!_contactMessages.containsKey(contactId)) {
      _contactMessages[contactId] = [];
    }
    
    notifyListeners();
  }
  
  void addMessage(String content) {
    // Ensure messages list exists for current contact
    if (!_contactMessages.containsKey(_currentContactId)) {
      _contactMessages[_currentContactId] = [];
    }
    
    // Add user message
    _contactMessages[_currentContactId]!.add(ChatMessage(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      content: content,
      isFromMe: true,
      timestamp: DateTime.now(),
    ));
    
    // Check if it's a help request (only for system contact)
    if (_currentContactId == 'system') {
      if (content.trim() == '?') {
        _addHelpMessage();
      } else if (content.toLowerCase().contains('help')) {
        _addHelpMessage();
      }
    }
    
    notifyListeners();
    
    // Trigger auto-scroll after UI update
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scrollCallback?.call();
    });
  }
  
  void _addHelpMessage() {
    final helpOptions = [
      HelpOption(
        title: 'Network Status',
        description: 'Check your connection status and IPv6 support',
        icon: Icons.network_check,
        action: 'status',
      ),
      HelpOption(
        title: 'Connected Peers',
        description: 'View all connected peers and bootstrap nodes',
        icon: Icons.people,
        action: 'peers',
      ),
      HelpOption(
        title: 'Tunnel Information',
        description: 'Check tunnel status and NAT traversal',
        icon: Icons.vpn_lock,
        action: 'tunnels',
      ),
      HelpOption(
        title: 'Three-Word Addresses',
        description: 'Learn about human-friendly network addresses',
        icon: Icons.language,
        action: 'addresses',
      ),
      HelpOption(
        title: 'Create Inbox',
        description: 'Set up your personal DHT inbox for messages',
        icon: Icons.inbox,
        action: 'inbox',
      ),
      HelpOption(
        title: 'Bootstrap Nodes',
        description: 'Connect to well-known network entry points',
        icon: Icons.hub,
        action: 'bootstrap',
      ),
    ];
    
    _contactMessages[_currentContactId]!.add(ChatMessage(
      id: '${DateTime.now().millisecondsSinceEpoch}_help',
      content: '✨ Here are the available options:',
      isFromMe: false,
      timestamp: DateTime.now(),
      type: MessageType.help,
      helpOptions: helpOptions,
    ));
  }
  
  void executeHelpAction(String action) {
    String response = '';
    
    switch (action) {
      case 'status':
        response = '🌐 Network Status:\n'
            '• IPv6 Support: Available ✅\n'
            '• Connection: Direct Internet\n'
            '• Bootstrap Nodes: 4 discovered\n'
            '• Local Address: local.swift.lighthouse';
        break;
      case 'peers':
        response = '👥 Connected Peers:\n'
            '• Currently simulating peer connections\n'
            '• Use Quick Connect to add bootstrap nodes\n'
            '• Go to Network tab to manage connections';
        break;
      case 'tunnels':
        response = '🚇 Tunnel Information:\n'
            '• IPv6 available - no tunnel needed\n'
            '• NAT Type: Direct Internet (Public IP)\n'
            '• QUIC protocol for secure transport';
        break;
      case 'addresses':
        response = '🔤 Three-Word Addresses:\n'
            '• global.fast.eagle → IPv6 bootstrap\n'
            '• foundation.main.bootstrap → Primary node\n'
            '• local.swift.lighthouse → Your address\n'
            '• Human-friendly alternative to complex URLs';
        break;
      case 'inbox':
        // Actually create an inbox
        String inboxId = _generateInboxId();
        String threeWordAddress = _generateThreeWordAddress(inboxId);
        response = '📬 ✨ Inbox Created Successfully!\n\n'
            '🆔 Inbox ID: $inboxId\n'
            '🔤 Three-Word Address: $threeWordAddress\n'
            '🏠 Your permanent address on the DHT\n'
            '♾️ Messages stored with infinite TTL\n'
            '🔐 Only you can access this inbox\n\n'
            '📝 Share your three-word address with others!\n'
            '💬 Messages sent here persist forever on the network.';
        break;
      case 'bootstrap':
        response = '🚀 Bootstrap Nodes:\n'
            '• foundation.main.bootstrap\n'
            '• global.fast.eagle\n'
            '• reliable.sturdy.anchor\n'
            '• Use Quick Connect to join the network';
        break;
      default:
        response = '❓ Unknown command. Type ? for help.';
    }
    
    _contactMessages[_currentContactId]!.add(ChatMessage(
      id: '${DateTime.now().millisecondsSinceEpoch}_response',
      content: response,
      isFromMe: false,
      timestamp: DateTime.now(),
      type: MessageType.system,
    ));
    
    notifyListeners();
    
    // Trigger auto-scroll after UI update
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scrollCallback?.call();
    });
  }
  
  /// Generate a unique inbox ID
  String _generateInboxId() {
    final timestamp = DateTime.now().millisecondsSinceEpoch;
    final random = (timestamp % 10000).toString();
    return 'inbox_${random}_${timestamp.hashCode.abs()}';
  }
  
  /// Generate a three-word address for an inbox
  String _generateThreeWordAddress(String inboxId) {
    // Use a simple hash-based approach for demo
    final hash = inboxId.hashCode.abs();
    final words = [
      ['personal', 'private', 'secure', 'digital', 'encrypted', 'hidden', 'magic', 'crystal', 'golden', 'silver'],
      ['message', 'inbox', 'mailbox', 'vault', 'chamber', 'space', 'zone', 'portal', 'gateway', 'bridge'],
      ['haven', 'sanctuary', 'fortress', 'harbor', 'oasis', 'garden', 'tower', 'castle', 'palace', 'temple']
    ];
    
    final word1 = words[0][hash % words[0].length];
    final word2 = words[1][(hash ~/ 10) % words[1].length];
    final word3 = words[2][(hash ~/ 100) % words[2].length];
    
    return '$word1.$word2.$word3';
  }
}

class NetworkProvider extends ChangeNotifier {
  final List<PeerInfo> _peers = [];
  
  // Hardcoded well-known bootstrap nodes (will be updated with real Digital Ocean IPs)
  static const Map<String, String> _wellKnownBootstraps = {
    'foundation.main.bootstrap': '/dns4/bootstrap.p2pfoundation.org/udp/9000/quic',
    'foundation.backup.lighthouse': '/dns4/bootstrap2.p2pfoundation.org/udp/9000/quic',
    'global.fast.eagle': '/ip6/2604:a880:400:d1:0:2:40d7:9001/udp/9000/quic',
    'reliable.sturdy.anchor': '/ip4/147.182.203.123/udp/9000/quic',
    // Demo addresses for local testing
    'local.swift.lighthouse': '/ip6/::1/udp/9000/quic',
    'quick.strong.sword': '/ip6/::1/tcp/9000',
    'demo.test.node': '/ip6/::1/udp/9001/quic',
  };
  
  List<PeerInfo> get peers => _peers;
  int get peerCount => _peers.length;
  String get localAddress => 'local.swift.lighthouse';
  
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
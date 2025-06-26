class PeerInfo {
  final String id;
  final String name;
  final String address;
  final DateTime connectionTime;
  final bool isOnline;
  final double? latency;
  final String? lastSeen;
  final int messageCount;

  PeerInfo({
    required this.id,
    required this.name,
    required this.address,
    required this.connectionTime,
    this.isOnline = false,
    this.latency,
    this.lastSeen,
    this.messageCount = 0,
  });

  PeerInfo copyWith({
    String? id,
    String? name,
    String? address,
    DateTime? connectionTime,
    bool? isOnline,
    double? latency,
    String? lastSeen,
    int? messageCount,
  }) {
    return PeerInfo(
      id: id ?? this.id,
      name: name ?? this.name,
      address: address ?? this.address,
      connectionTime: connectionTime ?? this.connectionTime,
      isOnline: isOnline ?? this.isOnline,
      latency: latency ?? this.latency,
      lastSeen: lastSeen ?? this.lastSeen,
      messageCount: messageCount ?? this.messageCount,
    );
  }

  String get displayName {
    return name != id ? name : 'Peer ${id.substring(0, 8)}';
  }

  String get shortId {
    return id.length > 8 ? id.substring(0, 8) : id;
  }

  String get connectionDuration {
    final duration = DateTime.now().difference(connectionTime);
    if (duration.inDays > 0) {
      return '${duration.inDays}d ago';
    } else if (duration.inHours > 0) {
      return '${duration.inHours}h ago';
    } else if (duration.inMinutes > 0) {
      return '${duration.inMinutes}m ago';
    } else {
      return 'Just now';
    }
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'address': address,
      'connectionTime': connectionTime.millisecondsSinceEpoch,
      'isOnline': isOnline,
      'latency': latency,
      'lastSeen': lastSeen,
      'messageCount': messageCount,
    };
  }

  factory PeerInfo.fromJson(Map<String, dynamic> json) {
    return PeerInfo(
      id: json['id'],
      name: json['name'],
      address: json['address'],
      connectionTime: DateTime.fromMillisecondsSinceEpoch(json['connectionTime']),
      isOnline: json['isOnline'] ?? false,
      latency: json['latency']?.toDouble(),
      lastSeen: json['lastSeen'],
      messageCount: json['messageCount'] ?? 0,
    );
  }

  @override
  String toString() {
    return 'PeerInfo{id: $shortId, name: $name, isOnline: $isOnline}';
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is PeerInfo && other.id == id;
  }

  @override
  int get hashCode => id.hashCode;
}
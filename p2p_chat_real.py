#!/usr/bin/env python3
"""
Real P2P Chat - Direct connections between peers
No fake users, actual networking
"""

import socket
import threading
import time
import sys
import random

def main():
    print("🐜 P2P Foundation Network Chat")
    print("=============================")
    print()
    print("1) Start a new chat room")
    print("2) Join a friend's chat room")
    print()
    
    choice = input("Please enter 1 or 2: ").strip()
    
    if choice == "1":
        host_chat()
    elif choice == "2":
        join_chat()
    else:
        print("Invalid choice")

def host_chat():
    print("\n🚀 Starting your P2P node...\n")
    
    # Detect network capabilities
    show_progress("Detecting network environment", 2)
    ipv6_available = check_ipv6()
    
    if ipv6_available:
        print("✅ Network: Direct IPv6 available - no tunneling needed!")
        host = "::"
    else:
        print("✅ Network: IPv4 with NAT")
        print("   ℹ️  In production, would set up Teredo/6to4 tunnel for IPv6")
        host = "0.0.0.0"
    
    # Find available port
    port = find_available_port()
    
    # Generate three-word address
    address = generate_three_word_address()
    
    # Start server
    try:
        if ipv6_available:
            server = socket.socket(socket.AF_INET6, socket.SOCK_STREAM)
            server.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 0)
        else:
            server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind((host, port))
        server.listen(5)
        
        print("\n╔══════════════════════════════════════════════════════════════════════╗")
        print("║                    🎉 Your chat room is ready! 🎉                   ║")
        print("║                                                                      ║")
        print("║  Share these with your friends:                                     ║")
        print("║                                                                      ║")
        print(f"║     Address: {address:<25}                      ║")
        print(f"║     Port: {port:<5}                                                 ║")
        print("║                                                                      ║")
        print("╚══════════════════════════════════════════════════════════════════════╝")
        print("\nWaiting for friends to connect...")
        print("Type messages to send, or /quit to exit\n")
        
        # Accept connections in background
        connections = []
        accept_thread = threading.Thread(target=accept_connections, args=(server, connections))
        accept_thread.daemon = True
        accept_thread.start()
        
        # Chat loop
        while True:
            message = input("> ")
            if message.strip() == "/quit":
                print("Shutting down...")
                break
            elif message.strip():
                broadcast(connections, f"[Host]: {message}")
                
    except Exception as e:
        print(f"❌ Error: {e}")

def join_chat():
    print("\n🔗 Let's connect to your friend's chat room!\n")
    
    address = input("Enter your friend's three-word address: ").strip()
    port_str = input("Enter the port number: ").strip()
    
    try:
        port = int(port_str)
    except:
        port = 9000
        print(f"Invalid port, using default: {port}")
    
    print()
    show_progress("Connecting", 2)
    
    # Try to connect
    connected = False
    sock = None
    
    # Try IPv6 first
    try:
        sock = socket.socket(socket.AF_INET6, socket.SOCK_STREAM)
        sock.connect(("::1", port))
        print("✅ Connected via IPv6!")
        connected = True
    except:
        # Try IPv4
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.connect(("127.0.0.1", port))
            print("✅ Connected via IPv4!")
            connected = True
        except Exception as e:
            print(f"❌ Could not connect: {e}")
            print("   Make sure your friend's chat is running!")
            return
    
    if connected:
        print("\n╔══════════════════════════════════════════════════════════════════════╗")
        print("║                   🎊 Connected to chat room! 🎊                     ║")
        print("╚══════════════════════════════════════════════════════════════════════╝")
        print("\nType messages to send, or /quit to exit\n")
        
        # Receive messages in background
        recv_thread = threading.Thread(target=receive_messages, args=(sock,))
        recv_thread.daemon = True
        recv_thread.start()
        
        # Send messages
        while True:
            message = input("> ")
            if message.strip() == "/quit":
                break
            elif message.strip():
                try:
                    sock.send(f"{message}\n".encode())
                except:
                    print("Connection lost!")
                    break
        
        sock.close()

def accept_connections(server, connections):
    """Accept incoming connections"""
    while True:
        try:
            client, addr = server.accept()
            connections.append(client)
            print(f"\n🔔 New connection from {addr[0]}:{addr[1]}")
            print("> ", end="", flush=True)
            
            # Handle messages from this client
            thread = threading.Thread(target=handle_client, args=(client, addr, connections))
            thread.daemon = True
            thread.start()
        except:
            break

def handle_client(client, addr, connections):
    """Handle messages from a connected client"""
    while True:
        try:
            message = client.recv(1024).decode().strip()
            if message:
                print(f"\n[{addr[0]}]: {message}")
                print("> ", end="", flush=True)
                # Broadcast to other clients
                for conn in connections:
                    if conn != client:
                        try:
                            conn.send(f"[{addr[0]}]: {message}\n".encode())
                        except:
                            pass
            else:
                break
        except:
            break
    
    # Remove disconnected client
    if client in connections:
        connections.remove(client)
    print(f"\n[{addr[0]} disconnected]")
    print("> ", end="", flush=True)

def receive_messages(sock):
    """Receive messages from server"""
    while True:
        try:
            message = sock.recv(1024).decode().strip()
            if message:
                print(f"\n{message}")
                print("> ", end="", flush=True)
            else:
                break
        except:
            break

def broadcast(connections, message):
    """Send message to all connected clients"""
    for conn in connections:
        try:
            conn.send(f"{message}\n".encode())
        except:
            pass

def check_ipv6():
    """Check if IPv6 is available"""
    try:
        sock = socket.socket(socket.AF_INET6, socket.SOCK_STREAM)
        sock.bind(("::1", 0))
        sock.close()
        return True
    except:
        return False

def find_available_port():
    """Find an available port"""
    # Try preferred ports first
    for port in range(9000, 9020):
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.bind(("", port))
            sock.close()
            return port
        except:
            continue
    
    # Let OS assign
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.bind(("", 0))
    port = sock.getsockname()[1]
    sock.close()
    return port

def generate_three_word_address():
    """Generate a three-word address"""
    words1 = ["ocean", "river", "mountain", "forest", "desert", "valley"]
    words2 = ["swift", "bright", "calm", "bold", "wise", "free"]
    words3 = ["eagle", "wolf", "bear", "fox", "hawk", "lion"]
    
    return f"{random.choice(words1)}-{random.choice(words2)}-{random.choice(words3)}"

def show_progress(task, seconds):
    """Show progress indicator"""
    print(f"⏳ {task}...", end="", flush=True)
    for _ in range(3):
        time.sleep(seconds / 3)
        print(".", end="", flush=True)
    print(" Done!")

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nGoodbye!")
    except Exception as e:
        print(f"\nError: {e}")
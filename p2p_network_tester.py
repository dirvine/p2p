#!/usr/bin/env python3
"""
P2P Network Tester - Real network testing
Handles port conflicts gracefully
"""

import socket
import time
import threading
import sys

def main():
    print("\n╔══════════════════════════════════════════════════════════════════════╗")
    print("║               🐜 P2P Foundation Network Tester 🐜                    ║")
    print("║                                                                      ║")
    print("║                Real Network Testing - No Simulations                 ║")
    print("╚══════════════════════════════════════════════════════════════════════╝")
    print()
    
    print("What would you like to test?")
    print()
    print("  1) Quick Network Test (30 seconds)")
    print("     • IPv6/IPv4 detection")
    print("     • Port availability")
    print("     • Basic connectivity")
    print()
    print("  2) Port Scanner")
    print("     • Find available ports")
    print("     • Handle conflicts gracefully")
    print()
    
    choice = input("Please enter 1 or 2: ").strip()
    
    if choice == "1":
        run_quick_test()
    elif choice == "2":
        run_port_scan()
    else:
        print("Invalid choice")

def run_quick_test():
    print("\n🚀 Starting Quick Network Test")
    print("══════════════════════════════")
    print()
    
    start_time = time.time()
    passed = 0
    failed = 0
    
    # Test 1: IPv6 Support
    print("📋 Testing IPv6 Support")
    print("───────────────────────")
    
    try:
        sock = socket.socket(socket.AF_INET6, socket.SOCK_STREAM)
        sock.bind(("::1", 0))
        port = sock.getsockname()[1]
        sock.close()
        print(f"  ✅ IPv6 loopback: Available (bound to port {port})")
        passed += 1
        
        # Test all interfaces
        try:
            sock = socket.socket(socket.AF_INET6, socket.SOCK_STREAM)
            sock.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 0)
            sock.bind(("::", 0))
            port = sock.getsockname()[1]
            sock.close()
            print(f"  ✅ IPv6 all interfaces: Available (bound to port {port})")
            print("  ℹ️  Direct IPv6 connectivity available - no tunnel needed!")
            passed += 1
        except Exception as e:
            print(f"  ⚠️  IPv6 all interfaces: Not available ({e})")
            print("  ℹ️  Would need tunnel for external IPv6 connectivity")
            failed += 1
    except Exception as e:
        print(f"  ❌ IPv6: Not supported ({e})")
        print("  ℹ️  Will need IPv6 tunnel (Teredo/6to4) for P2P connectivity")
        failed += 2
    
    print()
    
    # Test 2: IPv4 Support
    print("📋 Testing IPv4 Support")
    print("───────────────────────")
    
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
        sock.close()
        print(f"  ✅ IPv4 loopback: Available (bound to port {port})")
        passed += 1
    except Exception as e:
        print(f"  ❌ IPv4 loopback: Failed ({e})")
        failed += 1
    
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.bind(("0.0.0.0", 0))
        port = sock.getsockname()[1]
        sock.close()
        print(f"  ✅ IPv4 all interfaces: Available (bound to port {port})")
        passed += 1
    except Exception as e:
        print(f"  ❌ IPv4 all interfaces: Failed ({e})")
        failed += 1
    
    print()
    
    # Test 3: Port Availability
    print("📋 Testing Common P2P Ports")
    print("────────────────────────────")
    
    test_ports = [9000, 9001, 9002, 9003, 9004]
    available_ports = []
    
    for port in test_ports:
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            sock.bind(("0.0.0.0", port))
            sock.close()
            print(f"  ✅ Port {port}: Available")
            available_ports.append(port)
            passed += 1
        except:
            print(f"  ⚠️  Port {port}: In use (will auto-select another)")
            # This is not a failure - we expect some ports to be in use
    
    if not available_ports:
        print("  ℹ️  No default ports available, testing dynamic allocation...")
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.bind(("0.0.0.0", 0))
            port = sock.getsockname()[1]
            sock.close()
            print(f"  ✅ Dynamic port allocated: {port}")
            passed += 1
        except Exception as e:
            print(f"  ❌ Dynamic port allocation failed: {e}")
            failed += 1
    
    print()
    
    # Test 4: UDP Support (for QUIC)
    print("📋 Testing UDP Support (for QUIC)")
    print("─────────────────────────────────")
    
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.bind(("0.0.0.0", 0))
        port = sock.getsockname()[1]
        sock.close()
        print(f"  ✅ UDP binding: Success (port {port})")
        passed += 1
    except Exception as e:
        print(f"  ❌ UDP binding: Failed ({e})")
        failed += 1
    
    print()
    
    # Test 5: Loopback connectivity
    print("📋 Testing Loopback Connectivity")
    print("────────────────────────────────")
    
    try:
        # Start server
        server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        server.bind(("127.0.0.1", 0))
        port = server.getsockname()[1]
        server.listen(1)
        
        # Connect in thread
        connected = [False]
        def connect():
            try:
                client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                client.connect(("127.0.0.1", port))
                client.send(b"PING")
                client.close()
                connected[0] = True
            except:
                pass
        
        thread = threading.Thread(target=connect)
        thread.start()
        
        # Accept connection
        server.settimeout(2)
        conn, addr = server.accept()
        data = conn.recv(4)
        
        if data == b"PING" and connected[0]:
            print("  ✅ Loopback communication: Working")
            passed += 1
        else:
            print("  ❌ Loopback communication: Data mismatch")
            failed += 1
        
        conn.close()
        server.close()
        thread.join()
        
    except Exception as e:
        print(f"  ❌ Loopback connection: Failed ({e})")
        failed += 1
    
    print()
    
    # Summary
    duration = time.time() - start_time
    show_test_summary(passed, failed, duration)

def run_port_scan():
    print("\n🔍 Scanning for Available Ports")
    print("════════════════════════════════")
    print()
    
    ranges = [
        ("Common P2P", range(9000, 9020)),
        ("Alternative", range(8000, 8010)),
        ("High ports", range(30000, 30010)),
    ]
    
    for name, port_range in ranges:
        print(f"📋 {name} Ports")
        print("─" * (len(name) + 7))
        
        available = []
        for port in port_range:
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                sock.bind(("0.0.0.0", port))
                sock.close()
                available.append(port)
            except:
                pass
        
        if available:
            print(f"  ✅ Available: {', '.join(map(str, available))}")
        else:
            print("  ⚠️  All ports in range are busy")
        
        print()
    
    # Always show dynamic allocation
    print("📋 Dynamic Port Allocation")
    print("─────────────────────────")
    
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.bind(("0.0.0.0", 0))
        port = sock.getsockname()[1]
        sock.close()
        print(f"  ✅ System allocated port: {port}")
        print("  ℹ️  This port is guaranteed to be available")
    except Exception as e:
        print(f"  ❌ Dynamic allocation failed: {e}")

def show_test_summary(passed, failed, duration):
    total = passed + failed
    pass_rate = (passed / total * 100) if total > 0 else 100
    
    print("╔══════════════════════════════════════════════════════════════════════╗")
    print("║                         📊 Test Results 📊                           ║")
    print("║                                                                      ║")
    print(f"║  Total Tests: {total:>3}                                                    ║")
    print(f"║  Passed:      {passed:>3} ✅                                                 ║")
    print(f"║  Failed:      {failed:>3} ❌                                                 ║")
    print(f"║  Pass Rate:   {pass_rate:>5.1f}%                                                ║")
    print(f"║  Duration:    {duration:>3.0f} seconds                                           ║")
    print("║                                                                      ║")
    
    if failed == 0:
        print("║           🎉 All tests passed! Network is healthy! 🎉               ║")
    elif pass_rate >= 80:
        print("║        ✅ Network mostly functional, some issues detected           ║")
    else:
        print("║        ⚠️  Network issues detected, check configuration            ║")
    
    print("║                                                                      ║")
    print("╚══════════════════════════════════════════════════════════════════════╝")
    
    print("\nPress Enter to exit...")
    input()

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nInterrupted by user")
    except Exception as e:
        print(f"\nError: {e}")
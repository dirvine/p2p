// Copyright 2024 Saorsa Labs Limited
//
// Integration tests for four-word-networking library

#![cfg(feature = "four-word-addresses")]

use saorsa_core::{AddressBook, NetworkAddress};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[test]
fn test_four_word_ipv4_encoding() {
    // Create an IPv4 address
    let addr = NetworkAddress::from_ipv4(Ipv4Addr::new(192, 168, 1, 100), 8080);

    // Should have four-word representation
    assert!(addr.four_words().is_some());

    // The four-word string should be non-empty
    let four_words = addr.four_words().unwrap();
    assert!(!four_words.is_empty());

    // Should be able to parse it back
    let parsed = NetworkAddress::from_four_words(four_words).unwrap();
    assert_eq!(parsed.socket_addr(), addr.socket_addr());
}

#[test]
fn test_four_word_ipv6_encoding() {
    // Create an IPv6 address
    let addr = NetworkAddress::from_ipv6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1), 9000);

    // Should have four-word representation
    assert!(addr.four_words().is_some());

    // The four-word string should be non-empty
    let four_words = addr.four_words().unwrap();
    assert!(!four_words.is_empty());

    // Should be able to parse it back
    let parsed = NetworkAddress::from_four_words(four_words).unwrap();
    assert_eq!(parsed.socket_addr(), addr.socket_addr());
}

#[test]
fn test_four_word_round_trip() {
    let test_addresses = vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 9000),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 3000),
        SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8080),
    ];

    for socket_addr in test_addresses {
        let addr = NetworkAddress::new(socket_addr);

        if let Some(four_words) = addr.four_words() {
            // Parse back from four-words
            let parsed = NetworkAddress::from_four_words(four_words).unwrap();

            // Should have the same socket address
            assert_eq!(parsed.socket_addr(), socket_addr);

            // Should preserve the four-word representation
            assert_eq!(parsed.four_words(), Some(four_words));
        }
    }
}

#[test]
fn test_four_word_string_parsing() {
    // Test that NetworkAddress can be parsed from string
    // This would be a four-word string in real usage
    let addr_str = "127.0.0.1:8080"; // Regular format for now
    let addr: NetworkAddress = addr_str.parse().unwrap();

    assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(addr.port(), 8080);
}

#[test]
fn test_address_book_with_four_words() {
    let mut book = AddressBook::new();

    // Add some addresses
    let addr1 = NetworkAddress::from_ipv4(Ipv4Addr::new(192, 168, 1, 1), 9000);
    let addr2 = NetworkAddress::from_ipv4(Ipv4Addr::new(192, 168, 1, 2), 9001);

    book.add_address(addr1.clone());
    book.add_address(addr2.clone());

    assert_eq!(book.len(), 2);

    // Check that four-word representations are preserved
    for addr in book.addresses() {
        assert!(addr.four_words().is_some());
    }
}

#[test]
fn test_four_word_display() {
    let addr = NetworkAddress::from_ipv4(Ipv4Addr::new(192, 168, 1, 1), 9000);
    let display = format!("{}", addr);

    // Should include the socket address
    assert!(display.contains("192.168.1.1:9000"));

    // If four-words are available, should include them too
    if let Some(four_words) = addr.four_words() {
        assert!(display.contains(four_words));
    }
}

#[test]
fn test_multiple_address_formats() {
    // Test that we can parse different address formats
    let formats = vec![
        "127.0.0.1:8080",     // Standard IP:port
        "192.168.1.1:9000",   // Private IP
        "[::1]:8080",         // IPv6 localhost
        "[2001:db8::1]:9000", // IPv6 address
    ];

    for addr_str in formats {
        let addr: NetworkAddress = addr_str.parse().unwrap();

        // Should have socket address
        assert!(!addr.socket_addr().to_string().is_empty());

        // Should have four-word representation
        assert!(addr.four_words().is_some());
    }
}

#[test]
fn test_special_addresses() {
    // Test special addresses
    let localhost = NetworkAddress::from_ipv4(Ipv4Addr::LOCALHOST, 8080);
    assert!(localhost.is_loopback());
    assert!(localhost.four_words().is_some());

    let private = NetworkAddress::from_ipv4(Ipv4Addr::new(192, 168, 0, 1), 9000);
    assert!(private.is_private());
    assert!(private.four_words().is_some());

    let public = NetworkAddress::from_ipv4(Ipv4Addr::new(8, 8, 8, 8), 53);
    assert!(!public.is_private());
    assert!(!public.is_loopback());
    assert!(public.four_words().is_some());
}

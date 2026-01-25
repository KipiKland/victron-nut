use std::{error::Error, net::SocketAddr};

use mac_address::MacAddress;
use tokio::net::{UdpSocket};

fn build_magic_packet(mac_addr: &MacAddress) -> Vec<u8> {
    let mut magic_packet = vec![0; 102];
    // first 6 bytes are 0xff
    for i in 0..6 {
        magic_packet[i] = 0xff;
    }

    let mut index = 6;
    let mac_addr_bytes = mac_addr.bytes();

    // followed by 16 times of mac address
    for _ in 0..16 {
        for j in 0..mac_addr_bytes.len() {
            magic_packet[index] = mac_addr_bytes[j];
            index = index + 1;
        }
    }

    magic_packet
}

pub async fn send_wol(mac_addr: MacAddress, destination: SocketAddr) -> Result<(), Box<dyn Error>> {
    let magic_packet = build_magic_packet(&mac_addr);

    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.send_to(&magic_packet, destination).await?;

    Ok(())
}
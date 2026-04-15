<a id="readme-top"></a>

<div align="center">
  <h1 align="center">teltonika-avl-protocol</h3>
  <h3 align="center">
    Teltonika AVL protocol written in Rust
  </h3>
  <div align="center">

[![Crates.io](https://img.shields.io/crates/v/teltonika-avl-protocol.svg)](https://crates.io/crates/teltonika-avl-protocol)
[![Docs](https://img.shields.io/badge/docs-latest-blue)](https://docs.rs/teltonika-avl-protocol/latest/teltonika-avl-protocol/)
[![Docs](https://img.shields.io/badge/msrv-1.87.0-red)](https://docs.rs/teltonika-avl-protocol/latest/teltonika-avl-protocol/)

  </div>
</div>

## About the project

The AVL protocol defines a set of codecs that enable interfacing with a variety of Teltonika vehicle tracking devices.

Depending on the device feature set and physical configuration, codec packets can contain GPS data such as longitude/latitude coordinates, angle of travel, altitude, and current speed, as well as IO event data such as ignition status and more.

### Included

- Data-types and functionality for encoding and decoding AVL Codec8 packets.
- `no-std` compatible
- Re-exports `Heapless::Vec` as `StackVec` to avoid polluting namespace incase your project uses `std`

### Implemented Codecs

- Codec 8 (`0x08`)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Installation

```sh
cargo add teltonika-avl-protocol
```

or add to Cargo.toml dependencies, [crates.io](https://crates.io/crates/teltonika-avl-protocol) for latest version.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Codec8 packet handling

```rust
use teltonika_avl_protocol::{
    StackVec,
    codec8::{
        AvlDataRecord, AvlGpsElement, AvlN1Element, AvlN2Element, AvlN4Element, AvlN8Element,
        Codec8Packet, Coordinate, Priority,
    },
};

pub fn main() {
    let mut buf = [0; Codec8Packet::MAX_LENGTH];

    let packet = Codec8Packet {
        avl_data_records: StackVec::from_slice(&[AvlDataRecord {
            timestamp: 0x000000016b40d8ea30,
            priority: Priority::Medium,
            gps_element: AvlGpsElement {
                longitude: Coordinate(0.0),
                latitude: Coordinate(0.0),
                altitude: 0,
                angle: 0,
                satellites: 0,
                speed: 0,
            },
            event_io_id: 1,
            total_io_count: 4,
            n1_elements: StackVec::from_slice(&[AvlN1Element {
                id: 0x15,
                value: 0x03,
            }])
            .unwrap(),
            n2_elements: StackVec::from_slice(&[AvlN2Element {
                id: 0x42,
                value: 0x5e0f,
            }])
            .unwrap(),
            n4_elements: StackVec::from_slice(&[AvlN4Element {
                id: 0xf1,
                value: 0x0000601a,
            }])
            .unwrap(),
            n8_elements: StackVec::from_slice(&[AvlN8Element {
                id: 0x4e,
                value: 0x0,
            }])
            .unwrap(),
        }])
        .unwrap(),
    };

    let bytes_encoded = packet.encode(&mut buf).unwrap();

    let (bytes_decoded, packet_decoded) = Codec8Packet::decode(&buf[..bytes_encoded]).unwrap();

    assert_eq!(bytes_encoded, bytes_decoded);
    assert_eq!(packet, packet_decoded);
}
```

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Contributing

This project is open to contributions, create a new issue and let's discuss.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## License

Distributed under the MIT License. See `LICENSE.txt` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Acknowledgments
This is an independent open-source project and is not an official Teltonika project, product, or repository.

The Teltonika name and the following specification are copyright of Teltonika IOT Group.

Teltonika AVL Protocol specification:
https://wiki.teltonika-gps.com/view/Teltonika_AVL_Protocols

<p align="right">(<a href="#readme-top">back to top</a>)</p>

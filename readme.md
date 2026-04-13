<a id="readme-top"></a>

<div align="center">
  <h1 align="center">teletonika-avl-protocol</h3>
  <h3 align="center">
    Teletonika AVL protocol written in Rust
  </h3>
  <div align="center">

[![Crates.io](https://img.shields.io/crates/v/teletonika-avl-protocol.svg)](https://crates.io/crates/teletonika-avl-protocol)
[![Docs](https://img.shields.io/badge/docs-latest-blue)](https://docs.rs/teletonika-avl-protocol/latest/teletonika-avl-protocol/)
[![Docs](https://img.shields.io/badge/msrv-1.81.0-red)](https://docs.rs/teletonika-avl-protocol/latest/teletonika-avl-protocol/)

  </div>
</div>

## About the project

The AVL protocol defines a set of codecs which allow to the interfacing with a variety of teletonika vehicle tracking devices.
Depending on the device feature set, codec packets can contain GPS data such as long / lat coordinates, angle of travel, altitude and current speed, as well as IO Event data such as Ignition On, and many more.

### Included

- Data-types and functionality for encoding and decoding AVL Codec8 packets.
- Re-exports `Heapless::Vec` as `StackVec` to avoid polluting namespace incase your project uses `std`

### Implemented Codecs

- Codec 8 (`0x08`)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Installation

```sh
cargo add teletonika-avl-protocol
```

or add to Cargo.toml dependencies, [crates.io](https://crates.io/crates/teletonika-avl-protocol) for latest version.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Codec8 packet handling

```rust
use teletonika_avl_protocol::{
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

This is an independent open-source project and is not an official Teletonika project, product, or repository.

Teletonika AVL Protocol specification: 
https://wiki.teltonika-gps.com/view/Teltonika_AVL_Protocols

<p align="right">(<a href="#readme-top">back to top</a>)</p>

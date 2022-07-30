# COFIT — Communication over fixed interval, fixed frame, limited MTU transports

- Two types of communication
	- Unreliable packets
		- Fire-and-forget
		- Not ACKed
		- Not repeated
		- No congestion control
		- No serialization
		- No packet IDs
	- Reliable messages
		- Each ACKed
		- Automatically serialized/deserialized
		- Maximum size limited by MTU, single packet only
	- Reliable stream of raw data
		- Not ACKed
		- Initialized by a message
		- Not serialized/deserialized in any way
		- Transferred in enumerated chunks smaller than MTU
		- Dropped packets detected by discontinuity in chunk IDs
		- Reliability through re-request of dropped packets
		- Bandwidth estimation on tx side to avoid dropping

## Packet header identifiers

Acknowledgements repeat the exact same packet ID and content as the original message. As the communication is fixed interval and we can not transmit packets smaller than the MTU, the remaining transfer slot would be wasted anyways. This way we do not have to rely on hashing and potential collisions.

```
0b00______ = Message with ID __
0b01______ = ACK of packet with ID __

0b10______ = Stream packet with seq ID __, followed by two additional sequence ID bytes
0b11000000 = Stream REVT, followed by 22-bit sequence ID
0b11000001 = Stream CLSD, optionally followed by hash for integrity validation

0b11111111 = Raw packet
```

## Stream congestion control

Each individual packet in a reliable stream has a sequence identifier. If the rx side receives an out-of-seq packet, it can issue a REVT followed by a sequence ID. The tx side may then revert its stream to this identifier and restart transmission.

Since there are only a limited number of sequence IDs available, they will wrap around. This introduces a chance where the rx side lags for exactly the right amount of time and encounters a in-seq id which is actually one wrap-around ahead. To detect such a scenario, the second byte of each stream packet contains a section CRC. This CRC is calculated over all previously transmitted data up-to but not including the latest packet with seq ID `0`.

<!--
TODO: If the data repeats itself or is all-zero then this will not work :(
Instead we just use two bytes + the first 6-bit to get ~4 Mio. packets and the revert is just within the same section.
That covers about 1h10min of transfer at 1kHz. Such large transfers will likely never happen and if they do, never lag behind for that much. Additionally, it only costs us ~5% of speed which amounts to ~1.5sec for a 4MB transfer at 1kHz.

Also, do not allow wrap-around. Hard-cap the stream at 2^(6+8+8)*61 bytes — if it is longer, then multiple consecutive streams have to be used. That way reverts become way easier and have less edge cases. Also makes the API simpler.
-->
This way, each section of `2^6 = 64` receives a reasonably unique identifier which can be compared easily by the rx side. If it mismatches, then a REVT packet will be transmitted with the expected seq ID and section CRC. It is then up to the tx side to figure out how far to revert in order to reach this point. This could be implemented by either caching all previous section CRCs or re-iterating the stream up to that point — however, this is considered an implementation detail which should be hidden within the networking library.

## Stream integrity validation

While the stream itself should in theory reliably transmit the data, an additional validation option is included in the protocol. When finalizing a stream, an optional hash value over the whole stream content may be sent. If available, this value may optionally be parsed by the sending side to verify that the correct data has been received.

# Version 2

- Base network allows locking of underlying transport for send
- Network is instantiated with a number of protocols
- Each protocol defines a function which parses a packet and determines whether it would like to handle it
- The network can be in three states:
	- Receiving
	- Sending
	- Idle
- When idle, either
	- A message arrives which can be handled by a registered protocol (=> Receiving state)
	- The API is used to open up a channel with a given protocol (=> Sending state)
		- The used protocol does not have to be registered for reception!
- The network can either be in host or peripheral mode
	- This influences whether it assigns protocol numbers or is a "slave" in regards to protocols
	- When a peripheral wants to open a channel with a given protocol but that protocol has not been registered by the host, then it can not be sent
- Protocols each have a unique identifying name (string)
	- The network internally sends "openers" with a numeric ID
	- When a protocol has not been used before by the "host", it gets a numeric ID assigned and the "peripheral" is notified of this assignment
	- Either the peripheral ignores this (if no protocol with the given identifier has been registered), or stores this assignment

Open questions:
- How do we want to handle incoming messages while preserving the same API for outgoing ones?
	- Maybe have a `IncomingProtocol`, `OutgoingProtocol`, and `Protocol` or smth like that?
- How would the abstraction of req/ack, req/res, req/stream work?
	- i.e. how could you best implement a protocol which uses one of these three schemes

Packet prefixes:
```
00000000 => Reset & Connect (reset state, close channel, unbind protocol IDs)
00000001 => Assign protocol ID
1_______ => Open channel with protocol ID _
11111111 => Transmit protocol data
		While the last two could be merged, this makes it easier to parse and catch errors
```

When a reset packet is received, we bail out by throwing an error from the recv function used within a channel. It is then expected to abort/reset!

A reset is a two-way handshake where the host sends a reset, then the peripheral aborts any open channels and then replies with a reset as well. That way "stray" packets that a leftover channel might still be sending are discarded.

```rust
let transport = _;
let protocol_registry = ProtocolRegistry<3>::new([&protocol1, &protocol2, &protocol3]);

let mut reset_flag = false;
let transport = TransportWrapper::new(underlying_transport, &reset_flag);

fn open_channel(request) {
	let protocol_type = protocol_registry.map(request.protocol_id);

	if protocol1.can_handle(request.protocol_name) {
		protocol1.handle(request.data, &mut transport).await;
	}

	if protocol2.can_handle(request.protocol_name) {
		protocol2.handle(request.data, &mut transport).await;
	}
}

fn assign_protocol(assignment) {
	protocol_registry.assign(assignment.protocol_id, assignment.protocol_name);
}

fn reset() {
	protocol_registry.clear_assignments();
	reset_flag = false;
}

fn process(packet) {
	match packet {
		Packet::Reset => reset(),
		Packet::Assign => assign_protocol(packet as ProtocolAssignment),
		Packet::OpenChannel => open_channel(packet as ChannelRequest),
		Packet::ChannelData => error!("received channel data while no channel was open"),
	}
}

loop {
	let packet = transport.recv();
	process(packet);

	if reset_flag {
		reset();
	}
}
```

Outside API:
```rust
let protocol1 = _;
let protocol2 = _;
let transport = _;

let network = build_network! {
	MTU = 64,
	transport,
	protocols = [protocol1, protocol2]
};
```

Outside helpers:
```rust
struct TransportWrapper(&Transport, &mut ProtocolRegistry);

impl TransportWrapper {
	fn send(data) {
		transport.send(Packet::Data(data)).await;
	}

	fn recv() -> Result<[u8; MTU - 1], RecvError> {
		loop {
			match transport.recv().await {
				Packet::Assign(id, name) => registry.assign(id, name),
				Packet::OpenChannel(id, data) => error!("attempted to open channel while one was already open"),
				Packet::Data(data) => return Ok(data),
				Packet::Reset => {
					registry.unassign_all();
					transport.send(Packet::Reset);
					return Err(RecvError::ConnectionReset);
				}
			}
		}
	}
}

struct Network(&Transport, &mut ProtocolRegistry);

impl Network {
	fn open_channel(protocol) -> Protocol::ChannelHandle {

	}
}
```

Inside macro:
- Protocol registry mutex ensures that either a incoming channel is open or someone is transmitting
```rust
let registry = Mutex::new(build_protocol_registry!($protocols));
//	=> ProtocolRegistry::new([protocol1::NAME, protocol2::NAME]);

let net = NetworkStuff::new(transport, registry);

let open_channel = |id, data, net| {
	let protocol_name = registry.resolve(id);

	let mut registry_lock = registry.lock().await;
	let transport = TransportWrapper(&transport, &mut registry_lock, &reset_requested);

	$(
		if $protocol.matches(protocol_name) {
			protocol.handle(data, transport).await;
		}
	)*
};

let receive_task = async {
	loop {
		match transport.recv().await as Packet {
			Packet::Reset => reset_requested = true,
			Packet::Assign(id, name) => registry.assign(id, name),
			Packet::OpenChannel(id, data) => open_channel(id, data, &mut net),
			Packet::Data(_) => error!("data received while no channel was open"),
		}
	}
};

let network = Network::new(&transport, &registry);

(network, receive_task)
```

```rust
enum ChannelDirection {
    Inbound,
    Outbound,
}

enum NetworkState {
    Idle,
    ChannelOpen(ChannelDirection, core::task::Waker),
}

struct Network {
	registry: ProtocolRegistry<ProtocolCount>,
	transport: Transport<MTU>,

	// TODO Can be extracted into a struct
	buffer: UnsafeCell<[u8; MTU]>,
	buffer_empty: AtomicBool,
	buffer_waker: UnsafeCell<Option<Waker>>,

	// TODO Can be extracted into a struct
	channel_open: AtomicBool,
	channel_waker: UnsafeCell<Option<Waker>>,
}
```

open = false
waker = None
id = unspecified

=> Task starts
```rust
loop {
	let channel_opening = poll_fn(|| {
		if open {
			waker = None
			Poll::Ready
		} else {
			waker = ...
			Poll::Pending
		}
	});

	channel_opening.await;
	channel(id).await;
	open = false;
	waker = None;
}
```

=> In recv task
```rust
loop {
	match packet {
		Data(data) => ...,
		Open(target_id, data) => {
			if open.compare_exchange(expect: false, write: true).is_ok() {
				id = target_id;
				netbuf.store(data);
				waker.take().wake();
			} else {
				error!("attempted to open channel while it was already open");
			}
		}
	}
}
```

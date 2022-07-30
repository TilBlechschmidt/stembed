# List of messages the runtime supports

## Direct flash access

Assumed information:
- Flash size (=16M)
- Sector size (=4096)
- Write alignment (4)
- Partitioning knowledge (???)

### ReadFlash

Reads a region of memory from flash. Peripheral will emit multiple FlashContent messages that cover the requested range.
Additional trailing bytes may be transmitted to fill the remaining space in the last content message.

```
start = u24
end = u24
```

#### FlashContent

Chunk of data that has been read.

```
offset = u24
data = [u8; 60]
```

### WriteFlash

Writes a region of memory to flash without erasing, requires proper alignment.

```
offset = u24
data = [u8; 60]
```

#### FlashWritten

Acknowledges a write message and confirms that the data has been written.

```
offset = u24
data = [u8; 60]
```

### EraseFlash

Erases a region of flash.

```
startSector = u16
endSector = u16
```

#### FlashErased

Confirms that the given sectors have been erased.

```
startSector = u16
endSector = u16
```

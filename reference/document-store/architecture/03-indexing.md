# Indexing - PackedTableTools Packing Format

## Overview

`PackedTableTools.rs` implements a binary packing format for encoding table column data into compact byte representations. This format is used to efficiently store structured data within the document store's pages.

## Packing Format

The packing format encodes different column types into a compact binary representation:

### Boolean Columns

Boolean values are packed as a **bit string** - each boolean occupies a single bit rather than a full byte:

```text
Boolean column: 1 bit per value
  true  → 1
  false → 0

Example: [true, false, true, true] → 0b1011 (packed into partial byte)
```

### Integer Columns

Integer columns use a **size code** prefix (2 bits) to indicate the byte width of the integer value:

```text
Size Code | Bytes | Range
----------|-------|---------------------------
   00     |   1   | 0 to 255
   01     |   2   | 0 to 65,535
   10     |   4   | 0 to 4,294,967,295
   11     |   8   | 0 to 18,446,744,073,709,551,615
```

The size code is stored before the integer data, allowing the reader to know how many bytes to consume:

```text
Packed integer (value = 300):
  Size code: 01 (2 bytes needed)
  Data: [0x01, 0x2C]
  Total: 2 bits (size code) + 2 bytes (data)
```

### Text Columns

Text columns store the **length** of the string followed by the string data:

```text
Text column format:
  [length: variable int] [data: UTF-8 bytes]

Example: "hello"
  Length: 5
  Data: [0x68, 0x65, 0x6C, 0x6C, 0x6F]
```

## Data Types Summary

| Type | Encoding | Size |
|------|----------|------|
| Bool | Bit string | 1 bit per value |
| Int (small) | Size code `00` + 1 byte | 10 bits |
| Int (medium) | Size code `01` + 2 bytes | 18 bits |
| Int (large) | Size code `10` + 4 bytes | 34 bits |
| Int (huge) | Size code `11` + 8 bytes | 66 bits |
| Text | Length prefix + UTF-8 bytes | Variable |

## Packing Process

```text
Table Row → PackedTableTools.rs → Packed Binary

For each column in the row:
  1. Determine column type (bool, int, text)
  2. Apply type-specific encoding:
     - Bool: append single bit
     - Int: determine size code, append code + bytes
     - Text: append length + UTF-8 data
  3. Concatenate all encoded columns
  4. Store packed binary in page body
```

## Unpacking Process

```text
Packed Binary → PackedTableTools.rs → Table Row

For each column (using schema):
  1. Read column type from schema
  2. Apply type-specific decoding:
     - Bool: read 1 bit
     - Int: read 2-bit size code, then read N bytes
     - Text: read length, then read N bytes
  3. Reconstruct column value
```

## Design Rationale

- **Space efficiency** - Booleans as bits save 7/8 of storage compared to byte representation
- **Variable-width integers** - Small integers (common in practice) use fewer bytes
- **Self-describing integers** - Size codes eliminate the need for external schema for integer widths
- **UTF-8 text** - Standard encoding preserves Unicode compatibility

## Next Steps

- **[Linear Hashing](./02-linear-hashing.md)** - How packed records are stored in buckets
- **[System Overview](./01-system-overview.md)** - Where indexing fits in the architecture

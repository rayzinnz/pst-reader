# oveview
pst
Personal Storage Table
ost
Off-line Storage Table
https://en.wikipedia.org/wiki/Personal_Storage_Table
https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/141923d5-15ab-4ef1-a524-6dce75aae546


# Magic bytes
4 bytes
21 42 44 4E
!BDN

# Formats and size
The file is structured as a B-tree with 512 byte nodes and leaves
All PST files begin with the four-byte magic string "!BDN", a four-byte CRC number, and a two-byte magic string of "SM"


# Structure

Each PST file represents a message store that contains an arbitrary hierarchy of
  - Folder objects,
    - which contains Message objects,
	  - which can contain Attachment objects.

The PST file structures are logically arranged in three layers:
 - the NDB (Node Database) layer,
 - the LTP (Lists, Tables, and Properties) layer, (Heap, BTree, Property bags, Tables)
 - and the Messaging layer. (Message Store, Folders, Messages, Attachment)


## Node Database (NDB) Layer

From an implementation standpoint, the NDB layer consists of the header, file allocation information, blocks, nodes, and two BTrees: the Node BTree (NBT) and the Block BTree (BBT).

Each node reference is represented using a set of four properties that includes its
  - NID, (The parent NID is an optimization for the higher layers and has no meaning for the NDB Layer.)
  - parent NID, (The parent NID is an optimization for the higher layers and has no meaning for the NDB Layer.)
  - data BID, (The data BID points to the block that contains the data associated with the node)
  - and subnode BID (the subnode BID points to the block that contains references to subnodes of this node)


# Physical Organization of the PST File Format

## Header

https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/fc4c74cb-ec8a-42ff-a2c5-2d6e3fa16394

https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/c9876f5a-664b-46a3-9887-ba63f113abf5

Offset	Size	Description
0x0000	4	Magic (!BDN → 0x2142444E)
0x0004	4	CRC checksum of header
0x0008	4	Client magic / consistency marker
0x000C	2	Version
0x000E	2	File format type
0x0010	...	Various pointers and metadata

🔹 0x30 – 0x47 (24 bytes total)
Root Structure (ROOT)
It contains entry points into the PST’s B-tree system.

struct Root {
    dwReserved: u32,
    ibFileEof: u64,   // Logical EOF
    ibAMapLast: u64,  // Last allocation map
    cbAMapFree: u64,  // Free space in AMaps
    cbPMapFree: u64,  // Free space in PMaps
    BREFNBT: BREF,    // Node B-tree root
    BREFBBT: BREF,    // Block B-tree root
    fAMapValid: u8,
    bReserved: [u8; 3],
    wReserved: u32,
}

struct BREF {
    bid: u64,
    ib: u64,
}

HEADER
  ├── ROOT
  │     ├── BBT root  → physical blocks
  │     └── NBT root  → logical objects
  │
  ├── Allocation maps
  │
  └── ID generators

# Root

The BREF is a record that maps a BID to its absolute file offset location. = (bid, offset)

BREFNBT (Unicode: 16 bytes; ANSI: 8 bytes): A BREF structure (section 2.2.2.4) that references the root page of the Node BTree (NBT).

BREFBBT (Unicode: 16 bytes; ANSI: 8 bytes): A BREF structure that references the root page of the Block BTree (BBT).

## BREF BBT
root page of the Block BTree (BBT)

# Binary dictionary structure

## Outline

The dictionary is a prefix tree. Each node can either be a leaf, a branch, or both.

- Prefix size in each node defined at the beginning
	- Value is dependent on the number of children each prefix length would produce
	- Threshold value is determined during compilation and directly impacts lookup performance
		- Too many children and the linear search takes too long
		- Too few children and many reads are required
- Children are referenced through a key-value mapping
	- Number of children is stored at the beginning of node
	- Prefix-keys are stored in an array
		- Keys are sorted in ascending order
		- This array is searched linearly for a match
		- Future optimizations might include binary or heuristics based search
			- This could allow for bigger prefix sizes while keeping the search cost equal
	- Secondary array contains pointers to corresponding nodes
		- This array is accessed directly through the index discovered from the first array
- Leaf data is stored at the end of each node if applicable
	- In the future, deduplication may be employed by storing it at the end of a node by default but referencing another nodes leaf data instead if the exact same data has already been written before
	- This would trade file size over access speed but may reduce the number of stored translations in half
		- Plover dict contains 70352 unique translations in 147424 entries

## Data structure

- Everything is stored as big-endian
- General structure
	- DictInfo
	- Translation data
	- Root node
	- Child nodes

### DictInfo

- Located at the very beginning of the dictionary
- Translation data length (u32)

### Translation data

CommandLists are FormatterCommands stored back-to-back, terminated with a marker byte set to `0xFF`. The binary structure is described below:

```
00______ Write
  ******	String length, followed by UTF-8 byte sequence
01______ ChangeCapitalization
  000___	Unchanged
  001___	Lowercase
  010___	Capitalize
  011___	Uppercase
  100___	LowerThenCapitalize
  101___	LowercaseNext
  110___	CapitalizeNext
  111___	UppercaseNext
10______ ChangeAttachment
  00____ 	Delimited
  01____ 	Glue
  10____ 	Next
  11____ 	Always
110_____ ResetFormatting
11111111 End of CommandList
```

### Node

- Number of children (u8)
	- Value offset by +1 as empty nodes are not permitted
- Prefix length (u8)
- Translation pointer (u24)
	- Can be zero to indicate no translation being present
- Key array
	- n-bit prefixes
- Value array
	- 24-bit pointers
	- Can either point to a child node or translation data

## Compilation

- Happens in three steps
	1. Tree structure is built
	2. Data is serialized into a buffer
	3. Buffer is written to disk

## Traversal

When traversing, a situation might arrive where the current node contains leaf data but there is a child matching the next byte in the pipeline. Following it gets us to a node with two children, none of which match the next byte. However, the intersection node does not contain a translation. Thus we are "stuck" and would have to backtrack.

To prevent additional fetches, the latest node we encountered that had a translation will be stored in the form of its translation pointer and the depth. This way, we can jump directly to the translation and do not have to walk back up the tree.

Additionally, we have to make sure to only ever end on a stroke boundary and not a sub-stroke boundary since the tree only operates on bytes not strokes.

Temporary midnight notes:

- Use fixed-size, on-disk array to cover first two bytes of first stroke
- Use dynamically sized nodes for third byte of first stroke

=> 2 fetches (almost no block locality sadly) for first stroke
=> Dynamically sized array for third byte may contain inlined entries & pointers for child-nodes

Alternative: Use fixed-size array for first byte only and cache it in-memory. Would also
yield 2 fetches (one for the second, one for the third byte) while requires more system memory.


Random ideas:
- Third byte fetch could be used to fetch length-list for fourth layer
    - Indexing an array of variably sized data only works if you know how long each element is
    - This information could be embedded into the data fetched by the second fetch 


HashMap for first stroke maybe?
=> Buckets would be too large to fit into memory (~50k entries)
=> One lookup for bucket + another for variably sized data
=> Not better than fixed-size array while using/wasting more storage and being harder to implement

# Reading H-L

## Array use-case

1. Calculate array index for first two bytes
2. Read location of vector for third byte (fetch #1)
3. ISSUE: Vector contains too many elements to resonably search through (fetch #2)

## HashMap use-case

1. Calculate hash-bucket for first stroke
2. Fetch bucket and following elements (fetch #1)
    - Contains stroke (three-byte) and data location
    - If stroke does not match, do linear-probing for cache locality
3. Fetch data from target location (fetch #2)
    - Starts with outline content (if this stroke is in the dictionary)
    - Continues on with sub-stroke list (unknown datastructure so far)
4. Extract stroke data from fetched data

# Reading PO/TAE/TOE

1. 
## Word definitions
- "Key"
    - Phoneme or "unit" of a stenography system (e.g. S-, *, or -Z)
    - Could also represent an arbitrary action (e.g. FN1 used exclusively for executing system commands)
- "Key identifier"
    - Human readable and unique string identifying a key
    - Used to define environments
- "Hardware key"
    - Physical input on a device
    - No relation to stenography system
        - The hardware could still be built around a certain system and intended to be used with some specific mapping
    - Will be transferred to engine as a list of pressed keys (much like USB HID)
        - Keymap used to translates hardware keys into "environment" keys
        - Embedded engine could optimize process by scanning hardware keys in environment order

## Stroke
- Represented by 32-bit unsigned integers
- Strokes are environment-sensitive
    - Human readable strokes are relative to the system they are defined in (e.g. "HEP" only makes sense in the English steno system)
    - Binary strokes are only valid in the environment for which they were compiled

## Environments
- Defines the mapping between keys and bits
    - Mapping is usually defined by combining a set of dictionaries
        - Example: You have a word dictionary containing the steno keys STKPWHRAO*EUFRPBLGTSDZ and a command dictionary using e.g. F1,F2,F3
        - Combining the two results in a mapping where the first 22 bits are for the steno keys and the next 3 bits are for function keys
        - Leftover bits are just ignored
        - Attempting to build a mapping for a set of dictionaries that does not fit into 32 bit is considered a critical error
- When compiling dictionaries the above mapping is used to convert human readable key sequences into bit patterns
    - Without knowledge about the environment used to compile a dictionary/stroke it is impossible to use it
- Hardware keys have to be mapped to keys in order to be able to match against compiled strokes
    - This can be exploited when scanning the key-matrix by changing the order in which the matrix is scanned




- Left keys
    - STKPWHR
- Middle keys
    - AO*EU
- Right keys
    - FRPBLGTSDZ
- Extras
    - FN1,FN2
- Handle numbers by pre-processing the string in the Plover dict parser (replace number by key, add # at the beginning)

KA*PD|FN1,FN2


# Input mapping

There shall be three layers, each serving a specific purpose.

1. Raw key inputs
    - Array of keys, simply enumerated
    - May optionally provide friendly names
2. Stroke inputs
    - Assigns stenography keys to each of the enumerated raw keys
    - Defined by the user, bound to hardware/protocol
        - Serial protocols like GeminiPR will be treated as raw inputs
3. Dictionary
    - Lossy conversion from input strokes to dictionary strokes
    - Set of input stroke keys does not necessarily have to match dictionary
    - To allow use of partially matching dictionaries, automatic conversion takes place
    - Iterates over keys in dictionary stroke context, runs lookup on input stroke
        - Keys with the same names will be matched
        - Leftover keys present in the input stroke will be discarded

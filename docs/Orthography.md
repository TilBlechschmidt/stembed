- Rules derived from Plover
    - There, those are RegExp strings but they can be reduced into a simple algorithm
    - Rationale is that parsing, compiling, and executing regular expressions is hard on embedded platforms (and is not necessary in this case)
- American word list compiled into bloom filters
    - One filter for each priority level
- Orthography rules should be tied to or referenced by dictionary
    - Future thought: Dictionary can influence orthography engine

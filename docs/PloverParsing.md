```
cat "/Users/tibl/Library/Application Support/plover/main.json"
| grep '{'
| grep -v '{[.,!?:;]}'
| grep -v '{\^'
| grep -v '\^}'
| grep -v '{&'
| grep -v '{-|}'
| grep -v '{>}'
```

0. Parse JSON strings
    - Escape sequences and stuff :D
1. Consume everything outside of curly brackets as `Write` commands
2. Consume content of brackets using special handling
    - Treat escaped tokens as text
    - Handle tokens as special stuff
        - `^`   AttachmentMode::Next
            - If prefixed/suffixed by text inside brackets, use orthography-aware attach (to be implemented)
        - `&`   AttachmentMode::Glue
        - `-|`  CapitalizationMode::CapitalizeNext
        - `>`   CapitalizationMode::LowercaseNext
        - `<`   CapitalizationMode::UppercaseNext
        - `{}`  ResetFormatting (empty brackets)
        - [.,!?:;]
            - AttachmentMode::Next
            - Write(punctuation)
            - CapitalizationMode::Capitalize (not for `,`)
        - Otherwise:
            - Write(chars)

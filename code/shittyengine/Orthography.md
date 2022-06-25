# Orthographic attachment logic

<!-- SCRATCH THAT :D
1. Build data structure for rules below
2. Build tool which assigns indices to the various suffixes matched (i.e. `[ae]c` => `{ 0: 'ac   1: 'ec' }`)
3. Match attachments and suffixes against rules, store the appended text length in the regular undo info, additionally store attachment rule and index
4. When undoing, backspace like usual, then lookup the rule, generate the unmodified suffix, and attach it again (according to the post-pop state)
    - This will break when the suffix is composed of multiple parts and e.g. the capitalization changed in between
    - It is assumed that this is an edge case and is very unlikely to happen in a real-world setting
    - The limitation can be reflected in the dictionary language by binding the orthographic-attach command to a write command

- Use 16-bit (or larger) numbers for indexing rules & variants but compress the UndoInfo buffer by storing unequally sized values
    - Orthographic attach is rare in comparison to the total command volume
    - Optimization for the future. For now, just use 16-bit numbers which are known to work
- Store the rules in flash, load a rule index table into memory, lookup the rules on-demand
    - Would require dynamic memory/static buffer, evaluate if this is worth it over just iterating all rules from flash when attaching/undoing
    - Maybe bake the rule(s) to use during an orthographic attach operation into the dictionary so not all rules have to be evaluated
    - Optimization which can be skipped for now by just iterating all rules, we have plenty of performance on the nRF52
-->

## Word definition

write + en = tten
   $$   %%   @@@@

$$ = Suffix       (non-captured parts of the suffix are called trailing suffixes)
%% = Attachment
@@ = Replacement

## General notes

- When compiling rules into binary format, build a list of trailing suffixes (could be deduped)
- When orthographically attaching, store the index of the trailing suffix that was deleted
- When undoing, backspace the attached suffix, lookup the trailing suffix and put it back

## Binary rule format

- Attachment section (repeated)
    - SuffixOffset
    - MatchCommand (repeated)
- Delimiter
    - When encountered, no rule matched and thus a normal attachment occurs
- Suffix section (repeated)
    - MatchCommand (repeated, reverse order)
    - TrailingSuffix length                     <- used for determining how many characters to delete when attaching
    - TrailingSuffix SuffixOffset               <- used to figure out what to put back when undoing
    - Replacement (string)
- TrailingSuffix section (deduplicated)
    - null-terminated string (repeated)

## Implementation details

- Two buffers required, both are string slices referencing a potentially larger buffer (make that a common helper struct)
    - Suffix of last output
        - When applying commands, stores the latest `n` characters written out
        - When undoing, it is cleared and set to None
        - When attaching and the suffix is not known, fetch the last committed outline's commands and reconstruct it
            - Implicates that commands are applied FIRST and only THEN will the outline be committed to the state
            - Reconstruction will happen by returning an enum that asks for the last applied commands with a "commit" method (similar to core state)
    - Replacement buffer
        - Could be repurposed for loading TrailingSuffixes when undoing an orthographic attach
        - Data is loaded into this buffer once rule is matched
        - When generating output commands this is chained with the captured output
- Two capture ranges required
    - Captured content of the attachment
    - Captured content of the suffix (which should be prepended to the replacement)

- MatchCommand
    - ChangeCapture(bool)
    - Exactly(String)
    - OneOf([String])
    - NoneOf([String])
    - Anything { optional: bool }

## Rules

```python
# \-    => Output all capture groups in order without any additional characters in between
# \1    => Output capture group 1
# [!abc]  => Any character but `a`, `b`, or `c`

r'm														tor(y|ily)                                                        mator',
r'se													ar(y|ies)                                                         sor',
r'ie													ing                                                               ying',
r'te													en                                                                tten',
r'ic													(ical|ically)                                                     \-',
r'y														(ial|ially)                                                       \-',
r'i														if(y|ying|ied|ies|ication|ications)                               if',
r'ology													ic(al|ally)                                                       ologic',
r'ry													ica(l|lly|lity)                                                   rica',
r'ry													ity                                                               rity',
r'l														ity                                                               lity',
r'rm													tiv(e|ity|ities)                                                  rmativ',
r'e														tiv(e|ity|ities)                                                  ativ',
r'y														iz(e|es|ing|ed|er|ers|ation|ations|able|ability)                  iz',
r'y														is(e|es|ing|ed|er|ers|ation|ations|able|ability)                  is',
r'al													iz(e|ed|es|ing|er|ers|ation|ations|m|ms|able|ability|abilities)   aliz',
r'al													is(e|ed|es|ing|er|ers|ation|ations|m|ms|able|ability|abilities)   alis',
r'ar													iz(e|ed|es|ing|er|ers|ation|ations|m|ms)                          ariz',
r'ar													is(e|ed|es|ing|er|ers|ation|ations|m|ms)                          aris',
r'al													olog(y|ist|ists|ical|ically)                                      olog',

r'([aeiou]c)                                            ly                                                                ally',
r'([aeioubmnp])le                                       ly                                                                ly',
r'([naeiou])t                                           cy                                                                cy',
r'([naeiou])te                                          cy                                                                cy',
r'([cdfghlmnpr])y                                       ist                                                               ist',
r'([!aeiouy])y                                          s                                                                 ies',
r'([!aeiouy])y                                          ([!iy].*)                                                         i',
r'([!aeioy])e                                           ([aeiouy].*)                                                      \-',
r'([ae])                                                e(n|ns)                                                           \-',
r'([l])                                                 is(t|ts)                                                          is',
r'([lmnty])                                             iz(e|es|ing|ed|er|ers|ation|ations|m|ms|able|ability|abilities)   iz',
r'([lmnty])                                             is(e|es|ing|ed|er|ers|ation|ations|m|ms|able|ability|abilities)   is',

r'(t)e                                                  ry|ary                                                            ory',
r'(e)e                                                  (e.+)                                                             \-',
r'(s|sh|x|z|zh)                                         s                                                                 es',
# This rule prevents an edge case in the rule below it (e.g. monarches -> monarchs)
r'([gin]arch)                                           (s)                                                               \-',
r'(oa|ea|i|ee|oo|au|ou|l|n|t|r)(ch)                     s                                                                 es',
# This rule prevents an edge case in the consonant doubling rule below it (e.g. similarrish -> similarish)
r'(ar|er|or)                                            (ish)                                                             \-',

# The rule compiler will replace the replacement with \- and instead "enable" duplication of the 3rd capture group's content
# Using capture groups in the output is kind of a special case for now as they can only be used in-order and only used at most once
r'([!aeioqu]|qu)([aeiou])([bcdfgklmnprtvz])             ([aeiouy].*)                                                      \3'
```

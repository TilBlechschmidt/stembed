# Engine & Firmware notes

- Efficient output
	- Have a small ringbuf (couple of characters)
	- When writing, append to the ringbuf
	- When undoing, postpone the backspace keypress
		- Move a pointer in the ringpuf so the backspaced characters are kept
		- When the next output is received, continously move the pointer fwd and compare
			- If it matches, do nothing.
			- If it diverges, backspace everything in the ringbuf past that point and then type it out
	- A `flush` function is required for e.g. the undo command where an immediate backspace is desireable
	- In case the ringbuf bottom is hit, simply fall back to the original method of backspacing and then rewriting

## Keypress IR

```rust
#[derive(Copy, Clone)]
enum KeyPosition {
	Left(KeyColumn, KeyRow),
	Right(KeyColumn, KeyRow),
}

#[derive(Copy, Clone)]
enum KeyRow {
	Above = 0,
	Top = 1,
	Bottom = 2,
	Below = 3,
}

#[derive(Copy, Clone)]
enum KeyColumn {
	Pinky = 0,
	Ring = 1,
	Middle = 2,
	Index = 3,
	Thumb = 4,

	ExtraLeading = 5,
	ExtraTrailing = 6,
}
```

- Make a const keymap (using a macro which allows both `None` and `KeyPosition` by invoking .into())
- Build a struct which holds the intermediate representation state
	- Internally uses a 32-bit integer, wastes 4-bit which is fine
	- Provides nice APIs like iterating, addition, subtraction etc.
- When iterating the matrix, build a state by selectively setting the ones which are `Some(_)` in the keymap
	- Simply `continue` if the entry is `None` as to not query the physical key state and save time
- Implement `From<IRKeyState>` for the stroke for now, allows for a more flexible approach in the future
	- Either way, the runtime takes the IR and nothing else; that is the boundary :)
	- This way the actual stroke repr can be changed later on, even dynamically

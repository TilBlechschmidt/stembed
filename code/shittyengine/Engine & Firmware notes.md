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

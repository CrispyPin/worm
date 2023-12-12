# Stupid Worm Languag
- program space is an arbitrary sized grid
- the worm starts where you place the worm head @
- as it passes over commands, they get moved to the back of the worm
- values get pushed to stack (eaten) when passed over, worm body length increases
- the program gets rearranged every time the worm executes it

## commands
```
+- pop 2 values *, push sum/difference (uses the order they are popped, so `0 -` negates the top of the stack)
><^v change direction
0..9 push number to stack
/\ pop stack, reflect to the side if not zero
? reads one byte of input
= duplicate top of stack
! pop and write output as ascii char
" pop and write output as number
```
* pop when stack is empty yields zero

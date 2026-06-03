# Jeremiah 

Jeremiah is a light weight, powerful chess bot written entirely in rust.

The whole binary is around 3mb and targets x86_64. He thinks to a depth of 3 very quickly. any higher and it gets quite slow, especially with lots of pieces on the board. this is probably becuase i bundled too many features into the board, leading to costly clones. i will definetley add a move undo system to speed things up

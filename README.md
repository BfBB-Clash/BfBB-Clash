# BfBB Clash

BfBB Clash is a custom gamemode for SpongeBob SquarePants: Battle for Bikini Bottom where up to six players compete for the same set spatulas within the game world. Once one player obtains a spatula, that spatula is removed from the game world for every other player.

## How to Play

- Have dolphin open with the game running.
- Open the client and join a lobby.
- With all players on the Main Menu the game will start automatically when the host starts a new game.

## Building From Source

BfBB Clash is organized as a Cargo workspace with three separate crates.

With Cargo from the project root:

- `$ cargo build` Build all packages
- `$ cargo build -p <package>` where `<package>` is one of:
  - `clash` - the client
  - `clash-server` - the server
  - `clash-lib` - the shared library

#### License

<sup>
Licensed under <a href="LICENSE-APACHE">Apache License, Version
2.0</a>
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in BfBB Clash by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.
</sub>

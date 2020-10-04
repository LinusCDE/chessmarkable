# Chess

[![rm1](https://img.shields.io/badge/rM1-supported-green)](https://remarkable.com/store/remarkable)
[![rm2](https://img.shields.io/badge/rM2-unknown-yellow)](https://remarkable.com/store/remarkable-2)
[![launchers](https://img.shields.io/badge/Launchers-supported-green)](https://github.com/reHackable/awesome-reMarkable#launchers)

<!-- [![opkg](https://img.shields.io/badge/OPKG-chess-blue)](https://github.com/toltec-dev/toltec) -->
<!-- [![Mentioned in Awesome reMarkable](https://awesome.re/mentioned-badge.svg)](https://github.com/reHackable/awesome-reMarkable) -->

A chess game for the reMarkable tablet writting using the [pleco](https://crates.io/crates/pleco) chess library which is a port of [Stockfish](https://stockfishchess.org/)

<img src="https://transfer.cosmos-ink.net/122bBC/chess_main_menu.jpg" width="30%">&nbsp;<img src="https://transfer.cosmos-ink.net/QvXAm/chess_initial_board.jpg" width="30%">&nbsp;<img src="https://transfer.cosmos-ink.net/sFtOb/chess_board_castle.jpg" width="30%">

## Controlling

A chess piece can be moved in two ways:

1. Clicking it once and clicking the spot it's supposed to
2. Clicking it and moving the finger onto the square to move it there on release

The second method has the advantage that it doesn't highlight the chess piece or shows the possible moves.

## Installation

### Prebuilt binary/program

- Go the the [releases page](https://github.com/LinusCDE/chess/releases)
- Get the newest released "chess" file and copy it onto your remarkable, using e.g. FileZilla, WinSCP or scp.
- SSH into your remarkable and mark the file as executable with `chmod +x chess`
- Stop xochitl (the interface) with `systemctl stop xochitl`
- Start the game with `./chess`
- After you're done, restart xochitl with `systemctl start xochitl`

### Compiling

- Make sure to have rustup and a current toolchain (nightly might be needed)
- Install the [oecore toolchain](https://remarkable.engineering/).
  - If you're not using linux, you might want to adjust the path in `.cargo/config`
- Compile it with `cargo build --release`. It should automatically cross-compile.

## Todo

- Proper own icon(s)
- Check whether the difficulties are good (let me please know in the issues if not)
- Some more information for the user on invalid moves
- Fix potential errors that are currently not checked and instead can kill the game

## Credit

- The [pleco](https://crates.io/crates/pleco) library is used as the engine, checking valid moves and providing the bots
- The chess pices are from [pixabay here](https://pixabay.com/vectors/chess-pieces-set-symbols-game-26774/)

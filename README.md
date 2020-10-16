# chessMarkable

[![rm1](https://img.shields.io/badge/rM1-supported-green)](https://remarkable.com/store/remarkable)
[![rm2](https://img.shields.io/badge/rM2-unsupported-red)](https://remarkable.com/store/remarkable-2)
[![opkg](https://img.shields.io/badge/OPKG-chessmarkable-blue)](https://github.com/toltec-dev/toltec)
[![launchers](https://img.shields.io/badge/Launchers-supported-green)](https://github.com/reHackable/awesome-reMarkable#launchers)
[![Mentioned in Awesome reMarkable](https://awesome.re/mentioned-badge.svg)](https://github.com/reHackable/awesome-reMarkable)

A chess game for the reMarkable tablet writting using the [pleco](https://crates.io/crates/pleco) chess library which is a port of [Stockfish].(https://stockfishchess.org/)

<img src="https://transfer.cosmos-ink.net/lH2sG/scr1.jpg" width="30%">&nbsp;<img src="https://transfer.cosmos-ink.net/pGFAe/2.jpg" width="30%">&nbsp;<img src="https://transfer.cosmos-ink.net/LZ9QT/3.jpg" width="30%">

## Controlling

A chess piece can be moved in two ways:

1. Clicking it once and clicking the spot it's supposed to
2. Clicking it and moving the finger onto the square to move it there on release

The second method has the advantage that it doesn't highlight the chess piece or shows the possible moves.

## FEN

When running the Game with the enviroment variable `RUST_LOG` set to `debug`, the [FEN](https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation) of a board will be output on each move. This is useful for debugging but also for manually saving a game state or resuming it elsewhere since this notation should be compatible with other chess programs/engines.

The reverse can also be done. Running chessmarkable with `-i` set to a FEN, will start any game with the supplied board. That way a game can be resumed or debugging a edge case speed up.

## Installation

### Prebuilt binary/program

- Go the the [releases page](https://github.com/LinusCDE/chessmarkable/releases)
- Get the newest released binary file (the one without any extension) and copy it onto your remarkable, using e.g. FileZilla, WinSCP or scp.
- SSH into your remarkable and mark the file as executable with `chmod +x chess`
- Stop xochitl (the interface) with `systemctl stop xochitl`
- Start the game with `./chessmarkable` (or whatever the binary is called now)
- After you're done, restart xochitl with `systemctl start xochitl`

### Compiling

- Make sure to have rustup and a current toolchain (nightly might be needed)
- Install the [oecore toolchain](https://remarkable.engineering/).
  - If you're not using linux, you might want to adjust the path in `.cargo/config`
- Compile it with `cargo build --release`. It should automatically cross-compile.

## Todo

- Proper own icon(s)
- Clean the code

## reMarkable 2 support

As of now, the new framebuffer is not yet figured out. As soon as that happens and libremarkable gets updated, I can fix this sw to work on the rM 2.

## Credit

- The [pleco](https://crates.io/crates/pleco) library is used as the engine, checking valid moves and providing the bots
- The chess pices are from [pixabay here](https://pixabay.com/vectors/chess-pieces-set-symbols-game-26774/)

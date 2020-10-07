use crate::chess_game::ChessGame;
pub use crate::chess_game::{ChessOutcome, Player, SQ};
use crate::CLI_OPTS;
use anyhow::{Context, Result};
use pleco::tools::Searcher;
use std::thread;
use std::time::{Duration, SystemTime};
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task;

#[derive(Clone, Debug, PartialEq)]
pub enum ChessRequest {
    CurrentBoard,
    CurrentTotalMoves,
    CurrentOutcome,
    MovePiece { source: SQ, destination: SQ },
    Abort { message: String },
}

impl ChessRequest {
    /// Is a spectator allowed to send this request
    pub fn available_to_spectator(&self) -> bool {
        match self {
            ChessRequest::CurrentBoard | ChessRequest::CurrentTotalMoves => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChessUpdate {
    /// Usually the Response to `ChessRequest::CurrentBoard`.
    /// But may be sent at other points to synchronize state as well.
    Board {
        fen: String,
    },
    PlayerMovedAPiece {
        player: Player,
        moved_piece_source: SQ,
        moved_piece_destination: SQ,
    },
    /// Signal that a new player is now playing. The boar is the
    /// most recent one which can also be retreived by requesting a
    /// `ChessUpdate::Board` response.
    PlayerSwitch {
        player: Player,
        fen: String,
    },
    MovePieceFailed {
        // Response to `ChessRequest::MovePiece` when the action failed
        message: String,
        fen: String,
    },
    Outcome {
        outcome: Option<ChessOutcome>,
    },
    PossibleMoves {
        possible_moves: Vec<(SQ /* From */, SQ /* To */)>,
    },
    /// Something went wrong and the server wants to tell you about it
    GenericErrorResponse {
        message: String,
    },
}

pub async fn create_game(
    white: (Sender<ChessUpdate>, Receiver<ChessRequest>),
    black: (Sender<ChessUpdate>, Receiver<ChessRequest>),
    spectators: (Sender<ChessUpdate>, Receiver<ChessRequest>),
    starting_fen: Option<String>,
) -> Result<()> {
    let mut game = if let Some(ref fen) = starting_fen {
        ChessGame::from_fen(fen)?
    } else {
        ChessGame::default()
    };

    let (mut white_tx, mut white_rx) = white;
    let (mut black_tx, mut black_rx) = black;
    let (mut spectators_tx, mut spectators_rx) = spectators;

    let (combined_tx, mut combined_rx) = channel::<(Option<Player>, ChessRequest)>(1024);

    macro_rules! send_to_everyone {
        ($msg: expr) => {
            white_tx.send($msg.clone()).await.ok();
            black_tx.send($msg.clone()).await.ok();
            spectators_tx.send($msg).await.ok();
        };
    }

    // Redirect all rx streams into `combined_rx` with a supplied player for cleaner handling
    // TODO: Shorten/cleanup code
    let mut combined_white_tx = combined_tx.clone();
    task::spawn(async move {
        let player = Some(Player::White);
        loop {
            let update = match white_rx.next().await {
                Some(update) => update,
                None => {
                    combined_white_tx
                        .send((
                            player,
                            ChessRequest::Abort {
                                message: "[Internal] Connection lost".to_owned(),
                            },
                        ))
                        .await
                        .ok();
                    return;
                }
            };
            if let Err(_) = combined_white_tx.send((player, update)).await {
                return;
            }
        }
    });
    let mut combined_black_tx = combined_tx.clone();
    task::spawn(async move {
        loop {
            let update = match black_rx.next().await {
                Some(update) => update,
                None => return,
            };
            if let Err(_) = combined_black_tx.send((Some(Player::Black), update)).await {
                return;
            }
        }
    });
    let mut combined_spectators_tx = combined_tx;
    task::spawn(async move {
        loop {
            let update = match spectators_rx.next().await {
                Some(update) => update,
                None => return,
            };
            if let Err(_) = combined_spectators_tx.send((None, update)).await {
                return;
            }
        }
    });

    // Start (if not using a FEN then white starts)
    send_to_everyone!(ChessUpdate::PlayerSwitch {
        player: game.turn(),
        fen: game.fen()
    });
    // Send the starting player his possible moves
    let possible_moves: Vec<_> = game
        .possible_moves()
        .iter()
        .map(|bit_move| (bit_move.get_src(), bit_move.get_dest()))
        .collect();
    match game.turn() {
        Player::White => white_tx.clone(),
        Player::Black => black_tx.clone(),
    }
    .send(ChessUpdate::PossibleMoves { possible_moves })
    .await
    .ok();

    info!("Game initialized. Handling requests...");

    // Handle inputs
    loop {
        let (sender, request): (Option<Player>, ChessRequest) = match combined_rx.next().await {
            Some(res) => res,
            None => {
                break; // No senders connected anymore
            }
        };

        if sender.is_none() && !request.available_to_spectator() {
            spectators_tx
                .send(ChessUpdate::GenericErrorResponse {
                    message: "Spectators can't send this kind of request!".to_owned(),
                })
                .await
                .ok();
            continue;
        }

        macro_rules! send_to_sender {
            ($msg: expr) => {
                match sender {
                    Some(player) => match player {
                        Player::White => white_tx.send($msg).await.ok(),
                        Player::Black => black_tx.send($msg).await.ok(),
                    },
                    None => spectators_tx.send($msg).await.ok(),
                };
            };
        }

        macro_rules! send_to_other_player {
            ($msg: expr) => {
                match sender.context("Send to the other player")? {
                    Player::White => black_tx.send($msg).await.ok(),
                    Player::Black => white_tx.send($msg).await.ok(),
                };
            };
        }

        // Requests that players as well as spectators can send
        match request {
            ChessRequest::CurrentBoard => {
                send_to_sender!(ChessUpdate::Board { fen: game.fen() });
            }
            ChessRequest::CurrentTotalMoves => {
                todo!();
            }
            ChessRequest::CurrentOutcome => {
                send_to_sender!(ChessUpdate::Outcome {
                    outcome: game.outcome()
                });
            }
            _ => {} // Should be handles for a player request
        }

        // Requests that only players can send
        let sender = sender
            .context("available_to_spectator() is probably not up to date with the handlers (message that has to be playerspecific was sent from a spectator)!!!")?;
        match request {
            ChessRequest::MovePiece {
                source,
                destination,
            } => {
                let prev_outcome = game.outcome();
                match game.move_piece(source, destination) {
                    Ok(_) => {
                        // Dunno why, but rust won't compile when using just "Ok". Error in the matrix??
                        send_to_everyone!(ChessUpdate::PlayerMovedAPiece {
                            player: sender,
                            moved_piece_source: source,
                            moved_piece_destination: destination,
                        });
                        let new_outcome = game.outcome();
                        if prev_outcome != new_outcome {
                            send_to_everyone!(ChessUpdate::Outcome {
                                outcome: new_outcome
                            });
                        }

                        if new_outcome.is_none() {
                            // Signal other player that he can make his move
                            send_to_everyone!(ChessUpdate::PlayerSwitch {
                                player: game.turn(),
                                fen: game.fen(),
                            });

                            // Send possible moves to player
                            send_to_other_player!(ChessUpdate::PossibleMoves {
                                possible_moves: game
                                    .possible_moves()
                                    .iter()
                                    .map(|bit_move| (bit_move.get_src(), bit_move.get_dest()))
                                    .collect(),
                            });
                        }
                    }
                    Err(e) => {
                        send_to_sender!(ChessUpdate::MovePieceFailed {
                            message: format!("Denied by engine: {}", e),
                            fen: game.fen(),
                        });
                    }
                };
            }
            ChessRequest::Abort { message: String } => {
                // TODO
            }
            _ => {
                bail!("available_to_spectator() is probably not up to date with the handlers (player specific handler found a unhandled entry)!!!");
            }
        };
    }

    // Potential cleanup here
    info!("Game terminated seemingly gracefully");
    Ok(())
}

pub async fn create_bot<T: Searcher>(
    me: Player,
    depth: u16,
) -> Result<(Sender<ChessUpdate>, Receiver<ChessRequest>)> {
    let (update_tx, mut update_rx) = channel::<ChessUpdate>(256);
    let (mut request_tx, request_rx) = channel::<ChessRequest>(256);

    task::spawn(async move {
        info!("Bot spawned for {}", me);
        while let Some(update) = update_rx.recv().await {
            match update {
                ChessUpdate::PlayerSwitch { player, ref fen } => {
                    if player == me {
                        let board = pleco::Board::from_fen(fen)
                            .expect("Bot failed to parse the provided fen");

                        let bit_move = task::spawn_blocking(move || {
                            let started = SystemTime::now();
                            let bit_move = T::best_move(board, depth);
                            let elapsed = started.elapsed().unwrap_or(Duration::new(0, 0));
                            let reaction_delay =
                                Duration::from_millis(CLI_OPTS.bot_reaction_delay.into());

                            if elapsed < reaction_delay {
                                thread::sleep(reaction_delay - elapsed);
                            } else {
                                info!("Bot took a long time to think: {:?}", elapsed);
                            }
                            bit_move
                        })
                        .await
                        .context("Blocking heavy calculation")
                        .unwrap();

                        request_tx
                            .send(ChessRequest::MovePiece {
                                source: bit_move.get_src(),
                                destination: bit_move.get_dest(),
                            })
                            .await
                            .expect("Bot failed to send move");
                    }
                }
                ChessUpdate::MovePieceFailed { message, .. } => {
                    error!("A move from the bot was rejected: {}", message);
                    break;
                }
                ChessUpdate::Outcome { outcome } => {
                    if outcome.is_some() {
                        info!("Bot detected that the game ended");
                        break;
                    }
                }
                _ => {}
            }
        }
        info!("Bot task has ended");
    });

    Ok((update_tx, request_rx))
}

pub fn stubbed_spectator() -> (Sender<ChessUpdate>, Receiver<ChessRequest>) {
    // Channel size doesn't matter since the channels are closed after this
    // function returns since one side of each channel gets dropped at that point.
    let (update_tx, _) = channel::<ChessUpdate>(1);
    let (_, request_rx) = channel::<ChessRequest>(1);
    (update_tx, request_rx)
}

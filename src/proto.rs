use crate::game::ChessGame;
use crate::game::Player as PlecoPlayer;
pub use crate::game::{ChessOutcome, SQ};
use crate::{Player, Square};
use anyhow::{Context, Result};
use pleco::tools::Searcher;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::{Duration, SystemTime};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task;
use chess_pgn_parser::Game;

#[derive(Clone, Debug)]
pub struct ChessConfig {
    pub starting_fen: Option<String>,
    pub can_black_undo: bool,
    pub can_white_undo: bool,
    pub allow_undo_after_loose: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChessRequest {
    CurrentBoard,
    CurrentTotalMoves,
    CurrentOutcome,
    MovePiece { source: Square, destination: Square },
    Abort { message: String },
    UndoMoves { moves: u16 },
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChessUpdate {
    /// Usually the Response to `ChessRequest::CurrentBoard`.
    /// But may be sent at other points to synchronize state as well.
    Board {
        fen: String,
    },
    PlayerMovedAPiece {
        player: Player,
        moved_piece_source: Square,
        moved_piece_destination: Square,
    },
    /// Signal that a new player is now playing. The boar is the
    /// most recent one which can also be retreived by requesting a
    /// `ChessUpdate::Board` response.
    PlayerSwitch {
        player: Player,
        fen: String,
    },
    MovePieceFailedResponse {
        // Response to `ChessRequest::MovePiece` when the action failed
        message: String,
        fen: String,
    },
    Outcome {
        outcome: Option<ChessOutcome>,
    },
    PossibleMoves {
        possible_moves: Vec<(Square /* From */, Square /* To */)>,
    },
    /// Something went wrong and the server wants to tell you about it
    GenericErrorResponse {
        message: String,
    },
    UndoMovesFailedResponse {
        message: String,
    },
    MovesUndone {
        who: Player,
        moves: u16,
    },
    CurrentTotalMovesReponse {
        total_moves: u16,
    }
}

pub async fn create_game(
    white: (Sender<ChessUpdate>, Receiver<ChessRequest>),
    black: (Sender<ChessUpdate>, Receiver<ChessRequest>),
    spectators: (Sender<ChessUpdate>, Receiver<ChessRequest>),
    config: ChessConfig,
) -> Result<()> {
    let mut game = if let Some(ref fen) = config.starting_fen {
        ChessGame::from_fen(fen)?
    } else {
        ChessGame::default()
    };

    let (mut white_tx, white_rx) = white;
    let (mut black_tx, black_rx) = black;
    let (mut spectators_tx, spectators_rx) = spectators;

    let (combined_tx, combined_rx) = channel::<(Option<Player>, ChessRequest)>(1024);

    // Wrap with tokio_stream's wrapper to have them implement Stream
    let mut white_rx = ReceiverStream::new(white_rx);
    let mut black_rx = ReceiverStream::new(black_rx);
    let mut spectators_rx = ReceiverStream::new(spectators_rx);
    let mut combined_rx = ReceiverStream::new(combined_rx);

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
        .map(|bit_move| (bit_move.get_src().into(), bit_move.get_dest().into()))
        .collect();
    match game.turn() {
        PlecoPlayer::White => white_tx.clone(),
        PlecoPlayer::Black => black_tx.clone(),
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
                send_to_sender!(ChessUpdate::CurrentTotalMovesReponse {
                    total_moves: game.total_moves()
                });
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

                        // Signal other player that he can make his move (as long as not game over)
                        send_to_everyone!(ChessUpdate::PlayerSwitch {
                            player: game.turn(),
                            fen: game.fen(),
                        });

                        if new_outcome.is_none() {
                            // Send possible moves to player
                            send_to_other_player!(ChessUpdate::PossibleMoves {
                                possible_moves: game
                                    .possible_moves()
                                    .iter()
                                    .map(|bit_move| (
                                        bit_move.get_src().into(),
                                        bit_move.get_dest().into()
                                    ))
                                    .collect(),
                            });
                        }
                    }
                    Err(e) => {
                        send_to_sender!(ChessUpdate::MovePieceFailedResponse {
                            message: format!("Denied by engine: {}", e),
                            fen: game.fen(),
                        });
                    }
                };
            }
            ChessRequest::Abort { .. /* message */ } => {
                game.player_left(sender);
                break;
            },
            ChessRequest::UndoMoves { moves } => {
                let player_allowed = match sender {
                    Player::Black => config.can_black_undo,
                    Player::White => config.can_white_undo,
                };
                if ! player_allowed {
                    send_to_sender!(ChessUpdate::UndoMovesFailedResponse {
                        message: "You are not permitted to do that in this game.".to_owned(),
                    });
                } else if !(game.turn() == sender && game.outcome().is_none() || game.outcome().is_some() && config.allow_undo_after_loose) {
                    if config.allow_undo_after_loose {
                        send_to_sender!(ChessUpdate::UndoMovesFailedResponse {
                            message: "You can only undo when you are playing or it's game over.".to_owned(),
                        });
                    }else {
                        send_to_sender!(ChessUpdate::UndoMovesFailedResponse {
                            message: "You can only undo when you are playing.".to_owned(),
                    });
                    }
                }else {
                    let prev_outcome = game.outcome();
                    if let Err(e) = game.undo(moves) {
                        send_to_sender!(ChessUpdate::UndoMovesFailedResponse {
                            message: format!("Denied by engine: {}", e),
                        });
                    }else {
                        let new_outcome = game.outcome();
                        if prev_outcome != new_outcome {
                            send_to_everyone!(ChessUpdate::Outcome {
                                outcome: new_outcome
                            });
                        }
                        // Select current player and update board
                        send_to_everyone!(ChessUpdate::PlayerSwitch {
                            player: game.turn(),
                            fen: game.fen()
                        });
                        // Send the starting player his possible moves
                        let possible_moves: Vec<_> = game
                            .possible_moves()
                            .iter()
                            .map(|bit_move| (bit_move.get_src().into(), bit_move.get_dest().into()))
                            .collect();
                        match game.turn() {
                            PlecoPlayer::White => white_tx.clone(),
                            PlecoPlayer::Black => black_tx.clone(),
                        }
                        .send(ChessUpdate::PossibleMoves { possible_moves })
                        .await
                        .ok();
                        // Notify everyone of undo
                        send_to_everyone!(ChessUpdate::MovesUndone {
                            who: sender,
                            moves,
                        });

                    }
                }
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
    min_reaction_delay: Duration,
) -> Result<(Sender<ChessUpdate>, Receiver<ChessRequest>)> {
    let (update_tx, mut update_rx) = channel::<ChessUpdate>(256);
    let (mut request_tx, request_rx) = channel::<ChessRequest>(256);

    task::spawn(async move {
        info!("Bot spawned for {}", me);
        let mut current_outcome: Option<ChessOutcome> = None;
        while let Some(update) = update_rx.recv().await {
            match update {
                ChessUpdate::PlayerSwitch { player, ref fen } => {
                    if player == me && current_outcome.is_none() {
                        let board = pleco::Board::from_fen(fen)
                            .expect("Bot failed to parse the provided fen");

                        let bit_move = task::spawn_blocking(move || {
                            let started = SystemTime::now();
                            let bit_move = T::best_move(board, depth);
                            let elapsed = started.elapsed().unwrap_or(Duration::new(0, 0));

                            if elapsed < min_reaction_delay {
                                thread::sleep(min_reaction_delay - elapsed);
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
                                source: bit_move.get_src().into(),
                                destination: bit_move.get_dest().into(),
                            })
                            .await
                            .expect("Bot failed to send move");
                    }
                }
                ChessUpdate::MovePieceFailedResponse { message, .. } => {
                    error!("A move from the bot was rejected: {}", message);
                    break;
                }
                ChessUpdate::Outcome { outcome } => {
                    if outcome.is_some() {
                        info!("Bot detected that the game ended");
                    //break;
                    } else {
                        info!("Game continues. Bot will continue playing.");
                    }
                    current_outcome = outcome;
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

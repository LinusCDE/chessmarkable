#[macro_use]
extern crate peggler;

#[macro_use]
extern crate enum_primitive;
extern crate num;

mod model;

pub use model::{NAG, GameMove, GameTermination, MoveNumber, MoveSequence, Piece, File, Rank,
                Square, Move, MarkedMove, Game, AnnotationSymbol};

use model::GameTermination::{WhiteWins, BlackWins, DrawnGame, Unknown};

use peggler::{ParseError, ParseResult};

fn read_zero_or_more<F>(input: &str, predicate: F) -> ParseResult<&str>
    where
        F: Fn(char) -> bool,
{

    let end;

    let mut char_indices = input.char_indices();
    loop {
        match char_indices.next() {
            Some((index, char)) => {
                if predicate(char) {
                    continue;
                }
                end = index;
                break;
            }

            None => {
                end = input.len();
                break;
            }
        }
    }

    Ok((&input[..end], &input[end..]))
}

fn find_termination(input: &str) -> ParseResult<&str>
{

    let mut end= input.len()-1;
    let mut char_indices = input.char_indices();
    loop {
        match char_indices.next() {
            Some((index, char)) => {
                if char == '0' {
                    match char_indices.next() {
                        Some((index, char)) => {
                            if char == '-' {
                                match char_indices.next() {
                                    Some((index, char)) => {
                                        if char == '1' {
                                            match char_indices.next() {
                                                Some((index, char)) => {
                                                    if char != '"' && char != '}' {
                                                        end = index;
                                                        break;
                                                    }
                                                },
                                                _ => continue
                                            }
                                        }
                                    },
                                    _ => continue
                                }
                            }
                        },
                        _ => continue
                    }
                } else if char == '1' {
                    match char_indices.next() {
                        Some((index, char)) => {
                            if char == '-' {
                                match char_indices.next() {
                                    Some((index, char)) => {
                                        if char == '0' {
                                            match char_indices.next() {
                                                Some((index, char)) => {
                                                    if char != '"' && char != '}' {
                                                        end = index;
                                                        break;
                                                    }
                                                },
                                                _ => continue
                                            }
                                        }
                                    },
                                    _ => continue
                                }
                            } else if char == '/' {
                                //Ok yes I'm too lazy and there's probably a better way
                                //to do all of this anyway
                                end = index+6;
                                break;
                            }
                        },
                        _ => continue
                    }
                }
            }
            None => {
                end = input.len();
                break;
            }
        }
    }

    Ok((&input[..end], &input[end..]))
}


fn read_one_or_more<F>(input: &str, predicate: F) -> ParseResult<&str>
    where
        F: Fn(char) -> bool,
{

    let result = read_zero_or_more(input, predicate);

    match result {
        Err(_) => result,
        Ok((value, _)) if value.len() == 0 => Err(ParseError),
        Ok(_) => result,
    }
}

fn any_char(input: &str) -> ParseResult<char> {
    let mut char_indices = input.char_indices();
    match char_indices.next() {
        Some((_, char)) => {
            match char_indices.next() {
                Some((index, _)) => Ok((char, &input[index..])),
                None => Ok((char, &""[..])),
            }
        }
        None => Err(ParseError),
    }
}

fn pgn_integer(input: &str) -> ParseResult<u32> {
    read_one_or_more(input, |char| char.is_digit(10)).map(|r| (r.0.parse::<u32>().unwrap(), r.1))
}

fn pgn_symbol(input: &str) -> ParseResult<String> {
    read_one_or_more(
        input,
        |char| char.is_alphanumeric() || "_+#=:".contains(char),
    ).map(|r| (r.0.to_string(), r.1))
}

fn whitespace(input: &str) -> ParseResult<()> {
    read_one_or_more(input, |char| char.is_whitespace()).map(|r| ((), r.1))
}

fn inline_comment_contents(input: &str) -> ParseResult<String> {
    read_zero_or_more(input, |char| char != '\r' && char != '\n').map(|r| (r.0.to_string(), r.1))
}

fn block_comment_contents(input: &str) -> ParseResult<String> {
    read_zero_or_more(input, |char| char != '}').map(|r| (r.0.to_string(), r.1))
}

fn trim_newline_and_space(mut s: String) -> String {
    loop {
        if s.starts_with('\n') || s.starts_with('\r' ) || s.starts_with(' ') {
            let mut chars = s.chars();
            chars.next();
            s = chars.as_str().parse().unwrap()
        } else {
            break;
        }
    }
    s
}

pub fn read_games(input: &str) -> Result<Vec<Game>, ParseError> {
    let mut games: Vec<Game> = vec![];
    let mut rest = &input[..];

    let result = read_zero_or_more(&rest, |char| char.is_whitespace());
    let mut stripped_value: String;
    rest = result.unwrap().1;

    loop {
        let item = game(rest);
        match item {
            Ok(item) => {
                games.push(item.0);
                rest = item.1;
                if rest.len() == 0 {
                    break;
                }
            },
            Err(_) => {
                let term = find_termination(rest).unwrap();
                let s = term.1.to_string().to_owned();
                stripped_value = trim_newline_and_space(s);
                rest = &*stripped_value;
                if rest.len() == 0 {
                    break;
                }
            }
        }
    }

    Ok(games)
}

rule!(pgn_string_char:char =
      ["\\\""] => { '\"' } /
      ["\\\\"] => { '\\' } /
      (!["\""] x:any_char => { x }));

rule!(pgn_string:String = ["\""] chars:pgn_string_char* ["\""]
      => { chars.into_iter().collect() });

rule!(tag_pair:(String, String) =
      ["["] whitespace?
      name:pgn_symbol whitespace value:pgn_string
      whitespace? ["]"]
      => { (name, value) });

rule!(tag_section:Vec<(String, String)> =
      (tag_pair:tag_pair whitespace? => { tag_pair })*);

rule!(game_termination:GameTermination =
      ["1-0"] => { WhiteWins } /
      ["0-1"] => { BlackWins } /
      ["1/2-1/2"] => { DrawnGame } /
      ["*"] => { Unknown });

rule!(move_number:MoveNumber = number:pgn_integer ["."] black_marks:[".."]?
      => { match black_marks {
          Some(_) => MoveNumber::Black(number),
          None => MoveNumber::White(number)
      }});

rule!(file:File =
      ["a"] => { File::A } /
      ["b"] => { File::B } /
      ["c"] => { File::C } /
      ["d"] => { File::D } /
      ["e"] => { File::E } /
      ["f"] => { File::F } /
      ["g"] => { File::G } /
      ["h"] => { File::H });

rule!(rank:Rank =
      ["1"] => { Rank::R1 } /
      ["2"] => { Rank::R2 } /
      ["3"] => { Rank::R3 } /
      ["4"] => { Rank::R4 } /
      ["5"] => { Rank::R5 } /
      ["6"] => { Rank::R6 } /
      ["7"] => { Rank::R7 } /
      ["8"] => { Rank::R8 });

rule!(piece:Piece =
      ["P"] => { Piece::Pawn } /
      ["N"] => { Piece::Knight } /
      ["B"] => { Piece::Bishop } /
      ["R"] => { Piece::Rook } /
      ["Q"] => { Piece::Queen } /
      ["K"] => { Piece::King });

rule!(square:Square = file:file rank:rank
      => { Square::new_known(file, rank) });

rule!(move_disambiguation:Square =
        from:(square /
              file:file => { Square::new_file(file) } /
              rank:rank => { Square::new_rank(rank) } )
        &(["x"] => { } / file => { })
        => { from });

rule!(basic_move:Move =
      piece_:piece?
      from:move_disambiguation?
      capture_mark: ["x"]?
      to: square
      promoted_to: (["="] piece_:piece => { piece_ })?
      => { Move::BasicMove {
            piece: piece_.unwrap_or(Piece::Pawn),
            to: to,
            from: from.unwrap_or(Square::XX),
            is_capture: capture_mark.is_some(),
            promoted_to: promoted_to,
         }});

rule!(annotation_symbol:AnnotationSymbol =
      ["??"] => { AnnotationSymbol::Blunder } /
      ["?!"] => { AnnotationSymbol::Dubious } /
      ["?"] => { AnnotationSymbol::Mistake } /
      ["!?"] => { AnnotationSymbol::Interesting } /
      ["!!"] => { AnnotationSymbol::Brilliant } /
      ["!"] => { AnnotationSymbol::Good });

rule!(marked_move: MarkedMove =
      move_ : (basic_move /
               ["O-O-O"] => { Move::CastleQueenside } /
               ["O-O"] => { Move::CastleKingside })
      check_mark: ["+"]?
      checkmate_mark: ["#"]?
      annotation_symbol: annotation_symbol?
      => { MarkedMove {
          move_: move_,
          is_check: check_mark.is_some(),
          is_checkmate: checkmate_mark.is_some(),
          annotation_symbol: annotation_symbol
      }});

rule!(nag:NAG = ["$"] value:pgn_integer => { NAG(value) });

rule!(line_end:() = (["\r\n"] / ["\r"] / ["\n"]) => { });

rule!(inline_comment:String =
      [";"] value:inline_comment_contents line_end
      => { value });

rule!(block_comment:String =
      ["{"] value:block_comment_contents ["}"]
      => { value });

rule!(comment: String = inline_comment / block_comment);

rule!(move_number_ws:MoveNumber = x:move_number whitespace? => { x });
rule!(marked_move_ws:MarkedMove = x:marked_move whitespace? => { x });
rule!(nag_ws:NAG = x:nag whitespace? => { x });
rule!(comment_ws:String = x:comment whitespace? => { x });

rule!(variation: MoveSequence =
      ["("] whitespace? move_sequence:move_sequence [")"] whitespace?
      => { move_sequence });

rule!(game_move:GameMove =
      number: move_number_ws?
      move_: marked_move_ws
      nag: nag_ws?
      comment: comment_ws?
      variations: variation*
      => { GameMove {
            number: number,
            move_: move_,
            nag: nag,
            comment: comment,
            variations: variations }});

rule!(move_sequence:MoveSequence =
      comment: comment_ws?
      moves: (move_:game_move whitespace? => { move_ })*
      => { MoveSequence {
          comment: comment,
          moves: moves
      }});

rule!(game:Game =
      tags: tag_section
      move_sequence: move_sequence
      termination: game_termination
      whitespace?
      => { Game {
            tags: tags,
            comment: move_sequence.comment,
            moves: move_sequence.moves,
            termination: termination
          }});


#[cfg(test)]
mod tests {
    use super::{pgn_integer, pgn_string, pgn_symbol, read_one_or_more, tag_pair, tag_section,
                whitespace, game_termination, move_number, move_disambiguation, file, rank, piece,
                square, basic_move, marked_move, nag, line_end, inline_comment, block_comment,
                game_move, move_sequence, game, read_games};

    use model::{Move, MoveNumber, MoveSequence, NAG};
    use model::File::*;
    use model::Rank::*;
    use model::Square::*;
    use model::Piece::*;
    use model::Move::*;
    use model::MoveNumber::*;
    use model::GameTermination::*;
    use model::AnnotationSymbol::*;

    use peggler::{ParseResult, ParseError};


    fn run<P, T>(parser: P, input: &str) -> T
        where
            P: Fn(&str) -> ParseResult<T>,
    {
        let result = parser(input).unwrap();
        assert_eq!(result.1, "");
        result.0
    }

    #[test]
    fn test_read_one_or_more() {
        assert_eq!(read_one_or_more("abc", |char| char == 'a'), Ok(("a", "bc")));
    }

    #[test]
    fn test_string() {
        assert_eq!(run(pgn_string, "\"\""), "".to_string());
        assert_eq!(run(pgn_string, "\"abc\""), "abc".to_string());
        assert_eq!(run(pgn_string, "\"\\\"\""), "\"".to_string());
        assert_eq!(run(pgn_string, "\"\\\\\""), "\\".to_string());
        assert_eq!(run(pgn_string, "\"\\\\abc\""), "\\abc".to_string());
    }

    #[test]
    fn test_integer() {
        assert_eq!(run(pgn_integer, "1"), 1);
        assert_eq!(run(pgn_integer, "12"), 12);
        assert_eq!(run(pgn_integer, "123"), 123);
    }

    #[test]
    fn test_symbol() {
        assert_eq!(run(pgn_symbol, "abc123"), "abc123".to_string());
        assert_eq!(run(pgn_symbol, "_+#=:"), "_+#=:".to_string());
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(run(whitespace, " \t\r\n"), ());
    }

    #[test]
    fn test_tag_pair() {
        assert_eq!(run(tag_pair, "[Name \"Value\"]"), (
            "Name".to_string(),
            "Value".to_string(),
        ));
    }

    #[test]
    fn test_tag_section() {
        let input = "[Event \"F/S Return Match\"]\n\
                     [Site \"Belgrade, Serbia JUG\"]";
        let expected = vec![
            ("Event".to_string(), "F/S Return Match".to_string()),
            ("Site".to_string(), "Belgrade, Serbia JUG".to_string()),
        ];
        assert_eq!(run(tag_section, input), expected);
    }

    #[test]
    fn test_game_termination() {
        assert_eq!(run(game_termination, "1/2-1/2"), DrawnGame);
    }

    #[test]
    fn test_move_number() {
        assert_eq!(run(move_number, "12."), MoveNumber::White(12));
        assert_eq!(run(move_number, "12..."), MoveNumber::Black(12));
    }

    #[test]
    fn test_file() {
        assert_eq!(run(file, "a"), A);
        assert_eq!(run(file, "b"), B);
        assert_eq!(run(file, "h"), H);
    }

    #[test]
    fn test_rank() {
        assert_eq!(run(rank, "1"), R1);
        assert_eq!(run(rank, "2"), R2);
        assert_eq!(run(rank, "8"), R8);
    }

    #[test]
    fn test_piece() {
        assert_eq!(run(piece, "P"), Pawn);
        assert_eq!(run(piece, "N"), Knight);
        assert_eq!(run(piece, "K"), King);
    }

    #[test]
    fn test_square() {
        assert_eq!(run(square, "a1"), A1);
        assert_eq!(run(square, "b2"), B2);
        assert_eq!(run(square, "h8"), H8);
    }

    #[test]
    fn test_move_disambiguation() {
        assert_eq!(move_disambiguation("f"), Err(ParseError));
        assert_eq!(move_disambiguation("f1x"), Ok((F1, "x")));
        assert_eq!(move_disambiguation("f1g"), Ok((F1, "g")));
        assert_eq!(move_disambiguation("fg"), Ok((FX, "g")));
        assert_eq!(move_disambiguation("1g"), Ok((X1, "g")));
    }

    #[test]
    fn test_basic_move() {
        assert_eq!(run(basic_move, "e4"), Move::new(Pawn, E4));
        assert_eq!(run(basic_move, "Nf3"), Move::new(Knight, F3));
        assert_eq!(run(basic_move, "Ngf3"), Move::new(Knight, F3).from(GX));
        assert_eq!(run(basic_move, "Bb2"), Move::new(Bishop, B2));
        assert_eq!(run(basic_move, "R2h4"), Move::new(Rook, H4).from(X2));
        assert_eq!(run(basic_move, "Qa1d4"), Move::new(Queen, D4).from(A1));
        assert_eq!(run(basic_move, "Nxc5"), Move::new(Knight, C5).capture());
        assert_eq!(
            run(basic_move, "h8=Q"),
            Move::new(Pawn, H8).with_promotion(Queen)
        );
    }

    #[test]
    fn test_marked_move() {
        assert_eq!(run(marked_move, "Kh2"), Move::new(King, H2).no_mark());
        assert_eq!(run(marked_move, "Bb2+"), Move::new(Bishop, B2).check());
        assert_eq!(run(marked_move, "Qg7#"), Move::new(Queen, G7).checkmate());
        assert_eq!(run(marked_move, "O-O"), CastleKingside.no_mark());
        assert_eq!(run(marked_move, "O-O-O"), CastleQueenside.no_mark());
        assert_eq!(run(marked_move, "O-O+"), CastleKingside.check());
        assert_eq!(run(marked_move, "O-O-O#"), CastleQueenside.checkmate());
        assert_eq!(
            run(marked_move, "a3!!"),
            Move::new(Pawn, A3).no_mark().annotated(Brilliant)
        );
    }

    #[test]
    fn test_nag() {
        assert_eq!(run(nag, "$123"), NAG(123));
    }

    #[test]
    fn test_line_end() {
        assert_eq!(line_end("\r\n"), Ok(((), "")));
        assert_eq!(line_end("\r"), Ok(((), "")));
        assert_eq!(line_end("\n"), Ok(((), "")));
    }

    #[test]
    fn test_comment() {
        assert_eq!(run(inline_comment, ";abc\n"), "abc".to_string());
        assert_eq!(run(block_comment, "{abc}"), "abc".to_string());
    }

    #[test]
    fn test_game_move() {
        assert_eq!(
            run(game_move, "Kh2"),
            Move::new(King, H2).no_mark().numbered(None)
        );

        assert_eq!(
            run(game_move, "1. Bb2 {+0.51/15 0.21s}"),
            Move::new(Bishop, B2)
                .no_mark()
                .numbered(Some(White(1)))
                .comment("+0.51/15 0.21s".to_string())
        );

        assert_eq!(
            run(game_move, "1... e4"),
            Move::new(Pawn, E4).no_mark().numbered(Some(Black(1)))
        );

        assert_eq!(
            run(game_move, "10. O-O $12"),
            CastleKingside.no_mark().numbered(Some(White(10))).nag(
                NAG(12),
            )
        );
    }

    #[test]
    fn test_move_sequence() {
        assert_eq!(
            run(move_sequence, "10. O-O"),
            MoveSequence {
                comment: None,
                moves: vec![CastleKingside.no_mark().numbered(Some(White(10)))],
            }
        );

        assert_eq!(
            run(move_sequence, "10. O-O O-O-O"),
            MoveSequence {
                comment: None,
                moves: vec![
                    CastleKingside.no_mark().numbered(Some(White(10))),
                    CastleQueenside.no_mark().numbered(None),
                ],
            }
        );
    }

    #[test]
    fn test_game_move_with_variations() {
        assert_eq!(
            run(game_move, "4. Bc4 (4. Bb5)"),
            Move::new(Bishop, C4)
                .no_mark()
                .numbered(Some(White(4)))
                .with_variations(vec![
                    MoveSequence {
                        comment: None,
                        moves: vec![Move::new(Bishop, B5).no_mark().numbered(Some(White(4)))],
                    },
                ])
        );

        assert_eq!(
            run(game_move, "4. Bc4 (4. Bb5) (4. Kh2)"),
            Move::new(Bishop, C4)
                .no_mark()
                .numbered(Some(White(4)))
                .with_variations(vec![
                    MoveSequence {
                        comment: None,
                        moves: vec![Move::new(Bishop, B5).no_mark().numbered(Some(White(4)))],
                    },
                    MoveSequence {
                        comment: None,
                        moves: vec![Move::new(King, H2).no_mark().numbered(Some(White(4)))],
                    },
                ])
        );

        assert_eq!(
            run(game_move, "4. Bc4 (4. Bb5 (4. Kh2))"),
            Move::new(Bishop, C4)
                .no_mark()
                .numbered(Some(White(4)))
                .with_variations(vec![
                    MoveSequence {
                        comment: None,
                        moves: vec![
                            Move::new(Bishop, B5)
                                .no_mark()
                                .numbered(Some(White(4)))
                                .with_variations(vec![
                                    MoveSequence {
                                        comment: None,
                                        moves: vec![
                                            Move::new(King, H2).no_mark().numbered(
                                                Some(White(4))
                                            ),
                                        ],
                                    },
                                ]),
                        ],
                    },
                ])
        );
    }

    #[test]
    fn test_game() {
        //Game taken from http://www.chess.com/download/view/1001-chess-miniatures

        let result = game(
            "[Event \"Croatia\"]\n\
               [Site \"?\"]\n\
               [Date \"2004.??.??\"]\n\
               [Round \"?\"]\n\
               [White \"Gardijan\"]\n\
               [Black \"Sulc\"]\n\
               [Result \"0-1\"]\n\
               [ECO \"B20\"]\n\
               [PlyCount \"10\"]\n\
               \n\
               1. e4 c5 2. c4 Nc6 3. Ne2 Ne5 4. d4 (4. Ng3) 4... Qa5+ 5. \
               Bd2 $4 (5. Nec3) 5... Nd3# 0-1",
        );

        match result {
            Ok((game, _)) => {
                assert_eq!(game.termination, BlackWins);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_read_games() {
        let result = read_games("1. e4 e5 * 1. d4 d5 *").unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_read_games_with_leading_whitespace() {
        let result = read_games(" 1. e4 e5 * 1. d4 d5 *").unwrap();
        assert_eq!(result.len(), 2);
    }
}

use anyhow::{Result};
use serde::{Deserialize, Serialize};
use std::path::{PathBuf};
use glob::glob;

#[derive(Serialize, Deserialize, Clone)]
pub struct Pgn {
    pub path: PathBuf
}

pub fn read(from: usize, to: usize) -> Result<Vec<Pgn>> {
    let ref pgn_loc = crate::CLI_OPTS.pgn_location;
    if from > to {
        Err(anyhow!("from cant be greater than to"))
    } else if !pgn_loc.exists() {
        info!("No pgn directory found");
        Ok(Vec::new())
    } else {
        let elements_to_fetch = to - from + 1;
        let mut requested_pgns: Vec<Pgn> = Vec::with_capacity(elements_to_fetch);
        for entry in glob(&construct_png_loc_pattern_string(pgn_loc)).
            expect("Failed to read glob pattern")
            .skip(from)
            .take(elements_to_fetch) {
            match entry {
                Ok(path) => {
                    requested_pgns.push(Pgn {
                        path
                    })
                }
                Err(e) => println!("{:?}", e)
            }
        }
        Ok(requested_pgns)
    }
}

pub fn total_number_of_pgn() -> u32 {
    let mut pages: u32 = 0;
    let ref pgn_loc = crate::CLI_OPTS.pgn_location;
    if !pgn_loc.exists() {
        info!("No pgn directory found");
        0
    } else {
        for entry in glob(&construct_png_loc_pattern_string(pgn_loc))
            .expect("Failed to read glob pattern") {
            match entry {
                Ok(_) => pages = pages + 1,
                Err(e) => println!("{:?}", e)
            }
        }
        pages
    }
}

fn construct_png_loc_pattern_string(pgn_loc: &PathBuf) -> String {
    let mut pgn_loc_str = pgn_loc.to_owned().into_os_string().into_string().unwrap();
    pgn_loc_str.push_str("/*.pgn");
    pgn_loc_str
}

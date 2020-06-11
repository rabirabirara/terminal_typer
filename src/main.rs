use clap::{App, Arg, ArgGroup, ArgMatches};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path; // for parsing command line arguments
use std::time::{Duration, Instant}; // Instant used in time attack
use strum_macros::AsRefStr;

/* I could make writing this into a Rust tutorial.
-Covers dependencies
    : file read/write
    : time-related work
    : using command-line-argument-parser
-Derive on structs, implicitly Traits
-Declarative programming in rust

*/

// score: how many words typed correctly, in how much time
// reduced by errors - one for error skip, or one for each error
// Give randomized mistake message from list of possible prompts.
// difficulty ramps up; in word file, difficulty is measured by length of characters. max difficulty.
// multiple words at a time, separated by spaces (called multiple, could be flag or mode)
// could have "blind" flag - could have these difficulty enhancing flags which increase score`
// continue on error, or halt until getting it right.
// count errors up.

// Note: do not retrim String variables that come from the file - trim input!

// implement getchar() style for "classic" argument
const PROG_NAME: &'static str = env!("CARGO_PKG_NAME");
const PROG_AUTHOR: &'static str = env!("CARGO_PKG_AUTHORS");
const PROG_VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(PartialEq, Eq, AsRefStr)] // You *could* download strum and derive Enum names as static str to print.
enum Mode {
    TimeAttack,
    Endless,
    Race,
}

// TODO: Implement difficulties as Enums of u8 type.

#[derive(PartialEq, Eq)]
struct Score {
    correct: u32,
    errors: u32,
    time: Option<Duration>, // for race
    wpm: Option<u32>,
}

impl Score {
    pub fn default() -> Score {
        Score {
            correct: 0,
            errors: 0,
            time: None,
            wpm: None,
        }
    }
}

struct Modifiers {
    skip_err: bool,
    multiple: bool,
    classic: bool,
    accumulate: bool,
}

struct Game {
    mode: Mode,
    word_sets: HashMap<u32, Vec<String>>,
    options: Modifiers,
    count: Option<Vec<u32>>, // for race
}

fn read_file(filename: &str) -> Result<Vec<String>, std::io::Error> {
    let file_path = Path::new(filename);
    let file_object = OpenOptions::new().read(true).open(file_path)?;
    let file_buffer = BufReader::new(file_object);
    let words: Vec<String> = file_buffer
        .lines()
        .map(|l| l.unwrap().trim().to_string())
        .collect();
    Ok(words)
}

fn parse_to_sets(words: Vec<String>) -> HashMap<u32, Vec<String>> {
    // Don't need to trim() the Strings because they've been trimmed in read_file().
    // Ultimately this long branch is more readable than using a for loop, imo.
    // (That, and trying the for loop with a vector of vectors gave me problems with references.)

    let mut word_sets: HashMap<u32, Vec<String>> = HashMap::new();

    for (ch, i) in (3usize..=11usize).collect::<Vec<usize>>().chunks(3).into_iter().zip((1..=3).into_iter()) {
        let three = words
            .iter()
            .filter(|word| word.len() == ch[0])
            .cloned()
            .into_iter()
        .chain(words
            .iter()
            .filter(|word| word.len() == ch[1])
            .cloned()
            .into_iter()
        ).chain(words
            .iter()
            .filter(|word| word.len() == ch[2])
            .cloned()
            .into_iter()
        );
        let set: Vec<String> = three.collect();
        word_sets.insert(i, set);
    }
    
    
    let long: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() >= 12)
        .collect();
    word_sets.insert(4, long);

    word_sets
}

macro_rules! sleep {
    ($x:expr) => {
        std::thread::sleep(Duration::from_secs($x))
    };
}

fn count_down(secs: u32, mode: &Mode) {
    println!("Game Mode: {}", mode.as_ref());

    sleep!(2);
    for i in (1..=secs).rev() {
        println!("{}", i);
        sleep!(1);
    }
}

fn calculate_wpm(chars_typed: usize, time_total: f32) -> u32 {
    12 * chars_typed as u32 / time_total.round() as u32
}

// fn options(matches: &ArgMatches) -> HashMap<String, bool> {
//     let mut options: HashMap<String, bool> = HashMap::new();

//     let skip_err = matches.is_present("skip-errors");
//     let multiple = matches.is_present("multiple");
//     let classic = matches.is_present("classic");
//     let accumulate = matches.is_present("accumulate");

//     options.insert("skip_err".to_string(), skip_err);
//     options.insert("multiple".to_string(), multiple);
//     options.insert("classic".to_string(), classic);
//     options.insert("accumulate".to_string(), accumulate);

//     options
// }

fn options(matches: &ArgMatches) -> Modifiers {
    let skip_err = matches.is_present("skip-errors");
    let multiple = matches.is_present("multiple");
    let classic = matches.is_present("classic");
    let accumulate = matches.is_present("accumulate");

    let options: Modifiers = Modifiers {
        skip_err,
        multiple,
        classic,
        accumulate,
    };
    options
}

// I understand that there is code repetition in separating the functions like this, but I
// believe that the code is more clear when organized in this fashion.
// I can isolate bugs to a specific mode.
fn play(game: &Game) -> Option<Score> {
    println!("\x1b[2J\x1b[1;1HWelcome to the Typing Challenge! Type the words on screen as fast as you can, then press enter.");
    sleep!(4);
    // TODO: change mode selection to ingame prompt.
    return match game.mode {
        Mode::TimeAttack => play_time(game),
        _ => play_race_or_endless(game),
    };
}

// as difficulty increases, you get more/less time to type the word.
// Score is based on how many words you get correct before crashing.
// Since losing this mode occurs on time out, skip-err is not allowed.
// Can choose to reset timer (more score at higher diffs), or can accumulate time.
// TODO: Implement concurrent timer which interrupts player when time is up.  Eventually implement timer.
fn play_time(game: &Game) -> Option<Score> {
    count_down(3, &game.mode);

    let mut words_done: u32 = 0; // used to measure score
    let mut chars_typed: usize = 0; // used to measure characters per minute
    let mut words_queued: u32 = 0; // used to measure difficulties
    let mut errors: u32 = 0;
    let mut rng = thread_rng();
    let mut bytes;
    let mut quit = false;

    let mut difficulty: u32 = 1; // initialize queue with lowest difficulty word.
    let mut words: VecDeque<String> = VecDeque::new();
    if game.options.multiple {
        for _i in 0..9 {
            let word: String = game.word_sets.get(&difficulty)?.choose(&mut rng)?.clone();
            words_queued += 1;
            words.push_back(word);
        }
    } else {
        let word: String = game.word_sets.get(&difficulty)?.choose(&mut rng)?.clone();
        words_queued += 1;
        words.push_back(word);
    }

    let mut time_difficulty: f32;
    let mut time_passed: f32 = 0.;
    let mut time_total = 0.;
    let mut time_accumulated: f32 = 0.;
    while !quit {
        let time_start = Instant::now();
        let cur = words.pop_front()?.clone();
        // Difficulty is based on words_queued for consistency with stages.  It just makes sense.
        difficulty = match words_queued {
            0..=20 => 1,
            21..=40 => 2,
            41..=60 => 3,
            _ => 4,
        };
        time_difficulty = match difficulty {
            1 => 10.,
            2 => 8.,
            3 => 6.,
            _ => 5.,
        };
        time_difficulty += time_accumulated;
        let word: String = game
            .word_sets
            .get(&(difficulty as u32))?
            .choose(&mut rng)?
            .clone();
        words_queued += 1;
        words.push_back(word);

        let mut input = String::new();
        while !quit {
            println!(
                "\x1b[2J\x1b[1;1H{} | {} | {:.2}",
                words_done,
                errors,
                time_difficulty - time_passed
            );
            if game.options.multiple {
                let mut witer = words.iter();
                println!(
                    "\n{} {} {} {} {} {} {} {} {}",
                    cur,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?
                );
            } else {
                println!("\n{}", cur);
            }
            input.clear();
            bytes = io::stdin().lock().read_line(&mut input).unwrap_or_default();
            println!("{}", input);
            if bytes == 0 || input.contains("\t") {
                quit = true;
                break;
            }
            if input.trim_end() == "" {
                continue;
            }

            if input.trim_end() == cur {
                words_done += 1;
                chars_typed += cur.len();
                time_passed = Instant::now()
                    .saturating_duration_since(time_start)
                    .as_secs_f32();
                if time_passed >= time_difficulty {
                    quit = true;
                }
                if game.options.accumulate {
                    time_accumulated += 1.;
                }
                time_total += time_passed;
                break;
            } else {
                errors += 1;
            }
        }
    }
    if words_done > 0 {
        Some(Score {
            correct: words_done,
            errors,
            time: None,
            wpm: Some(calculate_wpm(chars_typed, time_total)),
        })
    } else {
        None
    }
}

fn play_race_or_endless(game: &Game) -> Option<Score> {
    count_down(3, &game.mode);

    let mut words_done: u32 = 0;
    let mut errors: u32 = 0;
    let mut rng = thread_rng();
    let mut bytes;
    let mut quit = false;
    let check: bool;

    let word_count: u32;
    let low: u32;
    let high: u32;
    if let Some(count_options) = &game.count {
        word_count = count_options[0];
        check = true;
        match count_options.len() {
            1 => {
                low = 1;
                high = 4;
            }
            2 => {
                low = count_options[1];
                high = low;
            }
            3 => {
                low = count_options[1];
                high = count_options[2];
            }
            _ => unreachable!(), // Unreachable: vetted out by CLAP parsing
        }
    } else {
        word_count = 0;
        check = false;
        low = 1;
        high = 4;
    }

    let mut difficulty: u32 = low; // initialize queue with lowest difficulty word.
    let mut words: VecDeque<String> = VecDeque::new();
    if game.options.multiple {
        for _i in 0..9 {
            // queue up nine words, so the tenth is added in the loop.
            let word: String = game.word_sets.get(&difficulty)?.choose(&mut rng)?.clone();
            difficulty = match game.mode {
                Mode::Race => ((word.len() as u32) % (high - low + 1)) + low,
                Mode::Endless => (word.len() as u32 % 4) + 1,
                _ => unreachable!(), // Unreachable because only called for above two modes
            };
            words.push_back(word);
        }
    } else {
        let word: String = game.word_sets.get(&low)?.choose(&mut rng)?.clone();
        words.push_back(word);
    }

    let now: Option<Instant>;
    if game.mode == Mode::Race {
        now = Some(Instant::now());
    } else {
        now = None;
    }

    while !quit {
        let cur = words.pop_front()?.clone();
        difficulty = match game.mode {
            Mode::Race => ((cur.len() as u32) % (high - low + 1)) + low,
            Mode::Endless => cur.len() as u32 % 4 + 1,
            _ => unreachable!(), // Unreachable because only called for above two modes
        };
        let word: String = game
            .word_sets
            .get(&(difficulty as u32))?
            .choose(&mut rng)?
            .clone();
        words.push_back(word);

        let mut input = String::new();
        while !quit {
            println!("\x1b[2J\x1b[1;1H{} | {}", words_done, errors);
            if game.options.multiple {
                let mut witer = words.iter();
                println!(
                    "\n{} {} {} {} {} {} {} {} {}",
                    cur,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?,
                    witer.next()?
                );
            } else {
                println!("\n{}", cur);
            }
            input.clear();
            bytes = io::stdin().lock().read_line(&mut input).unwrap_or_default();
            println!("{}", input);
            if bytes == 0 || input.contains("\t") {
                quit = true;
                break;
            }
            if input.trim_end() == "" {
                continue;
            }

            if input.trim_end() == cur {
                words_done += 1;
                if check && words_done == word_count {
                    quit = true;
                }
                break;
            } else {
                errors += 1;
                match game.options.skip_err {
                    true => break,
                    false => continue,
                }
            }
        }
    }

    match game.mode {
        Mode::Race => {
            if check && words_done != word_count {
                None
            } else {
                Some(Score {
                    correct: words_done,
                    errors,
                    time: Some(now?.elapsed()),
                    wpm: None,
                })
            }
        }
        _ => {
            if words_done == 0 {
                None
            } else {
                Some(Score {
                    correct: words_done,
                    errors,
                    time: None,
                    wpm: None,
                })
            }
        }
    }
}

// Print score and write it to scores.txt.
fn give_score(scores: &Score, mode: &Mode, options: &Modifiers) -> io::Result<()> {
    write_score(scores, mode, options)?;
    println!("\x1b[2J\x1b[1;1HGood game! Your score:");
    print!("Correct: {} | Errors: {}", scores.correct, scores.errors);
    if let Some(time) = scores.time {
        println!(" | Race time: {}", time.as_secs_f32().to_string());
    } else if let Some(wpm) = scores.wpm {
        println!(" | Approx. WPM: {}", wpm);
    }

    Ok(())
}

fn write_score(scores: &Score, mode: &Mode, options: &Modifiers) -> io::Result<()> {
    let scores_path = Path::new("scores.txt");
    let mut scores_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(scores_path)
        .expect("Expected valid path creation from \"scores.txt\".");

    let mode_string = if options.accumulate {
        mode.as_ref().to_owned() + " -a"
    } else {
        mode.as_ref().to_owned()
    };

    write!(
        &mut scores_file,
        "\nMode: {:<12}  :  Correct: {:<5} |  Errors: {:<5}",
        mode_string,
        scores.correct,
        scores.errors
    )?;

    match *mode {
        Mode::Race => {
            if let Some(duration) = scores.time {
                write!(&mut scores_file, "  |  Time: {:.2}", duration.as_secs_f32())?;
            } else {
                write!(&mut scores_file, "  |  Time: N/A")?;
            }
        }
        Mode::TimeAttack => {
            if let Some(wpm) = scores.wpm {
                write!(&mut scores_file, "  |  Approx. WPM: {}", wpm)?;
            } else {
                write!(&mut scores_file, "  |  Approx. WPM: N/A")?;
            }
        }
        _ => (),
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let matches = App::new(PROG_NAME)
        .author(PROG_AUTHOR)
        .version(PROG_VERSION)
        .about("A command-line typing game.  Input a TAB character to end a game.")
        .arg(
            Arg::with_name("input-file")
                .about("File containing word data for game.")
                .value_name("FILE")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::with_name("time-attack")
                .short('t')
                .long("time-attack")
                .about("Enters the game in TimeAttack Attack mode."), // .conflicts_with("endless"),  // BROKEN.  SUBMIT BUG REPORT?
        )
        .arg(
            Arg::with_name("endless")
                .short('e')
                .long("endless")
                .about("Enters the game in Endless mode."),
        )
        .arg(
            Arg::with_name("race")
                .short('r')
                .long("race")
                .min_values(1)
                .max_values(3)
                .value_name("WORD-COUNT | LO-DIFF | HI-DIFF")
                .validator(|x| {
                    let args = x.split_whitespace().collect::<Vec<&str>>();
                    for arg in &args {
                        match arg.trim().parse::<u32>() {
                            Ok(_n) => (),
                            _ => return Err(String::from("Failed to parse arguments for Race. Please give u32 integers instead."))
                        }
                    }
                    if args.len() == 3 && args[2] < args[1] {
                        return Err(String::from("The second argument for range must be greater than or equal to the first."));
                    }
                    Ok(())
                })
                .about("Enters the game in Race mode.  Input number of words to type."),
        )
        .group(
            ArgGroup::with_name("modes")
                .args(&["time-attack", "endless", "race"])
                .required(true),
        )
        .arg(   // lower/higher score.
            Arg::with_name("skip-errors")
                .short('s')
                .long("skip-errors")
                .conflicts_with("time-attack")
                .about("When enabled, skips past errors and counts them to display at game over."),
        )
        .arg(   // allows you to see multiple words at once, so you can look ahead.
            Arg::with_name("multiple")
                .short('m')
                .long("multiple")
                .about("When enabled, higher difficulties display multiple words. Recommended."),
        )
        .arg(   // TODO: Implement char by char checking.
            Arg::with_name("classic")
                .short('c')
                .long("classic")
                .about("When enabled, typing is parsed char by char (and so are errors)."),
        )
        .arg(
            Arg::with_name("accumulate")
                .short('a')
                .long("accumulate")
                .requires("time-attack")
                .about("When enabled in Time Attack mode, allows each correct word to add to the timer until 30 seconds.")
        )
        .get_matches();

    // .unwrap() is acceptable for this purpose because CLAP requires the argument input-file.
    let words = read_file(matches.value_of("input-file").unwrap())?;
    let word_sets = parse_to_sets(words);
    let options = options(&matches);
    let mode = if matches.is_present("time-attack") {
        Mode::TimeAttack
    } else if matches.is_present("endless") {
        Mode::Endless
    } else if matches.is_present("race") {
        Mode::Race
    } else {
        // I like the explicitness of seeing all match arms like this, hence the unreachable!().
        unreachable!();
    };
    // count better be a Some() after all that bullshit I did up above.
    let count = if mode == Mode::Race {
        Some(
            matches
                .values_of("race")
                .expect("Expected valid &str input as argument to --race, preliminary.")
                .filter_map(|value| value.trim().parse::<u32>().ok())
                .collect::<Vec<u32>>(),
        )
    } else {
        None
    };

    let game: Game = Game {
        mode,
        word_sets,
        options,
        count,
    };

    let scores = play(&game).unwrap_or(Score::default());
    if scores != Score::default() {
        match give_score(&scores, &game.mode, &game.options) {
            Ok(()) => println!("\nYour score has been recorded.  Thanks for playing!"),
            Err(error) => println!("Your score was not recorded.  Error: {:?}", error),
        }
    }

    Ok(())
}

/*
5/12
-Discovered more about iterators and about use of cloned().
iter() iterates over a collection by reference, meaning that if you call a function
like filter() on an iter() iterator, you'll produce a double reference and every item will have become a reference,
which cannot be returned outside of the function.
If you use into_iter(), you don't iterate by reference, but you can't call into_iter() on one collection twice.
So, use iter().cloned().
.cloned() is a method which allows simple values to be more or less copied (copied() is a twin sister).
In the docs, cloned() is described as useful for when you produce an iter over &T, but need T.

-Learned more about opening files using OpenOptions, and returning results in main.
Can use the ? operator when doing I/O work in main().

-Learned how to use if/let with Some(x) instead of x = ....unwrap().
-Struggled CONSIDERABLY with the built-in validator() method in clap.
The method requires a return type of Result<(), String>, a generic and exceedingly specific Result type.  Needless to say,
I was kind of disappointed it wasn't possible to wrap the value given in the Result.
I at least learned how to map Result::Err to a different type, using map_err() and a closure.
I tried in three ways to check for argument validity: one, with logic in main(); two, believing there must have been something, with
a library function from clap; three, a custom function passed as an argument to that library function.

5/13
-Just realized the first real, legitimate use for underscore prefixing a variable. It's in the .validator() code.
-More comfortable using clone() to turn a &value -> value.

-I came across a design decision which is a little troublesome for me.  Which is preferable: a HashMap, or a Vec of tuples?
If I use the HashMap, I have to use .get().unwrap().clone().  If I use the Vec of tuples, then I can use indexing, or the brackets
operator - but if so, not only do I erase the names (making it less sastisfying to scale and understand the code, and brittly reliant on
consistent order of insertion) from data, I also erase the need for tuples at all - because if I use the tuples and use get() on the Vec,
I will have to use get().unwrap().clone() anyway.  So in the interest of brevity, I have decided to stick with the plain vector method.
-Follow-up: I actually suddenly come up with the infinitely better idea to use a struct.  I have shot myself already, don't worry.

5/24
-There was this almost right code, in implementing the multiple option.
if game.options.multiple {
    let mut witer = words.iter();
    println!(
        "\n{} {} {} {} {} {} {} {} {}",
        witer.next()?,
        witer.next()?,//words.get(1)?,
        witer.next()?,
        witer.next()?,
        witer.next()?,
        witer.next()?,
        witer.next()?,
        witer.next()?,
        cur
    );
} else {
    println!("\n{}", cur);
}
What it does, is prints the correct word at the end, and the upcoming word at the front.  In this way, you would see
the next word, but not the correct one - in order to get the fastest time, you would have to look at the wrong word,
while typing the correct one from your memory.  Interesting mode, isn't it?  Implement as an option.

-For a bit, I wanted to read until a space was registered, as in "space stops the read instantly."  So I replaced
the input: String with a Vec<u32> of bytes, and inserted the following code after stdin():
    bytes = io::stdin().lock().read_until(32, &mut input).unwrap_or_default();
    let input = String::from_utf8(input).unwrap_or_default();
But it doesn't work.  What actually happens, is that it reads until space, yes, but still appends everything to
the buffer.  Space just doesn't work as a line feed character.

-Spent a lot of time thinking about how to read in EOF or ESC as a character.  Realized that PWSH didn't have an easy option
like Bash did - and in fact, trying it on both shells was useless.  Rust reads until the EOF, then it simply makes another
call to read_line... it never ends.  Instead, I opted to just use TAB.

-The count of words was implemented for Race, with added possible options for difficulty range.  Had to change
count to be an Option<Vec<u32>>.

-Ran into problem with unsigned types.  Need difficulty in u8, but input could be parsed as u32 (and only can
be parsed as u32).  Decided to remove use of u32 as it wasn't required for optimality.

-The rust compiler and rust-analyzer are truly bright products.  unreachable!() actually does what it's supposed to.

-Interesting bit of cyclic dependency here.
let cur = words.pop_front()?.clone();
let difficulty = ((cur.len() as u32) % (high - low + 1)) + low;

let word: String = game
    .word_sets
    .get(&(difficulty as u32))?
    .choose(&mut rng)?
    .clone();
words.push_back(word);

So, to pop something from the vector and make cur, we need something in the vector.  To put something in the vector,
we need difficulty.  To make difficulty, we need the length of current...  And so on.  To solve this, just
put something in the vector at first, anything is fine.

5/27
-Added better formatting to scores.txt.

6/10
Made big git changes, and merged play_race and play_endless.
I am still looking into timers, how to make the temrinal print a timer without disturbing the rest of output.
I may have to look into concurrency.

-Replaced work in read_file with better error handling.  It feels like I am learning to write better code!

-Fully fleshed out play_time, adding better timing, wpm calculation, and accumulate.

-Finally! I got to use filter_map in main.

-I attempted to clean parse sets and refactor it into a for loop.
let mut nine_sets: Vec<Vec<String>> = Vec::new();
for i in 0..=8 {
    nine_sets[i] = words
        .iter()
        .cloned()
        .filter(|word| word.len() == (i + 3))
        .collect();
}
... but this gave me reference problems.  I couldn't convert any of the indexes of nine_sets into an iterator (by move).
If I used .iter(), then I would have references to Strings.  I didn't want to copy them all over.
-I try again, this time using chunks().  I also notice that the order in which I was cloning the iterators in parse_sets
was inefficient - I was cloning the entire iterator for words, rather than just the filtered part!  I don't know if there
was a difference, however.

-In write_score, I attempt at implementing some way to show the game options enabled in the mode section.
This actually marks the first usage of to_owned() - for, to concatenate to a string, you need to own it.
I tried using format!("{}{}", mode.as_ref(), " -a"), but format!() actually allocates a String.  So I reverted it back.
*/

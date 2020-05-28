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

// I could make writing this into a Rust tutorial.

// score: how many words typed correctly, in how much time
// reduced by errors - one for error skip, or one for each error
// Give randomized mistake message from list of possible prompts.
// difficulty ramps up; in word file, difficulty is measured by length of characters. max difficulty.
// multiple words at a time, separated by spaces (called multiple, could be flag or mode)
// could have "blind" flag - could have these difficulty enhancing flags which increase score`
// continue on error, or halt until getting it right.
// count errors up.

// Note: do not retrim String variables that come from the file - trim input!

// TODO: implement custom difficulty range
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

#[derive(PartialEq, Eq)]
struct Score {
    correct: u32,
    errors: u32,
    time: Option<Duration>, // for race
}

impl Score {
    pub fn default() -> Score {
        Score {
            correct: 0,
            errors: 0,
            time: None,
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

fn read_file(filename: &str) -> Vec<String> {
    let file_path = Path::new(filename);
    let file_object = OpenOptions::new()
        .read(true)
        .open(file_path)
        .expect("Expect: valid file at file path. Fail to: open file with path!");
    let file_buffer = BufReader::new(file_object);
    let words: Vec<String> = file_buffer
        .lines()
        .map(|l| l.unwrap().trim().to_string())
        .collect();
    words
}

fn parse_to_sets(words: Vec<String>) -> HashMap<u32, Vec<String>> {
    // Don't need to trim() the Strings because they've been trimmed in read_file().
    // Ultimately this long branch is more readable than using a for loop, imo.
    let three: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() == 3)
        .collect();
    let four: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() == 4)
        .collect();
    let five: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() == 5)
        .collect();
    let six: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() == 6)
        .collect();
    let seven: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() == 7)
        .collect();
    let eight: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() == 8)
        .collect();
    let nine: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() == 9)
        .collect();
    let ten: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() == 10)
        .collect();
    let eleven: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() == 11)
        .collect();
    let twelve: Vec<String> = words
        .iter()
        .cloned()
        .filter(|word| word.len() >= 12)
        .collect();

    let first: Vec<String> = three
        .into_iter()
        .chain(four.into_iter())
        .chain(five.into_iter())
        .collect();
    let second: Vec<String> = six
        .into_iter()
        .chain(seven.into_iter())
        .chain(eight.into_iter())
        .collect();
    let third: Vec<String> = nine
        .into_iter()
        .chain(ten.into_iter())
        .chain(eleven.into_iter())
        .collect();
    let fourth: Vec<String> = twelve;

    let mut word_sets: HashMap<u32, Vec<String>> = HashMap::new();

    word_sets.insert(1, first);
    word_sets.insert(2, second);
    word_sets.insert(3, third);
    word_sets.insert(4, fourth);

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
        Mode::Endless => play_endless(game),
        Mode::Race => play_race(game),
    };
}

// as difficulty increases, you get more/less time to type the word.
// Score is based on how many words you get correct before crashing.
// Since losing this mode occurs on time out, skip-err is not allowed.
// Can choose to reset timer (more score at higher diffs), or can accumulate time.

// skip errors: resets timers.  lowers score.

// average typing speed would be 4/4.5 CPS. (48 WPM)
// good typing speed would be 5.5/6 CPS.    (72 WPM)
// considerable typing speed would be 7.5/8 CPS.    (96 WPM)
// fastest typing speed would be 9 CPS.     (108 WPM, just above my max speed. only triggered after a lot of words.)
fn play_time(game: &Game) -> Option<Score> {
    count_down(3, &game.mode);

    // if game.options.multiple {
    // for _i in 0..9 {
    //     // queue up nine words, so the tenth is added in the loop.
    //     let word: String = game.word_sets.get(&difficulty)?.choose(&mut rng)?.clone();
    //     words.push_back(word);
    //     words_queued += 1;
    // }

    // difficulty = match words_queued {
    //     0..=20 => 1,
    //     21..=40 => 2,
    //     41..=60 => 3,
    //     _ => 4,
    // };

    // if input.trim_end() == cur {
    //     words_done += 1;
    //     words_queued += 1;
    //     break;
    // } else {
    //     errors += 1;
    //     match game.options.skip_err {
    //         true => break,
    //         false => continue,
    //     }
    // }

    let temp = Score {
        correct: 0,
        errors: 0,
        time: None,
    };
    Some(temp)
}

// Endless: keep selecting random words until done.
// might also record WPM.  best choice is to turn on skip-err, multiple.
// in wpm parse, errors do not add.
// Difficulty not relevant for Endless.  Opt instead to make it just choose random words.
fn play_endless(game: &Game) -> Option<Score> {
    count_down(3, &game.mode);

    let mut difficulty: u32 = 1;
    let mut words_done: u32 = 0;
    let mut errors: u32 = 0;
    let mut rng = thread_rng();
    let mut bytes;
    let mut quit = false;

    let mut words: VecDeque<String> = VecDeque::new();
    if game.options.multiple {
        for _i in 0..9 {
            // queue up nine words, so the tenth is added in the loop.
            let word: String = game.word_sets.get(&difficulty)?.choose(&mut rng)?.clone();
            difficulty = word.len() as u32 % 4 + 1;
            words.push_back(word);
        }
    } else {
        let word: String = game.word_sets.get(&1)?.choose(&mut rng)?.clone();
        words.push_back(word);
    }

    while !quit {
        let cur = words.pop_front()?.clone();
        difficulty = (cur.len() as u32 % 4) + 1;

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

    Some(Score {
        correct: words_done,
        errors,
        time: None,
    })
}

// will write your time and WPM/CPM to a file.
// average cpm: 200 CPM     (WPM is CPM/5)
// up to 300
// up to 400
// up to 500 - final difficulty
fn play_race(game: &Game) -> Option<Score> {
    count_down(3, &game.mode);

    let mut words_done: u32 = 0;
    let mut errors: u32 = 0;
    let mut rng = thread_rng();
    let mut bytes;
    let mut quit = false;

    let word_count: u32;
    let low: u32;
    let high: u32;
    if let Some(count_options) = &game.count {
        word_count = count_options[0];
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
            _ => unreachable!(),
        }
    } else {
        unreachable!();
    }

    let mut difficulty: u32 = low; // initialize queue with lowest difficulty word.
    let mut words: VecDeque<String> = VecDeque::new();
    if game.options.multiple {
        for _i in 0..9 {
            // queue up nine words, so the tenth is added in the loop.
            let word: String = game.word_sets.get(&difficulty)?.choose(&mut rng)?.clone();
            difficulty = ((word.len() as u32) % (high - low + 1)) + low;
            words.push_back(word);
        }
    } else {
        let word: String = game.word_sets.get(&low)?.choose(&mut rng)?.clone();
        words.push_back(word);
    }

    let now = Instant::now();
    while words_done != word_count && !quit {
        let cur = words.pop_front()?.clone();
        difficulty = ((cur.len() as u32) % (high - low + 1)) + low;

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
    if words_done != word_count {
        None
    } else {
        Some(Score {
            correct: words_done,
            errors,
            time: Some(now.elapsed()),
        })
    }
}

// Print score and write it to scores.txt.
fn give_score(scores: &Score, mode: &Mode) -> Result<(), std::io::Error> {
    write_score(scores, mode)?;
    println!("\x1b[2J\x1b[1;1HGood game! Your score:");
    print!("Correct: {} | Errors: {}", scores.correct, scores.errors);
    if let Some(time) = scores.time {
        println!(" | Race time: {}", time.as_secs_f32().to_string());
    }
    Ok(())
}

fn write_score(scores: &Score, mode: &Mode) -> Result<(), std::io::Error> {
    let scores_path = Path::new("scores.txt");
    let mut scores_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(scores_path)
        .expect("Expected valid path creation from \"scores.txt\".");

    write!(
        &mut scores_file,
        "\nMode: {:<10} :  Correct: {:<5} |  Errors: {:<4}",
        mode.as_ref(),
        scores.correct,
        scores.errors
    )?;

    if *mode == Mode::Race {
        let time;
        if let Some(duration) = scores.time {
            time = duration.as_secs_f32().to_string();
        } else {
            time = "N/A".to_string();
        }
        write!(&mut scores_file, " | Time: {}", time)?;
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
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
                    if args.len() == 3 && !(args[2] > args[1]) {
                        return Err(String::from("Second argument for range must be larger than first."));
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
                .about("When enabled, skips past errors and counts them to display at game over."),
        )
        .arg(   // allows you to see multiple words at once, so you can look ahead.
            Arg::with_name("multiple")
                .short('m')
                .long("multiple")
                .about("When enabled, higher difficulties display multiple words. Recommended with --accumulate."),
        )
        .arg(   // Lower score.
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

    let words = read_file(matches.value_of("input-file").unwrap());
    let word_sets = parse_to_sets(words);
    let options = options(&matches);
    let mode = if matches.is_present("time-attack") {
        Mode::TimeAttack
    } else if matches.is_present("endless") {
        Mode::Endless
    } else if matches.is_present("race") {
        Mode::Race
    } else {
        unreachable!();
    };
    // count better be a Some() after all that bullshit I did up above.
    let count = if mode == Mode::Race {
        Some(
            matches
                .values_of("race")
                .expect("Expected valid &str input as argument to --race, preliminary.")
                .map(|value| value.trim().parse::<u32>().unwrap())
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
        match give_score(&scores, &game.mode) {
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
*/

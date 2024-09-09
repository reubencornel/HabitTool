extern crate base64_url;
extern crate getopts;
extern crate serde;
extern crate serde_json;

mod colorize;
use serde::{Deserialize, Serialize};
use std::cmp::Eq;
use std::fmt;
use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Write};
use std::mem::replace;
use std::vec::Vec;

use getopts::Options;
use std::env;
/// This tool should help me create, update and delete habits from a specified habit file.
/// The habit file contains all habits represented as a JSON data structure. The structure
/// looks is illustrated below
///
/// [ { name: "habitName", execution_count: 2, execution: [0,1,-1,0,0,0,0....<49>], archived_executions: {[], [], []....} },
///   { name: "habitName", execution_count: 3, execution: [0,1,-1,0,0,0,0....<49>]},
///   .
///   .
///   .
/// ]
///
/// I need to write the following functions
/// 1. Create Habit Entry
/// 2. Update An execution
/// 3. Delete a habit
/// 4. A function to serialize and deserialize the json struct.
/// 5. Display habit executions on the command line

const MAX_EXECUTIONS: usize = 49;
static INPUT_FILE: &'static str = "/Users/reuben/habitFile.json";

#[derive(Deserialize, Serialize)]
struct HabitExecution {
    name: String,
    executions: Vec<i8>,
    // Data flow for manual update.
    // 1. The first entry by the program manual update = false.
    // 2. If I make a manual update, the value is set to true.
    // 3. The mechanial update part picks up the data at a specified point in time:
    //    a. If manual_update = true, then it does nothing.
    //    b. If manual_update = false, it calls update habit with -1
    manual_update: bool,
    archived_executions: Vec<String>,
}

impl PartialEq for HabitExecution {
    fn eq(&self, other: &HabitExecution) -> bool {
        self.name == other.name
    }
}

impl Debug for HabitExecution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(&self).unwrap())
    }
}

impl Eq for HabitExecution {}

impl HabitExecution {
    fn new(name: String) -> HabitExecution {
        HabitExecution {
            name: name,
            executions: Vec::new(),
            manual_update: false,
            archived_executions: Vec::new(),
        }
    }
}

fn add_new_habit(name: String, habits: &mut Vec<HabitExecution>) {
    habits.push(HabitExecution::new(name));
}

fn update_habit(habit: &mut HabitExecution, value: i8, manual_update: bool) {
    if habit.executions.len() >= MAX_EXECUTIONS {
        // time to setup up a new habit execution
        archive_execution(habit);
    }

    if !habit.manual_update {
        habit.executions.push(value);
    }
    habit.manual_update = manual_update;
}

// Find the first entry that has not been updated and mark it to the value specified.
fn update_execution(
    habits: &mut Vec<HabitExecution>,
    name: String,
    value: i8,
    manual_update: bool,
) -> Result<String, String> {
    // Find the habit
    for mut habit in habits.iter_mut() {
        if name == habit.name {
            update_habit(&mut habit, value, manual_update);
            return Ok(name);
        }
    }
    return Err("Habit  not found".to_string());
}

fn archive_execution(habit: &mut HabitExecution) {
    let new_array: Vec<i8> = Vec::new();
    let old_executions = replace(&mut habit.executions, new_array);

    // Map the old executions to binary.
    let mut archive_array: Vec<u8> = Vec::new();
    let mut byte_rep: u8 = 0;
    for (i, value) in old_executions.into_iter().enumerate() {
        if i != 0 && i % 8 == 0 {
            archive_array.push(byte_rep);
            byte_rep = 0;
        }

        byte_rep = byte_rep << 1;
        if value == 1 {
            byte_rep = byte_rep | 1;
        }
    }

    // Account for the last bit
    byte_rep = byte_rep << 7;
    archive_array.push(byte_rep);

    let encoded_execution = base64_url::encode(&archive_array);
    habit.archived_executions.push(encoded_execution);
}

fn deserialize_file(file_name: &String) -> Vec<HabitExecution> {
    let mut file_handle = match File::open(file_name) {
        Ok(handle) => handle,
        Err(_) => panic!("Could not find input file: {}", file_name),
    };
    let mut input: String = String::new();
    file_handle.read_to_string(&mut input).unwrap();
    serde_json::from_str(input.as_str()).unwrap()
}

fn serialize(habits: &Vec<HabitExecution>, file_name: &String) {
    let mut file_handle = match File::create(file_name) {
        Ok(handle) => handle,
        Err(_) => panic!("Could not open file: {}", file_name),
    };

    file_handle
        .write_all(serde_json::to_string(habits).unwrap().as_bytes())
        .unwrap();
}

fn get_options() -> Options {
    let mut options = Options::new();
    options.optopt("i", "input", "File that stores habit data", "FILE");
    options.optflag("h", "help", "Describes the options to this program");
    options.optflag(
        "u",
        "update",
        "Update the habit action to done. Requires -n supplied",
    );
    options.optflag(
        "m",
        "mechanical",
        "Update flag to be used by the chron program",
    );
    options.optmulti("n", "name", "Name of the habit to be updated", "NAME");
    options.optflag(
        "d",
        "display",
        "Display a pretty picture of the habit specified by name",
    );

    options
}

enum ArgumentAction<'a> {
    MechanicalUpdate(String),
    UserUpdate(Vec<String>, String),
    DisplayUsage(String, &'a Options),
    Display(Vec<String>, String),
}
// TODO parse arguments, and call the right function.

fn parse_matches<'a>(args: Vec<String>, opts: &'a Options) -> ArgumentAction<'a> {
    let program_name = args[0].clone();
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!(f.to_string())
        }
    };

    if matches.opt_present("h") {
        ArgumentAction::DisplayUsage(program_name, opts)
    } else if matches.opt_present("u") {
        if !matches.opt_present("i") {
            ArgumentAction::DisplayUsage(program_name, opts)
        } else {
            let file_name = matches.opt_str("i").unwrap();
            let habit_names = matches.opt_strs("n");

            if habit_names.len() == 0 {
                ArgumentAction::DisplayUsage(program_name, opts)
            } else {
                ArgumentAction::UserUpdate(habit_names, file_name)
            }
        }
    } else if matches.opt_present("m") {
        if !matches.opt_present("i") {
            ArgumentAction::DisplayUsage(program_name, opts)
        } else {
            let file_name = matches.opt_str("i").unwrap();
            ArgumentAction::MechanicalUpdate(file_name)
        }
    } else if matches.opt_present("d") {
        if !matches.opt_present("i") {
            ArgumentAction::DisplayUsage(program_name, opts)
        } else {
            let file_name = matches.opt_str("i").unwrap();
            let habit_names = matches.opt_strs("n");

            if habit_names.len() == 0 {
                ArgumentAction::DisplayUsage(program_name, opts)
            } else {
                ArgumentAction::Display(habit_names, file_name)
            }
        }
    } else {
        ArgumentAction::DisplayUsage(program_name, opts)
    }
}

fn display_usage(program_name: String, opts: &Options) {
    let usage = format!("Usage: {} [options]", program_name);

    print!("{}", opts.usage(&usage));
}

fn perform_user_updates(habit_names: Vec<String>, input_file: String) {
    let mut habits = deserialize_file(&input_file);
    for name in habit_names.iter() {
        update_execution(&mut habits, name.clone(), 1, true);
    }
    serialize(&habits, &input_file);
}

fn perform_mechanical_update(input_file: String) {
    let mut habits = deserialize_file(&input_file);
    let names = habits
        .iter()
        .map(|x| x.name.clone())
        .collect::<Vec<String>>();
    for name in names.iter() {
        update_execution(&mut habits, name.clone(), -1, false);
    }
    serialize(&habits, &input_file);
}

fn map_executions_to_display_chars(
    habit: &HabitExecution,
    color_mapping: &ColorMapping,
) -> Vec<String> {
    let mut diplay_today: bool = true;

    (0..49)
        .map(|x| {
            if x >= habit.executions.len() {
                if diplay_today == true && habit.manual_update == false {
                    // Display T if the habit has not been executed today
                    // Lets me know which day I'm executing
                    diplay_today = false;
                    color_mapping.get_string_with_color("T", "orange", "")
                } else {
                    color_mapping.get_string_with_color(".", "blue", "")
                }
            } else {
                // If I've executed the habit display "X", "O" otherwise
                if habit.executions[x] == 1 {
                    color_mapping.get_string_with_color("X", "green", "")
                } else {
                    color_mapping.get_string_with_color("O", "red", "")
                }
            }
        })
        .collect::<Vec<String>>()
}

/// Notes about this function
/// I want this function to display habits in
/// Meditation    | Coding
/// X X X X X X O |
/// X X X O X X X |
/// X X X . . . . |
use colorize::colorize::ColorMapping;

fn display_habits(habit_names: Vec<String>, input_file: String) {
    // find the habits from habit vector.
    // For the habit found, map it to an output string
    let mut collected_strings: Vec<Vec<String>> = Vec::new();
    let mut output_strings: Vec<String> = Vec::new();
    let color_mapping = ColorMapping::new();
    let habits = deserialize_file(&input_file);

    let mut first_line = String::new();
    for habit_name in habit_names.iter() {
        for habit in habits.iter() {
            if habit.name == habit_name.as_str() {
                collected_strings.push(map_executions_to_display_chars(&habit, &color_mapping));
                let formatted_habit_name = habit_name.chars().take(15).collect::<String>();
                first_line = first_line + format!(" |{:<15}| ", formatted_habit_name).as_str();
            }
        }
    }
    output_strings.push(first_line);

    let mut index = 0;

    for _ in 0..(MAX_EXECUTIONS / 7) {
        let mut str1: String = String::new();
        for string in collected_strings.iter() {
            str1 = str1 + " |";
            for k in index..(index + 7) {
                str1 = str1 + " " + string[k].as_str();
            }
            str1 = str1 + " | ";
        }
        index = index + 7;
        output_strings.push(str1);
    }

    println!("{}", output_strings.join("\n"));
}

fn perform_action(action: ArgumentAction) {
    match action {
        ArgumentAction::DisplayUsage(program_name, opts) => display_usage(program_name, opts),
        ArgumentAction::UserUpdate(habit_names, file_name) => {
            perform_user_updates(habit_names, file_name)
        }
        ArgumentAction::MechanicalUpdate(file_name) => perform_mechanical_update(file_name),
        ArgumentAction::Display(habit_names, file_name) => display_habits(habit_names, file_name),
    }
}

fn main() {
    let opts = get_options();
    let args: Vec<String> = env::args().collect();
    let action = parse_matches(args, &opts);
    perform_action(action);
}

#[cfg(test)]
mod test {
    use super::update_execution;
    use super::HabitExecution;
    use rustc_serialize::json;

    #[test]
    fn create_new_habit_execution() {
        let execution = HabitExecution::new("Test".to_string());
        assert!(execution.name == "Test".to_string());
        assert!(execution.archived_executions.len() == 0);
    }

    #[test]
    fn test_two_habits_are_equal() {
        let execution = HabitExecution::new("Test".to_string());
        let execution2 = HabitExecution::new("Test".to_string());
        assert!(execution == execution2);
    }

    #[test]
    fn test_update_basic_habit() {
        let mut list_of_habits: Vec<HabitExecution> = vec![HabitExecution::new("Test".to_string())];
        update_execution(&mut list_of_habits, "Test".to_string(), -1, false);
        println!("{:?}", list_of_habits[0]);
        assert!(list_of_habits[0].executions[0] == -1);

        update_execution(&mut list_of_habits, "Test".to_string(), -1, false);
        println!("{:?}", list_of_habits[0]);
        assert!(list_of_habits[0].executions[1] == -1);
    }

    #[test]
    fn test_mechanical_update() {
        let mut habit = HabitExecution::new("Test".to_string());
        habit.manual_update = true;
        let mut list_of_habits: Vec<HabitExecution> = vec![habit];
        update_execution(&mut list_of_habits, "Test".to_string(), -1, false);

        assert!(list_of_habits[0].manual_update == false);
        assert!(list_of_habits[0].executions.len() == 0);

        update_execution(&mut list_of_habits, "Test".to_string(), 1, true);

        assert!(list_of_habits[0].manual_update == true);
        assert!(list_of_habits[0].executions.len() == 1);

        update_execution(&mut list_of_habits, "Test".to_string(), -1, false);
        assert!(list_of_habits[0].manual_update == false);
        assert!(list_of_habits[0].executions.len() == 1);
    }

    #[test]
    fn archive_habit_execution() {
        let mut habit = HabitExecution::new("Test".to_string());
        habit.executions.push(1);
        habit.executions.push(1);
        habit.executions.push(-1);
        for _ in 0..4 {
            habit.executions.push(0);
        }
        habit.executions.push(1);
        habit.executions.push(1);
        for _ in 0..38 {
            habit.executions.push(0);
        }
        habit.executions.push(1);
        habit.executions.push(1);

        let mut list_of_habits: Vec<HabitExecution> = vec![habit];
        update_execution(&mut list_of_habits, "Test".to_string(), 1, false);
        println!("{:?}", list_of_habits[0]);
        assert!(list_of_habits[0].executions[0] == 1);
        assert!(list_of_habits[0].archived_executions.len() == 1);
        assert!(list_of_habits[0].archived_executions[0] == "wYAAAAABgA");

        let mut decoded_string = json::encode(&list_of_habits).unwrap();
        assert!("[{\"name\":\"Test\",\"executions\":[1],\"manual_update\":false,\"archived_executions\":[\"wYAAAAABgA\"]}]" == decoded_string);

        update_execution(&mut list_of_habits, "Test".to_string(), -1, false);
        decoded_string = json::encode(&list_of_habits).unwrap();
        println!("{}", decoded_string);
        assert!("[{\"name\":\"Test\",\"executions\":[1,-1],\"manual_update\":false,\"archived_executions\":[\"wYAAAAABgA\"]}]" == decoded_string);

        let h1: Vec<HabitExecution> =  json::decode("[{\"name\":\"Test\",\"executions\":[1,1],\"manual_update\":true,\"archived_executions\":[\"wYAAAAABgA\"]}]").unwrap();
        assert!(h1[0].executions[0] == 1);
        assert!(h1[0].archived_executions[0] == "wYAAAAABgA");
    }
}

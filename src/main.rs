use std::{
    collections::HashMap,
    env::args,
    fmt::Display,
    fs,
    io::{Write, stdin, stdout},
    process::exit,
    thread,
    time::Duration,
};

use rand::Rng;

#[derive(Debug)]
enum LangError {
    NoFilename,
    FileNotFound,
    NoCmdNamed(String),
    NotEnoughArgs,
    NoWaypoint(String),
    NotInteger,
    ZeroDivision,
    UnknownOperation,
}

impl Display for LangError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            LangError::NoFilename => "No filename given".to_string(),
            LangError::FileNotFound => "No file named".to_string(),
            LangError::NoCmdNamed(cmd) => format!("No command named {cmd}"),
            LangError::NotEnoughArgs => "Not enough args".to_string(),
            LangError::NoWaypoint(waypoint) => format!("No waypoint named `{waypoint}`"),
            LangError::NotInteger => "Not an integer".to_string(),
            LangError::ZeroDivision => "Can't divide through zero".to_string(),
            LangError::UnknownOperation => "Unknown operation".to_string(),
        };

        write!(f, "{message}")
    }
}

type Line = usize;

fn main() {
    if let Err((err, line)) = interpret() {
        error(err, line);
    }
}

fn interpret() -> Result<(), (LangError, Option<Line>)> {
    let filename = args().nth(1).ok_or((LangError::NoFilename, None))?;
    let file = fs::read_to_string(filename).map_err(|_| (LangError::FileNotFound, None))?;
    let code: Vec<String> = file
        .lines()
        .map(|line| line.trim_start().to_string())
        .collect();

    let mut waypoints: HashMap<String, Line> = HashMap::new();

    for (line, content) in code.iter().enumerate() {
        let mut parts = content.split(' ');

        if let Some(keyword) = parts.next()
            && keyword.to_lowercase() == "at"
        {
            waypoints.insert(parts.next().ok_or_args_err(&line)?.to_string(), line);
        }
    }

    let mut variables = HashMap::from([
        ("%OPEN".to_string(), "{{".to_string()),
        ("%CLOES".to_string(), "}}".to_string()),
    ]);
    let mut is_running = true;
    let mut cursor = 0;
    let mut lastpos: Vec<Line> = Vec::new();

    let mut rng = rand::rng();

    while is_running
        && cursor < code.len()
        && let Some(line) = code.get(cursor)
    {
        let mut line = line.clone();

        loop {
            let Some(start) = line.find("{{") else {
                break;
            };

            let Some(mut end) = line.find("}}") else {
                break;
            };
            end += 2;

            let variable_name = &line[start + 2..end - 2];

            if let Some(replacement) = variables.get(variable_name) {
                line.replace_range(start..end, replacement);
            }
        }

        let mut line = line.split(' ');

        let cmd = line.next().ok_or_args_err(&cursor)?.to_lowercase();
        let cmd = cmd.trim();

        let args: Vec<&str> = line.collect();

        match cmd {
            _ if cmd.starts_with("//") => {}

            "quit" | "exit" => is_running = false,

            "if" => {
                let condition = match args[1] {
                    "==" | "eq" => args[0] == args[2],
                    "!=" | "noteq" => args[0] != args[2],
                    _ => return Err((LangError::UnknownOperation, Some(cursor))),
                };

                if condition {
                    match args[3] {
                        "t" => {
                            match waypoints.get(args[4]) {
                                Some(waypoint_position) => {
                                    lastpos.push(cursor);
                                    cursor = *waypoint_position;
                                }
                                None => {
                                    return Err((
                                        LangError::NoWaypoint(args[4].to_string()),
                                        Some(cursor),
                                    ));
                                }
                            }

                            continue;
                        }

                        "r" => {
                            cursor = lastpos.remove(lastpos.len() - 1) + 1;
                            continue;
                        }

                        "c" => {
                            cursor = args[4].parse().ok_or_not_int_err(&cursor)?;
                            continue;
                        }

                        _ => {}
                    }
                }
            }

            "at" => {}

            "to" => {
                match waypoints.get(args[0]) {
                    Some(waypoint_position) => {
                        lastpos.push(cursor);
                        cursor = *waypoint_position;
                    }
                    None => return Err((LangError::NoWaypoint(args[0].to_string()), Some(cursor))),
                }

                continue;
            }

            "ret" => {
                cursor = lastpos.last().ok_or_args_err(&cursor)? + 1;
                lastpos.remove(lastpos.len() - 1);
                continue;
            }

            "cursor" => {
                let Ok(new_cursor_position) = args.first().ok_or_args_err(&cursor)?.parse() else {
                    return Err((LangError::NotInteger, Some(cursor)));
                };

                cursor = new_cursor_position;
                continue;
            }

            "set" => {
                let value = args
                    .iter()
                    .skip(1)
                    .fold(String::new(), |result, val| result + val);

                variables.insert(
                    args.first().ok_or_args_err(&cursor)?.trim().to_string(),
                    value,
                );
            }

            "input" => {
                let mut buffer = String::new();

                let prompt = args
                    .iter()
                    .skip(1)
                    .fold(String::new(), |result, val| result + " " + val);

                print!("{prompt}");
                stdout().flush().unwrap();
                stdin().read_line(&mut buffer).unwrap();

                variables.insert(
                    args.first().ok_or_args_err(&cursor)?.to_string(),
                    buffer.trim().to_string(),
                );
            }

            "rand" => {
                let mut args_iter = args.iter().skip(1);

                let first_number = args_iter
                    .next()
                    .ok_or_args_err(&cursor)?
                    .parse()
                    .ok_or_not_int_err(&cursor)?;

                let second_number = args
                    .get(3)
                    .map(|num| num.parse().ok_or_not_int_err(&cursor))
                    .transpose()?;

                let range = match second_number {
                    Some(range_end) => first_number..range_end,
                    None => 0..first_number,
                };

                let random_number = rng.random_range(range);

                variables.insert(
                    args.first().ok_or_args_err(&cursor)?.to_string(),
                    random_number.to_string(),
                );
            }

            "math" => {
                let first_number: f32 = args[2].parse().ok_or_not_int_err(&cursor)?;
                let second_number = args
                    .get(3)
                    .ok_or_args_err(&cursor)
                    .and_then(|arg| arg.parse::<f32>().ok_or_not_int_err(&cursor));

                let result = match args[1] {
                    "add" => first_number + second_number?,

                    "sub" => first_number - second_number?,

                    "mul" => first_number * second_number?,

                    "div" => {
                        let second_number = second_number?;

                        if second_number == 0.0 {
                            return Err((LangError::ZeroDivision, Some(cursor)));
                        } else {
                            first_number / second_number
                        }
                    }

                    "mod" => first_number % second_number?,

                    "pow" => first_number.powf(second_number?),

                    "sqrt" => first_number.sqrt(),

                    _ => return Err((LangError::UnknownOperation, Some(cursor))),
                };

                variables.insert(
                    args.first().ok_or_args_err(&cursor)?.to_string(),
                    result.to_string(),
                );
            }

            "wait" => {
                let seconds = args
                    .first()
                    .ok_or_args_err(&cursor)?
                    .parse()
                    .ok_or_not_int_err(&cursor)?;
                thread::sleep(Duration::from_secs_f32(seconds));
            }

            "out" => {
                if let Some(first_arg) = args.first()
                    && first_arg.starts_with("end=")
                {
                    let end = args
                        .first()
                        .ok_or_args_err(&cursor)?
                        .chars()
                        .skip(4)
                        .fold(String::new(), |result, val| format!("{result}{val}"));

                    let value = args.into_iter().skip(1).collect::<Vec<&str>>().join(" ");

                    print!("{value}{end}",);
                } else {
                    let value = args.join(" ");

                    println!("{value}");
                }
            }

            "clear_term" => {
                clearscreen::clear().expect("failed to clear screen");
            }

            "" => {}

            _ => return Err((LangError::NoCmdNamed(cmd.to_string()), Some(cursor))),
        }

        cursor += 1;
    }

    Ok(())
}

trait UnwrapOrLangError<T> {
    fn ok_or_args_err(self, cursor: &usize) -> Result<T, (LangError, Option<Line>)>;
    fn ok_or_not_int_err(self, cursor: &usize) -> Result<T, (LangError, Option<Line>)>;
}

impl<T> UnwrapOrLangError<T> for Option<T> {
    fn ok_or_args_err(self, cursor: &usize) -> Result<T, (LangError, Option<Line>)> {
        self.ok_or((LangError::NotEnoughArgs, Some(*cursor)))
    }

    fn ok_or_not_int_err(self, cursor: &usize) -> Result<T, (LangError, Option<Line>)> {
        self.ok_or((LangError::NotInteger, Some(*cursor)))
    }
}

impl<T, E> UnwrapOrLangError<T> for Result<T, E> {
    fn ok_or_args_err(self, cursor: &usize) -> Result<T, (LangError, Option<Line>)> {
        self.map_err(|_| (LangError::NotEnoughArgs, Some(*cursor)))
    }

    fn ok_or_not_int_err(self, cursor: &usize) -> Result<T, (LangError, Option<Line>)> {
        self.map_err(|_| (LangError::NotInteger, Some(*cursor)))
    }
}

fn error(reason: LangError, line: Option<Line>) {
    match line {
        Some(line) => eprintln!("Error on line {line}: {reason}"),
        None => eprintln!("Error: {reason}"),
    }

    exit(1);
}

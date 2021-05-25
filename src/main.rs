use std::env;
use std::process;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use cron::Schedule;
use seahorse::App;

struct Error {
    message: String,
    returncode: i32,
}
impl Error {
    pub fn new<S: AsRef<str>>(message: S) -> Self {
        Self {
            message: message.as_ref().into(),
            returncode: 1,
        }
    }

    pub fn new_with_code<S: AsRef<str>>(message: S, returncode: i32) -> Self {
        Self {
            message: message.as_ref().into(),
            returncode,
        }
    }
}

const DESCRIPTION: &str = "
\tTiny replacement if you just need a single cronjob.

\tTakes two arguments: A cron-compatible schedule, and the command to be executed.
\tThe command is executed directly (no shell involved). If you need more complex
\tlogic, wrap your workflow into a shell script.

Behavior:

\tIn case the command fails, the entire cron daemon will be crashed with the same return code.
\tThis allows your process supervisor to be noticed about this problem.
\tstdout/stderr of the command are forwarded as well.
\tCommands will not be executed in parallel: The daemon waits for one to end first.
\tAll environment variables are inherited to the child process.

Examples:

\tnanocron '* * * * * *' '/bin/command-to-run-every-second'
\tnanocron '0 * * * * *' '/bin/command-to-run-every-minute'
\tnanocron '0 0 * * * *' '/bin/command-to-run-every-hour'
\tnanocron '0 0 0 * * *' '/bin/command-to-run-every-day'
\tnanocron '0 0 0 1 * *' '/bin/command-to-run-every-first-of-the-month'
";

fn main() {
    let args: Vec<String> = env::args().collect();
    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(DESCRIPTION)
        .version(env!("CARGO_PKG_VERSION"))
        .usage("nanocron [schedule] [command]")
        .action(|c| {
            if let Err(e) = run(&c.args) {
                eprintln!("[nanocron] Error: {}", e.message);
                process::exit(e.returncode);
            }
        });

    app.run(args);
}

///
/// Run the actual application.
///
fn run(args: &[String]) -> Result<(), Error> {
    if args.is_empty() {
        return Err(Error::new(
            "Missing both required arguments (schedule + command). Use --help to get more information.",
        ));
    }

    if args.len() != 2 {
        return Err(Error::new(
            "Expecting exactly 2 arguments: The schedule, and the command to execute. Use --help to get more information.",
        ));
    }

    let expression: String = args[0].clone();

    let schedule: Schedule = match Schedule::from_str(&expression) {
        Ok(s) => s,
        Err(e) => {
            return Err(Error::new(format!(
                "Invalid schedule '{}': {:?}",
                expression, e
            )));
        }
    };

    let command: String = args[1].clone();

    let mut command_args: Vec<String> = match shell_words::split(&command) {
        Ok(p) => p,
        Err(e) => {
            return Err(Error::new(format!(
                "Failed to properly split command '{}': {:?}",
                command, e
            )));
        }
    };
    let program = command_args.remove(0);

    println!("[nanocron] Starting up");
    println!("[nanocron] schedule = {}", expression);
    println!("[nanocron] command  = {}", command);

    let mut next = match schedule.upcoming(Utc).next() {
        Some(n) => n,
        None => {
            return Err(Error::new("Failed to calculate next execution time"));
        }
    };

    loop {
        let now: DateTime<Utc> = Utc::now();

        if next > now {
            let mut wait_time = match (next - now).to_std() {
                Ok(w) => w,
                Err(e) => {
                    return Err(Error::new(format!(
                        "Failed to calculate wait time: {:?}",
                        e
                    )));
                }
            };

            // We never sleep longer than 5 minutes to ensure that if something
            // funky happens with the system time, we're able to recover somehow.
            if wait_time.as_secs() > 300 {
                wait_time = Duration::from_secs(300);
            }

            println!(
                "[nanocron] Waiting {:?} | Next planned execution: {:?}",
                wait_time, next
            );
            thread::sleep(wait_time);
        } else {
            println!("[nanocron] Executing: '{}'", command);

            let spawned = Command::new(&program)
                .args(&command_args)
                .stdin(Stdio::null())
                .spawn();
            match spawned {
                Ok(mut child) => match child.wait() {
                    Ok(returncode) => {
                        if !returncode.success() {
                            match returncode.code() {
                                Some(code) => {
                                    return Err(Error::new_with_code(
                                        format!("Child exited with code {}", code),
                                        code,
                                    ));
                                }
                                None => {
                                    return Err(Error::new(
                                        "Child process was killed using a signal",
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(Error::new(format!(
                            "Failed to execute binary '{}': {:?}",
                            program, e
                        )));
                    }
                },
                Err(e) => {
                    return Err(Error::new(format!(
                        "Failed to execute binary '{}': {:?}",
                        program, e
                    )));
                }
            }

            next = match schedule.upcoming(Utc).next() {
                Some(n) => n,
                None => {
                    return Err(Error::new("Failed to calculate next execution time"));
                }
            };
        }
    }
}

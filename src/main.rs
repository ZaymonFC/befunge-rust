use std::{
    fs::File,
    io::BufRead,
    io::{self},
    marker::PhantomData,
    path::Path,
};

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    match File::open(filename) {
        Ok(file) => Ok(io::BufReader::new(file).lines()),
        Err(e) => Err(e),
    }
}

fn parse_program(filename: &str) -> Vec<String> {
    read_lines(filename)
        .expect("Something went wrong reading the source")
        .collect::<Result<Vec<_>, _>>()
        .expect("Single line failed to unwrap?")
}

fn put_ret<T>(v: Vec<T>, val: T) -> Vec<T>
where
    T: Clone,
{
    let mut d = v;
    d.push(val);
    d
}

struct ProgramPosition {
    row: i32,
    col: i32,
}

#[derive(Clone, Copy, Debug)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn get_move(d: Self) -> ProgramPosition {
        match d {
            Direction::Up => ProgramPosition { row: -1, col: 0 },
            Direction::Right => ProgramPosition { row: 0, col: 1 },
            Direction::Down => ProgramPosition { row: 1, col: 0 },
            Direction::Left => ProgramPosition { row: 0, col: -1 },
        }
    }
}

#[derive(Debug, Clone)]
enum Operator {
    // Data Pushers - Constructors?
    PushDigit(u8), // 0-9 Push this number on the stack
    PushCharacter(char),

    // Operators
    Addition,       // +	Addition: Pop a and b, then push a+b
    Subtraction,    // -	Subtraction: Pop a and b, then push b-a
    Multiplication, // *	Multiplication: Pop a and b, then push a*b
    Division,       // /	Integer division: Pop a and b, then push b/a, rounded towards 0.

    ToggleStringMode, // start/stop interpreting program data as a string on ""

    Pop,
    Duplicate,
    PopMoveHorizontal,
    PopMoveVertical,

    SetDirection(Direction),

    NoOp,
    Unknown(char),
    End,
}

#[derive(Clone, Copy, Debug)]
enum ReaderMode {
    Normal,
    String,
}

#[derive(Clone, Copy, Debug)]
enum StackData {
    Digit(i64),
    Ascii(char),
}

impl StackData {
    fn get_char(s: Self) -> char {
        match s {
            StackData::Ascii(c) => c,
            StackData::Digit(d) => (d as u8) as char,
        }
    }

    fn get_int(s: Self) -> i64 {
        match s {
            StackData::Ascii(c) => c as i64,
            StackData::Digit(d) => d,
        }
    }
}

impl From<Operator> for StackData {
    fn from(op: Operator) -> Self {
        match op {
            Operator::PushCharacter(c) => StackData::Ascii(c),
            Operator::PushDigit(d) => StackData::Digit(d as i64),
            _ => panic!("This operation cannot push to the stack"),
        }
    }
}

fn parse_operator(reader_mode: ReaderMode, c: char) -> Operator {
    match reader_mode {
        ReaderMode::Normal => match c {
            '0'..='9' => Operator::PushDigit(c.to_digit(10).unwrap() as u8),
            ' ' => Operator::NoOp,

            '+' => Operator::Addition,
            '-' => Operator::Subtraction,
            '*' => Operator::Multiplication,
            '/' => Operator::Division,

            '\"' => Operator::ToggleStringMode,

            ':' => Operator::Duplicate,

            ',' => Operator::Pop,
            '_' => Operator::PopMoveHorizontal,
            '|' => Operator::PopMoveVertical,

            '>' => Operator::SetDirection(Direction::Right),
            '<' => Operator::SetDirection(Direction::Left),
            '^' => Operator::SetDirection(Direction::Up),
            'v' => Operator::SetDirection(Direction::Down),

            '@' => Operator::End,

            c => Operator::Unknown(c),
        },
        ReaderMode::String => match c {
            '\"' => Operator::ToggleStringMode,
            _ => Operator::PushCharacter(c),
        },
    }
}

fn mathematical_operation<F>(stack: Vec<StackData>, operation: F) -> Vec<StackData>
where
    F: Fn(i64, i64) -> i64,
{
    let mut data = stack;
    let opx = data.pop();
    let opy = data.pop();

    match (opx, opy) {
        (Some(StackData::Digit(a)), Some(StackData::Digit(b))) => {
            data.push(StackData::Digit(operation(a, b)));
            data
        }
        _ => {
            panic!("Attempted to do math with: {:?} {:?}", opx, opy);
        }
    }
}

#[derive(Clone, Debug)]
struct InterpreterState {
    direction: Direction,
    row: i32,
    col: i32,
    mode: ReaderMode,
    stack: Vec<StackData>,
    program: Vec<String>,
    output: Vec<char>,
    terminated: bool,
}

impl InterpreterState {
    fn new(program: Vec<String>) -> Self {
        Self {
            direction: Direction::Right,
            row: 0,
            col: 0,
            mode: ReaderMode::Normal,
            stack: Vec::new(),
            program,
            output: Vec::new(),
            terminated: false,
        }
    }
}

fn get_operator(
    program: Vec<String>,
    ProgramPosition { row, col }: ProgramPosition,
    reader_mode: ReaderMode,
) -> Operator {
    let line = program.get(row as usize).expect("Valid Line");
    let operator = line.chars().nth(col as usize).expect("Valid column");

    parse_operator(reader_mode, operator)
}

trait Interpretable<S, Op> {
    fn interpret(s: S, op: Op) -> S;
}

#[derive(Debug)]
struct Interpreter<State, Op> {
    state: State,
    _op: PhantomData<Op>,
}

impl Interpretable<InterpreterState, Operator> for Interpreter<InterpreterState, Operator> {
    fn interpret(state: InterpreterState, operator: Operator) -> InterpreterState {
        let partial_update = match operator {
            Operator::PushDigit(d) => InterpreterState {
                stack: {
                    let mut next = state.stack.clone();
                    next.push(StackData::Digit(d as i64));
                    next
                },
                ..state
            },
            Operator::PushCharacter(c) => {
                let mut new_stack = state.stack.clone();
                new_stack.push(StackData::Ascii(c));

                InterpreterState {
                    stack: {
                        let mut next = state.stack.clone();
                        next.push(StackData::Ascii(c));
                        next
                    },
                    ..state
                }
            }
            Operator::Addition => InterpreterState {
                stack: mathematical_operation(state.stack, |x, y| x + y),
                ..state
            },
            Operator::Subtraction => InterpreterState {
                stack: mathematical_operation(state.stack, |x, y| x - y),
                ..state
            },
            Operator::Multiplication => InterpreterState {
                stack: mathematical_operation(state.stack, |x, y| x * y),
                ..state
            },
            Operator::Division => InterpreterState {
                stack: mathematical_operation(state.stack, |x, y| x / y),
                ..state
            },

            Operator::Duplicate => {
                let mut new_stack = state.stack.clone();
                new_stack.push(new_stack.last().expect("Nothing to duplicate").to_owned());

                InterpreterState {
                    stack: new_stack,
                    ..state
                }
            }

            Operator::Pop => {
                let mut new_stack = state.stack.clone();
                let out = new_stack.pop().expect("No value to Pop");
                let out = StackData::get_char(out);

                InterpreterState {
                    stack: new_stack,
                    output: put_ret(state.output, out),
                    ..state
                }
            }
            Operator::PopMoveHorizontal => {
                let mut new_stack = state.stack.clone();
                let out = new_stack.pop().unwrap_or(StackData::Digit(0));
                let out = StackData::get_int(out);

                InterpreterState {
                    stack: new_stack,
                    direction: match out {
                        0 => Direction::Right,
                        _ => Direction::Left,
                    },
                    ..state
                }
            }
            Operator::PopMoveVertical => {
                let mut new_stack = state.stack.clone();
                let out = new_stack.pop().unwrap_or(StackData::Digit(0));
                let out = StackData::get_int(out);

                InterpreterState {
                    stack: new_stack,
                    direction: match out {
                        0 => Direction::Down,
                        _ => Direction::Up,
                    },
                    ..state
                }
            }
            Operator::ToggleStringMode => InterpreterState {
                mode: {
                    match state.mode {
                        ReaderMode::String => ReaderMode::Normal,
                        ReaderMode::Normal => ReaderMode::String,
                    }
                },
                ..state
            },
            Operator::SetDirection(direction) => InterpreterState { direction, ..state },
            Operator::NoOp => state,
            Operator::End => InterpreterState {
                terminated: true,
                ..state
            },
            Operator::Unknown(c) => panic!("We didn't know what to do here. Operator: {}", c),
        };

        let mv = Direction::get_move(partial_update.direction);

        InterpreterState {
            row: partial_update.row + mv.row,
            col: partial_update.col + mv.col,
            stack: partial_update.stack.clone(),
            output: partial_update.output.clone(),
            program: partial_update.program.clone(),
            ..partial_update
        }
    }
}

impl Iterator for Interpreter<InterpreterState, Operator> {
    type Item = InterpreterState;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.state.terminated {
            let position = ProgramPosition {
                row: self.state.row,
                col: self.state.col,
            };

            let operator = get_operator(self.state.program.clone(), position, self.state.mode);

            let new_state = Self::interpret(self.state.clone(), operator);
            self.state = new_state;

            Some(self.state.clone())
        } else {
            None
        }
    }
}

fn main() {
    let filename = "./hello-world.bf";
    let program = parse_program(filename);

    let interpreter = Interpreter {
        state: InterpreterState::new(program),
        _op: PhantomData::<Operator>,
    };

    for state in interpreter {
        println!(
            "Row: {}, Col: {}, Stack: {:?} Output: {:?}",
            state.row, state.col, state.stack, state.output
        )
    }
}

use std::{env, fs, io::stdin, process::exit};

use owo_colors::OwoColorize;

#[derive(Debug)]
struct SandWormInterpreter {
	program: Vec<Vec<u8>>,
	width: usize,
	height: usize,
	/// worm body locations
	worm: Vec<(usize, usize)>,
	worm_head: (usize, usize),
	/// queue for outputting commands at the back of the worm
	worm_out: Vec<u8>,
	worm_in: Vec<u8>,
	direction: Direction,
	input: Vec<u8>,
	input_index: usize,
	output: Vec<u8>,
	state: State,
}

#[derive(Debug, Default)]
enum Direction {
	Up,
	Down,
	Left,
	#[default]
	Right,
}

#[derive(Debug, Default, PartialEq)]
enum State {
	#[default]
	Running,
	EndOfProgram,
}

fn main() {
	let args: Vec<_> = env::args().collect();
	if args.len() <= 1 {
		println!("usage: worm source_file [input_file]");
		exit(0);
	}
	let filename = &args[1];
	let source = fs::read_to_string(filename).unwrap_or_else(|err| {
		println!("Error reading file: {err}");
		exit(1);
	});
	let input_data = args
		.get(2)
		.map(|path| {
			fs::read(path).unwrap_or_else(|err| {
				println!("Error reading file: {err}");
				exit(1);
			})
		})
		.unwrap_or_default();

	let mut interpreter = SandWormInterpreter::new(&source, input_data);

	loop {
		interpreter.show();
		let mut input_text = String::new();
		stdin().read_line(&mut input_text).unwrap();
		let action: Vec<_> = input_text.trim().split_ascii_whitespace().collect();
		if input_text.starts_with("input ") {
			interpreter.input.extend(
				&input_text
					.strip_suffix('\n')
					.unwrap_or(&input_text)
					.as_bytes()[6..],
			);
			continue;
		}
		match action.as_slice() {
			[] | ["step"] => interpreter.step_once(),
			["step", num] => _ = num.parse().map(|n| interpreter.step(n)),
			// ["run"] => interpreter.run(),
			["q" | "exit" | "quit"] => break,

			_ => println!("{}", "unrecognised command".red()),
		}
	}
}

impl SandWormInterpreter {
	fn new(source: &str, input: Vec<u8>) -> Self {
		let (program, start_pos) = parse(source);

		Self {
			width: program[0].len(),
			height: program.len(),
			program,
			worm: Vec::new(),
			worm_head: start_pos,
			worm_in: Vec::new(),
			worm_out: Vec::new(),
			input,
			output: Vec::new(),
			state: State::default(),
			direction: Direction::default(),
			input_index: 0,
		}
	}

	fn step(&mut self, n: usize) {
		for _ in 0..n {
			if self.state != State::Running {
				break;
			}
			self.step_once();
		}
	}

	fn show(&self) {
		dbg!(&self);
		println!(
			"{:?}",
			self.worm.iter().map(|p| self.get(*p)).collect::<Vec<_>>()
		);
		for (row, line) in self.program.iter().enumerate() {
			for (col, &byte) in line.iter().enumerate() {
				if self.worm.contains(&(col, row)) {
					if byte < 10 {
						print!("{:x}", byte.on_green());
					} else {
						print!("{}", "*".green().on_red());
					}
				} else if self.worm_head == (col, row) {
					if byte == b'@' {
						print!("{}", "@".on_yellow());
					} else {
						panic!("worm head corrupted");
					}
				} else if byte == 0 || byte == b' ' {
					print!(" ");
				} else if byte.is_ascii_alphanumeric() || byte.is_ascii_punctuation() {
					print!("{}", byte as char);
				} else {
					print!("{}", "*".green());
				}
			}
			println!();
		}
		println!("output: {}", String::from_utf8_lossy(&self.output));
		println!("input: {}", String::from_utf8_lossy(&self.input));
	}

	fn step_once(&mut self) {
		if self.state != State::Running {
			return;
		}
		let front = self.front();
		if front.0 >= self.width || front.1 >= self.height {
			self.state = State::EndOfProgram;
			return;
		}
		let instruction = self.get(front);
		let mut dont_push_instruction = false;

		match instruction {
			b'0'..=b'9' => {
				self.worm_in.push(instruction - 48);
			}
			b'+' => {
				let a = self.shrink();
				self.worm_out.insert(0, instruction);
				let b = self.shrink();
				dont_push_instruction = true;
				self.worm_in.push(a.wrapping_add(b));
			}
			b'-' => {
				let a = self.shrink();
				self.worm_out.insert(0, instruction);
				dont_push_instruction = true;
				let b = self.shrink();
				self.worm_in.push(a.wrapping_sub(b));
			}
			b'v' => self.direction = Direction::Down,
			b'^' => self.direction = Direction::Up,
			b'<' => self.direction = Direction::Left,
			b'>' => self.direction = Direction::Right,
			b'"' => {
				let n = self.shrink();
				self.output.extend(n.to_string().as_bytes());
			}
			b'!' => {
				let n = self.shrink();
				self.output.push(n);
			}
			b'?' => {
				let val = self
					.input
					.get(self.input_index)
					.copied()
					.unwrap_or_default();
				self.input_index += 1;
				self.worm_in.push(val);
			}
			b'=' => {
				let last_val = self.worm.last().map(|&p| self.get(p)).unwrap_or_default();
				self.worm_in.push(last_val);
			}
			b'\\' => {
				let val = self.shrink();
				if val != 0 {
					self.direction = match self.direction {
						Direction::Up => Direction::Left,
						Direction::Down => Direction::Right,
						Direction::Left => Direction::Up,
						Direction::Right => Direction::Down,
					}
				}
			}
			b'/' => {
				let val = self.shrink();
				if val != 0 {
					self.direction = match self.direction {
						Direction::Up => Direction::Right,
						Direction::Down => Direction::Left,
						Direction::Left => Direction::Down,
						Direction::Right => Direction::Up,
					}
				}
			}
			b' ' | 0 => dont_push_instruction = true,
			b'_' => self.worm_in.push(b' '),
			other => self.worm_in.push(other),
		}
		if !dont_push_instruction {
			self.worm_out.insert(0, instruction);
		}
		self.move_to(front);
	}

	fn move_to(&mut self, front: (usize, usize)) {
		if let Some(input) = self.worm_in.pop() {
			*self.get_mut(self.worm_head) = input;
			self.worm.push(self.worm_head);
		} else {
			let mut next = self.worm_head;
			for body_segment in self.worm.iter_mut().rev() {
				self.program[next.1][next.0] = self.program[body_segment.1][body_segment.0];
				(*body_segment, next) = (next, *body_segment);
			}
			*self.get_mut(next) = self.worm_out.pop().unwrap_or(b' ');
		}
		self.worm_head = front;
		*self.get_mut(front) = b'@';
	}

	/// get the front number and move the body forward (leaves the head where it was).
	/// also shits out any queued instruction
	fn shrink(&mut self) -> u8 {
		if let Some(neck) = self.worm.pop() {
			let ret = self.get(neck);
			let mut next = neck;
			for body_segment in self.worm.iter_mut().rev() {
				self.program[next.1][next.0] = self.program[body_segment.1][body_segment.0];
				(*body_segment, next) = (next, *body_segment);
			}
			*self.get_mut(next) = self.worm_out.pop().unwrap_or(b' ');
			ret
		} else {
			0
		}
	}

	fn get(&self, pos: (usize, usize)) -> u8 {
		self.program[pos.1][pos.0]
	}

	fn get_mut(&mut self, pos: (usize, usize)) -> &mut u8 {
		&mut self.program[pos.1][pos.0]
	}

	fn front(&self) -> (usize, usize) {
		let mut front = self.worm_head;
		match self.direction {
			Direction::Up => front.1 = front.1.wrapping_sub(1),
			Direction::Down => front.1 = front.1.saturating_add(1),
			Direction::Left => front.0 = front.0.wrapping_sub(1),
			Direction::Right => front.0 = front.0.saturating_add(1),
		}
		front
	}
}

fn parse(source: &str) -> (Vec<Vec<u8>>, (usize, usize)) {
	let mut program = Vec::new();
	let mut width = 0;
	let mut start_pos = (0, 0);
	for (row, line) in source.lines().enumerate() {
		width = width.max(line.len());
		if let Some(col) = line.find('@') {
			start_pos = (col, row);
		}
		program.push(line.as_bytes().to_vec());
	}
	for line in &mut program {
		line.resize(width, 0);
	}

	(program, start_pos)
}

use std::{
    fs::File,
    io::{self, Read, Write},
    process::exit,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use byte_unit::{Bit, Byte, UnitType};
use clap::{
    builder::ValueParser,
    error::Result,
    ArgAction, CommandFactory, Parser, ValueEnum,
};
use clap_complete::{generate, Shell};
use rand::Rng;

type ErrorBox = Box<dyn std::error::Error + Send + Sync + 'static>;

fn parse_buffer_size_var(s: &str) -> Result<Byte, ErrorBox> {
    let len = s.len();
    if s.chars().last() == Some('b') || (len > 3 && s[len - 3..].eq_ignore_ascii_case("bit")) {
        Err("Bit units are not allowed")?;
    }
    let b = Byte::from_str(s)?;
    let size = b.as_u128();
    if size == 0 {
        Err("Buffer size must not be 0")?;
    }
    Ok(b)
}

struct AsciiGenerator {
    // Range: [0, 94] 0..95
    index: u8,
}

impl AsciiGenerator {
    fn new() -> Self {
        AsciiGenerator { index: 0 }
    }
}

impl Read for AsciiGenerator {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = buf.len();
        for i in 0..len {
            buf[i] = 0x20 + self.index;
            self.index = if self.index < 94 { self.index + 1 } else { 0 }
        }
        Ok(len)
    }
}

struct NullGenerator {}

impl NullGenerator {
    fn new() -> Self {
        NullGenerator {}
    }
}

impl Read for NullGenerator {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // buf.fill(0);
        Ok(buf.len())
    }
}

struct MemoryGenerator {
    data: Vec<u8>,
    index: usize,
    circular: bool,
}

impl MemoryGenerator {
    fn new(bytes: Vec<u8>, circular: bool) -> Self {
        MemoryGenerator {
            data: bytes,
            index: 0,
            circular,
        }
    }
}

impl Read for MemoryGenerator {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data_size = self.data.len();
        let buffer_size = buf.len();
        let mut readed_size = 0usize;

        while readed_size < buffer_size {
            let mut remaining_size = data_size - self.index;
            if remaining_size == 0 {
                self.index = 0;
            } else {
                remaining_size = buffer_size - readed_size;
            }

            for i in 0..remaining_size {
                buf[readed_size + i] = self.data[self.index + i];
            }
            self.index += remaining_size;
            readed_size += remaining_size;
            if !self.circular {
                break;
            }
        }
        Ok(readed_size)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, ValueEnum)]
enum Generator {
    // Printable characters
    Text,
    // Null characters
    Null,
    // Random bytes
    Random,
    // Random printable characters
    RandomText,
}

fn get_io_speed(size: u128, nanos: u128) -> String {
    let b = size * 1_000_000_000 / nanos;
    let bit = Bit::from_u128(b * 8).unwrap();
    let b = Byte::from_u128(b).unwrap();
    format!(
        "{:#.2}/s, {:#.2}/s, {:#.2}/s, {:#.2}/s",
        b.get_appropriate_unit(UnitType::Binary),
        b.get_appropriate_unit(UnitType::Decimal),
        bit.get_appropriate_unit(UnitType::Binary),
        bit.get_appropriate_unit(UnitType::Decimal),
    )
}

#[derive(Parser, Debug, PartialEq)]
#[command(
    version,
    about,
    long_about,
    next_line_help = true,
    disable_version_flag = true,
)]
struct Cli {
    #[arg(short, long, help = "Input file")]
    input: Option<String>,
    #[arg(short, long, help = "Output file. Output to memory by default.")]
    output: Option<String>,
    #[arg(
        short,
        long,
        value_enum,
        value_name = "CONTENT",
        conflicts_with = "input",
        requires = "output",
        help = "Generate output content.
If it is random type, all generated into memory first;
and if count is 0, memory size only is buffer size.
"
    )]
    generator: Option<Generator>,
    #[arg(
        short,
        long,
        value_parser = ValueParser::new(parse_buffer_size_var),
        default_value = "4KiB",

        /*
1 Kib = 1 Kibit = 128 Bytes
1 Kb  = 1 Kbit  = 125 Byte
         */
        help = "Buffer size, like:
1 KiB = 1 Ki = 1024 Bytes
1 KB  = 1 K  = 1000 Bytes
",
    )]
    buffer_size: Byte,
    #[arg(
        short,
        long,
        default_value_t = 0,
        help = "Buffer count.
0: Read and write until EOF or SIGINT.
"
    )]
    count: u64,
    #[arg(
        long,
        exclusive = true,
        value_enum,
        value_name = "SHELL",
        help = "Print shell completion script\n"
    )]
    completion: Option<Shell>,
    #[arg(short, long, exclusive = true, action = ArgAction::Version, help = "Print version")]
    version: Option<bool>,
    #[arg(short = 'V', long, global = true, help = "Verbose mode")]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();
    let cmd = &mut Cli::command();

    if let Some(shell) = cli.completion {
        generate(shell, cmd, cmd.get_name().to_string(), &mut io::stdout());
        return;
    }

    if cli.input.is_none() && cli.generator.is_none() {
        eprintln!("No input source\n");
        cmd.print_long_help().unwrap();
        exit(1);
    }

    println!(
        "Buffer size: {} Byte ({:#}, {:#})",
        cli.buffer_size.as_u128(),
        cli.buffer_size.get_appropriate_unit(UnitType::Binary),
        cli.buffer_size.get_appropriate_unit(UnitType::Decimal),
    );

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let buffer_size = cli.buffer_size.as_u128();
    let buffer_size_usize = buffer_size as usize;
    let final_size = cli.count as u128 * buffer_size;
    let mut input: Box<dyn Read> = match cli.input {
        Some(input) => Box::new(File::open(input).unwrap()),
        None => {
            let generator = cli.generator.unwrap();
            let mut generate_instant = None;
            let generate_size = if cli.count == 0 {
                buffer_size
            } else {
                final_size
            };
            let input: Box<dyn Read> = match generator {
                Generator::Text => Box::new(AsciiGenerator::new()),
                Generator::Null => Box::new(NullGenerator::new()),
                Generator::Random | Generator::RandomText => {
                    generate_instant = Some(Instant::now());
                    let b = Byte::from_u128(generate_size).unwrap();
                    println!(
                        "Generating into memory, size: {} Byte ({:#}, {:#})",
                        generate_size,
                        b.get_appropriate_unit(UnitType::Binary),
                        b.get_appropriate_unit(UnitType::Decimal),
                    );
                    let mut bytes = vec![0; generate_size as usize];
                    bytes.fill_with(|| {
                        rand::thread_rng().gen_range(match generator {
                            Generator::Random => 0u8..0xff,
                            Generator::RandomText => 0x20u8..0x7f,
                            _ => todo!(),
                        })
                    });
                    Box::new(MemoryGenerator::new(bytes, cli.count == 0))
                }
            };
            if let Some(instant) = generate_instant {
                let duration = instant.elapsed();
                println!("Generation duration: {:?}", duration);
                println!(
                    "Generation speed: {}",
                    get_io_speed(generate_size, duration.as_nanos())
                );
            }
            input
        }
    };

    let mut output = cli.output.map(|s| File::create(s).unwrap());
    let mut buffer = vec![0u8; buffer_size_usize];
    let mut count = 0usize;
    let mut size = 0u128;
    let instant = Instant::now();
    loop {
        let s = input.read(&mut buffer).unwrap();
        if s == 0 {
            break;
        }
        if let Some(ref mut output) = output {
            output.write_all(&buffer).unwrap();
        }
        count += 1;
        size += s as u128;
        if !running.load(Ordering::SeqCst) {
            break;
        }
        if cli.count > 0 {
            let s = final_size - size;
            if s < buffer_size {
                buffer = Vec::from(&buffer[0..s as usize]);
            }
        }
    }
    let duration = instant.elapsed();
    println!("RW duration: {duration:?}");
    let b = Byte::from_u128(size).unwrap();
    println!("RW count: {count}");
    println!(
        "RW size: {size} Byte ({:#}, {:#})",
        b.get_appropriate_unit(UnitType::Binary),
        b.get_appropriate_unit(UnitType::Decimal),
    );
    println!(
        "RW speed: {}",
        get_io_speed(b.as_u128(), duration.as_nanos())
    );
}

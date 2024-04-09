use std::{
    fs::OpenOptions,
    io::{self, Read, Write},
    str::FromStr,
    time::Instant,
};

use byte_unit::{Bit, Byte, UnitType};
use clap::{
    builder::ValueParser, error::Result, ArgAction, Args, CommandFactory, Parser, Subcommand,
    ValueEnum,
};
use clap_complete::{generate, Shell};

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
    println!(
        "Buffer size: {size} Byte ({:#}, {:#})",
        b.get_appropriate_unit(UnitType::Binary),
        b.get_appropriate_unit(UnitType::Decimal),
    );
    Ok(b)
}

#[derive(Args, Debug, PartialEq)]
struct CommonArgs {
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
"
    )]
    buffer_size: Byte,
    #[arg(short, long, default_value_t = 0)]
    count: u64,
}

impl CommonArgs {
    fn create_buffer(&self) -> Vec<u8> {
        vec![0; self.buffer_size.as_u128() as usize]
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Copy, Clone, ValueEnum)]
enum WriteSrc {
    #[default]
    Ascii,
    Random,
    Null,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Commands {
    #[command(about = "Read file to memory")]
    Read {
        #[arg(help = "Input file")]
        file: String,

        #[command(flatten)]
        common: CommonArgs,
    },
    #[command(about = "Write to file. First generate it into memory")]
    Write {
        #[arg(help = "Output file")]
        file: String,
        #[arg(short, long, value_enum)]
        src: WriteSrc,

        #[command(flatten)]
        common: CommonArgs,
    },
    #[command(about = "Copy file")]
    Copy {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,

        #[command(flatten)]
        common: CommonArgs,
    },
    #[command(about = "Print shell completion script")]
    Completion {
        #[arg(value_name = "SHELL", value_enum)]
        shell: Shell,
    },
}

#[derive(Parser, Debug, PartialEq)]
#[command(
    version,
    about,
    long_about,
    next_line_help = true,
    disable_version_flag = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, action = ArgAction::Version, help = "Print version")]
    version: Option<bool>,
    #[arg(short = 'V', long, global = true, help = "Verbose mode")]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Completion { shell } => {
            let cmd = &mut Cli::command();
            generate(*shell, cmd, cmd.get_name().to_string(), &mut io::stdout());
        }
        Commands::Read { file, common } => {
            let mut buffer = common.create_buffer();
            let mut file = OpenOptions::new().read(true).open(file).unwrap();
            let mut size = 0u128;
            let instant = Instant::now();
            loop {
                let s = file.read(buffer.by_ref()).unwrap();
                if s == 0 {
                    break;
                }
                size += s as u128;
            }
            let duration = instant.elapsed();
            println!("Duration: {duration:?}");
            let b = Byte::from_u128(size).unwrap();
            println!(
                "Total size: {size} Byte ({:#}, {:#})",
                b.get_appropriate_unit(UnitType::Binary),
                b.get_appropriate_unit(UnitType::Decimal),
            );
            let b = b.as_u128() * 1_000_000_000 / duration.as_nanos();
            let bit = Bit::from_u128(b * 8).unwrap();
            let b = Byte::from_u128(b).unwrap();
            println!(
                "Speed: {:#.2}/s, {:#.2}/s, {:#.2}/s, {:#.2}/s",
                b.get_appropriate_unit(UnitType::Binary),
                b.get_appropriate_unit(UnitType::Decimal),
                bit.get_appropriate_unit(UnitType::Binary),
                bit.get_appropriate_unit(UnitType::Decimal),
            );
        }
        Commands::Write { file, src, common } => {
            todo!()
        }
        Commands::Copy {
            input,
            output,
            common,
        } => {
            todo!()
        }
    }
}

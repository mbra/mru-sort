use anyhow::{ anyhow, Result, Context };
use clap::{Args, Parser, Subcommand};
use serde_json;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // Record item usage timestamp
    Use(UseArgs),
    // Sort items on stdin by recent usage
    Sort(SortArgs),
}

#[derive(Args)]
struct UseArgs {
    realm: String,
    value: String,
}

#[derive(Args)]
struct SortArgs {
    realm: String,

    #[arg(short, long)]
    reverse: bool,

    #[arg(short, long)]
    unique: bool,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
struct Sortable {
    score: u64,
    val: String,
}

type MruDb = std::collections::HashMap<String, u64>;

fn get_mru(path: &dyn AsRef<std::path::Path>) -> Result<MruDb, anyhow::Error> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(MruDb::new());
            }
            return Err(anyhow!("unable to open mru db"));
        }
    };
    let reader = std::io::BufReader::new(file);
    let db: MruDb = serde_json::from_reader(reader)
        .context("mru parsing failed")?;
    return Ok(db);
}

fn store_mru(path: &dyn AsRef<std::path::Path>, mru: &MruDb) -> Result<(), anyhow::Error> {
    let writer = std::io::BufWriter::new(
        std::fs::File::create(path)
            .context("Failed opening mru db: {confpath}")?,
    );
    serde_json::to_writer(writer, &mru)
        .context("Error storing new mru db")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let xdg_dirs = xdg::BaseDirectories::with_prefix("mru-sort")?;


    match &cli.command {
        Commands::Use(args) => {
            let confpath = xdg_dirs.place_data_file(&args.realm).context("resolve db path")?;
            let mut mru = get_mru(&confpath).context("Retrieve mru info")?;
            mru.insert(
                args.value.clone(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs(),
            );
            store_mru(&confpath, &mru)
        }
        Commands::Sort(args) => {
            let confpath = xdg_dirs.place_data_file(&args.realm).context("resolve db path")?;
            let mru = get_mru(&confpath).context("Retrieve mru info")?;
            let mut lines: std::vec::Vec<Sortable> = std::io::stdin()
                .lines()
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .map(|i| { Sortable { score: *mru.get(i).unwrap_or(&0), val: i.clone()}})
                .collect();

            lines.sort();

            if args.unique {
                lines.dedup();
            }

            if args.reverse {
                lines.reverse();
            }

            for line in lines {
                println!("{}", line.val);
            }
            Ok(())
        },
    }
}

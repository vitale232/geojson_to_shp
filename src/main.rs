use std::{env, process};

use geojson_to_shp::Cli;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut cli = Cli::new(&args).unwrap_or_else(|err| {
        eprintln!("A problem occurred while parsing the args: {}", err);
        process::exit(1);
    });

    let mut writer = cli.to_writer().unwrap_or_else(|err| {
        eprintln!("An error occurred while creating the Writer: {:?}", err);
        process::exit(1);
    });

    writer.write().unwrap_or_else(|err| {
        eprintln!("An error occurred while writing the shapefile: {:?}", err);
        process::exit(1);
    });
}

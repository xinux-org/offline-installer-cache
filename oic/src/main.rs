pub mod iterwrite;
pub mod parse;

use std::fs;

use anyhow::{Result, anyhow};
use clap::Parser;

use crate::iterwrite::MakeConfig;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    base_dir: String,

    #[arg(short, long)]
    out_dir: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Hello {}!", args.base_dir);

    let config = parse::parse_config(&args.base_dir)?;

    if fs::exists(&args.out_dir).unwrap_or(false) {
        fs::remove_dir_all(&args.out_dir).map_err(|err| {
            anyhow!(
                "failed to clean output directory: {}. {}",
                args.out_dir,
                &err
            )
        })?;
    }

    fs::create_dir_all(&args.out_dir).map_err(|err| {
        anyhow!(
            "failed to create output directory: {}. {}",
            args.out_dir,
            &err
        )
    })?;

    // println!("{config:#?}");

    println!("----------------");
    let variants: Vec<(String, MakeConfig)> = config
        .choices
        .into_iter()
        .enumerate()
        .flat_map(|(choices_index, choice)| match choice {
            parse::ChoiceEnum::Configuration { file: _, config } => config
                .steps
                .into_iter()
                .enumerate()
                .flat_map(|(step_index, step)| match step {
                    parse::StepType::List {
                        id: _,
                        multiple: _,
                        required: _,
                        title: _,
                        choices,
                    } => choices
                        .into_iter()
                        .enumerate()
                        .map(|(inner_choices_index, choices)| {
                            let temp_dir = format!(
                                "{}/{choices_index}-{step_index}-{inner_choices_index}",
                                &args.out_dir
                            );

                            let config = MakeConfig::with_list(
                                vec![("".to_string(), choices)].into_iter().collect(),
                            );

                            (temp_dir, config)
                        })
                        .collect(),
                    _ => vec![],
                })
                .collect(),
            parse::ChoiceEnum::Live => vec![],
        })
        .collect();

    // println!("{variants:?}");

    variants.iter().for_each(|(temp_dir, config)| {
        if let Err(err) = iterwrite::iterwrite(
            &args.base_dir,
            temp_dir,
            config,
            temp_dir,
            true,
            "x86_64-linux",
        ) {
            eprintln!("{err}");
        }
    });

    Ok(())
}

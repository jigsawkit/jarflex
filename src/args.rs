use std::io;
use clap::{arg, Command};
use derive_getters::Getters;
use crate::build;

const DEF_TARGET_PATH: &str = "./target/";

#[derive(Getters)]
pub(crate) struct Args {
    jar_file: String,
    output: String,
    s_name: String,
    t_name: String,
}

pub(crate) fn get_args() -> io::Result<Args> {
    let matches = Command::new(build::PROJECT_NAME)
        .version(shadow_rs::formatcp!(
            "{ver}\n{target} (build {time})",
            ver = build::PKG_VERSION,
            target = build::BUILD_TARGET,
            time = build::BUILD_TIME_3339
        ))
        .arg_required_else_help(true)
        .args(&[
            arg!(-s <sname> "Source package name").id("source").required(false),
            arg!(-t <tname> "Target package name").id("target").required(false),
            arg!(-o <output> "Path to store generated files").required(false),
            arg!(<jarfile> "JAR files that need to be operated").required(true).num_args(1),
        ])
        .get_matches();
    // JAR 文件
    let jar_file = matches.get_one::<String>("jarfile").unwrap();

    // package
    let source_name = matches.get_one::<String>("source").unwrap();
    let source_name = source_name.replace(".", "/");

    let target_name = matches.get_one::<String>("target").unwrap();
    let target_name = target_name.replace(".", "/");

    // 输出路径
    let output = matches.get_one::<String>("output").unwrap();
    let output = matches.get_one::<String>("output");
    let output = match output {
        None => DEF_TARGET_PATH,
        Some(str) => str
    };

    Ok(Args {
        jar_file: jar_file.to_string(),
        output: output.to_string(),
        s_name: source_name.to_string(),
        t_name: target_name.to_string(),
    })
}


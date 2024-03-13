use std::fs::File;
use std::{fs, io};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{exit};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use clap::{arg, Command};
use exitcode::{DATAERR, IOERR, OK, USAGE};
use shadow_rs::{new, shadow};
use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;

shadow!(build);

fn main() {
    let matches = Command::new(build::PROJECT_NAME)
        .version(shadow_rs::formatcp!(
            "{ver}\n{target} (build {time})",
            ver = build::PKG_VERSION,
            target = build::BUILD_TARGET,
            time = build::BUILD_TIME_3339
        ))
        .arg_required_else_help(true)
        .args(&[
            arg!(-s <sname> "源目录名").id("source").required(false),
            arg!(-t <tname> "目标目录名").id("target").required(false),
            arg!(-o <output> "输出文件").required(false),
            arg!(<jarfile> "JAR文件").required(true).num_args(1),
        ])
        .get_matches();

    let source_name = matches.get_one::<String>("source").unwrap();
    let target_name = matches.get_one::<String>("target").unwrap();

    let jar_file = matches.get_one::<String>("jarfile").unwrap();
    if !jar_file.ends_with(".jar") {
        println!("[ERROR] {}不是有效的JAR文件", jar_file);
        exit(USAGE)
    }

    let jar_path = Path::new(jar_file);
    let file_name = jar_path.file_name().unwrap();
    let file_name = file_name.to_str().unwrap();

    let default_target_path = "./target/";
    let output = matches.get_one::<String>("output");
    let output = match output {
        None => default_target_path,
        Some(str) => str
    };

    fs::create_dir_all(output).unwrap();
    println!("{}", output);
    let output = output.to_string() + file_name;
    let new_jar_file = File::create(output).unwrap();

    let mut new_jar_file = ZipWriter::new(new_jar_file);

    let jar_file = File::open(jar_path);
    if jar_file.is_err() {
        println!("[ERROR] 读取{}文件失败\n{}", jar_path.to_str().unwrap(), jar_file.err().unwrap());
        exit(IOERR)
    }
    let jar_file = jar_file.unwrap();
    let mut zip = ZipArchive::new(jar_file);
    if zip.is_err() {
        println!("[ERROR] 解压缩{}文件失败\n{}", jar_path.to_str().unwrap(), zip.err().unwrap());
        exit(IOERR)
    }
    let mut zip = zip.unwrap();

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).expect("[ERROR] Failed to read entry for jar file");

        let mut bytecode = Vec::new();
        entry.read_to_end(&mut bytecode).expect("[ERROR] Failed to read bytecode");

        println!("\t file: {}", entry.name());
        if !entry.name().ends_with(".class") {
            let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            let _ = new_jar_file.start_file(entry.name(), options);

            // 将修改后的字节码写入新的压缩包文件
            let _ = new_jar_file.write_all(&bytecode);
            continue;
        }

        // println!("file name: {}", entry.name());

        let result = rename(&bytecode, source_name, target_name);
        if result.is_err() {
            println!("[ERROR] Failed to rename {} -> {}", source_name, target_name);
            println!("{}", result.err().unwrap());
            exit(DATAERR)
        }

        let mut new_file_name = entry.name().to_string();
        if new_file_name.starts_with(source_name) {
            new_file_name = new_file_name.replace(source_name, target_name);
        }

        let bytecode = result.unwrap();
        // 创建一个新的文件项
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let _ = new_jar_file.start_file(new_file_name, options);

        // 将修改后的字节码写入新的压缩包文件
        let _ = new_jar_file.write_all(&bytecode);
    }

    new_jar_file.finish().expect("压缩JAR文件失败");

    println!("[INFO] JAR upgrade success.");

    exit(OK)
}

fn rename(bytecode: &Vec<u8>, source: &String, target: &String) -> io::Result<Vec<u8>> {
    let mut reader = std::io::Cursor::new(bytecode);
    let magic = reader.read_u32::<BigEndian>()?;
    let minor_version = reader.read_u16::<BigEndian>()?;
    let major_version = reader.read_u16::<BigEndian>()?;
    // println!("magic: {:X}", magic);

    // 读取常量池计数器
    let constant_pool_count = reader.read_u16::<BigEndian>()?;

    // 创建一个新的字节码缓冲区
    let mut modified_bytecode = vec![];
    modified_bytecode.write_u32::<BigEndian>(magic)?;
    modified_bytecode.write_u16::<BigEndian>(minor_version)?;
    modified_bytecode.write_u16::<BigEndian>(major_version)?;
    modified_bytecode.write_u16::<BigEndian>(constant_pool_count)?;

    // 复制常量池
    for i in 1..constant_pool_count {
        let tag = reader.read_u8()?;
        modified_bytecode.write_u8(tag)?;
        // println!("i: {}, tag: {}", i, tag);
        match tag {
            // UTF-8 常量
            1 => {
                let length = reader.read_u16::<BigEndian>()?;
                // println!("length: {}", length);
                let mut bytes = vec![0; length as usize];
                reader.read_exact(&mut bytes)?;
                // println!("byte len: {}", bytes.len());
                let value = String::from_utf8_lossy(&bytes);
                // println!("val: {:?}", value.clone());

                // 替换包名
                let new_value = value.replace(source, target);
                // println!("new val: {:?}", new_value.clone());
                let new_length = new_value.len() as u16;
                modified_bytecode.write_u16::<BigEndian>(new_length)?;
                modified_bytecode.write_all(new_value.as_bytes())?;
            }
            // 其他常量类型，直接复制到新的字节码中
            _ => {
                let length = match tag {
                    5 | 6 => 8,   // Long 或 Double 常量
                    3 | 4 | 9..=12 | 14 | 17..=18 => 4,
                    15 => 3,
                    _ => 2,   // 其他常量类型
                };
                // modified_bytecode.write_u16::<BigEndian>(length)?;
                let mut bytes = vec![0; length as usize];
                reader.read_exact(&mut bytes)?;
                modified_bytecode.write_all(&bytes)?;
            }
        }
    }

    // 复制剩余的字节码内容
    io::copy(&mut reader, &mut modified_bytecode)?;

    Ok(modified_bytecode)
}
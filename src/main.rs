mod args;

use std::fs::File;
use std::{fs, io};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{exit};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use exitcode::{DATAERR, IOERR, OK, USAGE};
use shadow_rs::{shadow};
use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;

shadow!(build);

fn main() {

    let args = args::get_args();
    if args.is_err() {
        exit(USAGE)
    }
    let args = args.unwrap();

    // 校验
    println!("[INF] checking files ...");
    let jar_file = args.jar_file();
    if !jar_file.ends_with(".jar") {
        println!("[ERR] '{}' is not a valid JAR file.", jar_file);
        exit(USAGE)
    }

    // JAR文件
    let jar_path = Path::new(jar_file);
    let jar_file_name = jar_path.file_name().unwrap();
    let jar_file_name = jar_file_name.to_str().unwrap();

    // 输出目录，不存在时创建
    let output = args.output();
    fs::create_dir_all(output).unwrap();

    // 输出文件：输出目录+文件名
    let output = output.to_string() + jar_file_name;
    let new_jar_file = File::create(output).unwrap();
    let mut new_jar_file = ZipWriter::new(new_jar_file);

    // 替换的package
    let source_name = args.s_name();
    let target_name = args.t_name();

    // 解析JAR文件
    let jar_file = File::open(jar_path);
    if jar_file.is_err() {
        println!("[ERR] Failed to read '{}' file.\n\t{}", jar_path.to_str().unwrap(), jar_file.err().unwrap());
        exit(IOERR)
    }
    let jar_file = jar_file.unwrap();
    let mut zip = ZipArchive::new(jar_file);
    if zip.is_err() {
        println!("[ERR] Failed to read '{}' file.\n\t{}", jar_path.to_str().unwrap(), zip.err().unwrap());
        exit(IOERR)
    }
    let mut zip = zip.unwrap();

    println!("[INF] Start operation");
    for i in 0..zip.len() {
        let entry = zip.by_index(i);
        if entry.is_err() {
            println!("[ERR] Failed to read entry for jar file.\n\t{}", entry.err().unwrap());
            exit(IOERR)
        }
        let mut entry = entry.unwrap();

        let mut bytecode = Vec::new();
        if let Err(err) = entry.read_to_end(&mut bytecode) {
            println!("[ERR] Failed to read bytecode.\n\t{}", err.to_string());
            exit(IOERR)
        };

        println!("\t - {}", entry.name());
        let mut new_file_name = entry.name().to_string();
        if new_file_name.starts_with(source_name) {
            new_file_name = new_file_name.replace(source_name, target_name);
            println!("\t *\t -> {}", new_file_name.clone());
        }

        if !entry.name().ends_with(".class") {
            let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            let _ = new_jar_file.start_file(new_file_name, options);

            // 将修改后的字节码写入新的压缩包文件
            let _ = new_jar_file.write_all(&bytecode);
            continue;
        }

        // let flag = new_file_name.ends_with("TBaseType.class");
        let flag = false;
        let result = rename(&bytecode, source_name, target_name, flag);
        if result.is_err() {
            println!("[ERR] Failed to rename {} -> {}", source_name, target_name);
            println!("\t{}", result.err().unwrap());
            exit(DATAERR)
        }

        let bytecode = result.unwrap();
        // 创建一个新的文件项
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let _ = new_jar_file.start_file(new_file_name, options);

        // 将修改后的字节码写入新的压缩包文件
        let _ = new_jar_file.write_all(&bytecode);
    }

    println!("[INF] Compressing JAR file ...");
    if let Err(err) = new_jar_file.finish() {
        println!("[ERR] Compressing JAR file failed.\n\t{}", err.to_string());
        exit(IOERR)
    };

    println!("[INF] JAR upgrade success.");

    exit(OK)
}

fn rename(bytecode: &Vec<u8>, source: &String, target: &String, flag: bool) -> io::Result<Vec<u8>> {
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
        if flag {
            println!("i: {}, tag: {}", i, tag);
        }
        match tag {
            // UTF-8 常量
            1 => {
                let length = reader.read_u16::<BigEndian>()?;
                if flag {
                    println!("length: {}", length);
                }
                let mut bytes = vec![0; length as usize];
                reader.read_exact(&mut bytes)?;
                let value = String::from_utf8_lossy(&bytes);
                if flag {
                    println!("val: {:?}", value.clone());
                }

                // 替换包名
                let new_value = value.replace(source, target);
                if flag {
                    println!("new val: {:?}", new_value.clone());
                }
                let new_length = new_value.len() as u16;
                modified_bytecode.write_u16::<BigEndian>(new_length)?;
                modified_bytecode.write_all(new_value.as_bytes())?;
            }
            // 其他常量类型，直接复制到新的字节码中
            _ => {
                let length = match tag {
                    5 | 6 => 8,   // Long 或 Double 常量
                    19 => 5,
                    3 | 4 | 9..=12 | 14 | 17 | 18 => 4,
                    15 => 3,
                    0 => 0,
                    _ => 2,   // 其他常量类型
                };
                if length != 0 {
                    // modified_bytecode.write_u16::<BigEndian>(length)?;
                    let mut bytes = vec![0; length as usize];
                    reader.read_exact(&mut bytes)?;
                    modified_bytecode.write_all(&bytes)?;
                }
            }
        }
    }

    // 复制剩余的字节码内容
    io::copy(&mut reader, &mut modified_bytecode)?;

    Ok(modified_bytecode)
}
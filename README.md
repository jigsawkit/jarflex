<div align="center">

# JarFlex

_✨ A tool designed for modifying JAR files ✨_

</div>

## Introduction 

JarFlex is an open-source, out-of-the-box tool for modifying JAR files. 
Suitable for the need to change the class in the jar file when there is no source code, such as renaming package and class names, reducing or adding class files, encryption jars, etc.

## Usage

Use the **-h** or **--help** parameter to view detailed usage documentation.
For example:

```shell
$ jarflex -h

Usage: jarflex.exe [OPTIONS] <jarfile>

Arguments:
  <jarfile>  JAR files that need to be operated

Options:
  -s <sname>       Source package name
  -t <tname>       Target package name
  -o <output>      Path to store generated files
  -h, --help       Print help
  -V, --version    Print version
```

## Build

```shell
$ cd ./jarflex

$ cargo build --release
```

### cross compile

```shell
$ rustup target install x86_64-unknown-linux-gnu
```

If you need to build packages for other system platforms, it is recommended to use cross.

```shell
$ cargo install cross
```

Then, specify **--target** to complete packaging

```shell
$ cross build --target x86_64-unknown-linux-gnu 
```